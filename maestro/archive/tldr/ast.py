"""
AST (Abstract Syntax Tree) Analysis Layer

Layer 1 of TLDR analysis. Extracts function signatures, imports,
class definitions, and code structure without implementation details.
Achieves ~500 token representation vs 10,000+ token raw file.
"""

import ast
import os
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any, Set, Tuple
from pathlib import Path


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


@dataclass
class FileAnalysis:
    """Complete AST analysis of a file"""
    path: str
    language: str
    imports: List[ImportInfo] = field(default_factory=list)
    functions: Dict[str, FunctionInfo] = field(default_factory=dict)
    classes: Dict[str, ClassInfo] = field(default_factory=dict)
    globals_: Set[str] = field(default_factory=set)
    docstring: Optional[str] = None
    line_count: int = 0


class ASTAnalyzer:
    """
    AST Analyzer for extracting code structure

    Provides Layer 1 analysis: function signatures, imports, class definitions
    without implementation details. Achieves ~95% token savings compared to
    reading raw files.
    """

    def __init__(self, max_file_size: int = 1048576):
        """
        Initialize the AST analyzer

        Args:
            max_file_size: Maximum file size to analyze (default 1MB)
        """
        self.max_file_size = max_file_size

    def analyze_file(self, path: str) -> Optional[FileAnalysis]:
        """
        Analyze a Python file and extract its structure

        Args:
            path: Path to the file

        Returns:
            FileAnalysis or None if analysis fails
        """
        path = os.path.abspath(path)

        # Check file exists and size
        if not os.path.isfile(path):
            return None

        file_size = os.path.getsize(path)
        if file_size > self.max_file_size:
            return None

        # Read file
        try:
            with open(path, "r", encoding="utf-8") as f:
                source = f.read()
        except Exception:
            return None

        return self.analyze_source(source, path)

    def analyze_source(self, source: str, path: str = "<source>") -> Optional[FileAnalysis]:
        """
        Analyze Python source code string

        Args:
            source: Python source code
            path: Optional file path for reference

        Returns:
            FileAnalysis or None if parsing fails
        """
        try:
            tree = ast.parse(source)
        except SyntaxError:
            return None

        line_count = source.count("\n") + 1

        analysis = FileAnalysis(
            path=path,
            language="python",
            line_count=line_count,
        )

        # Extract module docstring
        if (tree.body and isinstance(tree.body[0], ast.Expr) and
                isinstance(tree.body[0].value, ast.Constant)):
            analysis.docstring = tree.body[0].value.value

        # Visit AST
        visitor = _ASTVisitor()
        visitor.visit(tree)

        analysis.imports = visitor.imports
        analysis.functions = visitor.functions
        analysis.classes = visitor.classes
        analysis.globals_ = visitor.globals

        return analysis

    def extract_function_signature(self, path: str, function_name: str) -> Optional[Dict[str, Any]]:
        """
        Extract a specific function's signature

        Args:
            path: Path to the file
            function_name: Name of the function

        Returns:
            Dictionary with function signature info
        """
        analysis = self.analyze_file(path)
        if not analysis:
            return None

        if function_name in analysis.functions:
            func = analysis.functions[function_name]
            return {
                "name": func.name,
                "line": func.line,
                "args": func.args,
                "returns": func.returns,
                "decorators": func.decorators,
                "is_async": func.is_async,
                "is_method": func.is_method,
                "docstring": func.docstring,
            }

        # Check methods in classes
        for cls in analysis.classes.values():
            if function_name in cls.methods:
                method = cls.methods[function_name]
                return {
                    "name": method.name,
                    "class": cls.name,
                    "line": method.line,
                    "args": method.args,
                    "returns": method.returns,
                    "decorators": method.decorators,
                    "is_async": method.is_async,
                    "is_method": True,
                    "docstring": method.docstring,
                }

        return None

    def get_imports(self, path: str) -> List[ImportInfo]:
        """
        Get all imports from a file

        Args:
            path: Path to the file

        Returns:
            List of ImportInfo objects
        """
        analysis = self.analyze_file(path)
        if not analysis:
            return []
        return analysis.imports

    def get_function_names(self, path: str) -> List[str]:
        """
        Get all function names from a file

        Args:
            path: Path to the file

        Returns:
            List of function names
        """
        analysis = self.analyze_file(path)
        if not analysis:
            return []

        names = list(analysis.functions.keys())
        for cls in analysis.classes.values():
            names.extend(cls.methods.keys())

        return sorted(names)

    def get_class_names(self, path: str) -> List[str]:
        """
        Get all class names from a file

        Args:
            path: Path to the file

        Returns:
            List of class names
        """
        analysis = self.analyze_file(path)
        if not analysis:
            return []
        return sorted(analysis.classes.keys())

    def to_llm_string(self, analysis: FileAnalysis, max_detail: bool = False) -> str:
        """
        Convert analysis to LLM-friendly string

        Args:
            analysis: FileAnalysis to convert
            max_detail: Include all details

        Returns:
            Compact string representation
        """
        lines = [f"File: {analysis.path} ({analysis.line_count} lines)"]

        if analysis.docstring:
            lines.append(f'"""{analysis.docstring[:100]}..."""')

        # Imports
        if analysis.imports:
            lines.append("\n# Imports")
            for imp in analysis.imports[:20]:  # Limit output
                if imp.name:
                    lines.append(f"from {imp.module} import {imp.name}")
                else:
                    lines.append(f"import {imp.module}")

        # Classes
        if analysis.classes:
            lines.append("\n# Classes")
            for name, cls in sorted(analysis.classes.items()):
                bases = f"({', '.join(cls.bases)})" if cls.bases else ""
                lines.append(f"class {name}{bases}:")
                if cls.docstring:
                    lines.append(f'    """{cls.docstring[:80]}"""')
                for method_name, method in sorted(cls.methods.items()):
                    async_prefix = "async " if method.is_async else ""
                    args = ", ".join(method.args)
                    lines.append(f"    {async_prefix}def {method_name}({args})")

        # Functions
        if analysis.functions:
            lines.append("\n# Functions")
            for name, func in sorted(analysis.functions.items()):
                async_prefix = "async " if func.is_async else ""
                args = ", ".join(func.args)
                returns = f" -> {func.returns}" if func.returns else ""
                lines.append(f"{async_prefix}def {name}({args}){returns}")

        return "\n".join(lines)


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


def extract_function_calls(source: str) -> Set[str]:
    """
    Extract all function calls from source code

    Args:
        source: Python source code

    Returns:
        Set of function names called
    """
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return set()

    calls = set()

    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            if isinstance(node.func, ast.Name):
                calls.add(node.func.id)
            elif isinstance(node.func, ast.Attribute):
                calls.add(node.func.attr)

    return calls
