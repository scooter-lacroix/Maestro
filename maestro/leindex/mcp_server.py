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
from typing import Optional, Dict, Any, List
from contextlib import asynccontextmanager

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
from .async_indexer import AsyncBatchIndexer as AsyncIndexer
from .config_manager import ConfigManager
from .constants import SETTINGS_DIR
from .ignore_patterns import IgnorePatternMatcher
from .logger_config import logger
from .search.ranking import ResultRanker, RankingConfig
from .search.result_merger import SearchResultMerger, MergedSearchResult
from .analyzers.ast import ASTAnalyzer
from .analyzers.callgraph import CallGraphAnalyzer
from .analyzers.cfg import CFGAnalyzer
from .analyzers.dfg import DFGAnalyzer
from .analyzers.slicing import SlicingAnalyzer


# ============================================================================
# Lifespan Manager for Server State
# ============================================================================

@asynccontextmanager
async def indexer_lifespan(server: FastMCP):
    """
    Lifespan manager for LeIndex MCP server.

    Manages initialization and cleanup of indexer resources.
    """
    # Initialization
    logger.info("Initializing LeIndex MCP server...")

    # Initialize config manager
    config_manager = ConfigManager()

    # Initialize DAL instance
    dal_instance: Optional[DALInterface] = None
    try:
        dal_instance = get_dal_instance()
        logger.info("DAL instance initialized")
    except Exception as e:
        logger.warning(f"Failed to initialize DAL: {e}")

    # Initialize file change tracker
    file_change_tracker = FileChangeTracker()
    logger.info("File change tracker initialized")

    # Initialize async indexer
    async_indexer: Optional[AsyncIndexer] = None
    try:
        async_indexer = AsyncIndexer(
            storage_backend=dal_instance,
            file_change_tracker=file_change_tracker
        )
        await async_indexer.start()
        logger.info("Async indexer started")
    except Exception as e:
        logger.warning(f"Failed to start async indexer: {e}")

    # Initialize analyzers
    analyzers = {
        'ast': ASTAnalyzer(),
        'callgraph': CallGraphAnalyzer(),
        'cfg': CFGAnalyzer(),
        'dfg': DFGAnalyzer(),
        'slicing': SlicingAnalyzer(),
    }
    logger.info("Code analyzers initialized")

    # Store in lifespan context
    server.state = {
        'dal': dal_instance,
        'file_change_tracker': file_change_tracker,
        'async_indexer': async_indexer,
        'analyzers': analyzers,
        'config_manager': config_manager,
        'base_path': None,
    }

    yield

    # Cleanup
    logger.info("Shutting down LeIndex MCP server...")

    if async_indexer:
        try:
            await async_indexer.stop()
            logger.info("Async indexer stopped")
        except Exception as e:
            logger.error(f"Error stopping async indexer: {e}")

    if file_change_tracker:
        try:
            file_change_tracker.flush()
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
else:
    logger.warning("MCP not available, server will be disabled")
    mcp = None


# ============================================================================
# MCP Tools: Project Management
# ============================================================================

@mcp.tool()
async def set_project_path(project_path: str, ctx: Context) -> Dict[str, Any]:
    """
    Set the project path for indexing and search operations.

    Args:
        project_path: Absolute path to the project directory

    Returns:
        Status dictionary with success/error information
    """
    if not os.path.exists(project_path):
        return {
            "success": False,
            "error": f"Project path does not exist: {project_path}"
        }

    if not os.path.isdir(project_path):
        return {
            "success": False,
            "error": f"Path is not a directory: {project_path}"
        }

    # Store in lifespan context
    ctx.request_context.lifespan_context.base_path = project_path

    logger.info(f"Project path set to: {project_path}")

    return {
        "success": True,
        "project_path": project_path,
        "message": f"Project path set to {project_path}"
    }


@mcp.tool()
async def index_project(ctx: Context) -> Dict[str, Any]:
    """
    Index the current project directory.

    Performs a full index of all tracked files in the project,
    building search indexes and code analysis data.

    Returns:
        Status dictionary with indexing statistics
    """
    base_path = ctx.request_context.lifespan_context.base_path
    dal = ctx.request_context.lifespan_context.dal
    async_indexer = ctx.request_context.lifespan_context.async_indexer

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    if not async_indexer:
        return {
            "success": False,
            "error": "Async indexer not available"
        }

    try:
        # Trigger indexing
        result = await async_indexer.index_project(base_path)

        return {
            "success": True,
            "files_indexed": result.get('files_indexed', 0),
            "errors": result.get('errors', []),
            "duration_seconds": result.get('duration', 0)
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

@mcp.tool()
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
    base_path = ctx.request_context.lifespan_context.base_path
    dal = ctx.request_context.lifespan_context.dal

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    if not dal:
        return {
            "success": False,
            "error": "Storage backend not available"
        }

    try:
        # Perform search via DAL
        results = dal.search(query, limit=limit)

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

@mcp.tool()
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

    # Default to all layers if not specified
    if layers is None:
        layers = ['ast', 'callgraph', 'cfg', 'dfg', 'slicing']

    # Normalize file path
    full_path = os.path.join(base_path, file_path)

    if not os.path.exists(full_path):
        return {
            "success": False,
            "error": f"File not found: {file_path}"
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

@mcp.tool()
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
    base_path = ctx.request_context.lifespan_context.base_path
    file_change_tracker = ctx.request_context.lifespan_context.file_change_tracker

    if not base_path:
        return {
            "success": False,
            "error": "Project path not set. Use set_project_path first."
        }

    try:
        full_path = os.path.join(base_path, file_path)

        if not os.path.exists(full_path):
            return {
                "success": False,
                "error": f"File not found: {file_path}"
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
