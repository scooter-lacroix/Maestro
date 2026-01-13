"""
LeIndex MCP Server for Maestro

This MCP server provides code indexing, search, and analysis capabilities
for Maestro's unified code intelligence system.

Core Features:
- Fast full-text search via Tantivy
- Semantic code search via vector embeddings
- 5-layer code analysis (AST, Call Graph, CFG, DFG, Slicing)
- File change tracking and version history
- Incremental indexing with async processing
"""

import os
import sys
import asyncio
import logging
from pathlib import Path
from typing import Optional, Dict, Any, List, Iterable
from contextlib import asynccontextmanager
from dataclasses import dataclass, field

# Import security utilities
from .security_utils import is_safe_path, is_approved_project_path, sanitize_file_path

# Import FastMCP for MCP server creation
try:
    from mcp.server.fastmcp import FastMCP, Context
    from mcp import types
    MCP_AVAILABLE = True
except ImportError:
    MCP_AVAILABLE = False
    FastMCP = None
    Context = None

# Import leindex components
from .tantivy_config import tantivy_config, TANTIVY_AVAILABLE
from .storage.dal_factory import get_dal_instance
from .storage.storage_interface import DALInterface
from .incremental_indexer import IncrementalIndexer
from .file_change_tracker import FileChangeTracker
from .async_indexer import AsyncRealtimeIndexer
from .async_task_queue import IndexingPriority
from .config_manager import ConfigManager
from .constants import SETTINGS_DIR
from .ignore_patterns import IgnorePatternMatcher
from .logger_config import logger
from .project_settings import ProjectSettings
from .search.ranking import ResultRanker, RankingConfig
from .search.result_merger import SearchResultMerger, MergedSearchResult
from .analyzers.ast import ASTAnalyzer
from .analyzers.callgraph import CallGraphAnalyzer
from .analyzers.cfg import CFGAnalyzer
from .analyzers.dfg import DFGAnalyzer
from .analyzers.slicing import SlicingAnalyzer




@dataclass
class LifespanState:
    dal: Optional[DALInterface] = None
    file_change_tracker: Optional[FileChangeTracker] = None
    async_indexer: Optional[AsyncRealtimeIndexer] = None
    analyzers: Dict[str, Any] = field(default_factory=dict)
    config_manager: Optional[ConfigManager] = None
    base_path: Optional[str] = None
    ignore_matcher: Optional[IgnorePatternMatcher] = None
    project_settings: Optional[ProjectSettings] = None
    incremental_indexer: Optional[IncrementalIndexer] = None


def _get_lifespan_state(ctx: Context) -> LifespanState:
    return ctx.request_context.lifespan_context


async def _ensure_project_initialized(state: LifespanState, project_path: str) -> None:
    if state.base_path == project_path and state.async_indexer:
        return

    if state.async_indexer:
        try:
            await state.async_indexer.stop()
        except Exception as e:
            logger.warning(f"Failed to stop existing async indexer: {e}")

    state.base_path = project_path
    state.project_settings = ProjectSettings(base_path=project_path, skip_load=False)
    state.incremental_indexer = IncrementalIndexer(state.project_settings)
    state.ignore_matcher = IgnorePatternMatcher(project_path)

    if state.dal and state.dal.metadata:
        state.file_change_tracker = FileChangeTracker(
            state.dal.metadata,
            state.incremental_indexer
        )
    else:
        state.file_change_tracker = None
        logger.warning("Metadata backend not available; file change tracker disabled")

    if state.dal and state.dal.search:
        state.async_indexer = AsyncRealtimeIndexer(
            storage_backend=state.dal.search,
            base_path=project_path
        )
        await state.async_indexer.start()
        logger.info("Async indexer started")
    else:
        state.async_indexer = None
        logger.warning("Search backend not available; async indexer disabled")


def _select_search_backend(dal: Optional[DALInterface]):
    if not dal or not dal.search:
        return None

    try:
        from .search_utils import SearchBackendSelector
    except ImportError:
        SearchBackendSelector = None

    if SearchBackendSelector:
        return SearchBackendSelector.get_search_backend(dal)

    return dal.search


async def _collect_index_targets(base_path: str, matcher: Optional[IgnorePatternMatcher]) -> Iterable[str]:
    try:
        from .fast_scanner import FastParallelScanner

        scanner = FastParallelScanner(ignore_matcher=matcher)
        results = await scanner.scan(base_path)
        for root, _, files in results:
            for filename in files:
                full_path = os.path.join(root, filename)
                yield os.path.relpath(full_path, base_path)
        return
    except Exception as e:
        logger.warning(f"Fast scanner failed, falling back to os.walk: {e}")

    for root, _, files in os.walk(base_path):
        for filename in files:
            full_path = os.path.join(root, filename)
            if matcher and matcher.should_ignore(full_path):
                continue
            yield os.path.relpath(full_path, base_path)

@asynccontextmanager
async def indexer_lifespan(server: FastMCP):
    """
    Lifespan manager for LeIndex MCP server.

    Manages initialization and cleanup of indexer resources.
    """
    logger.info("Initializing LeIndex MCP server...")

    config_manager = ConfigManager()

    dal_instance: Optional[DALInterface] = None
    try:
        dal_instance = get_dal_instance()
        logger.info("DAL instance initialized")
    except Exception as e:
        logger.warning(f"Failed to initialize DAL: {e}")

    state = LifespanState(
        dal=dal_instance,
        config_manager=config_manager,
        analyzers={
            "ast": ASTAnalyzer(),
            "callgraph": CallGraphAnalyzer(),
            "cfg": CFGAnalyzer(),
            "dfg": DFGAnalyzer(),
            "slicing": SlicingAnalyzer(),
        },
    )
    logger.info("Code analyzers initialized")

    server.state = state

    try:
        yield state
    finally:
        logger.info("Shutting down LeIndex MCP server...")

        if state.async_indexer:
            try:
                await state.async_indexer.stop()
                logger.info("Async indexer stopped")
            except Exception as e:
                logger.error(f"Error stopping async indexer: {e}")

        if state.file_change_tracker:
            try:
                state.file_change_tracker.flush()
                logger.info("File change tracker flushed")
            except Exception as e:
                logger.error(f"Error flushing file change tracker: {e}")

        logger.info("LeIndex MCP server shutdown complete")
# ============================================================================
# MCP Server Initialization
# ============================================================================

# Create FastMCP instance
if MCP_AVAILABLE:
    mcp = FastMCP(
        name="leindex",
        lifespan=indexer_lifespan
    )

    def mcp_tool():
        return mcp.tool()
else:
    logger.warning("MCP not available, server will be disabled")
    mcp = None

    def mcp_tool():
        def decorator(func):
            return func
        return decorator


# ============================================================================
# MCP Tools: Project Management
# ============================================================================

@mcp_tool()
async def set_project_path(project_path: str, ctx: Context) -> Dict[str, Any]:
    """
    Set the project path for indexing and search operations.

    Args:
        project_path: Absolute path to the project directory

    Returns:
        Status dictionary with success/error information
    """
    state = _get_lifespan_state(ctx)

    # Sanitize the project path
    sanitized_path = sanitize_file_path(project_path)
    if not sanitized_path:
        return {
            "success": False,
            "error": "Invalid project path containing dangerous characters"
        }

    if not os.path.exists(sanitized_path):
        return {
            "success": False,
            "error": f"Project path does not exist: {sanitized_path}"
        }

    if not os.path.isdir(sanitized_path):
        return {
            "success": False,
            "error": f"Path is not a directory: {sanitized_path}"
        }

    # Validate that the project path is approved (within allowed directories)
    # For now, we allow any existing directory, but this can be restricted
    if not is_approved_project_path(sanitized_path):
        return {
            "success": False,
            "error": f"Project path is not in an approved location: {sanitized_path}"
        }

    await _ensure_project_initialized(state, sanitized_path)

    logger.info(f"Project path set to: {sanitized_path}")

    return {
        "success": True,
        "project_path": sanitized_path,
        "message": f"Project path set to {sanitized_path}"
    }


@mcp_tool()
async def index_project(ctx: Context) -> Dict[str, Any]:
    """
    Index the current project directory.

    Performs a full index of all tracked files in the project,
    building search indexes and code analysis data.

    Returns:
        Status dictionary with indexing statistics
    """
    state = _get_lifespan_state(ctx)
    base_path = state.base_path

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    if not state.async_indexer:
        return {
            "success": False,
            "error": "Async indexer not available"
        }

    try:
        tasks = [
            state.async_indexer.enqueue_change(
                file_path=path,
                change_type="index",
                priority=IndexingPriority.NORMAL
            )
            async for path in _collect_index_targets(base_path, state.ignore_matcher)
        ]
        await asyncio.gather(*tasks)
        await state.async_indexer.wait_for_completion()

        return {
            "success": True,
            "files_indexed": len(tasks),
            "errors": [],
            "duration_seconds": 0
        }
    except Exception as e:
        logger.error(f"Error indexing project: {e}")
        return {
            "success": False,
            "error": str(e)
        }


# ============================================================================
# MCP Tools: Search
# ============================================================================

@mcp_tool()
async def search_code(
    query: str,
    limit: int = 10,
    ctx: Context = None
) -> Dict[str, Any]:
    """
    Search code using full-text and semantic search.

    Performs hybrid search combining Tantivy BM25 scores with
    semantic similarity for best results.

    Args:
        query: Search query string
        limit: Maximum number of results to return

    Returns:
        Search results with file paths, scores, and content snippets
    """
    state = _get_lifespan_state(ctx)
    base_path = state.base_path
    dal = state.dal

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    search_backend = _select_search_backend(dal)
    if not search_backend:
        return {
            "success": False,
            "error": "Search backend not available"
        }

    try:
        results = search_backend.search_files(query)

        return {
            "success": True,
            "query": query,
            "results": results,
            "count": len(results)
        }
    except Exception as e:
        logger.error(f"Error searching code: {e}")
        return {
            "success": False,
            "error": str(e)
        }


# ============================================================================
# MCP Tools: Code Analysis
# ============================================================================

@mcp_tool()
async def analyze_file(
    file_path: str,
    layers: List[str] = None,
    ctx: Context = None
) -> Dict[str, Any]:
    """
    Analyze a file using 5-layer code analysis.

    Available layers:
    - ast: Abstract Syntax Tree
    - callgraph: Function call graph
    - cfg: Control Flow Graph
    - dfg: Data Flow Graph
    - slicing: Program slicing

    Args:
        file_path: Path to the file to analyze (relative to project root)
        layers: List of layers to analyze (default: all)

    Returns:
        Analysis results for requested layers
    """
    base_path = ctx.request_context.lifespan_context.base_path
    analyzers = ctx.request_context.lifespan_context.analyzers

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    # Sanitize the file path
    sanitized_file_path = sanitize_file_path(file_path)
    if not sanitized_file_path:
        return {
            "success": False,
            "error": "Invalid file path containing dangerous characters"
        }

    # Default to all layers if not specified
    if layers is None:
        layers = ['ast', 'callgraph', 'cfg', 'dfg', 'slicing']

    # Validate that the file path is safely contained within the project directory
    if not is_safe_path(base_path, sanitized_file_path):
        return {
            "success": False,
            "error": f"File path is not within the project directory: {sanitized_file_path}"
        }

    # Normalize file path
    full_path = os.path.join(base_path, sanitized_file_path)

    if not os.path.exists(full_path):
        return {
            "success": False,
            "error": f"File not found: {sanitized_file_path}"
        }

    try:
        results = {}

        # Read file content
        with open(full_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # Run requested analyzers
        for layer in layers:
            if layer not in analyzers:
                continue

            analyzer = analyzers[layer]
            try:
                analysis = analyzer.analyze(content, file_path)
                results[layer] = {
                    "success": True,
                    "llm_string": analyzer.to_llm_string(analysis),
                    "analysis": analysis
                }
            except Exception as e:
                results[layer] = {
                    "success": False,
                    "error": str(e)
                }

        return {
            "success": True,
            "file_path": file_path,
            "layers": results
        }

    except Exception as e:
        logger.error(f"Error analyzing file: {e}")
        return {
            "success": False,
            "error": str(e)
        }


# ============================================================================
# MCP Tools: File History
# ============================================================================

@mcp_tool()
async def get_file_history(
    file_path: str,
    ctx: Context = None
) -> Dict[str, Any]:
    """
    Get the change history for a file.

    Returns version history including changes, timestamps,
    and change categories.

    Args:
        file_path: Path to the file (relative to project root)

    Returns:
        File history with changes and metadata
    """
    state = _get_lifespan_state(ctx)
    base_path = state.base_path
    dal = state.dal
    file_change_tracker = state.file_change_tracker

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    if not file_change_tracker:
        return {
            "success": False,
            "error": "File change tracker not available"
        }

    try:
        # Sanitize the file path
        sanitized_file_path = sanitize_file_path(file_path)
        if not sanitized_file_path:
            return {
                "success": False,
                "error": "Invalid file path containing dangerous characters"
            }

        # Validate that the file path is safely contained within the project directory
        if not is_safe_path(base_path, sanitized_file_path):
            return {
                "success": False,
                "error": f"File path is not within the project directory: {sanitized_file_path}"
            }

        full_path = os.path.join(base_path, sanitized_file_path)

        if not os.path.exists(full_path):
            return {
                "success": False,
                "error": f"File not found: {sanitized_file_path}"
            }

        # Get history from tracker
        history = file_change_tracker.get_file_history(full_path)

        return {
            "success": True,
            "file_path": file_path,
            "history": history
        }

    except Exception as e:
        logger.error(f"Error getting file history: {e}")
        return {
            "success": False,
            "error": str(e)
        }


# ============================================================================
# Server Entry Point
# ============================================================================

def main():
    """Main entry point for running the MCP server."""
    if not MCP_AVAILABLE:
        logger.error("MCP not available. Install with: pip install mcp")
        sys.exit(1)

    if mcp is None:
        logger.error("Failed to initialize MCP server")
        sys.exit(1)

    # Run server
    mcp.run()


if __name__ == "__main__":
    main()
