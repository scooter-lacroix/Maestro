"""
Call Graph Analysis Layer

Layer 2 of TLDR analysis. Builds cross-file function call graphs
to understand code relationships and navigate dependencies.
Adds ~440 tokens for comprehensive call relationships.
"""

import os
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Tuple, Any
from collections import defaultdict
from pathlib import Path

from maestro.tldr.ast import ASTAnalyzer, FileAnalysis, FunctionInfo, ImportInfo


@dataclass
class CallEdge:
    """Represents a call relationship between functions"""
    caller: str  # Function name
    callee: str  # Function being called
    caller_file: str
    callee_file: Optional[str] = None  # None if unknown/builtin
    line: int = 0


@dataclass
class FunctionNode:
    """Represents a function in the call graph"""
    name: str
    file: str
    line: int
    is_method: bool = False
    class_name: Optional[str] = None
    calls: Set[str] = field(default_factory=set)
    called_by: Set[str] = field(default_factory=set)


@dataclass
class CallGraph:
    """Complete call graph for a project"""
    functions: Dict[str, FunctionNode] = field(default_factory=dict)
    edges: List[CallEdge] = field(default_factory=list)
    file_map: Dict[str, str] = field(default_factory=dict)  # function -> file

    def get_callers(self, function_name: str, file_path: Optional[str] = None) -> List[FunctionNode]:
        """Get all functions that call the given function"""
        callers = []
        for func in self.functions.values():
            if function_name in func.calls:
                if file_path is None or func.file == file_path:
                    callers.append(func)
        return callers

    def get_callees(self, function_name: str, file_path: Optional[str] = None) -> List[FunctionNode]:
        """Get all functions called by the given function"""
        key = f"{file_path}:{function_name}" if file_path else function_name
        func = self.functions.get(key)
        if not func:
            return []

        callees = []
        for call_name in func.calls:
            for candidate in self.functions.values():
                if candidate.name == call_name or candidate.name.endswith(call_name):
                    callees.append(candidate)
        return callees

    def find_path(self, from_func: str, to_func: str) -> Optional[List[str]]:
        """Find a call path between two functions using BFS"""
        if from_func not in self.file_map and to_func not in self.file_map:
            return None

        from_key = self.file_map.get(from_func, from_func)
        to_key = self.file_map.get(to_func, to_func)

        if from_key not in self.functions or to_key not in self.functions:
            return None

        # BFS
        queue = [(from_key, [from_key])]
        visited = {from_key}

        while queue:
            current, path = queue.pop(0)
            if current == to_key:
                return path

            for callee_key in self.functions[current].calls:
                if callee_key not in visited:
                    visited.add(callee_key)
                    queue.append((callee_key, path + [callee_key]))

        return None


class CallGraphAnalyzer:
    """
    Call Graph Analyzer for understanding code relationships

    Provides Layer 2 analysis: cross-file call graphs showing
    which functions call which, enabling impact analysis and
    intelligent code navigation.
    """

    def __init__(self, ast_analyzer: Optional[ASTAnalyzer] = None):
        """
        Initialize the call graph analyzer

        Args:
            ast_analyzer: Optional ASTAnalyzer instance
        """
        self.ast_analyzer = ast_analyzer or ASTAnalyzer()
        self._cache: Dict[str, FileAnalysis] = {}

    def build_file_graph(self, path: str) -> Optional[CallGraph]:
        """
        Build call graph for a single file

        Args:
            path: Path to the file

        Returns:
            CallGraph or None if analysis fails
        """
        analysis = self.ast_analyzer.analyze_file(path)
        if not analysis:
            return None

        graph = CallGraph()

        # Add functions to graph
        for func_name, func_info in analysis.functions.items():
            key = f"{path}:{func_name}"
            node = FunctionNode(
                name=func_name,
                file=path,
                line=func_info.line,
                is_method=func_info.is_method,
            )
            graph.functions[key] = node
            graph.file_map[func_name] = key

        # Add class methods
        for cls_name, cls_info in analysis.classes.items():
            for method_name, method_info in cls_info.methods.items():
                key = f"{path}:{cls_name}.{method_name}"
                node = FunctionNode(
                    name=method_name,
                    file=path,
                    line=method_info.line,
                    is_method=True,
                    class_name=cls_name,
                )
                graph.functions[key] = node
                graph.file_map[f"{cls_name}.{method_name}"] = key

        # Add edges based on function calls
        for func_name, func_info in analysis.functions.items():
            caller_key = f"{path}:{func_name}"
            for call_name in func_info.calls:
                # Check if it's a local call
                callee_key = f"{path}:{call_name}"
                if callee_key in graph.functions:
                    caller_node = graph.functions[caller_key]
                    caller_node.calls.add(callee_key)
                    graph.edges.append(CallEdge(
                        caller=func_name,
                        callee=call_name,
                        caller_file=path,
                        callee_file=path,
                    ))

        return graph

    def build_project_graph(
        self,
        root_path: str,
        pattern: str = "*.py",
        exclude_dirs: Optional[List[str]] = None,
    ) -> CallGraph:
        """
        Build call graph for an entire project

        Args:
            root_path: Root directory of the project
            pattern: File pattern to include
            exclude_dirs: Directories to exclude

        Returns:
            Complete CallGraph for the project
        """
        if exclude_dirs is None:
            exclude_dirs = ["__pycache__", ".venv", "venv", "node_modules", ".git"]

        graph = CallGraph()
        root_path = os.path.abspath(root_path)

        # First pass: collect all files and their analyses
        file_analyses: Dict[str, FileAnalysis] = {}
        for py_file in Path(root_path).rglob(pattern):
            file_path = str(py_file)

            # Skip excluded directories
            if any(excl in file_path for excl in exclude_dirs):
                continue

            analysis = self.ast_analyzer.analyze_file(file_path)
            if analysis:
                file_analyses[file_path] = analysis

        # Second pass: build graph with cross-file references
        for file_path, analysis in file_analyses.items():
            # Add functions
            for func_name, func_info in analysis.functions.items():
                key = f"{file_path}:{func_name}"
                node = FunctionNode(
                    name=func_name,
                    file=file_path,
                    line=func_info.line,
                    is_method=func_info.is_method,
                )
                node.calls = func_info.calls
                graph.functions[key] = node
                graph.file_map[func_name] = key

            # Add class methods
            for cls_name, cls_info in analysis.classes.items():
                for method_name, method_info in cls_info.methods.items():
                    key = f"{file_path}:{cls_name}.{method_name}"
                    node = FunctionNode(
                        name=method_name,
                        file=file_path,
                        line=method_info.line,
                        is_method=True,
                        class_name=cls_name,
                    )
                    node.calls = method_info.calls
                    graph.functions[key] = node
                    graph.file_map[f"{cls_name}.{method_name}"] = key

        # Build cross-file edges based on imports
        for file_path, analysis in file_analyses.items():
            # Map imports to files
            import_map = self._resolve_imports(analysis.imports, root_path, file_analyses)

            # Connect cross-file calls
            for func_name, func_info in analysis.functions.items():
                caller_key = f"{file_path}:{func_name}"
                caller_node = graph.functions.get(caller_key)

                if caller_node:
                    for call_name in func_info.calls:
                        # Find the actual file for this call
                        for imported_module, target_file in import_map.items():
                            if call_name.startswith(imported_module) or f"{imported_module}." in call_name:
                                callee_key = f"{target_file}:{call_name}"
                                if callee_key in graph.functions:
                                    caller_node.calls.add(callee_key)
                                    graph.edges.append(CallEdge(
                                        caller=func_name,
                                        callee=call_name,
                                        caller_file=file_path,
                                        callee_file=target_file,
                                    ))

        return graph

    def _resolve_imports(
        self,
        imports: List[ImportInfo],
        root_path: str,
        file_analyses: Dict[str, FileAnalysis],
    ) -> Dict[str, str]:
        """Resolve import statements to actual file paths"""
        import_map = {}

        for imp in imports:
            module_path = imp.module.replace(".", os.sep)
            possible_paths = [
                os.path.join(root_path, f"{module_path}.py"),
                os.path.join(root_path, module_path, "__init__.py"),
            ]

            for path in possible_paths:
                if path in file_analyses:
                    import_map[imp.module] = path
                    if imp.name:
                        import_map[f"{imp.module}.{imp.name}"] = path
                    break

        return import_map

    def get_intra_file_calls(self, path: str) -> Dict[str, Set[str]]:
        """
        Get intra-file call relationships

        Args:
            path: Path to the file

        Returns:
            Dictionary mapping function names to sets of called functions
        """
        graph = self.build_file_graph(path)
        if not graph:
            return {}

        result = {}
        for key, node in graph.functions.items():
            func_name = key.split(":")[-1]
            result[func_name] = {c.split(":")[-1] for c in node.calls}

        return result

    def analyze_impact(
        self,
        function_name: str,
        root_path: str,
        depth: int = 3,
    ) -> Dict[str, Any]:
        """
        Analyze the impact of changing a function

        Args:
            function_name: Name of the function to analyze
            root_path: Project root path
            depth: Maximum depth of impact analysis

        Returns:
            Dictionary with impact information
        """
        graph = self.build_project_graph(root_path)

        # Find all nodes matching the function name
        matching_nodes = [
            (key, node) for key, node in graph.functions.items()
            if key.endswith(f":{function_name}") or node.name == function_name
        ]

        if not matching_nodes:
            return {"function": function_name, "impact": [], "callers": [], "callees": []}

        # Build impact tree
        impact = []
        all_callers = set()
        all_callees = set()

        for key, node in matching_nodes:
            # Find what calls this function (impact of change)
            callers = self._find_impact_chain(graph, key, direction="callers", max_depth=depth)
            all_callers.update(callers)

            # Find what this function calls (ripple effect)
            callees = self._find_impact_chain(graph, key, direction="callees", max_depth=depth)
            all_callees.update(callees)

            impact.append({
                "location": key,
                "callers": list(callers),
                "callees": list(callees),
            })

        return {
            "function": function_name,
            "matching_locations": len(matching_nodes),
            "impact": impact,
            "all_callers": sorted(all_callers),
            "all_callees": sorted(all_callees),
        }

    def _find_impact_chain(
        self,
        graph: CallGraph,
        start_key: str,
        direction: str = "callers",
        max_depth: int = 3,
    ) -> Set[str]:
        """Find impact chain using BFS"""
        result = set()
        visited = {start_key}
        queue = [(start_key, 0)]

        while queue:
            current, depth = queue.pop(0)
            if depth >= max_depth:
                continue

            if direction == "callers":
                # Find who calls current
                for key, node in graph.functions.items():
                    if current in node.calls:
                        if key not in visited:
                            visited.add(key)
                            result.add(key)
                            queue.append((key, depth + 1))
            else:
                # Find what current calls
                current_node = graph.functions.get(current)
                if current_node:
                    for call_key in current_node.calls:
                        if call_key not in visited:
                            visited.add(call_key)
                            result.add(call_key)
                            queue.append((call_key, depth + 1))

        return result

    def find_entry_points(self, root_path: str) -> List[str]:
        """
        Find potential entry points in the project

        Entry points are functions that are not called by other functions
        in the project (likely called externally or are main functions).

        Args:
            root_path: Project root path

        Returns:
            List of entry point function names
        """
        graph = self.build_project_graph(root_path)

        # Find functions that are never called (within the project)
        all_called = set()
        for node in graph.functions.values():
            all_called.update(node.calls)

        entry_points = []
        for key, node in graph.functions.items():
            # Skip if called by someone else in the project
            if key not in all_called:
                # Also check if it looks like an entry point
                func_name = node.name
                if func_name in ("main", "run", "start", "__init__", "setup"):
                    entry_points.append(f"{key}")

        return sorted(entry_points)

    def find_dead_code(
        self,
        root_path: str,
        entry_points: Optional[List[str]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Find potentially dead code (unreachable functions)

        Args:
            root_path: Project root path
            entry_points: Known entry points (auto-detected if None)

        Returns:
            List of potentially dead functions
        """
        graph = self.build_project_graph(root_path)

        if entry_points is None:
            entry_points = self.find_entry_points(root_path)

        if not entry_points:
            entry_points = [f"{root_path}:{name}" for name in ("main", "run", "start")]

        # Find reachable functions from entry points
        reachable = set(entry_points)
        queue = list(entry_points)

        while queue:
            current = queue.pop(0)
            node = graph.functions.get(current)
            if node:
                for call_key in node.calls:
                    if call_key not in reachable:
                        reachable.add(call_key)
                        queue.append(call_key)

        # Find unreachable functions
        dead_code = []
        for key, node in graph.functions.items():
            if key not in reachable:
                # Skip special methods
                if not node.name.startswith("__") or node.name.endswith("__"):
                    dead_code.append({
                        "function": node.name,
                        "location": key,
                        "file": node.file,
                        "line": node.line,
                        "is_method": node.is_method,
                    })

        return sorted(dead_code, key=lambda x: (x["file"], x["line"]))

    def detect_cycles(self, root_path: str) -> List[List[str]]:
        """
        Detect circular dependencies in the call graph

        Args:
            root_path: Project root path

        Returns:
            List of cycles (each cycle is a list of function keys)
        """
        graph = self.build_project_graph(root_path)

        # Use DFS to detect cycles
        cycles = []
        visited = set()
        rec_stack = set()
        path = []

        def dfs(key: str) -> bool:
            visited.add(key)
            rec_stack.add(key)
            path.append(key)

            node = graph.functions.get(key)
            if node:
                for call_key in node.calls:
                    if call_key not in visited:
                        if dfs(call_key):
                            return True
                    elif call_key in rec_stack:
                        # Found a cycle
                        cycle_start = path.index(call_key)
                        cycles.append(path[cycle_start:] + [call_key])
                        return True

            path.pop()
            rec_stack.remove(key)
            return False

        for key in graph.functions:
            if key not in visited:
                dfs(key)

        return cycles

    def to_llm_string(self, graph: CallGraph, max_functions: int = 50) -> str:
        """
        Convert call graph to LLM-friendly string

        Args:
            graph: CallGraph to convert
            max_functions: Maximum functions to include

        Returns:
            Compact string representation
        """
        lines = [f"# Call Graph ({len(graph.functions)} functions)"]

        # Group by file
        by_file: Dict[str, List[FunctionNode]] = defaultdict(list)
        for key, node in graph.functions.items():
            by_file[node.file].append(node)

        # Limit output
        file_count = 0
        for file_path, nodes in sorted(by_file.items()):
            if file_count >= 10:
                lines.append(f"... and {len(by_file) - file_count} more files")
                break

            rel_path = os.path.relpath(file_path) if file_path != os.path.abspath(file_path) else file_path
            lines.append(f"\n## {rel_path}")

            for func_node in sorted(nodes, key=lambda n: n.line)[:max_functions]:
                method_prefix = f"{func_node.class_name}." if func_node.class_name else ""
                if func_node.calls:
                    called = [c.split(":")[-1] for c in list(func_node.calls)[:5]]
                    more = f" +{len(func_node.calls) - 5}" if len(func_node.calls) > 5 else ""
                    lines.append(f"  {method_prefix}{func_node.name} -> [{', '.join(called)}{more}]")
                else:
                    lines.append(f"  {method_prefix}{func_node.name} (leaf)")

            file_count += 1

        return "\n".join(lines)
