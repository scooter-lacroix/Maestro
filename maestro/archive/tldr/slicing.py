"""
Program Slicing Layer

Layer 5 of TLDR analysis. Combines CFG and DFG to perform
program slicing - finding all statements that affect a given point.
Adds ~150 tokens for dependency analysis.
"""

from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Tuple, Any
from enum import Enum

from maestro.tldr.cfg import CFGAnalyzer, ControlFlowGraph
from maestro.tldr.dfg import DFGAnalyzer, DataFlowGraph, VarAction


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

    def get_summary(self) -> str:
        """Get a summary of the slice"""
        return (
            f"Slice @ line {self.target_line}: "
            f"{len(self.relevant_lines)} lines, "
            f"{len(self.relevant_variables)} variables"
        )


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


class SlicingAnalyzer:
    """
    Program Slicing Analyzer

    Provides Layer 5 analysis: program slicing to find all statements
    that affect (or are affected by) a given point in the code.
    """

    def __init__(self) -> None:
        """Initialize the slicing analyzer"""
        self.cfg_analyzer = CFGAnalyzer()
        self.dfg_analyzer = DFGAnalyzer()

    def build_pdg(
        self,
        source: str,
        function_name: str,
        file_path: str = "<source>",
    ) -> Optional[ProgramDependenceGraph]:
        """
        Build a Program Dependence Graph (PDG)

        Combines control flow (CFG) and data flow (DFG) to create
        a complete dependence graph.

        Args:
            source: Python source code
            function_name: Name of the function
            file_path: Optional file path

        Returns:
            ProgramDependenceGraph or None if analysis fails
        """
        cfg = self.cfg_analyzer.analyze_function(source, function_name, file_path)
        dfg = self.dfg_analyzer.analyze_function(source, function_name, file_path)

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

        Find all statements that affect the given line.

        Args:
            source: Python source code
            function_name: Name of the function
            target_line: Target line number
            file_path: Optional file path

        Returns:
            SliceResult or None if analysis fails
        """
        pdg = self.build_pdg(source, function_name, file_path)
        if not pdg:
            return None

        result = SliceResult(
            function_name=function_name,
            target_line=target_line,
            direction=SliceDirection.BACKWARD,
        )

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

    def slice_forward(
        self,
        source: str,
        function_name: str,
        target_line: int,
        file_path: str = "<source>",
    ) -> Optional[SliceResult]:
        """
        Perform forward program slice

        Find all statements affected by the given line.

        Args:
            source: Python source code
            function_name: Name of the function
            target_line: Target line number
            file_path: Optional file path

        Returns:
            SliceResult or None if analysis fails
        """
        pdg = self.build_pdg(source, function_name, file_path)
        if not pdg:
            return None

        result = SliceResult(
            function_name=function_name,
            target_line=target_line,
            direction=SliceDirection.FORWARD,
        )

        # BFS forwards to find all successors
        visited = {target_line}
        queue = list(pdg.get_successors(target_line))

        while queue:
            line = queue.pop(0)
            if line in visited:
                continue

            visited.add(line)
            result.relevant_lines.add(line)
            queue.extend(pdg.get_successors(line) - visited)

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

    def slice_variable(
        self,
        source: str,
        function_name: str,
        variable: str,
        file_path: str = "<source>",
    ) -> Optional[SliceResult]:
        """
        Perform slice focused on a specific variable

        Find all statements related to the given variable.

        Args:
            source: Python source code
            function_name: Name of the function
            variable: Variable name to slice
            file_path: Optional file path

        Returns:
            SliceResult or None if analysis fails
        """
        dfg = self.dfg_analyzer.analyze_function(source, function_name, file_path)
        if not dfg or variable not in dfg.variables:
            return None

        var_info = dfg.variables[variable]

        result = SliceResult(
            function_name=function_name,
            target_line=var_info.defining_line,
            direction=SliceDirection.BOTH,
        )

        result.relevant_variables.add(variable)

        # Add all lines where this variable is accessed
        for access in var_info.accesses:
            result.relevant_lines.add(access.line)

        # Find dependencies
        data_deps = dfg.get_data_dependencies(variable)
        affected_by = dfg.get_affected_by(variable)

        result.relevant_variables.update(data_deps)
        result.relevant_variables.update(affected_by)

        # Add lines for related variables
        for dep_var in data_deps | affected_by:
            if dep_var in dfg.variables:
                for access in dfg.variables[dep_var].accesses:
                    result.relevant_lines.add(access.line)

        return result

    def compute_chop(
        self,
        source: str,
        function_name: str,
        from_line: int,
        to_line: int,
        file_path: str = "<source>",
    ) -> Set[int]:
        """
        Compute a CHOP (slice between two points)

        Find all statements on paths from from_line to to_line.

        Args:
            source: Python source code
            function_name: Name of the function
            from_line: Starting line
            to_line: Target line
            file_path: Optional file path

        Returns:
            Set of relevant line numbers
        """
        # Forward slice from from_line
        forward = self.slice_forward(source, function_name, from_line, file_path)
        if not forward:
            return set()

        # Backward slice from to_line
        backward = self.slice_backward(source, function_name, to_line, file_path)
        if not backward:
            return set()

        # Intersection gives CHOP
        return forward.relevant_lines & backward.relevant_lines

    def analyze_ripple_effect(
        self,
        source: str,
        function_name: str,
        changed_line: int,
        file_path: str = "<source>",
    ) -> Dict[str, Any]:
        """
        Analyze the ripple effect of changing a line

        Args:
            source: Python source code
            function_name: Name of the function
            changed_line: Line that would be changed
            file_path: Optional file path

        Returns:
            Dictionary with ripple effect analysis
        """
        forward = self.slice_forward(source, function_name, changed_line, file_path)
        if not forward:
            return {"error": "Could not analyze function"}

        return {
            "changed_line": changed_line,
            "affected_lines_count": len(forward.relevant_lines),
            "affected_lines": sorted(forward.relevant_lines),
            "affected_variables": sorted(forward.relevant_variables),
            "dependencies": forward.dependencies,
        }

    def get_slice_summary(
        self,
        source: str,
        function_name: str,
        line: int,
        file_path: str = "<source>",
    ) -> str:
        """
        Get a text summary of a slice at a line

        Args:
            source: Python source code
            function_name: Name of the function
            line: Line number
            file_path: Optional file path

        Returns:
            Text summary
        """
        backward = self.slice_backward(source, function_name, line, file_path)
        forward = self.slice_forward(source, function_name, line, file_path)

        lines = [f"## Program Slice: {function_name} @ line {line}"]

        if backward:
            lines.append(f"\n### Backward Slice (affects line {line}):")
            lines.append(f"Lines: {len(backward.relevant_lines)}")
            lines.append(f"Variables: {', '.join(sorted(backward.relevant_variables))}")
            if backward.dependencies:
                lines.append("\nDependencies:")
                for dep_line, vars_str in backward.dependencies[:10]:
                    lines.append(f"  Line {dep_line}: {vars_str}")

        if forward:
            lines.append(f"\n### Forward Slice (affected by line {line}):")
            lines.append(f"Lines: {len(forward.relevant_lines)}")
            lines.append(f"Variables: {', '.join(sorted(forward.relevant_variables))}")

        return "\n".join(lines)

    def to_llm_string(self, pdg: ProgramDependenceGraph) -> str:
        """
        Convert PDG to LLM-friendly string

        Args:
            pdg: ProgramDependenceGraph to convert

        Returns:
            Compact string representation
        """
        lines = [
            f"## Program Dependence Graph: {pdg.function_name}",
            f"Edges: {len(pdg.edges)} ({len([e for e in pdg.edges if e[2] == 'control'])} control, "
            f"{len([e for e in pdg.edges if e[2] == 'data'])} data)"
        ]

        # Show key dependencies
        data_edges = sorted([e for e in pdg.edges if e[2] == "data"])
        if data_edges:
            lines.append("\n### Key Data Dependencies:")
            for from_line, to_line, _ in data_edges[:20]:
                lines.append(f"  Line {from_line} -> Line {to_line}")

        return "\n".join(lines)
