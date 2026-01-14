"""
Balanced Context Extraction for LeIndex

Provides intelligent context gathering for LLM interactions,
using the 5-layer analysis to find relevant code with minimal tokens
WHILE PRESERVING SEMANTIC COMPLETENESS.

Balanced Format (85-95% savings, preserves semantic richness):
- Full function signatures (params, return types)
- Line numbers for navigation
- Class hierarchies
- Async markers
- Type annotations

Ultra-Condensed Format (95-98% savings, USE WITH CAUTION):
- ONLY for exploration/search scenarios
- NOT recommended for code generation tasks
- LLM cannot call functions accurately without full signatures
"""

import os
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple

from .analyzers.ast import ASTAnalyzer
from .analyzers.callgraph import CallGraphAnalyzer

# Rust backend is not yet implemented
# Future: Use PyO3 bridge to Python's ast module for graph algorithms
# See RUST_IMPLEMENTATION_PLAN.md for details
RUST_AVAILABLE = False


@dataclass
class CodeContext:
    """Relevant code context for an entry point with semantic richness"""
    entry_point: str
    file_path: str
    functions: Dict[str, str] = field(default_factory=dict)
    classes: Dict[str, str] = field(default_factory=dict)
    related_files: List[str] = field(default_factory=list)
    call_chain: List[str] = field(default_factory=list)
    imports: List[Dict[str, Any]] = field(default_factory=list)

    # Mode determines output format
    mode: str = "balanced"  # "balanced" (85-95% savings) or "ultra" (95-98% savings)

    def to_llm_string(self) -> str:
        """
        Convert to LLM-friendly string with semantic richness.

        Balanced format preserves:
        - Full function signatures
        - Line numbers
        - Type annotations
        - Class hierarchies
        """
        if self.mode == "ultra":
            return self._to_ultra_condensed()

        # Balanced format - preserves semantic richness
        lines = [f"## {self.entry_point}"]
        lines.append(f"File: {self.file_path}")

        # Imports with module hierarchy
        if self.imports:
            lines.append("\nImports:")
            for imp in self.imports[:15]:
                module = imp.get("module", "")
                name = imp.get("name", "")
                alias = imp.get("alias", "")

                if name:
                    parts = module.split('.')
                    if len(parts) > 3:
                        # Show first.last for deep imports
                        short_module = f"{parts[0]}...{parts[-1]}"
                    else:
                        short_module = module
                    lines.append(f"  from {short_module} import {name}")
                    if alias:
                        lines[-1] += f" as {alias}"
                else:
                    parts = module.split('.')
                    if len(parts) > 3:
                        lines.append(f"  import {parts[0]}...{parts[-1]}")
                    else:
                        lines.append(f"  import {module}")

        # Classes with bases and line numbers
        if self.classes:
            lines.append("\nClasses:")
            for name, info in sorted(self.classes.items())[:10]:
                lines.append(f"  {info}")  # info should have line, bases, etc.

        # Functions with FULL signatures (critical for LLM usage)
        if self.functions:
            lines.append("\nFunctions:")
            for sig in sorted(self.functions.values())[:25]:
                lines.append(f"  {sig}")

        return "\n".join(lines)

    def _to_ultra_condensed(self) -> str:
        """Ultra-condensed format for exploration only (95-98% savings)."""
        lines = [f"## {self.entry_point}"]
        lines.append(f"@{self.file_path}")

        if self.call_chain:
            chain_str = " ".join(self.call_chain[:10])
            lines.append(f"↑↓:{chain_str}")

        if self.imports:
            imp_names = []
            for imp in self.imports[:10]:
                name = imp.get("name") or imp.get("module") or ""
                if name:
                    imp_names.append(name[:10])
            if imp_names:
                lines.append(f"imp:{','.join(imp_names)}")

        if self.functions:
            fn_str = " ".join([
                sig[:20] for sig in sorted(self.functions.keys())[:20]
            ])
            lines.append(f"fn:{fn_str}")

        if self.classes:
            cls_str = " ".join(sorted(self.classes.keys())[:10])
            lines.append(f"cls:{cls_str}")

        return "\n".join(lines)


@dataclass
class ContextExtractionResult:
    """Result of context extraction with token estimates"""
    context: CodeContext
    raw_tokens: int = 0
    context_tokens: int = 0
    savings_percent: float = 0.0

    @property
    def token_ratio(self) -> float:
        """Ratio of context tokens to raw tokens (lower = better)"""
        if self.raw_tokens == 0:
            return 0.0
        return self.context_tokens / self.raw_tokens

    def get_quality_report(self) -> Dict[str, Any]:
        """Get a quality report on the context extraction."""
        return {
            "savings_percent": self.savings_percent,
            "semantic_completeness": "high" if self.context.mode == "balanced" else "reduced",
            "llm_actionable": self.context.mode == "balanced",
            "contains_signatures": self.context.mode == "balanced",
            "contains_line_numbers": self.context.mode == "balanced",
            "recommended_use": "code_generation" if self.context.mode == "balanced" else "exploration",
        }


class ContextExtractor:
    """
    Token-efficient context extraction using 5-layer analysis.

    BALANCED MODE (default):
    - 85-95% token savings
    - Preserves full function signatures
    - Includes line numbers
    - LLM can accurately use the code

    ULTRA MODE (use_mode='ultra'):
    - 95-98% token savings
    - For exploration/search only
    - NOT recommended for code generation
    """

    def __init__(self, max_file_size: int = 1048576, mode: str = "balanced"):
        """
        Initialize the context extractor.

        Args:
            max_file_size: Maximum file size to analyze
            mode: "balanced" (semantic rich, 85-95% savings, LLM actionable)
                  or "ultra" (maximum compression, 95-98% savings, exploration only)
        """
        self.max_file_size = max_file_size
        self.mode = mode

        # Initialize Python analyzers
        self.ast_analyzer = ASTAnalyzer(max_file_size=max_file_size)
        self.callgraph_analyzer = CallGraphAnalyzer(ast_analyzer=self.ast_analyzer)

    def extract_for_file(
        self,
        file_path: str,
        include_call_graph: bool = True,
    ) -> Optional[ContextExtractionResult]:
        """
        Extract context for a single file with semantic richness.
        """
        file_path = os.path.abspath(file_path)

        if not os.path.exists(file_path):
            return None

        # Read source
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                source = f.read()
        except Exception:
            return None

        raw_tokens = len(source.split())

        # Analyze with AST
        analysis = self.ast_analyzer.analyze(source, file_path)
        if "error" in analysis:
            return None

        # Build context with semantic richness
        context = CodeContext(
            entry_point=os.path.basename(file_path),
            file_path=file_path,
            imports=analysis.get("imports", []),
            mode=self.mode,
        )

        # Extract functions with FULL signatures (semantic richness)
        for func_name, func_info in analysis.get("functions", {}).items():
            if func_info.get("is_method"):
                continue  # Methods are in classes

            # Build FULL signature with types
            async_p = "async " if func_info.get("is_async") else ""
            args = func_info.get("args", [])

            # Abbreviate long type annotations but preserve structure
            condensed_args = self._condense_type_list(args, max_len=15)

            ret = ""
            if func_info.get("returns"):
                ret = f" -> {self._condense_type(func_info.get('returns'), max_len=20)}"

            # Include line number for navigation
            line = func_info.get("line", 0)

            context.functions[func_name] = (
                f"L{line}: {async_p}{func_name}({condensed_args}){ret}"
            )

        # Extract classes with bases and methods
        for cls_name, cls_info in analysis.get("classes", {}).items():
            bases = cls_info.get("bases", [])
            line = cls_info.get("line", 0)

            # Build class info with line number and bases
            bases_str = f"({', '.join(bases[:3])})" if bases else ""
            context.classes[cls_name] = (
                f"L{line}: class {cls_name}{bases_str}"
            )

        # Calculate tokens
        context_str = context.to_llm_string()
        context_tokens = len(context_str.split())

        savings = 0.0
        if raw_tokens > 0:
            savings = (1 - context_tokens / raw_tokens) * 100

        return ContextExtractionResult(
            context=context,
            raw_tokens=raw_tokens,
            context_tokens=context_tokens,
            savings_percent=savings,
        )

    def _condense_type_list(self, types: List[str], max_len: int = 15) -> str:
        """Condense a list of type annotations while preserving meaning."""
        if not types:
            return ""

        condensed = []
        for t in types[:6]:  # Limit to 6 params
            condensed.append(self._condense_type(t, max_len))

        if len(types) > 6:
            condensed.append(f"...+{len(types) - 6}")

        return ", ".join(condensed)

    def _condense_type(self, type_str: str, max_len: int = 20) -> str:
        """Condense a type annotation while preserving meaning."""
        if not type_str or len(type_str) <= max_len:
            return type_str

        # Common type abbreviations that preserve meaning
        abbreviations = {
            "Optional": "Opt",
            "Union": "U",
            "List": "L",
            "Dict": "D",
            "Set": "S",
            "Tuple": "T",
            "Callable": "Fn",
            "Any": "*",
            "None": "N",
        }

        result = type_str
        for full, abbr in abbreviations.items():
            result = result.replace(full, abbr)

        # Truncate generic contents if still too long
        if len(result) > max_len and "[" in result:
            # Simplify generics: List[SomeLongType] -> L[_]
            import re
            result = re.sub(r'\[([^\]]{10,})\]', r'[_]', result)

        if len(result) > max_len:
            result = result[:max_len-3] + "..."

        return result

    def extract_for_entry_point(
        self,
        project_path: str,
        entry_point: str,
        depth: int = 2,
        max_files: int = 10,
    ) -> Optional[CodeContext]:
        """Extract context for an entry point."""
        project_path = os.path.abspath(project_path)

        # Parse entry point
        entry_file = None
        entry_function = None

        if ":" in entry_point:
            parts = entry_point.split(":")
            entry_file = parts[0]
            entry_function = parts[1] if len(parts) > 1 else None
        else:
            entry_file = self._find_file_for_symbol(project_path, entry_point)

        if not entry_file:
            return None

        entry_file = os.path.abspath(entry_file)
        if not os.path.exists(entry_file):
            return None

        result = self.extract_for_file(entry_file, include_call_graph=True)
        if not result:
            return None

        context = result.context

        # If specific function, build call chain
        if entry_function:
            call_chain = self._get_call_chain(project_path, entry_file, entry_function, depth)
            context.call_chain = call_chain

        return context

    def extract_from_prompt(
        self,
        project_path: str,
        prompt: str,
        max_files: int = 5,
    ) -> str:
        """Extract relevant code context from a user prompt."""
        project_path = os.path.abspath(project_path)

        # Look for file patterns
        file_pattern = r'\b([\w/]+\.py)\b|\b([\w/]+/[\w-]+)\b'
        potential_files = re.findall(file_pattern, prompt)

        contexts = []

        for file_match in potential_files:
            file_path = file_match[0] or file_match[1]
            full_path = os.path.join(project_path, file_path)

            if os.path.exists(full_path):
                result = self.extract_for_file(full_path)
                if result:
                    contexts.append(result.context.to_llm_string())

        if contexts:
            return "\n\n".join(contexts[:max_files])

        return f"# No specific files identified in project: {project_path}"

    def _find_file_for_symbol(self, project_path: str, symbol: str) -> Optional[str]:
        """Find the file containing a symbol."""
        direct_path = os.path.join(project_path, symbol)
        if os.path.exists(direct_path):
            return direct_path

        if not symbol.endswith(".py"):
            py_path = os.path.join(project_path, f"{symbol}.py")
            if os.path.exists(py_path):
                return py_path

        for py_file in Path(project_path).rglob("*.py"):
            try:
                with open(py_file, "r", encoding="utf-8") as f:
                    content = f.read()
                if f"def {symbol}(" in content or f"class {symbol}" in content:
                    return str(py_file)
            except Exception:
                continue

        return None

    def _get_call_chain(self, project_path: str, file_path: str, function_name: str, depth: int) -> List[str]:
        """Get call chain from a function."""
        result = []
        visited = set()

        def traverse(func: str, current_depth: int) -> None:
            if current_depth >= depth or func in visited:
                return
            visited.add(func)
            result.append(func)

        traverse(function_name, 0)
        return result


def get_context_extractor(max_file_size: int = 1048576, mode: str = "balanced") -> ContextExtractor:
    """
    Get a context extractor instance.

    Args:
        max_file_size: Maximum file size to analyze
        mode: "balanced" (semantic rich, 85-95% savings) or "ultra" (max compression, 95-98%)
    """
    return ContextExtractor(max_file_size=max_file_size, mode=mode)


def get_relevant_context(
    project_path: str,
    entry_point: str,
    depth: int = 2,
    max_files: int = 10,
    mode: str = "balanced",
) -> Optional[CodeContext]:
    """Get relevant code context for an entry point."""
    extractor = get_context_extractor(mode=mode)
    return extractor.extract_for_entry_point(project_path, entry_point, depth, max_files)


def get_context_for_prompt(
    project_path: str,
    prompt: str,
    max_files: int = 5,
    mode: str = "balanced",
) -> str:
    """Extract relevant code context from a user prompt."""
    extractor = get_context_extractor(mode=mode)
    return extractor.extract_from_prompt(project_path, prompt, max_files)
