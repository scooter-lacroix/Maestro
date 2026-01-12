"""
DFG (Data Flow Graph) Analysis Layer

Layer 4 of TLDR analysis. Tracks variable definitions, uses,
and modifications throughout the code.
Adds ~130 tokens for data flow understanding.
"""

import ast
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Tuple, Any
from enum import Enum


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
        """Count number of definitions"""
        return sum(1 for a in self.accesses if a.action == VarAction.DEFINE)

    def get_use_count(self) -> int:
        """Count number of uses"""
        return sum(1 for a in self.accesses if a.action in (VarAction.READ, VarAction.MODIFY))

    def get_modify_count(self) -> int:
        """Count number of modifications"""
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

    def get_data_dependencies(self, variable: str) -> Set[str]:
        """Get variables that influence the given variable"""
        result: Set[str] = set()
        var_info = self.variables.get(variable)
        if not var_info:
            return result

        for access in var_info.accesses:
            if access.action in (VarAction.DEFINE, VarAction.MODIFY) and access.context:
                # Extract variable names from context (simplified)
                for other_var in self.variables:
                    if other_var != variable and other_var in access.context:
                        result.add(other_var)

        return result

    def get_affected_by(self, variable: str) -> Set[str]:
        """Get variables influenced by the given variable"""
        result = set()

        for var_name, var_info in self.variables.items():
            if var_name == variable:
                continue

            for access in var_info.accesses:
                if access.action in (VarAction.READ, VarAction.MODIFY) and access.context:
                    if variable in access.context:
                        result.add(var_name)

        return result


class DFGAnalyzer:
    """
    Data Flow Graph Analyzer

    Provides Layer 4 analysis: variable definition and use tracking,
    data dependencies, and influence analysis.
    """

    def __init__(self) -> None:
        """Initialize the DFG analyzer"""

    def analyze_function(
        self,
        source: str,
        function_name: str,
        file_path: str = "<source>",
    ) -> Optional[DataFlowGraph]:
        """
        Analyze data flow of a function

        Args:
            source: Python source code
            function_name: Name of the function
            file_path: Optional file path

        Returns:
            DataFlowGraph or None if analysis fails
        """
        try:
            tree = ast.parse(source)
        except SyntaxError:
            return None

        # Find the function
        function_node = None
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                if node.name == function_name:
                    function_node = node
                    break

        if not function_node:
            return None

        dfg = DataFlowGraph(
            function_name=function_name,
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
                defining_scope=function_name,
                type_hint=type_hint,
                is_parameter=True,
            )

        # Analyze function body
        visitor = _DFGVisitor(function_name, dfg)
        visitor.visit(function_node)

        # Find return values
        for node in ast.walk(function_node):
            if isinstance(node, ast.Return) and node.value:
                return_vars = self._extract_return_vars(node.value, dfg)
                dfg.returns.extend(return_vars)

        return dfg

    def _extract_return_vars(self, return_node: ast.AST, dfg: DataFlowGraph) -> List[str]:
        """Extract variable names from return statement"""
        vars_found = []

        if isinstance(return_node, ast.Name):
            if return_node.id in dfg.variables:
                vars_found.append(return_node.id)
        elif isinstance(return_node, ast.Tuple):
            for elt in return_node.elts:
                if isinstance(elt, ast.Name) and elt.id in dfg.variables:
                    vars_found.append(elt.id)

        return vars_found

    def get_variable_lifecycle(
        self,
        source: str,
        function_name: str,
        variable: str,
    ) -> Optional[Dict[str, Any]]:
        """
        Get the lifecycle of a variable

        Args:
            source: Python source code
            function_name: Name of the function
            variable: Variable name

        Returns:
            Dictionary with lifecycle information
        """
        dfg = self.analyze_function(source, function_name)
        if not dfg or variable not in dfg.variables:
            return None

        var_info = dfg.variables[variable]

        return {
            "name": variable,
            "defined_at": var_info.defining_line,
            "type": var_info.type_hint,
            "is_parameter": var_info.is_parameter,
            "is_global": var_info.is_global,
            "definition_count": var_info.get_def_count(),
            "use_count": var_info.get_use_count(),
            "modify_count": var_info.get_modify_count(),
            "influenced_by": list(dfg.get_data_dependencies(variable)),
            "influences": list(dfg.get_affected_by(variable)),
            "accesses": [
                {"line": a.line, "action": a.action, "context": a.context}
                for a in var_info.accesses
            ],
        }

    def find_unused_variables(
        self,
        source: str,
        function_name: str,
    ) -> List[str]:
        """
        Find variables that are defined but never used

        Args:
            source: Python source code
            function_name: Name of the function

        Returns:
            List of unused variable names
        """
        dfg = self.analyze_function(source, function_name)
        if not dfg:
            return []

        unused = []
        for var_name, var_info in dfg.variables.items():
            if var_info.is_parameter:
                continue
            if var_info.get_use_count() == 0:
                unused.append(var_name)

        return unused

    def find_undefined_variables(
        self,
        source: str,
        function_name: str,
    ) -> List[str]:
        """
        Find variables that are used but not defined within the function

        Args:
            source: Python source code
            function_name: Name of the function

        Returns:
            List of potentially undefined variable names
        """
        dfg = self.analyze_function(source, function_name)
        if not dfg:
            return []

        # Check variables that are read but not defined
        undefined = []
        for var_name, var_info in dfg.variables.items():
            if var_info.defining_line == 0:  # Sentinel for undefined
                undefined.append(var_name)

        return undefined

    def slice_backward(
        self,
        source: str,
        function_name: str,
        line: int,
    ) -> List[Tuple[str, int, str]]:
        """
        Perform backward program slice

        Find all statements that influence the value at a given line.

        Args:
            source: Python source code
            function_name: Name of the function
            line: Line number to slice from

        Returns:
            List of (variable, line, context) tuples
        """
        dfg = self.analyze_function(source, function_name)
        if not dfg:
            return []

        # Find variables used at the target line
        target_vars = set()
        for var_info in dfg.variables.values():
            for access in var_info.accesses:
                if access.line == line and access.action in (VarAction.READ, VarAction.MODIFY):
                    target_vars.add(var_info.name)

        # Find all influences
        result = []
        visited = set()

        def find_influences(var: str, depth: int = 0) -> None:
            if depth > 10 or var in visited:
                return
            visited.add(var)

            var_info = dfg.variables.get(var)
            if not var_info:
                return

            # Add defining locations
            for access in var_info.accesses:
                if access.action in (VarAction.DEFINE, VarAction.MODIFY):
                    result.append((var, access.line, access.context or ""))

                    # Find what influenced this definition
                    influences = dfg.get_data_dependencies(var)
                    for influencer in influences:
                        find_influences(influencer, depth + 1)

        for var in target_vars:
            find_influences(var)

        # Deduplicate and sort
        seen = set()
        unique = []
        for item in result:
            key = (item[0], item[1])
            if key not in seen:
                seen.add(key)
                unique.append(item)

        return sorted(unique, key=lambda x: x[1])

    def slice_forward(
        self,
        source: str,
        function_name: str,
        variable: str,
    ) -> List[Tuple[str, int, str]]:
        """
        Perform forward program slice

        Find all statements influenced by the given variable.

        Args:
            source: Python source code
            function_name: Name of the function
            variable: Variable name to slice from

        Returns:
            List of (variable, line, context) tuples
        """
        dfg = self.analyze_function(source, function_name)
        if not dfg:
            return []

        result = []
        visited = set()

        def find_affected(var: str) -> None:
            if var in visited:
                return
            visited.add(var)

            var_info = dfg.variables.get(var)
            if not var_info:
                return

            for access in var_info.accesses:
                if access.action in (VarAction.READ, VarAction.MODIFY):
                    result.append((var, access.line, access.context or ""))

                    # Find what this affects
                    if access.action == VarAction.MODIFY:
                        affected = dfg.get_affected_by(var)
                        for affected_var in affected:
                            find_affected(affected_var)

        find_affected(variable)

        # Deduplicate and sort
        seen = set()
        unique = []
        for item in result:
            key = (item[0], item[1])
            if key not in seen:
                seen.add(key)
                unique.append(item)

        return sorted(unique, key=lambda x: x[1])

    def to_llm_string(self, dfg: DataFlowGraph) -> str:
        """
        Convert DFG to LLM-friendly string

        Args:
            dfg: DataFlowGraph to convert

        Returns:
            Compact string representation
        """
        lines = [
            f"## Data Flow: {dfg.function_name}",
            f"Parameters: {', '.join(dfg.parameters)}",
        ]

        if dfg.returns:
            lines.append(f"Returns: {', '.join(dfg.returns)}")

        if dfg.globals_used:
            lines.append(f"Globals: {', '.join(sorted(dfg.globals_used))}")

        # Show key variables
        important_vars = []
        for name, var in dfg.variables.items():
            if var.is_parameter or var.get_use_count() > 1:
                important_vars.append(var)

        if important_vars:
            lines.append("\n### Variables:")
            for var in sorted(important_vars, key=lambda v: v.defining_line):
                type_str = f": {var.type_hint}" if var.type_hint else ""
                uses = var.get_use_count()
                mods = var.get_modify_count()
                lines.append(f"  {var.name}{type_str} - defined:{var.defining_line}, uses:{uses}, mods:{mods}")

        return "\n".join(lines)


class _DFGVisitor(ast.NodeVisitor):
    """AST visitor for building data flow graph"""

    def __init__(self, scope: str, dfg: DataFlowGraph):
        self.scope = scope
        self.dfg = dfg
        self.current_line = 0

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        # Don't recurse into nested functions
        self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        # Don't recurse into nested functions
        self.generic_visit(node)

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        # Don't recurse into classes
        pass

    def visit_Name(self, node: ast.Name) -> None:
        self.current_line = node.lineno

        var_name = node.id
        var_info = self.dfg.variables.get(var_name)

        if isinstance(node.ctx, ast.Load):
            # Variable is being read
            action = VarAction.READ
        elif isinstance(node.ctx, ast.Store):
            # Variable is being defined/modified
            action = VarAction.MODIFY if var_info else VarAction.DEFINE
        else:
            action = VarAction.READ

        # Create variable info if needed
        if not var_info:
            var_info = VariableInfo(
                name=var_name,
                defining_line=node.lineno if action == VarAction.DEFINE else 0,
                defining_scope=self.scope,
            )
            self.dfg.variables[var_name] = var_info

        # Add access
        var_info.accesses.append(VariableAccess(
            name=var_name,
            action=action,
            line=node.lineno,
            scope=self.scope,
        ))

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

    def visit_Assign(self, node: ast.Assign) -> None:
        self.current_line = node.lineno

        # Track assignment context
        context = ast.unparse(node)[:100]

        for target in node.targets:
            if isinstance(target, ast.Name):
                var_name = target.id
                var_info = self.dfg.variables.get(var_name)
                if not var_info:
                    var_info = VariableInfo(
                        name=var_name,
                        defining_line=node.lineno,
                        defining_scope=self.scope,
                    )
                    self.dfg.variables[var_name] = var_info

                var_info.accesses.append(VariableAccess(
                    name=var_name,
                    action=VarAction.DEFINE,
                    line=node.lineno,
                    scope=self.scope,
                    context=context,
                ))

        self.generic_visit(node)

    def visit_AugAssign(self, node: ast.AugAssign) -> None:
        self.current_line = node.lineno

        if isinstance(node.target, ast.Name):
            var_name = node.target.id
            var_info = self.dfg.variables.get(var_name)
            if not var_info:
                var_info = VariableInfo(
                    name=var_name,
                    defining_line=node.lineno,
                    defining_scope=self.scope,
                )
                self.dfg.variables[var_name] = var_info

            var_info.accesses.append(VariableAccess(
                name=var_name,
                action=VarAction.MODIFY,
                line=node.lineno,
                scope=self.scope,
                context=ast.unparse(node)[:100],
            ))

        self.generic_visit(node)

    def visit_For(self, node: ast.For) -> None:
        self.current_line = node.lineno

        # Loop variable
        if isinstance(node.target, ast.Name):
            var_name = node.target.id
            var_info = VariableInfo(
                name=var_name,
                defining_line=node.lineno,
                defining_scope=self.scope,
            )
            self.dfg.variables[var_name] = var_info
            var_info.accesses.append(VariableAccess(
                name=var_name,
                action=VarAction.DEFINE,
                line=node.lineno,
                scope=self.scope,
            ))

        self.generic_visit(node)

    def visit_comprehension(self, node: ast.comprehension) -> None:
        # Comprehension target variable
        if isinstance(node.target, ast.Name):
            var_name = node.target.id
            # Comprehensions create their own scope
            var_info = VariableInfo(
                name=var_name,
                defining_line=getattr(node, 'lineno', 0),
                defining_scope=f"{self.scope}_comp",
            )
            self.dfg.variables[var_name] = var_info

        self.generic_visit(node)
