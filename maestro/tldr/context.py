"""
Context extraction for TLDR analysis

Provides intelligent context gathering for LLM interactions,
using the 5-layer analysis to find relevant code with minimal tokens.
"""

import os
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Any, Tuple
from pathlib import Path

from maestro.tldr.analyzer import TLRDAnalyzer, AnalysisContext
from maestro.tldr.callgraph import CallGraph


@dataclass
class CodeContext:
    """Relevant code context for an entry point"""
    entry_point: str
    file_path: str
    functions: Dict[str, str] = field(default_factory=dict)  # name -> signature
    classes: Dict[str, str] = field(default_factory=dict)  # name -> summary
    related_files: List[str] = field(default_factory=list)
    call_chain: List[str] = field(default_factory=list)

    def to_llm_string(self) -> str:
        """Convert to LLM-friendly string"""
        lines = [f"## Context: {self.entry_point}"]
        lines.append(f"File: {self.file_path}")

        if self.call_chain:
            lines.append("\n### Call Chain:")
            lines.extend(f"  {c}" for c in self.call_chain)

        if self.functions:
            lines.append("\n### Functions:")
            for name, sig in sorted(self.functions.items()):
                lines.append(f"  {sig}")

        if self.classes:
            lines.append("\n### Classes:")
            for name, summary in sorted(self.classes.items()):
                lines.append(f"  {name}: {summary}")

        if self.related_files:
            lines.append("\n### Related Files:")
            for f in self.related_files[:10]:
                lines.append(f"  {f}")

        return "\n".join(lines)


def get_relevant_context(
    project_path: str,
    entry_point: str,
    depth: int = 2,
    max_files: int = 10,
    language: str = "python",
) -> Optional[CodeContext]:
    """
    Get relevant code context for an entry point

    Uses call graph traversal to find all functions and files
    that are relevant to understanding the entry point.

    Args:
        project_path: Root path of the project
        entry_point: Function or file to start from
        depth: Maximum depth of call graph traversal
        max_files: Maximum number of files to include
        language: Programming language

    Returns:
        CodeContext with relevant code
    """
    analyzer = TLRDAnalyzer()
    project_path = os.path.abspath(project_path)

    # Find the entry point file
    entry_file = None
    entry_function = None

    if ":" in entry_point:
        # Format: file:function or class.method
        parts = entry_point.split(":")
        entry_file = parts[0]
        entry_function = parts[1] if len(parts) > 1 else None
    else:
        # Search for file or function
        entry_file = _find_file_for_symbol(project_path, entry_point)
        if not entry_file:
            entry_function = entry_point

    if not entry_file:
        return None

    entry_file = os.path.abspath(entry_file)
    if not os.path.exists(entry_file):
        return None

    context = CodeContext(
        entry_point=entry_point,
        file_path=entry_file,
    )

    # Analyze the entry file
    ast_analysis = analyzer.ast_analyzer.analyze_file(entry_file)
    if ast_analysis:
        # Add functions
        for func_name, func_info in ast_analysis.functions.items():
            args = ", ".join(func_info.args)
            context.functions[func_name] = f"def {func_name}({args})"

        # Add classes
        for cls_name, cls_info in ast_analysis.classes.items():
            context.classes[cls_name] = f"{cls_name}({', '.join(cls_info.bases)})"

            # Add methods
            for method_name, method_info in cls_info.methods.items():
                args = ", ".join(method_info.args)
                full_name = f"{cls_name}.{method_name}"
                context.functions[full_name] = f"def {full_name}({args})"

    # Build call graph and traverse
    call_graph = analyzer.callgraph_analyzer.build_project_graph(project_path)

    if entry_function:
        # Find calls from this function
        call_chain = _get_call_chain(call_graph, entry_file, entry_function, depth)
        context.call_chain = call_chain

        # Add called functions to context
        for func_name in call_chain:
            if func_name not in context.functions:
                context.functions[func_name] = f"# {func_name} (external)"

        # Find what calls this function (impact analysis)
        callers = call_graph.get_callers(entry_function, entry_file)
        for caller in callers[:5]:
            related_file = caller.file
            if related_file != entry_file and related_file not in context.related_files:
                context.related_files.append(related_file)

    # Find related files through imports
    if ast_analysis:
        for imp in ast_analysis.imports:
            if imp.module:
                # Try to find the actual file
                related = _resolve_import_to_file(project_path, imp.module, entry_file)
                if related and related != entry_file:
                    context.related_files.append(related)

    # Limit related files
    context.related_files = context.related_files[:max_files]

    return context


def _find_file_for_symbol(project_path: str, symbol: str) -> Optional[str]:
    """Find the file containing a symbol"""
    # Try as direct file path
    direct_path = os.path.join(project_path, symbol)
    if os.path.exists(direct_path):
        return direct_path

    # Try with .py extension
    if not symbol.endswith(".py"):
        py_path = os.path.join(project_path, f"{symbol}.py")
        if os.path.exists(py_path):
            return py_path

    # Search for file containing the symbol
    for py_file in Path(project_path).rglob("*.py"):
        try:
            with open(py_file, "r", encoding="utf-8") as f:
                content = f.read()
            # Check if symbol is defined as a function or class
            if f"def {symbol}(" in content or f"class {symbol}" in content:
                return str(py_file)
        except Exception:
            continue

    return None


def _get_call_chain(
    call_graph: CallGraph,
    file_path: str,
    function_name: str,
    depth: int,
) -> List[str]:
    """Get call chain from a function"""
    result = []
    visited = set()

    def traverse(func: str, current_depth: int) -> None:
        if current_depth >= depth or func in visited:
            return
        visited.add(func)

        # Find the node
        key = f"{file_path}:{func}"
        node = call_graph.functions.get(key)

        if node:
            for call_key in node.calls:
                call_name = call_key.split(":")[-1]
                result.append(call_name)
                traverse(call_name, current_depth + 1)

    traverse(function_name, 0)
    return result


def _resolve_import_to_file(project_path: str, module: str, current_file: str) -> Optional[str]:
    """Resolve an import statement to a file path"""
    # Handle relative imports
    if module.startswith("."):
        base_dir = os.path.dirname(current_file)
        parts = module.split(".")
        for part in parts[1:]:
            if part:
                base_dir = os.path.join(base_dir, part)

        # Try as file
        file_path = f"{base_dir}.py"
        if os.path.exists(file_path):
            return file_path

        # Try as package
        init_path = os.path.join(base_dir, "__init__.py")
        if os.path.exists(init_path):
            return init_path

        return None

    # Handle absolute imports
    module_path = module.replace(".", os.sep)
    possible_paths = [
        os.path.join(project_path, f"{module_path}.py"),
        os.path.join(project_path, module_path, "__init__.py"),
    ]

    for path in possible_paths:
        if os.path.exists(path):
            return path

    return None


def get_context_for_prompt(
    project_path: str,
    prompt: str,
    max_files: int = 5,
) -> str:
    """
    Extract relevant code context from a user prompt

    Analyzes the prompt to identify mentioned files, functions,
    or classes and returns relevant context.

    Args:
        project_path: Root path of the project
        prompt: User's prompt
        max_files: Maximum number of files to analyze

    Returns:
        Formatted context string for LLM
    """
    # Extract potential symbols from prompt
    import re

    # Look for file patterns (path/to/file.py or path/to/file)
    file_pattern = r'\b([\w/]+\.py)\b|\b([\w/]+/[\w-]+)\b'
    potential_files = re.findall(file_pattern, prompt)

    # Look for function/class patterns (function_name or ClassName)
    symbol_pattern = r'\b[a-z][a-z_]+\b|\b[A-Z][a-zA-Z0-9]+\b'
    potential_symbols = re.findall(symbol_pattern, prompt)

    analyzer = TLRDAnalyzer()
    contexts = []

    for file_match in potential_files:
        file_path = file_match[0] or file_match[1]
        full_path = os.path.join(project_path, file_path)

        if os.path.exists(full_path):
            result = analyzer.analyze_file(full_path, layers=[1, 2])
            contexts.append(result.to_llm_string())

    # Limit contexts
    if contexts:
        return "\n\n".join(contexts[:max_files])

    return f"# No specific files identified in project: {project_path}"
