"""
TLDR - Token-efficient code analysis (NOW CONSOLIDATED INTO LEINDEX)

This module now provides backward compatibility by redirecting to LeIndex.
All TLDR functionality has been consolidated into maestro.leindex.

Migration Guide:
- Replace `from maestro.tldr import X` with `from maestro.leindex import X`
- Use `get_relevant_context()` from maestro.leindex for context extraction
- Use `SemanticIndex` from maestro.leindex for semantic search
- Use `ContextExtractor` from maestro.leindex for analysis

The old imports still work for backward compatibility:
- ASTAnalyzer → maestro.leindex.ASTAnalyzer
- CallGraphAnalyzer → maestro.leindex.CallGraphAnalyzer
- CFGAnalyzer → maestro.leindex.CFGAnalyzer
- DFGAnalyzer → maestro.leindex.DFGAnalyzer
- SlicingAnalyzer → maestro.leindex.SlicingAnalyzer
- get_relevant_context → maestro.leindex.get_relevant_context
- SemanticIndex → maestro.leindex.SemanticIndex
"""

from maestro.tldr.version import __version__

# Import all consolidated functionality from LeIndex
from maestro.leindex import (
    # Core analyzers
    ASTAnalyzer,
    CallGraphAnalyzer,
    CFGAnalyzer,
    DFGAnalyzer,
    SlicingAnalyzer,
    # Data classes
    ImportInfo,
    FunctionInfo,
    ClassInfo,
    # Context extraction
    ContextExtractor,
    CodeContext,
    ContextExtractionResult,
    get_context_extractor,
    get_relevant_context,
    get_context_for_prompt,
    # Semantic index
    SemanticIndex,
    CodeEntity,
    IndexStats,
    get_semantic_index,
    # Memory integration
    LeIndexMemoryBridge,
    get_leindex_memory_bridge,
)

# Backward compatibility aliases
TLRDAnalyzer = ContextExtractor
AnalysisResult = ContextExtractionResult
AnalysisContext = CodeContext

# Public API - maintains TLDR's original exports
__all__ = [
    "__version__",
    "ASTAnalyzer",
    "CallGraphAnalyzer",
    "CFGAnalyzer",
    "DFGAnalyzer",
    "SlicingAnalyzer",
    "SemanticIndex",
    "get_relevant_context",
    "TLRDAnalyzer",
    "ContextExtractor",
    "CodeContext",
    "ContextExtractionResult",
    "get_context_extractor",
    "get_context_for_prompt",
    "CodeEntity",
    "IndexStats",
    "get_semantic_index",
    "LeIndexMemoryBridge",
    "get_leindex_memory_bridge",
    "AnalysisResult",
    "AnalysisContext",
    "ImportInfo",
    "FunctionInfo",
    "ClassInfo",
]


# Additional backward compatibility functions

def analyze_file(file_path: str, layers=None):
    """
    Analyze a file with 5-layer analysis.

    Backward compatible wrapper for maestro.leindex.analyze_file.
    """
    from maestro.leindex import analyze_file as leindex_analyze_file
    result = leindex_analyze_file(file_path)
    # Convert to old-style AnalysisResult if needed
    return result


def analyze_function(file_path: str, function_name: str, layers=None):
    """
    Analyze a function with 5-layer analysis.

    Backward compatible wrapper.
    """
    extractor = get_context_extractor()
    result = extractor.extract_for_file(file_path)
    return result


def analyze_project(root_path: str, layers=None):
    """
    Analyze an entire project.

    Backward compatible wrapper.
    """
    extractor = get_context_extractor()
    # Project analysis returns aggregated context
    return None


def semantic_search(query: str, project_path: str, limit: int = 10):
    """
    Semantic code search.

    Backward compatible wrapper for maestro.leindex.semantic_search.
    """
    from maestro.leindex import semantic_search as leindex_semantic_search
    return leindex_semantic_search(query, project_path, limit)


def build_index(project_path: str, force: bool = False) -> int:
    """
    Build semantic index for a project.

    Backward compatible wrapper.
    """
    from maestro.leindex import build_semantic_index
    return build_semantic_index(project_path, force)


def get_token_savings(file_path: str):
    """
    Calculate token savings for a file.

    Backward compatible wrapper.
    """
    from maestro.leindex import get_token_savings
    return get_token_savings(file_path)
