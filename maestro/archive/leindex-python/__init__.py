"""
LeIndex - Maestro's Unified Code Intelligence System

Consolidates TLDR's token-efficient code analysis with LeIndex's search capabilities.
This is the primary module for code analysis, search, and context extraction in Maestro.

Features:
- 5-layer code analysis (AST, Call Graph, CFG, DFG, Slicing)
- Token-efficient context extraction (90%+ token reduction)
- Semantic code search using embeddings
- Full-text search via Tantivy
- Memory integration for persistent code insights
- MCP server integration

Usage:
    from maestro.leindex import (
        ASTAnalyzer,
        ContextExtractor,
        get_relevant_context,
        SemanticIndex,
    )

    # Extract token-efficient context
    context = get_relevant_context("/path/to/project", "main.py")
    print(context.to_llm_string())

    # Semantic search
    index = SemanticIndex()
    index.index_project("/path/to/project")
    results = index.search("authentication functions")
"""

__version__ = "2.0.0"

# Core analyzers (5-layer analysis)
from .analyzers.base import BaseAnalyzer
from .analyzers.ast import ASTAnalyzer, ImportInfo, FunctionInfo, ClassInfo
from .analyzers.callgraph import CallGraphAnalyzer
from .analyzers.cfg import CFGAnalyzer
from .analyzers.dfg import DFGAnalyzer
from .analyzers.slicing import SlicingAnalyzer

# Context extraction (token-efficient)
from .context_extraction import (
    ContextExtractor,
    CodeContext,
    ContextExtractionResult,
    get_context_extractor,
    get_relevant_context,
    get_context_for_prompt,
)

# Semantic index (natural language search)
from .semantic_index import (
    SemanticIndex,
    CodeEntity,
    IndexStats,
    get_semantic_index,
)

# Memory integration
from .memory_integration import (
    LeIndexMemoryBridge,
    get_leindex_memory_bridge,
)

# Storage and search
from .storage.storage_interface import DALInterface
from .storage.dal_factory import get_dal_instance

# Public API
__all__ = [
    # Version
    "__version__",

    # Core analyzers
    "BaseAnalyzer",
    "ASTAnalyzer",
    "CallGraphAnalyzer",
    "CFGAnalyzer",
    "DFGAnalyzer",
    "SlicingAnalyzer",

    # Data classes
    "ImportInfo",
    "FunctionInfo",
    "ClassInfo",

    # Context extraction
    "ContextExtractor",
    "CodeContext",
    "ContextExtractionResult",
    "get_context_extractor",
    "get_relevant_context",
    "get_context_for_prompt",

    # Semantic index
    "SemanticIndex",
    "CodeEntity",
    "IndexStats",
    "get_semantic_index",

    # Memory integration
    "LeIndexMemoryBridge",
    "get_leindex_memory_bridge",

    # Storage
    "DALInterface",
    "get_dal_instance",
]


# Backward compatibility aliases (TLDR names mapped to LeIndex)
TLDRAnalyzer = ContextExtractor  # Main entry point for analysis
AnalysisResult = ContextExtractionResult  # Result of analysis
AnalysisContext = CodeContext  # Context for analysis


def analyze_file(file_path: str, include_call_graph: bool = True) -> ContextExtractionResult:
    """
    Analyze a file with 5-layer analysis and return token-efficient context.

    Convenience function that combines AST and call graph analysis.

    Args:
        file_path: Path to the file
        include_call_graph: Whether to include call graph analysis

    Returns:
        ContextExtractionResult with context and token statistics
    """
    extractor = get_context_extractor()
    return extractor.extract_for_file(file_path, include_call_graph)


def semantic_search(query: str, project_path: str, limit: int = 10):
    """
    Search code using natural language.

    Convenience function for semantic code search.

    Args:
        query: Natural language query
        project_path: Path to the project
        limit: Maximum results

    Returns:
        List of (CodeEntity, score) tuples
    """
    index = get_semantic_index()

    # Build index if needed
    stats = index.get_stats()
    if stats.total_entities == 0:
        index.index_project(project_path)
        index.save()

    return index.search(query, limit=limit)


def build_semantic_index(project_path: str, force: bool = False) -> int:
    """
    Build semantic index for a project.

    Args:
        project_path: Path to the project
        force: Force rebuild

    Returns:
        Number of entities indexed
    """
    index = get_semantic_index()

    if force:
        index.clear()

    count = index.index_project(project_path)
    index.save()

    return count


def get_token_savings(file_path: str) -> dict:
    """
    Calculate token savings for a file using LeIndex analysis.

    Args:
        file_path: Path to the file

    Returns:
        Dictionary with token usage statistics
    """
    result = analyze_file(file_path)
    if result:
        return {
            "file": file_path,
            "raw_tokens": result.raw_tokens,
            "context_tokens": result.context_tokens,
            "savings_percent": result.savings_percent,
            "token_ratio": result.token_ratio,
        }
    return {
        "file": file_path,
        "raw_tokens": 0,
        "context_tokens": 0,
        "savings_percent": 0.0,
        "token_ratio": 0.0,
    }
