from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Set, Tuple, Any
from enum import Enum

from .base import BaseAnalyzer
from .cfg import CFGAnalyzer, ControlFlowGraph
from .dfg import DFGAnalyzer, DataFlowGraph, VarAction

class SliceDirection(str, Enum):
    """Direction of program slice"""
    BACKWARD = "backward"
    FORWARD = "forward"
    BOTH = "both"

@dataclass
class SliceResult:
    """Result of a program slice"""
    function_name: str
    target_line: int
    direction: SliceDirection
    relevant_lines: Set[int] = field(default_factory=set)
    relevant_variables: Set[str] = field(default_factory=set)
    dependencies: List[Tuple[int, str]] = field(default_factory=list)

@dataclass
class ProgramDependenceGraph:
    """Combined control and data dependence graph"""
    function_name: str
    file_path: str
    cfg: Optional[ControlFlowGraph] = None
    dfg: Optional[DataFlowGraph] = None

    # Combined edges: (from_line, to_line, edge_type)
    # edge_type is 'control' or 'data'
    edges: Set[Tuple[int, int, str]] = field(default_factory=set)

    def get_predecessors(self, line: int) -> Set[int]:
        """Get all lines that influence the given line"""
        return {f for f, t, _ in self.edges if t == line}

    def get_successors(self, line: int) -> Set[int]:
        """Get all lines influenced by the given line"""
        return {t for f, t, _ in self.edges if f == line}

class SlicingAnalyzer(BaseAnalyzer):
    """
    Program Slicing Analyzer.
    Layer 5 analysis: program slicing to find all statements that affect (or are affected by) a given point.
    """

    def __init__(self) -> None:
        self.cfg_analyzer = CFGAnalyzer()
        self.dfg_analyzer = DFGAnalyzer()

    def analyze(self, code: str, file_path: str) -> Dict[str, Any]:
        """
        Analyze code to build PDG for all functions.
        """
        # We need function definitions to iterate over
        import ast
        try:
            tree = ast.parse(code)
        except SyntaxError:
            return {
                "path": file_path,
                "error": "SyntaxError"
            }

        functions_pdg = {}

        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                pdg = self.build_pdg(code, node.name, file_path)
                if pdg:
                    functions_pdg[node.name] = {
                        "function_name": pdg.function_name,
                        "file_path": pdg.file_path,
                        "edges": [list(e) for e in sorted(pdg.edges)] # Convert tuples to lists for JSON
                    }

        return {
            "path": file_path,
            "functions": functions_pdg
        }

    def build_pdg(
        self,
        source: str,
        function_name: str,
        file_path: str = "<source>",
    ) -> Optional[ProgramDependenceGraph]:
        """
        Build a Program Dependence Graph (PDG)
        """
        # We need to access the internal _build_cfg and _analyze_function methods
        # or expose analyze_function in CFG/DFG analyzers to return objects instead of dicts.
        # Looking at previous implementations, analyze() returns dicts.
        # But _build_cfg and _analyze_function return objects.

        # Let's find the function node first to use internal methods if possible,
        # or implement a way to get objects.
        import ast
        try:
            tree = ast.parse(source)
        except SyntaxError:
            return None

        function_node = None
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                if node.name == function_name:
                    function_node = node
                    break

        if not function_node:
            return None

        # Access internal methods to get objects
        cfg = self.cfg_analyzer._build_cfg(function_node, file_path)
        dfg = self.dfg_analyzer._analyze_function(function_node, file_path)

        if not cfg and not dfg:
            return None

        pdg = ProgramDependenceGraph(
            function_name=function_name,
            file_path=file_path,
            cfg=cfg,
            dfg=dfg,
        )

        # Add control dependence edges from CFG
        if cfg:
            pdg.edges.update(self._get_control_edges(cfg))

        # Add data dependence edges from DFG
        if dfg:
            pdg.edges.update(self._get_data_edges(dfg))

        return pdg

    def _get_control_edges(self, cfg: ControlFlowGraph) -> Set[Tuple[int, int, str]]:
        """Extract control dependence edges from CFG"""
        edges = set()

        for node_id, node in cfg.nodes.items():
            for succ_id in node.successors:
                succ = cfg.nodes.get(succ_id)
                if succ:
                    edges.add((node.line, succ.line, "control"))

        return edges

    def _get_data_edges(self, dfg: DataFlowGraph) -> Set[Tuple[int, int, str]]:
        """Extract data dependence edges from DFG"""
        edges = set()

        for var_name, var_info in dfg.variables.items():
            # Find def-use chains
            defs = [a for a in var_info.accesses if a.action in (VarAction.DEFINE, VarAction.MODIFY)]
            uses = [a for a in var_info.accesses if a.action in (VarAction.READ, VarAction.MODIFY)]

            for d in defs:
                for u in uses:
                    if u.line > d.line:
                        edges.add((d.line, u.line, "data"))

        return edges

    def slice_backward(
        self,
        source: str,
        function_name: str,
        target_line: int,
        file_path: str = "<source>",
    ) -> Optional[SliceResult]:
        """
        Perform backward program slice
        """
        pdg = self.build_pdg(source, function_name, file_path)
        if not pdg:
            return None

        result = SliceResult(
            function_name=function_name,
            target_line=target_line,
            direction=SliceDirection.BACKWARD,
        )

        # Include the target line in the slice
        result.relevant_lines.add(target_line)

        # BFS backwards to find all predecessors
        visited = {target_line}
        queue = list(pdg.get_predecessors(target_line))

        while queue:
            line = queue.pop(0)
            if line in visited:
                continue

            visited.add(line)
            result.relevant_lines.add(line)
            queue.extend(pdg.get_predecessors(line) - visited)

        # Get relevant variables
        if pdg.dfg:
            for var_name, var_info in pdg.dfg.variables.items():
                for access in var_info.accesses:
                    if access.line in result.relevant_lines:
                        result.relevant_variables.add(var_name)
                        break

            # Build dependency list
            for line in sorted(result.relevant_lines):
                vars_at_line = []
                for var_name, var_info in pdg.dfg.variables.items():
                    for access in var_info.accesses:
                        if access.line == line:
                            vars_at_line.append(var_name)
                if vars_at_line:
                    result.dependencies.append((line, ", ".join(vars_at_line)))

        return result

    def to_llm_string(self, analysis_result: Dict[str, Any]) -> str:
        lines = [f"File: {analysis_result['path']}"]

        for func_name, pdg_info in analysis_result.get("functions", {}).items():
            edges = pdg_info.get("edges", [])
            control_edges = len([e for e in edges if e[2] == 'control'])
            data_edges = len([e for e in edges if e[2] == 'data'])

            lines.append(f"\nFunction: {func_name}")
            lines.append(f"  Edges: {len(edges)} ({control_edges} control, {data_edges} data)")

            if data_edges > 0:
                lines.append("  Key Data Dependencies:")
                data_edges_list = sorted([e for e in edges if e[2] == "data"])
                for from_line, to_line, _ in data_edges_list[:5]:
                    lines.append(f"    Line {from_line} -> Line {to_line}")

        return "\n".join(lines)
