"""
LeIndex - Maestro's Unified Code Intelligence System

LeIndex provides fast, accurate code search and analysis capabilities for Maestro,
combining multiple search backends with sophisticated 5-layer code analysis.

Core Modules:
- search: Full-text and semantic search with ranking
- storage: Pluggable storage backends (SQLite, DuckDB, Tantivy)
- analyzers: 5-layer code analysis (AST, CallGraph, CFG, DFG, Slicing)
- mcp_server: MCP server integration for Maestro

Public API:
"""

# Version info
__version__ = "2.0.0"
__author__ = "Maestro Project"

# ============================================================================
# Core Configuration
# ============================================================================

from .tantivy_config import (
    TantivyConfig,
    tantivy_config,
    TANTIVY_AVAILABLE,
)

from .config_manager import ConfigManager
from .constants import SETTINGS_DIR

# ============================================================================
# Storage Backend
# ============================================================================

from .storage.dal_factory import get_dal_instance
from .storage.storage_interface import (
    DALInterface,
    StorageInterface,
    FileMetadataInterface,
    SearchInterface,
)

from .storage.sqlite_storage import (
    SQLiteDAL,
    SQLiteStorage,
    SQLiteFileMetadata,
    SQLiteSearch,
)

from .storage.duckdb_storage import DuckDBDAL

from .storage.tantivy_storage import (
    TantivySearch,
    TantivyNotAvailableError,
)

# ============================================================================
# Indexing
# ============================================================================

from .incremental_indexer import IncrementalIndexer
from .file_change_tracker import (
    FileChangeTracker,
    ChangeCategory,
    ChangeAnalyzer,
)
from .async_indexer import AsyncIndexer, AsyncBatchIndexer

# ============================================================================
# Search & Ranking
# ============================================================================

from .search.ranking import (
    ResultRanker,
    SearchResult,
    RankingConfig,
    UserBehaviorTracker,
    PathImportanceClassifier,
    PathImportance,
    create_default_ranker,
)

from .search.result_merger import (
    SearchResultMerger,
    MergedSearchResult,
    SearchBackend,
    ScoreNormalizer,
    ResultConverter,
    reciprocal_rank_fusion,
    merge_search_results,
)

from .search.base import parse_search_output, create_safe_fuzzy_pattern

# ============================================================================
# Code Analyzers (5-Layer Analysis)
# ============================================================================

from .analyzers.base import BaseAnalyzer
from .analyzers.ast import ASTAnalyzer
from .analyzers.callgraph import CallGraphAnalyzer
from .analyzers.cfg import CFGAnalyzer
from .analyzers.dfg import DFGAnalyzer
from .analyzers.slicing import SlicingAnalyzer

# ============================================================================
# MCP Server
# ============================================================================

try:
    from .mcp_server import mcp, indexer_lifespan
    MCP_AVAILABLE = True
except ImportError:
    MCP_AVAILABLE = False

# ============================================================================
# Utilities
# ============================================================================

from .ignore_patterns import IgnorePatternMatcher
from .content_extractor import ContentExtractor
from .logger_config import logger, setup_logging
from .lazy_loader import LazyContentManager

# ============================================================================
# Public API Exports
# ============================================================================

__all__ = [
    # Version
    "__version__",
    "MCP_AVAILABLE",

    # Configuration
    "TantivyConfig",
    "tantivy_config",
    "TANTIVY_AVAILABLE",
    "ConfigManager",
    "SETTINGS_DIR",

    # Storage
    "get_dal_instance",
    "DALInterface",
    "StorageInterface",
    "FileMetadataInterface",
    "SearchInterface",
    "SQLiteDAL",
    "SQLiteStorage",
    "SQLiteFileMetadata",
    "SQLiteSearch",
    "DuckDBDAL",
    "TantivySearch",
    "TantivyNotAvailableError",

    # Indexing
    "IncrementalIndexer",
    "FileChangeTracker",
    "ChangeCategory",
    "ChangeAnalyzer",
    "AsyncIndexer",
    "AsyncBatchIndexer",

    # Search & Ranking
    "ResultRanker",
    "SearchResult",
    "RankingConfig",
    "UserBehaviorTracker",
    "PathImportanceClassifier",
    "PathImportance",
    "create_default_ranker",
    "SearchResultMerger",
    "MergedSearchResult",
    "SearchBackend",
    "ScoreNormalizer",
    "ResultConverter",
    "reciprocal_rank_fusion",
    "merge_search_results",
    "parse_search_output",
    "create_safe_fuzzy_pattern",

    # Analyzers
    "BaseAnalyzer",
    "ASTAnalyzer",
    "CallGraphAnalyzer",
    "CFGAnalyzer",
    "DFGAnalyzer",
    "SlicingAnalyzer",

    # MCP Server
    "mcp",
    "indexer_lifespan",

    # Utilities
    "IgnorePatternMatcher",
    "ContentExtractor",
    "logger",
    "setup_logging",
    "LazyContentManager",
]


# ============================================================================
# Convenience Functions
# ============================================================================

def create_indexer(project_path: str, backend: str = "sqlite") -> DALInterface:
    """
    Create a configured indexer for a project.

    Args:
        project_path: Path to the project directory
        backend: Storage backend to use ('sqlite' or 'duckdb')

    Returns:
        Configured DALInterface instance
    """
    dal = get_dal_instance(backend=backend)
    return dal


def search_code(
    query: str,
    project_path: str,
    limit: int = 10,
    backend: str = "sqlite"
) -> list:
    """
    Convenience function for searching code.

    Args:
        query: Search query
        project_path: Path to project
        limit: Max results
        backend: Storage backend

    Returns:
        List of search results
    """
    dal = get_dal_instance(backend=backend)
    results = dal.search(query, limit=limit)
    return results


def analyze_code_file(
    file_path: str,
    layers: list = None
) -> dict:
    """
    Analyze a code file using 5-layer analysis.

    Args:
        file_path: Path to the file
        layers: List of layers to run (default: all)

    Returns:
        Dictionary with analysis results for each layer
    """
    if layers is None:
        layers = ['ast', 'callgraph', 'cfg', 'dfg', 'slicing']

    analyzers = {
        'ast': ASTAnalyzer(),
        'callgraph': CallGraphAnalyzer(),
        'cfg': CFGAnalyzer(),
        'dfg': DFGAnalyzer(),
        'slicing': SlicingAnalyzer(),
    }

    results = {}

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    for layer in layers:
        if layer not in analyzers:
            continue
        analyzer = analyzers[layer]
        analysis = analyzer.analyze(content, file_path)
        results[layer] = analyzer.to_llm_string(analysis)

    return results
