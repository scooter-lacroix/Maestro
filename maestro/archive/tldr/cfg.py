"""
CFG (Control Flow Graph) Analysis Layer

Layer 3 of TLDR analysis. Analyzes control flow complexity,
branching patterns, and cyclomatic complexity.
Adds ~110 tokens for complexity metrics.
"""

import ast
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Tuple, Any
from enum import Enum


class NodeType(str, Enum):
    """Types of nodes in the control flow graph"""
    ENTRY = "entry"
    EXIT = "exit"
    BASIC_BLOCK = "basic_block"
    CONDITION = "condition"
    LOOP = "loop"
    TRY = "try"
    EXCEPT = "except"
    FINALLY = "finally"


@dataclass
class CFGNode:
    """A node in the control flow graph"""
    id: str
    type: NodeType
    line: int
    condition: Optional[str] = None
    statements: List[str] = field(default_factory=list)
    successors: Set[str] = field(default_factory=set)
    predecessors: Set[str] = field(default_factory=set)


@dataclass
class ComplexityMetrics:
    """Complexity metrics for a function"""
    cyclomatic_complexity: int = 1
    decision_points: int = 0
    loop_count: int = 0
    branch_count: int = 0
    try_count: int = 0
    except_count: int = 0
    max_nesting_depth: int = 0

    def complexity_score(self) -> str:
        """Get complexity rating"""
        if self.cyclomatic_complexity <= 5:
            return "low"
        elif self.cyclomatic_complexity <= 10:
            return "moderate"
        elif self.cyclomatic_complexity <= 20:
            return "high"
        else:
            return "very_high"


@dataclass
class ControlFlowGraph:
    """Complete control flow graph for a function"""
    function_name: str
    file_path: str
    start_line: int
    end_line: int
    nodes: Dict[str, CFGNode] = field(default_factory=dict)
    entry_node: Optional[str] = None
    exit_nodes: Set[str] = field(default_factory=set)
    metrics: ComplexityMetrics = field(default_factory=ComplexityMetrics)

    def get_paths(self) -> List[List[str]]:
        """Get all possible execution paths"""
        paths: List[List[str]] = []
        if self.entry_node is not None:
            self._find_paths(self.entry_node, [], paths, set())
        return paths

    def _find_paths(self, node_id: str, current: List[str], paths: List[List[str]], visited: Set[str]) -> None:
        if node_id in self.exit_nodes or node_id not in self.nodes:
            if current:
                paths.append(current.copy())
            return

        # Avoid infinite loops
        if node_id in visited:
            paths.append(current.copy())
            return

        visited.add(node_id)
        current.append(node_id)

        node = self.nodes.get(node_id)
        if node:
            for succ in sorted(node.successors):
                self._find_paths(succ, current, paths, visited.copy())

        current.pop()


class CFGAnalyzer:
    """
    Control Flow Graph Analyzer

    Provides Layer 3 analysis: complexity metrics, control flow
    patterns, and execution path analysis.
    """

    def __init__(self) -> None:
        """Initialize the CFG analyzer"""
        self._node_counter = 0

    def analyze_function(
        self,
        source: str,
        function_name: str,
        file_path: str = "<source>",
    ) -> Optional[ControlFlowGraph]:
        """
        Analyze control flow of a function

        Args:
            source: Python source code
            function_name: Name of the function to analyze
            file_path: Optional file path

        Returns:
            ControlFlowGraph or None if analysis fails
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

        self._node_counter = 0
        cfg = ControlFlowGraph(
            function_name=function_name,
            file_path=file_path,
            start_line=function_node.lineno,
            end_line=function_node.end_lineno or function_node.lineno,
        )

        # Build the CFG
        entry_id = self._new_node_id()
        cfg.entry_node = entry_id
        cfg.nodes[entry_id] = CFGNode(id=entry_id, type=NodeType.ENTRY, line=function_node.lineno)

        # Process function body
        exit_ids = self._process_block(function_node.body, cfg, entry_id)

        # Create common exit node
        exit_id = self._new_node_id()
        cfg.exit_nodes.add(exit_id)
        cfg.nodes[exit_id] = CFGNode(id=exit_id, type=NodeType.EXIT, line=function_node.end_lineno or function_node.lineno)

        # Connect all exit paths to common exit
        for ex in exit_ids:
            if ex in cfg.nodes:
                cfg.nodes[ex].successors.add(exit_id)
                cfg.nodes[exit_id].predecessors.add(ex)

        # Calculate metrics
        cfg.metrics = self._calculate_metrics(cfg)

        return cfg

    def _new_node_id(self) -> str:
        """Generate a unique node ID"""
        self._node_counter += 1
        return f"n{self._node_counter}"

    def _process_block(
        self,
        statements: List[ast.stmt],
        cfg: ControlFlowGraph,
        entry_id: str,
    ) -> Set[str]:
        """
        Process a block of statements

        Returns:
            Set of exit node IDs from this block
        """
        if not statements:
            return {entry_id}

        current_exits = {entry_id}

        for stmt in statements:
            new_exits = set()
            for exit_id in current_exits:
                if isinstance(stmt, ast.If):
                    new_exits.update(self._process_if(stmt, cfg, exit_id))
                elif isinstance(stmt, (ast.For, ast.While)):
                    new_exits.update(self._process_loop(stmt, cfg, exit_id))
                elif isinstance(stmt, ast.Try):
                    new_exits.update(self._process_try(stmt, cfg, exit_id))
                elif isinstance(stmt, ast.Return):
                    new_exits.update(self._process_return(stmt, cfg, exit_id))
                else:
                    # Basic statement - add to current block
                    node = cfg.nodes.get(exit_id)
                    if node and node.type == NodeType.BASIC_BLOCK:
                        node.statements.append(ast.unparse(stmt)[:100])
                        new_exits.add(exit_id)
                    else:
                        # Create new basic block
                        new_id = self._new_node_id()
                        cfg.nodes[new_id] = CFGNode(
                            id=new_id,
                            type=NodeType.BASIC_BLOCK,
                            line=stmt.lineno,
                            statements=[ast.unparse(stmt)[:100]]
                        )
                        cfg.nodes[exit_id].successors.add(new_id)
                        cfg.nodes[new_id].predecessors.add(exit_id)
                        new_exits.add(new_id)

            current_exits = new_exits

        return current_exits

    def _process_if(self, if_node: ast.If, cfg: ControlFlowGraph, entry_id: str) -> Set[str]:
        """Process an if statement"""
        # Create condition node
        cond_id = self._new_node_id()
        cfg.nodes[cond_id] = CFGNode(
            id=cond_id,
            type=NodeType.CONDITION,
            line=if_node.lineno,
            condition=ast.unparse(if_node.test)[:100],
        )

        # Link entry to condition
        cfg.nodes[entry_id].successors.add(cond_id)
        cfg.nodes[cond_id].predecessors.add(entry_id)

        # Process then block
        then_exits = self._process_block(if_node.body, cfg, cond_id)

        # Process else block
        else_exits = set()
        if if_node.orelse:
            else_exits = self._process_block(if_node.orelse, cfg, cond_id)
        else:
            else_exits = {cond_id}

        return then_exits | else_exits

    def _process_loop(
        self,
        loop_node: ast.For | ast.While,
        cfg: ControlFlowGraph,
        entry_id: str,
    ) -> Set[str]:
        """Process a for or while loop"""
        # Create loop node
        loop_id = self._new_node_id()
        cfg.nodes[loop_id] = CFGNode(
            id=loop_id,
            type=NodeType.LOOP,
            line=loop_node.lineno,
            condition=ast.unparse(loop_node.iter if isinstance(loop_node, ast.For) else loop_node.test)[:100],
        )

        # Link entry to loop
        cfg.nodes[entry_id].successors.add(loop_id)
        cfg.nodes[loop_id].predecessors.add(entry_id)

        # Process loop body
        body_exits = self._process_block(loop_node.body, cfg, loop_id)

        # Link body exits back to loop (back edge)
        for exit_id in body_exits:
            if exit_id in cfg.nodes:
                cfg.nodes[exit_id].successors.add(loop_id)
                cfg.nodes[loop_id].predecessors.add(exit_id)

        # Process else block (executed if loop doesn't break)
        if loop_node.orelse:
            else_exits = self._process_block(loop_node.orelse, cfg, loop_id)
        else:
            else_exits = {loop_id}

        return else_exits

    def _process_try(self, try_node: ast.Try, cfg: ControlFlowGraph, entry_id: str) -> Set[str]:
        """Process a try statement"""
        # Create try node
        try_id = self._new_node_id()
        cfg.nodes[try_id] = CFGNode(
            id=try_id,
            type=NodeType.TRY,
            line=try_node.lineno,
        )

        cfg.nodes[entry_id].successors.add(try_id)
        cfg.nodes[try_id].predecessors.add(entry_id)

        # Process try block
        try_exits = self._process_block(try_node.body, cfg, try_id)

        # Process except blocks
        all_exits = set(try_exits)
        for handler in try_node.handlers:
            except_id = self._new_node_id()
            exc_type = ast.unparse(handler.type) if handler.type else "Exception"
            cfg.nodes[except_id] = CFGNode(
                id=except_id,
                type=NodeType.EXCEPT,
                line=handler.lineno,
                condition=f"except {exc_type}",
            )
            cfg.nodes[try_id].successors.add(except_id)
            cfg.nodes[except_id].predecessors.add(try_id)

            except_exits = self._process_block(handler.body, cfg, except_id)
            all_exits.update(except_exits)

        # Process finally block
        if try_node.finalbody:
            finally_id = self._new_node_id()
            cfg.nodes[finally_id] = CFGNode(
                id=finally_id,
                type=NodeType.FINALLY,
                line=try_node.finalbody[0].lineno,
            )

            # All exits go to finally
            for exit_id in all_exits.copy():
                if exit_id in cfg.nodes:
                    cfg.nodes[exit_id].successors.add(finally_id)
                    cfg.nodes[finally_id].predecessors.add(exit_id)
                    all_exits.remove(exit_id)

            finally_exits = self._process_block(try_node.finalbody, cfg, finally_id)
            all_exits.update(finally_exits)

        return all_exits

    def _process_return(self, return_node: ast.Return, cfg: ControlFlowGraph, entry_id: str) -> Set[str]:
        """Process a return statement"""
        ret_id = self._new_node_id()
        cfg.nodes[ret_id] = CFGNode(
            id=ret_id,
            type=NodeType.EXIT,
            line=return_node.lineno,
            statements=["return"],
        )

        cfg.nodes[entry_id].successors.add(ret_id)
        cfg.nodes[ret_id].predecessors.add(entry_id)
        cfg.exit_nodes.add(ret_id)

        return {ret_id}

    def _calculate_metrics(self, cfg: ControlFlowGraph) -> ComplexityMetrics:
        """Calculate complexity metrics for the CFG"""
        metrics = ComplexityMetrics()

        # Count node types
        for node in cfg.nodes.values():
            if node.type == NodeType.CONDITION:
                metrics.decision_points += 1
                metrics.branch_count += 2
            elif node.type == NodeType.LOOP:
                metrics.loop_count += 1
                metrics.decision_points += 1
            elif node.type == NodeType.TRY:
                metrics.try_count += 1
            elif node.type == NodeType.EXCEPT:
                metrics.except_count += 1

        # Cyclomatic complexity = E - N + 2*P
        # Simplified: 1 + number of decision points
        metrics.cyclomatic_complexity = 1 + metrics.decision_points + metrics.except_count

        # Calculate max nesting depth
        metrics.max_nesting_depth = self._calculate_nesting_depth(cfg)

        return metrics

    def _calculate_nesting_depth(self, cfg: ControlFlowGraph) -> int:
        """Calculate maximum nesting depth"""
        depth = 0

        def dfs(node_id: str, current_depth: int) -> int:
            nonlocal depth
            depth = max(depth, current_depth)

            node = cfg.nodes.get(node_id)
            if not node:
                return current_depth

            for succ in node.successors:
                extra = 1 if node.type in (NodeType.CONDITION, NodeType.LOOP, NodeType.TRY) else 0
                dfs(succ, current_depth + extra)

            return current_depth

        if cfg.entry_node:
            dfs(cfg.entry_node, 0)

        return depth

    def get_complexity(self, source: str, function_name: str) -> Optional[ComplexityMetrics]:
        """
        Get complexity metrics for a function

        Args:
            source: Python source code
            function_name: Name of the function

        Returns:
            ComplexityMetrics or None if analysis fails
        """
        cfg = self.analyze_function(source, function_name)
        if not cfg:
            return None
        return cfg.metrics

    def compare_functions(
        self,
        source: str,
        function_names: List[str],
    ) -> Dict[str, ComplexityMetrics]:
        """
        Compare complexity of multiple functions

        Args:
            source: Python source code
            function_names: List of function names to compare

        Returns:
            Dictionary mapping function names to metrics
        """
        results = {}
        for name in function_names:
            metrics = self.get_complexity(source, name)
            if metrics:
                results[name] = metrics
        return results

    def find_complex_functions(
        self,
        source: str,
        threshold: int = 10,
    ) -> List[Tuple[str, ComplexityMetrics]]:
        """
        Find all functions with complexity above threshold

        Args:
            source: Python source code
            threshold: Complexity threshold

        Returns:
            List of (function_name, metrics) tuples
        """
        try:
            tree = ast.parse(source)
        except SyntaxError:
            return []

        complex_funcs = []

        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                metrics = self.get_complexity(source, node.name)
                if metrics and metrics.cyclomatic_complexity >= threshold:
                    complex_funcs.append((node.name, metrics))

        return sorted(complex_funcs, key=lambda x: x[1].cyclomatic_complexity, reverse=True)

    def to_llm_string(self, cfg: ControlFlowGraph) -> str:
        """
        Convert CFG to LLM-friendly string

        Args:
            cfg: ControlFlowGraph to convert

        Returns:
            Compact string representation
        """
        lines = [
            f"## Control Flow: {cfg.function_name}",
            f"Lines: {cfg.start_line}-{cfg.end_line}",
            f"Complexity: {cfg.metrics.cyclomatic_complexity} ({cfg.metrics.complexity_score()})",
        ]

        if cfg.metrics.decision_points:
            lines.append(f"Decision Points: {cfg.metrics.decision_points}")
        if cfg.metrics.loop_count:
            lines.append(f"Loops: {cfg.metrics.loop_count}")
        if cfg.metrics.try_count:
            lines.append(f"Try Blocks: {cfg.metrics.try_count}")

        # Show control structure
        if cfg.metrics.cyclomatic_complexity > 1:
            lines.append("\n### Structure:")
            for node_id, node in sorted(cfg.nodes.items(), key=lambda x: x[1].line):
                if node.type == NodeType.ENTRY:
                    lines.append(f"  ENTRY -> {', '.join(sorted(node.successors))}")
                elif node.type == NodeType.CONDITION:
                    lines.append(f"  IF {node.condition} -> {', '.join(sorted(node.successors))}")
                elif node.type == NodeType.LOOP:
                    lines.append(f"  LOOP {node.condition} -> {', '.join(sorted(node.successors))}")
                elif node.type == NodeType.EXCEPT:
                    lines.append(f"  {node.condition} -> {', '.join(sorted(node.successors))}")

        return "\n".join(lines)
