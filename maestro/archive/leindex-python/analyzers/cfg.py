import ast
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Set, Any, Tuple
from enum import Enum
from .base import BaseAnalyzer

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

class CFGAnalyzer(BaseAnalyzer):
    """
    Control Flow Graph Analyzer.
    Layer 3 analysis: complexity metrics, control flow patterns.
    """

    def __init__(self) -> None:
        self._node_counter = 0

    def analyze(self, code: str, file_path: str) -> Dict[str, Any]:
        """
        Analyze code to calculate CFG metrics for all functions.
        """
        try:
            tree = ast.parse(code)
        except SyntaxError:
             return {
                "path": file_path,
                "error": "SyntaxError"
            }

        function_metrics = {}

        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                cfg = self._build_cfg(node, file_path)
                if cfg:
                    function_metrics[node.name] = asdict(cfg.metrics)
                    function_metrics[node.name]['score'] = cfg.metrics.complexity_score()

        return {
            "path": file_path,
            "functions": function_metrics
        }

    def _build_cfg(self, function_node: ast.AST, file_path: str) -> Optional[ControlFlowGraph]:
        self._node_counter = 0
        cfg = ControlFlowGraph(
            function_name=function_node.name,
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
        self._node_counter += 1
        return f"n{self._node_counter}"

    def _process_block(
        self,
        statements: List[ast.stmt],
        cfg: ControlFlowGraph,
        entry_id: str,
    ) -> Set[str]:
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
                    node = cfg.nodes.get(exit_id)
                    if node and node.type == NodeType.BASIC_BLOCK:
                        node.statements.append(ast.unparse(stmt)[:100])
                        new_exits.add(exit_id)
                    else:
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
        cond_id = self._new_node_id()
        cfg.nodes[cond_id] = CFGNode(
            id=cond_id,
            type=NodeType.CONDITION,
            line=if_node.lineno,
            condition=ast.unparse(if_node.test)[:100],
        )

        cfg.nodes[entry_id].successors.add(cond_id)
        cfg.nodes[cond_id].predecessors.add(entry_id)

        then_exits = self._process_block(if_node.body, cfg, cond_id)

        if if_node.orelse:
            else_exits = self._process_block(if_node.orelse, cfg, cond_id)
        else:
            else_exits = {cond_id}

        return then_exits | else_exits

    def _process_loop(self, loop_node: ast.For | ast.While, cfg: ControlFlowGraph, entry_id: str) -> Set[str]:
        loop_id = self._new_node_id()
        condition = ast.unparse(loop_node.iter if isinstance(loop_node, ast.For) else loop_node.test)[:100]
        cfg.nodes[loop_id] = CFGNode(
            id=loop_id,
            type=NodeType.LOOP,
            line=loop_node.lineno,
            condition=condition,
        )

        cfg.nodes[entry_id].successors.add(loop_id)
        cfg.nodes[loop_id].predecessors.add(entry_id)

        body_exits = self._process_block(loop_node.body, cfg, loop_id)

        for exit_id in body_exits:
            if exit_id in cfg.nodes:
                cfg.nodes[exit_id].successors.add(loop_id)
                cfg.nodes[loop_id].predecessors.add(exit_id)

        if loop_node.orelse:
            else_exits = self._process_block(loop_node.orelse, cfg, loop_id)
        else:
            else_exits = {loop_id}

        return else_exits

    def _process_try(self, try_node: ast.Try, cfg: ControlFlowGraph, entry_id: str) -> Set[str]:
        try_id = self._new_node_id()
        cfg.nodes[try_id] = CFGNode(
            id=try_id,
            type=NodeType.TRY,
            line=try_node.lineno,
        )

        cfg.nodes[entry_id].successors.add(try_id)
        cfg.nodes[try_id].predecessors.add(entry_id)

        try_exits = self._process_block(try_node.body, cfg, try_id)

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

        if try_node.finalbody:
            finally_id = self._new_node_id()
            cfg.nodes[finally_id] = CFGNode(
                id=finally_id,
                type=NodeType.FINALLY,
                line=try_node.finalbody[0].lineno,
            )

            for exit_id in all_exits.copy():
                if exit_id in cfg.nodes:
                    cfg.nodes[exit_id].successors.add(finally_id)
                    cfg.nodes[finally_id].predecessors.add(exit_id)
                    all_exits.remove(exit_id)

            finally_exits = self._process_block(try_node.finalbody, cfg, finally_id)
            all_exits.update(finally_exits)

        return all_exits

    def _process_return(self, return_node: ast.Return, cfg: ControlFlowGraph, entry_id: str) -> Set[str]:
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
        metrics = ComplexityMetrics()

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

        metrics.cyclomatic_complexity = 1 + metrics.decision_points + metrics.except_count
        metrics.max_nesting_depth = self._calculate_nesting_depth(cfg)

        return metrics

    def _calculate_nesting_depth(self, cfg: ControlFlowGraph) -> int:
        depth = 0
        # Use a max recursion depth safeguard or iterative approach if needed,
        # but for simple CFGs DFS is fine. We need to avoid cycles.
        visited = set()

        def dfs(node_id: str, current_depth: int):
            nonlocal depth
            if node_id in visited:
                return
            visited.add(node_id)

            depth = max(depth, current_depth)

            node = cfg.nodes.get(node_id)
            if not node:
                return

            for succ in node.successors:
                extra = 1 if node.type in (NodeType.CONDITION, NodeType.LOOP, NodeType.TRY) else 0
                dfs(succ, current_depth + extra)

            visited.remove(node_id)

        if cfg.entry_node:
            dfs(cfg.entry_node, 0)

        return depth

    def to_llm_string(self, analysis_result: Dict[str, Any]) -> str:
        lines = [f"File: {analysis_result['path']}"]

        for func_name, metrics in analysis_result.get("functions", {}).items():
            lines.append(f"\nFunction: {func_name}")
            lines.append(f"  Complexity: {metrics['cyclomatic_complexity']} ({metrics['score']})")
            if metrics['decision_points']:
                lines.append(f"  Decision Points: {metrics['decision_points']}")
            if metrics['loop_count']:
                lines.append(f"  Loops: {metrics['loop_count']}")

        return "\n".join(lines)
