import ast
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Set, Any
from enum import Enum
from .base import BaseAnalyzer

class VarAction(str, Enum):
    """Types of variable actions"""
    DEFINE = "define"
    READ = "read"
    MODIFY = "modify"
    DELETE = "delete"

@dataclass
class VariableAccess:
    """A single variable access event"""
    name: str
    action: VarAction
    line: int
    scope: str
    context: Optional[str] = None

@dataclass
class VariableInfo:
    """Complete information about a variable"""
    name: str
    defining_line: int
    defining_scope: str
    type_hint: Optional[str] = None
    is_parameter: bool = False
    is_global: bool = False
    is_nonlocal: bool = False
    accesses: List[VariableAccess] = field(default_factory=list)

    def get_def_count(self) -> int:
        return sum(1 for a in self.accesses if a.action == VarAction.DEFINE)

    def get_use_count(self) -> int:
        return sum(1 for a in self.accesses if a.action in (VarAction.READ, VarAction.MODIFY))

    def get_modify_count(self) -> int:
        return sum(1 for a in self.accesses if a.action == VarAction.MODIFY)

@dataclass
class DataFlowGraph:
    """Complete data flow graph for a function"""
    function_name: str
    file_path: str
    start_line: int
    end_line: int
    variables: Dict[str, VariableInfo] = field(default_factory=dict)
    parameters: List[str] = field(default_factory=list)
    returns: List[str] = field(default_factory=list)
    globals_used: Set[str] = field(default_factory=set)

class DFGAnalyzer(BaseAnalyzer):
    """
    Data Flow Graph Analyzer.
    Layer 4 analysis: variable definition and use tracking.
    """

    def analyze(self, code: str, file_path: str) -> Dict[str, Any]:
        """
        Analyze code to build DFG for all functions.
        """
        try:
            tree = ast.parse(code)
        except SyntaxError:
            return {
                "path": file_path,
                "error": "SyntaxError"
            }

        functions_dfg = {}

        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                dfg = self._analyze_function(node, file_path)
                if dfg:
                    # Convert to dict for output
                    dfg_dict = asdict(dfg)
                    # Convert sets to lists
                    dfg_dict["globals_used"] = list(dfg.globals_used)
                    functions_dfg[node.name] = dfg_dict

        return {
            "path": file_path,
            "functions": functions_dfg
        }

    def _analyze_function(self, function_node: ast.AST, file_path: str) -> Optional[DataFlowGraph]:
        dfg = DataFlowGraph(
            function_name=function_node.name,
            file_path=file_path,
            start_line=function_node.lineno,
            end_line=function_node.end_lineno or function_node.lineno,
        )

        # Extract parameters
        for arg in function_node.args.args:
            dfg.parameters.append(arg.arg)
            type_hint = ast.unparse(arg.annotation) if arg.annotation else None
            dfg.variables[arg.arg] = VariableInfo(
                name=arg.arg,
                defining_line=function_node.lineno,
                defining_scope=function_node.name,
                type_hint=type_hint,
                is_parameter=True,
            )

        # Analyze function body
        visitor = _DFGVisitor(function_node.name, dfg)
        visitor.visit(function_node)

        # Find return values
        for node in ast.walk(function_node):
            if isinstance(node, ast.Return) and node.value:
                return_vars = self._extract_return_vars(node.value, dfg)
                dfg.returns.extend(return_vars)

        return dfg

    def _extract_return_vars(self, return_node: ast.AST, dfg: DataFlowGraph) -> List[str]:
        vars_found = []
        if isinstance(return_node, ast.Name):
            if return_node.id in dfg.variables:
                vars_found.append(return_node.id)
        elif isinstance(return_node, ast.Tuple):
            for elt in return_node.elts:
                if isinstance(elt, ast.Name) and elt.id in dfg.variables:
                    vars_found.append(elt.id)
        return vars_found

    def to_llm_string(self, analysis_result: Dict[str, Any]) -> str:
        lines = [f"File: {analysis_result['path']}"]

        for func_name, dfg in analysis_result.get("functions", {}).items():
            lines.append(f"\nFunction: {func_name}")
            lines.append(f"  Parameters: {', '.join(dfg['parameters'])}")

            if dfg['returns']:
                lines.append(f"  Returns: {', '.join(dfg['returns'])}")

            lines.append("  Variables:")
            # Filter important vars (params or used > 1)
            for var_name, var_info in dfg['variables'].items():
                use_count = sum(1 for a in var_info['accesses'] if a['action'] in ('read', 'modify'))
                if var_info['is_parameter'] or use_count > 0:
                     lines.append(f"    {var_name} - defined:{var_info['defining_line']}, uses:{use_count}")

        return "\n".join(lines)

class _DFGVisitor(ast.NodeVisitor):
    """AST visitor for building data flow graph"""

    def __init__(self, scope: str, dfg: DataFlowGraph):
        self.scope = scope
        self.dfg = dfg
        self.current_context = None

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        if node.name == self.scope:
            self.generic_visit(node)
        else:
            pass

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        if node.name == self.scope:
            self.generic_visit(node)
        else:
            pass

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        pass

    def visit_Name(self, node: ast.Name) -> None:
        var_name = node.id
        var_info = self.dfg.variables.get(var_name)

        if isinstance(node.ctx, ast.Load):
            action = VarAction.READ
        elif isinstance(node.ctx, ast.Store):
            # If variable exists, it's a modify/reassignment. If not, it's a define.
            # But wait, python variables are defined by assignment.
            # If it's the first time we see it in this scope, it's a define.
            action = VarAction.MODIFY if var_info else VarAction.DEFINE
        else:
            action = VarAction.READ

        if not var_info:
            var_info = VariableInfo(
                name=var_name,
                defining_line=node.lineno if action == VarAction.DEFINE else 0,
                defining_scope=self.scope,
            )
            self.dfg.variables[var_name] = var_info

        var_info.accesses.append(VariableAccess(
            name=var_name,
            action=action,
            line=node.lineno,
            scope=self.scope,
            context=self.current_context
        ))

        self.generic_visit(node)

    def visit_Assign(self, node: ast.Assign) -> None:
        old_context = self.current_context
        self.current_context = ast.unparse(node)[:100]
        self.generic_visit(node)
        self.current_context = old_context

    def visit_AugAssign(self, node: ast.AugAssign) -> None:
        old_context = self.current_context
        self.current_context = ast.unparse(node)[:100]
        self.generic_visit(node)
        self.current_context = old_context

    def visit_For(self, node: ast.For) -> None:
        # Loop variables are targets (Store)
        # We process them normally via visit_Name
        self.generic_visit(node)

    def visit_Global(self, node: ast.Global) -> None:
        for name in node.names:
            self.dfg.globals_used.add(name)
            if name in self.dfg.variables:
                self.dfg.variables[name].is_global = True

    def visit_Nonlocal(self, node: ast.Nonlocal) -> None:
        for name in node.names:
            if name in self.dfg.variables:
                self.dfg.variables[name].is_nonlocal = True

