import ast
import os
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Any, Set, Tuple
from .base import BaseAnalyzer

@dataclass
class ImportInfo:
    """Information about an import statement"""
    module: str
    name: Optional[str] = None  # For 'from x import y'
    alias: Optional[str] = None
    line: int = 0

@dataclass
class FunctionInfo:
    """Information about a function definition"""
    name: str
    line: int
    end_line: int
    args: List[str] = field(default_factory=list)
    returns: Optional[str] = None
    decorators: List[str] = field(default_factory=list)
    is_async: bool = False
    is_method: bool = False
    docstring: Optional[str] = None
    calls: Set[str] = field(default_factory=set)

@dataclass
class ClassInfo:
    """Information about a class definition"""
    name: str
    line: int
    end_line: int
    bases: List[str] = field(default_factory=list)
    decorators: List[str] = field(default_factory=list)
    docstring: Optional[str] = None
    methods: Dict[str, FunctionInfo] = field(default_factory=dict)

class _ASTVisitor(ast.NodeVisitor):
    """Internal AST visitor for extracting structure"""

    def __init__(self) -> None:
        self.imports: List[ImportInfo] = []
        self.functions: Dict[str, FunctionInfo] = {}
        self.classes: Dict[str, ClassInfo] = {}
        self.globals: Set[str] = set()
        self._current_class: Optional[ClassInfo] = None

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            self.imports.append(ImportInfo(
                module=alias.name,
                alias=alias.asname,
                line=node.lineno,
            ))
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        module = node.module or ""
        for alias in node.names:
            self.imports.append(ImportInfo(
                module=module,
                name=alias.name,
                alias=alias.asname,
                line=node.lineno,
            ))
        self.generic_visit(node)

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._process_function(node, False)
        self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self._process_function(node, True)
        self.generic_visit(node)

    def _process_function(self, node: ast.FunctionDef | ast.AsyncFunctionDef, is_async: bool) -> None:
        """Extract function information"""
        # Get decorators
        decorators = [d.id if isinstance(d, ast.Name) else ast.unparse(d) for d in node.decorator_list]

        # Get arguments
        args = []
        for arg in node.args.args:
            arg_str = arg.arg
            if arg.annotation:
                arg_str += f": {ast.unparse(arg.annotation)}"
            args.append(arg_str)

        # Get return type
        returns = ast.unparse(node.returns) if node.returns else None

        # Get docstring
        docstring = ast.get_docstring(node)

        func_info = FunctionInfo(
            name=node.name,
            line=node.lineno,
            end_line=node.end_lineno or node.lineno,
            args=args,
            returns=returns,
            decorators=decorators,
            is_async=is_async,
            is_method=self._current_class is not None,
            docstring=docstring,
        )

        # Extract function calls
        for child in ast.walk(node):
            if isinstance(child, ast.Call):
                if isinstance(child.func, ast.Name):
                    func_info.calls.add(child.func.id)
                elif isinstance(child.func, ast.Attribute):
                    # Try to capture full name (e.g. module.func)
                    if isinstance(child.func.value, ast.Name):
                        func_info.calls.add(f"{child.func.value.id}.{child.func.attr}")
                    else:
                        func_info.calls.add(child.func.attr)

        if self._current_class:
            self._current_class.methods[node.name] = func_info
        else:
            self.functions[node.name] = func_info

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        # Get decorators
        decorators = [d.id if isinstance(d, ast.Name) else ast.unparse(d) for d in node.decorator_list]

        # Get base classes
        bases = [ast.unparse(base) for base in node.bases]

        # Get docstring
        docstring = ast.get_docstring(node)

        class_info = ClassInfo(
            name=node.name,
            line=node.lineno,
            end_line=node.end_lineno or node.lineno,
            bases=bases,
            decorators=decorators,
            docstring=docstring,
        )

        # Save previous class and set current
        prev_class = self._current_class
        self._current_class = class_info
        self.classes[node.name] = class_info

        # Visit class body
        self.generic_visit(node)

        # Restore previous class
        self._current_class = prev_class

    def visit_Assign(self, node: ast.Assign) -> None:
        # Track global assignments
        if not self._current_class:
            for target in node.targets:
                if isinstance(target, ast.Name):
                    self.globals.add(target.id)
        self.generic_visit(node)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        # Track annotated global assignments
        if not self._current_class and isinstance(node.target, ast.Name):
            self.globals.add(node.target.id)
        self.generic_visit(node)

class ASTAnalyzer(BaseAnalyzer):
    """
    AST Analyzer for extracting code structure.
    Implements Layer 1 analysis: function signatures, imports, class definitions.
    """

    def __init__(self, max_file_size: int = 1048576):
        self.max_file_size = max_file_size

    def analyze(self, code: str, file_path: str) -> Dict[str, Any]:
        """
        Analyze Python source code string.
        """
        try:
            tree = ast.parse(code)
        except SyntaxError:
            return {
                "path": file_path,
                "error": "SyntaxError",
                "language": "python",
                "line_count": code.count("\n") + 1
            }

        line_count = code.count("\n") + 1

        # Extract module docstring
        docstring = None
        if (tree.body and isinstance(tree.body[0], ast.Expr) and
                isinstance(tree.body[0].value, ast.Constant)):
            docstring = tree.body[0].value.value

        # Visit AST
        visitor = _ASTVisitor()
        visitor.visit(tree)

        # Convert data classes to dicts for serialization
        return {
            "path": file_path,
            "language": "python",
            "line_count": line_count,
            "docstring": docstring,
            "imports": [asdict(i) for i in visitor.imports],
            "functions": {k: self._func_to_dict(v) for k, v in visitor.functions.items()},
            "classes": {k: self._class_to_dict(v) for k, v in visitor.classes.items()},
            "globals": list(visitor.globals)
        }

    def _func_to_dict(self, func: FunctionInfo) -> Dict[str, Any]:
        d = asdict(func)
        d['calls'] = list(d['calls']) # Convert set to list
        return d

    def _class_to_dict(self, cls: ClassInfo) -> Dict[str, Any]:
        d = asdict(cls)
        # methods are already converted to dicts by asdict recursion
        # we just need to fix the calls set -> list in them
        for method in d['methods'].values():
            if 'calls' in method and isinstance(method['calls'], set):
                method['calls'] = list(method['calls'])
        return d

    def to_llm_string(self, analysis_result: Dict[str, Any]) -> str:
        """
        Convert analysis results to a token-efficient string representation.
        """
        if "error" in analysis_result:
            return f"File: {analysis_result['path']}\nError: {analysis_result['error']}"

        lines = [f"File: {analysis_result['path']} ({analysis_result['line_count']} lines)"]

        if analysis_result.get("docstring"):
            lines.append(f'"""{analysis_result["docstring"][:100]}..."""')

        # Imports
        imports = analysis_result.get("imports", [])
        if imports:
            lines.append("\n# Imports")
            for imp in imports[:20]:  # Limit output
                if imp["name"]:
                    lines.append(f"from {imp['module']} import {imp['name']}")
                else:
                    lines.append(f"import {imp['module']}")

        # Classes
        classes = analysis_result.get("classes", {})
        if classes:
            lines.append("\n# Classes")
            for name, cls in sorted(classes.items()):
                bases = f"({', '.join(cls['bases'])})" if cls['bases'] else ""
                lines.append(f"class {name}{bases}:")
                if cls.get("docstring"):
                    lines.append(f'    """{cls["docstring"][:80]}"""')

                methods = cls.get("methods", {})
                for method_name, method in sorted(methods.items()):
                    async_prefix = "async " if method["is_async"] else ""
                    args = ", ".join(method["args"])
                    lines.append(f"    {async_prefix}def {method_name}({args})")

        # Functions
        functions = analysis_result.get("functions", {})
        if functions:
            lines.append("\n# Functions")
            for name, func in sorted(functions.items()):
                async_prefix = "async " if func["is_async"] else ""
                args = ", ".join(func["args"])
                returns = f" -> {func['returns']}" if func['returns'] else ""
                lines.append(f"{async_prefix}def {name}({args}){returns}")

        return "\n".join(lines)
