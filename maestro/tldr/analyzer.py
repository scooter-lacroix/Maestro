"""
Main TLDR Analyzer - Unified code analysis interface

Orchestrates all 5 layers of analysis to provide comprehensive
code understanding with minimal token usage.
"""

import os
from dataclasses import dataclass, field
from typing import Optional, List, Dict, Set, Any, Tuple
from pathlib import Path

from maestro.tldr.ast import ASTAnalyzer, FileAnalysis
from maestro.tldr.callgraph import CallGraphAnalyzer, CallGraph
from maestro.tldr.cfg import CFGAnalyzer, ControlFlowGraph
from maestro.tldr.dfg import DFGAnalyzer, DataFlowGraph
from maestro.tldr.slicing import SlicingAnalyzer, SliceResult
from maestro.tldr.semantic import SemanticIndex, CodeEntity


@dataclass
class AnalysisContext:
    """Context for code analysis"""
    project_path: str
    file_path: Optional[str] = None
    function_name: Optional[str] = None
    line_number: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "project_path": self.project_path,
            "file_path": self.file_path,
            "function_name": self.function_name,
            "line_number": self.line_number,
        }


@dataclass
class AnalysisResult:
    """Result of TLDR analysis"""
    context: AnalysisContext
    ast_analysis: Optional[FileAnalysis] = None
    call_graph: Optional[CallGraph] = None
    cfg: Optional[ControlFlowGraph] = None
    dfg: Optional[DataFlowGraph] = None
    slice_result: Optional[SliceResult] = None

    def token_estimate(self) -> Dict[str, int]:
        """Estimate token usage for this analysis"""
        total = 0

        # Layer 1: AST (~500 tokens)
        ast_tokens = 500 if self.ast_analysis else 0
        total += ast_tokens

        # Layer 2: Call Graph (~440 tokens)
        cg_tokens = 440 if self.call_graph else 0
        total += cg_tokens

        # Layer 3: CFG (~110 tokens)
        cfg_tokens = 110 if self.cfg else 0
        total += cfg_tokens

        # Layer 4: DFG (~130 tokens)
        dfg_tokens = 130 if self.dfg else 0
        total += dfg_tokens

        # Layer 5: Slicing (~150 tokens)
        slice_tokens = 150 if self.slice_result else 0
        total += slice_tokens

        return {
            "ast": ast_tokens,
            "call_graph": cg_tokens,
            "cfg": cfg_tokens,
            "dfg": dfg_tokens,
            "slicing": slice_tokens,
            "total": total,
        }

    def to_llm_string(self, max_detail: bool = False) -> str:
        """Convert to LLM-friendly string"""
        lines = [f"# TLDR Analysis: {self.context.project_path}"]

        if self.context.file_path:
            lines.append(f"File: {self.context.file_path}")

        if self.context.function_name:
            lines.append(f"Function: {self.context.function_name}")

        lines.append("")

        # Token estimate
        tokens = self.token_estimate()
        lines.append(f"Token Estimate: {tokens['total']} (vs ~10,000+ raw)")

        # Layer 1: AST
        if self.ast_analysis:
            from maestro.tldr.ast import ASTAnalyzer
            analyzer = ASTAnalyzer()
            lines.append("\n## Layer 1: AST")
            lines.append(analyzer.to_llm_string(self.ast_analysis, max_detail))

        # Layer 2: Call Graph
        if self.call_graph:
            from maestro.tldr.callgraph import CallGraphAnalyzer
            cg_analyzer = CallGraphAnalyzer()
            lines.append("\n## Layer 2: Call Graph")
            lines.append(cg_analyzer.to_llm_string(self.call_graph))

        # Layer 3: CFG
        if self.cfg:
            from maestro.tldr.cfg import CFGAnalyzer
            cfg_analyzer = CFGAnalyzer()
            lines.append("\n## Layer 3: Control Flow")
            lines.append(cfg_analyzer.to_llm_string(self.cfg))

        # Layer 4: DFG
        if self.dfg:
            from maestro.tldr.dfg import DFGAnalyzer
            dfg_analyzer = DFGAnalyzer()
            lines.append("\n## Layer 4: Data Flow")
            lines.append(dfg_analyzer.to_llm_string(self.dfg))

        # Layer 5: Slicing
        if self.slice_result:
            lines.append("\n## Layer 5: Slicing")
            lines.append(f"Slice: {self.slice_result.get_summary()}")

        return "\n".join(lines)


class TLRDAnalyzer:
    """
    Main TLDR Analyzer

    Orchestrates all 5 layers of analysis:
    1. AST - Function signatures, imports, classes
    2. Call Graph - Cross-file function relationships
    3. CFG - Control flow complexity
    4. DFG - Variable definitions and uses
    5. Slicing - Program dependence analysis
    """

    def __init__(
        self,
        max_file_size: int = 1048576,
        index_path: Optional[str] = None,
    ):
        """
        Initialize the TLDR analyzer

        Args:
            max_file_size: Maximum file size to analyze
            index_path: Path for semantic index
        """
        self.max_file_size = max_file_size
        self.index_path = index_path

        # Layer analyzers
        self.ast_analyzer = ASTAnalyzer(max_file_size)
        self.callgraph_analyzer = CallGraphAnalyzer(self.ast_analyzer)
        self.cfg_analyzer = CFGAnalyzer()
        self.dfg_analyzer = DFGAnalyzer()
        self.slicing_analyzer = SlicingAnalyzer()

        # Semantic index
        self.semantic_index: Optional[SemanticIndex] = None

    def analyze_file(
        self,
        file_path: str,
        layers: Optional[List[int]] = None,
    ) -> AnalysisResult:
        """
        Analyze a file with specified layers

        Args:
            file_path: Path to the file
            layers: List of layer numbers (1-5), None for all

        Returns:
            AnalysisResult with requested layers
        """
        if layers is None:
            layers = [1, 2, 3, 4, 5]

        file_path = os.path.abspath(file_path)
        context = AnalysisContext(
            project_path=os.path.dirname(file_path),
            file_path=file_path,
        )

        result = AnalysisResult(context=context)

        # Layer 1: AST
        if 1 in layers:
            result.ast_analysis = self.ast_analyzer.analyze_file(file_path)

        # Layer 2: Call Graph (single file)
        if 2 in layers:
            result.call_graph = self.callgraph_analyzer.build_file_graph(file_path)

        return result

    def analyze_function(
        self,
        file_path: str,
        function_name: str,
        layers: Optional[List[int]] = None,
    ) -> AnalysisResult:
        """
        Analyze a function with specified layers

        Args:
            file_path: Path to the file
            function_name: Name of the function
            layers: List of layer numbers (1-5), None for all

        Returns:
            AnalysisResult with requested layers
        """
        if layers is None:
            layers = [1, 2, 3, 4, 5]

        file_path = os.path.abspath(file_path)
        context = AnalysisContext(
            project_path=os.path.dirname(file_path),
            file_path=file_path,
            function_name=function_name,
        )

        result = AnalysisResult(context=context)

        # Read source
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                source = f.read()
        except Exception:
            return result

        # Layer 1: AST
        if 1 in layers:
            result.ast_analysis = self.ast_analyzer.analyze_file(file_path)

        # Layer 3: CFG
        if 3 in layers:
            result.cfg = self.cfg_analyzer.analyze_function(source, function_name, file_path)

        # Layer 4: DFG
        if 4 in layers:
            result.dfg = self.dfg_analyzer.analyze_function(source, function_name, file_path)

        # Layer 5: Slicing (requires CFG and DFG)
        if 5 in layers and (result.cfg or result.dfg):
            from maestro.tldr.slicing import SliceDirection
            pdg = self.slicing_analyzer.build_pdg(source, function_name, file_path)
            if pdg:
                result.slice_result = SliceResult(
                    function_name=function_name,
                    target_line=0,
                    direction=SliceDirection.BOTH,
                )

        return result

    def analyze_project(
        self,
        root_path: str,
        layers: Optional[List[int]] = None,
    ) -> AnalysisResult:
        """
        Analyze an entire project

        Args:
            root_path: Root directory of the project
            layers: List of layer numbers (1-5), None for all

        Returns:
            AnalysisResult with requested layers
        """
        if layers is None:
            layers = [1, 2]

        root_path = os.path.abspath(root_path)
        context = AnalysisContext(project_path=root_path)

        result = AnalysisResult(context=context)

        # Layer 1: AST for all files
        if 1 in layers:
            pass  # Project-level AST aggregation

        # Layer 2: Project call graph
        if 2 in layers:
            result.call_graph = self.callgraph_analyzer.build_project_graph(root_path)

        return result

    def slice_at_line(
        self,
        file_path: str,
        function_name: str,
        line: int,
        direction: str = "backward",
    ) -> Optional[SliceResult]:
        """
        Perform program slice at a specific line

        Args:
            file_path: Path to the file
            function_name: Name of the function
            line: Line number
            direction: "backward", "forward", or "both"

        Returns:
            SliceResult or None
        """
        file_path = os.path.abspath(file_path)

        # Read source
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                source = f.read()
        except Exception:
            return None

        if direction == "backward":
            return self.slicing_analyzer.slice_backward(
                source, function_name, line, file_path
            )
        elif direction == "forward":
            return self.slicing_analyzer.slice_forward(
                source, function_name, line, file_path
            )
        else:
            # Both - combine results
            backward = self.slicing_analyzer.slice_backward(
                source, function_name, line, file_path
            )
            forward = self.slicing_analyzer.slice_forward(
                source, function_name, line, file_path
            )

            if backward and forward:
                backward.relevant_lines.update(forward.relevant_lines)
                backward.relevant_variables.update(forward.relevant_variables)
                return backward

            return backward or forward

    def get_semantic_index(self) -> SemanticIndex:
        """Get or create the semantic index"""
        if self.semantic_index is None:
            self.semantic_index = SemanticIndex(index_path=self.index_path)
            self.semantic_index.load()
        return self.semantic_index

    def semantic_search(
        self,
        query: str,
        project_path: str,
        limit: int = 10,
    ) -> List[Tuple[CodeEntity, float]]:
        """
        Search code using natural language

        Args:
            query: Natural language query
            project_path: Project root path
            limit: Maximum results

        Returns:
            List of (entity, score) tuples
        """
        index = self.get_semantic_index()

        # Check if index needs rebuilding
        stats = index.get_stats()
        if stats.total_entities == 0:
            index.index_project(project_path)
            index.save()

        return index.search(query, limit=limit)

    def build_index(
        self,
        project_path: str,
        force: bool = False,
    ) -> int:
        """
        Build semantic index for a project

        Args:
            project_path: Root directory
            force: Force rebuild

        Returns:
            Number of entities indexed
        """
        index = self.get_semantic_index()

        if force:
            index.clear()

        count = index.index_project(project_path)
        index.save()

        return count

    def get_token_savings(
        self,
        file_path: str,
    ) -> Dict[str, Any]:
        """
        Calculate token savings for a file

        Args:
            file_path: Path to the file

        Returns:
            Dictionary with token usage statistics
        """
        file_path = os.path.abspath(file_path)

        # Get raw file size
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                raw_content = f.read()
            raw_tokens = len(raw_content.split())
        except Exception:
            raw_tokens = 0

        # Analyze with TLDR
        result = self.analyze_file(file_path, layers=[1])
        tldr_tokens = result.token_estimate()["total"]

        savings_percent = 0.0
        if raw_tokens > 0:
            savings_percent = (1 - tldr_tokens / raw_tokens) * 100

        return {
            "file": file_path,
            "raw_tokens": raw_tokens,
            "tldr_tokens": tldr_tokens,
            "savings_percent": round(savings_percent, 1),
        }
