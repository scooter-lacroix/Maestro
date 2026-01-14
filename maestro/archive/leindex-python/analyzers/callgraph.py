import os
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Dict, Set, Any
from pathlib import Path
from collections import defaultdict

from .base import BaseAnalyzer
from .ast import ASTAnalyzer

@dataclass
class CallEdge:
    """Represents a call relationship between functions"""
    caller: str  # Function name
    callee: str  # Function being called
    caller_file: str
    callee_file: Optional[str] = None
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

class CallGraphAnalyzer(BaseAnalyzer):
    """
    Call Graph Analyzer for understanding code relationships.
    Layer 2 of analysis.
    """

    def __init__(self, ast_analyzer: Optional[ASTAnalyzer] = None):
        self.ast_analyzer = ast_analyzer or ASTAnalyzer()

    def analyze(self, code: str, file_path: str) -> Dict[str, Any]:
        """
        Analyze code to build a single-file call graph.
        Note: This only captures intra-file dependencies.
        For full project analysis, use build_project_graph.
        """
        graph = self.build_file_graph(file_path, code)
        if not graph:
            return {}

        return self._graph_to_dict(graph)

    def build_file_graph(self, path: str, code: Optional[str] = None) -> Optional[CallGraph]:
        """Build call graph for a single file"""
        if code:
            analysis = self.ast_analyzer.analyze(code, path)
        else:
            # If no code provided, read from file
            try:
                with open(path, "r", encoding="utf-8") as f:
                    code_content = f.read()
                analysis = self.ast_analyzer.analyze(code_content, path)
            except Exception:
                return None

        if not analysis or "error" in analysis:
            return None

        graph = CallGraph()

        # Add functions to graph
        for func_name, func_info in analysis.get("functions", {}).items():
            key = f"{path}:{func_name}"
            node = FunctionNode(
                name=func_name,
                file=path,
                line=func_info["line"],
                is_method=func_info["is_method"],
            )
            graph.functions[key] = node
            graph.file_map[func_name] = key

        # Add class methods
        for cls_name, cls_info in analysis.get("classes", {}).items():
            for method_name, method_info in cls_info.get("methods", {}).items():
                key = f"{path}:{cls_name}.{method_name}"
                node = FunctionNode(
                    name=method_name,
                    file=path,
                    line=method_info["line"],
                    is_method=True,
                    class_name=cls_name,
                )
                graph.functions[key] = node
                graph.file_map[f"{cls_name}.{method_name}"] = key

        # Add edges based on function calls (Intra-file only at this stage)
        for func_name, func_info in analysis.get("functions", {}).items():
            caller_key = f"{path}:{func_name}"
            for call_name in func_info.get("calls", []):
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

        # Check methods calls as well
        for cls_name, cls_info in analysis.get("classes", {}).items():
            for method_name, method_info in cls_info.get("methods", {}).items():
                caller_key = f"{path}:{cls_name}.{method_name}"
                for call_name in method_info.get("calls", []):
                    # Check if local function/method
                    callee_key = f"{path}:{call_name}"
                    # Also check for class methods if not found directly
                    if callee_key not in graph.functions:
                         # Try finding it as a method of the same class (self.method)
                         # This is a simplification; deeper resolution needs type inference
                         pass

                    if callee_key in graph.functions:
                        caller_node = graph.functions[caller_key]
                        caller_node.calls.add(callee_key)
                        graph.edges.append(CallEdge(
                            caller=method_name,
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
        """Build call graph for an entire project"""
        if exclude_dirs is None:
            exclude_dirs = ["__pycache__", ".venv", "venv", "node_modules", ".git"]

        graph = CallGraph()
        root_path = os.path.abspath(root_path)

        # First pass: collect all files and their analyses
        file_analyses: Dict[str, Dict[str, Any]] = {}
        for py_file in Path(root_path).rglob(pattern):
            file_path = str(py_file)
            if any(excl in file_path for excl in exclude_dirs):
                continue

            try:
                with open(file_path, "r", encoding="utf-8") as f:
                    code = f.read()
                analysis = self.ast_analyzer.analyze(code, file_path)
                if analysis and "error" not in analysis:
                    file_analyses[file_path] = analysis
            except Exception:
                continue

        # Second pass: build graph nodes
        for file_path, analysis in file_analyses.items():
            # Add functions
            for func_name, func_info in analysis.get("functions", {}).items():
                key = f"{file_path}:{func_name}"
                node = FunctionNode(
                    name=func_name,
                    file=file_path,
                    line=func_info["line"],
                    is_method=func_info["is_method"],
                )
                node.calls = set(func_info.get("calls", [])) # Temporary store raw names
                graph.functions[key] = node
                graph.file_map[func_name] = key

            # Add class methods
            for cls_name, cls_info in analysis.get("classes", {}).items():
                for method_name, method_info in cls_info.get("methods", {}).items():
                    key = f"{file_path}:{cls_name}.{method_name}"
                    node = FunctionNode(
                        name=method_name,
                        file=file_path,
                        line=method_info["line"],
                        is_method=True,
                        class_name=cls_name,
                    )
                    node.calls = set(method_info.get("calls", [])) # Temporary store raw names
                    graph.functions[key] = node
                    graph.file_map[f"{cls_name}.{method_name}"] = key

        # Third pass: resolve calls (edges)
        for file_path, analysis in file_analyses.items():
            import_map = self._resolve_imports(analysis.get("imports", []), root_path, file_analyses)

            # Iterate all nodes in this file
            nodes_in_file = [node for node in graph.functions.values() if node.file == file_path]

            for node in nodes_in_file:
                raw_calls = list(node.calls)
                node.calls = set() # Reset to store resolved keys

                for call_name in raw_calls:
                    # 1. Check local definition
                    local_key = f"{file_path}:{call_name}"
                    if local_key in graph.functions:
                        node.calls.add(local_key)
                        graph.edges.append(CallEdge(node.name, call_name, file_path, file_path))
                        continue

                    # 2. Check imported definition
                    # Simple resolution: if call_name matches imported module alias or name
                    # logic:
                    # import utils -> utils.helper() -> call_name="utils.helper"
                    # from utils import helper -> helper() -> call_name="helper"

                    target_file = None
                    resolved_call_name = call_name

                    # Case: call is "utils.helper" and we have "import utils"
                    if "." in call_name:
                        parts = call_name.split(".")
                        module_part = parts[0]
                        func_part = parts[1] # Simplification
                        if module_part in import_map:
                            target_file = import_map[module_part]
                            resolved_call_name = func_part

                    # Case: call is "helper" and we have "from utils import helper"
                    # import_map might have key "utils.helper" -> path if we mapped it that way
                    # In _resolve_imports we map:
                    # module -> path
                    # module.name -> path

                    # But if we have 'from utils import helper', the AST analyzer sees 'helper' in calls.
                    # import_map["utils"] = path/to/utils.py
                    # We need to find if 'helper' was imported.

                    # Let's look at imports in analysis again
                    imports = analysis.get("imports", [])
                    for imp in imports:
                        # from X import Y as Z
                        # call Z()
                        local_alias = imp.get("alias") or imp.get("name")

                        if local_alias == call_name:
                             # This call matches an import
                             # Determine target file
                             module = imp.get("module")
                             if module in import_map:
                                 target_file = import_map[module]
                                 if imp.get("name"):
                                     resolved_call_name = imp.get("name")
                                 # If it's just 'import module', we shouldn't be here because call_name matches alias 'module'
                                 # but usually we call module.func()

                    if target_file:
                        target_key = f"{target_file}:{resolved_call_name}"
                        if target_key in graph.functions:
                            node.calls.add(target_key)
                            graph.edges.append(CallEdge(node.name, resolved_call_name, file_path, target_file))

        return graph

    def _resolve_imports(
        self,
        imports: List[Dict[str, Any]],
        root_path: str,
        file_analyses: Dict[str, Any],
    ) -> Dict[str, str]:
        """Resolve import statements to actual file paths"""
        import_map = {}

        for imp in imports:
            module = imp["module"]
            if not module: continue

            module_path = module.replace(".", os.sep)
            possible_paths = [
                os.path.join(root_path, f"{module_path}.py"),
                os.path.join(root_path, module_path, "__init__.py"),
            ]

            for path in possible_paths:
                if path in file_analyses:
                    import_map[module] = path
                    break

        return import_map

    def _graph_to_dict(self, graph: CallGraph) -> Dict[str, Any]:
        return {
            "functions": {k: asdict(v) for k, v in graph.functions.items()},
            "edges": [asdict(e) for e in graph.edges],
            "file_map": graph.file_map
        }

    def to_llm_string(self, graph_or_dict: Any) -> str:
        """
        Convert call graph to LLM-friendly string.
        Accepts either CallGraph object or dictionary.
        """
        if isinstance(graph_or_dict, dict):
            # Basic dict representation if we only have the dict
            # Reconstruct minimal object or just print
            functions = graph_or_dict.get("functions", {})
        else:
            functions = {k: asdict(v) for k, v in graph_or_dict.functions.items()}

        lines = [f"# Call Graph ({len(functions)} functions)"]

        # Group by file
        by_file = defaultdict(list)
        for key, node in functions.items():
            by_file[node["file"]].append(node)

        for file_path, nodes in sorted(by_file.items()):
            rel_path = os.path.basename(file_path) # Simplify path for LLM
            lines.append(f"\n## {rel_path}")

            for node in sorted(nodes, key=lambda n: n["line"]):
                name = node["name"]
                if node.get("class_name"):
                    name = f"{node['class_name']}.{name}"

                calls = node.get("calls", [])
                if calls:
                    # calls is a list of keys "file:func"
                    # simplify to just func name for display
                    called_names = [c.split(":")[-1] for c in calls if isinstance(c, str)]
                    called_str = ", ".join(called_names[:5])
                    if len(called_names) > 5:
                        called_str += "..."
                    lines.append(f"  {name} -> [{called_str}]")
                else:
                    lines.append(f"  {name}")

        return "\n".join(lines)
