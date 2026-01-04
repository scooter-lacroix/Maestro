"""
Maestro Memory Dashboard

FastAPI web dashboard for visualizing and managing Maestro memory.
Integrates with MaestroMemoryService to provide:
- Project and track memory visualization
- Memory search and retrieval
- Statistics and analytics
- Real-time updates via WebSocket
"""

import json
import os
import re
import html
from datetime import datetime, UTC
from pathlib import Path
from typing import Optional, Dict, List, Any
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException, Query, WebSocket, WebSocketDisconnect, Request, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.gzip import GZipMiddleware
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from starlette.exceptions import HTTPException as StarletteHTTPException
from pydantic import BaseModel, Field, validator
from loguru import logger

from .service import MaestroMemoryService


# =============================================================================
# Issue 5 & 6: Security Configuration
# =============================================================================


# Issue 5: CORS configuration from environment variable
ALLOWED_ORIGINS = os.environ.get(
    "MAESTRO_ALLOWED_ORIGINS",
    "http://localhost:3000,http://localhost:18765"
).split(",")

# Issue 6: Define allowed base directories for project_path validation
ALLOWED_BASE_DIRS = os.environ.get(
    "MAESTRO_ALLOWED_BASE_DIRS",
    "/home/stan/Prod"
).split(",")

# Issue 6: Regex pattern for project_path validation
# Allow alphanumeric, hyphens, underscores, forward slashes, but no suspicious patterns
PROJECT_PATH_PATTERN = re.compile(
    r'^[a-zA-Z0-9_\-/]+$',  # Alphanumeric, underscore, hyphen, forward slash
    re.IGNORECASE
)


# =============================================================================
# Issue 6: Input Validation Utilities
# =============================================================================


def validate_project_path(project_path: str) -> str:
    """
    Validate project path for security (Issue 6).

    Args:
        project_path: Project path to validate

    Raises:
        HTTPException: If validation fails

    Returns:
        Validated project path
    """
    if not project_path or not isinstance(project_path, str):
        raise HTTPException(
            status_code=400,
            detail="Project path must be a non-empty string"
        )

    # Check for suspicious patterns
    suspicious_patterns = ['..', '\\\\', '~/', '/etc/', '/sys/', '/proc/', '\\windows\\', '\x00']
    for pattern in suspicious_patterns:
        if pattern in project_path.lower():
            raise HTTPException(
                status_code=400,
                detail=f"Suspicious path pattern detected: {pattern}"
            )

    # Validate against allowed pattern
    if not PROJECT_PATH_PATTERN.match(project_path):
        raise HTTPException(
            status_code=400,
            detail="Project path contains invalid characters"
        )

    # Check if path is within allowed base directories
    resolved_path = Path(project_path).resolve()
    is_allowed = any(
        str(resolved_path).startswith(allowed_dir)
        for allowed_dir in ALLOWED_BASE_DIRS
    )

    if not is_allowed:
        raise HTTPException(
            status_code=403,
            detail=f"Project path not in allowed directories: {', '.join(ALLOWED_BASE_DIRS)}"
        )

    return project_path


def sanitize_dict_for_json(data: Dict[str, Any]) -> Dict[str, Any]:
    """
    Sanitize dictionary values to prevent XSS in JSON responses.

    This function escapes HTML special characters in string values to prevent
    XSS attacks when the JSON is parsed and displayed in a browser.

    Args:
        data: Dictionary to sanitize

    Returns:
        Sanitized dictionary
    """
    if not isinstance(data, dict):
        return data

    sanitized = {}
    for key, value in data.items():
        if isinstance(value, str):
            # Escape HTML special characters
            sanitized[key] = html.escape(value)
        elif isinstance(value, dict):
            sanitized[key] = sanitize_dict_for_json(value)
        elif isinstance(value, list):
            sanitized[key] = [
                sanitize_dict_for_json(item) if isinstance(item, dict) else
                html.escape(item) if isinstance(item, str) else item
                for item in value
            ]
        else:
            sanitized[key] = value

    return sanitized


# =============================================================================
# Pydantic Models for Request/Response Validation
# =============================================================================


class MemoryListResponse(BaseModel):
    """Response model for memory list"""
    success: bool
    memories: List[Dict[str, Any]] = []
    total: int = 0
    project_path: Optional[str] = None


class TrackContextResponse(BaseModel):
    """Response model for track context"""
    success: bool
    track_id: Optional[str] = None
    memories: List[Dict[str, Any]] = []
    total: int = 0


class ProjectListResponse(BaseModel):
    """Response model for project list"""
    success: bool
    projects: List[Dict[str, Any]] = []
    total: int = 0


class TrackListResponse(BaseModel):
    """Response model for track list"""
    success: bool
    tracks: List[Dict[str, Any]] = []
    total: int = 0


class SearchResponse(BaseModel):
    """Response model for memory search"""
    success: bool
    query: str
    results: List[Dict[str, Any]] = []
    total: int = 0


class StatsResponse(BaseModel):
    """Response model for statistics"""
    success: bool
    total_memories: int = 0
    total_projects: int = 0
    total_tracks: int = 0
    memories_by_command: Dict[str, int] = {}
    memories_by_project: Dict[str, int] = {}


class ErrorResponse(BaseModel):
    """Standard error response"""
    success: bool = False
    error: str
    detail: Optional[str] = None


# =============================================================================
# Dashboard Application Factory
# =============================================================================


@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    Lifespan context manager for startup and shutdown events.

    This replaces the deprecated @app.on_event("startup") and @app.on_event("shutdown").
    """
    # Get database_path from app state or use default
    database_path = getattr(app.state, 'database_path', None)
    if database_path is None:
        database_path = Path.home() / ".maestro" / "maestro.db"

    # Ensure database_path is a Path object
    if isinstance(database_path, str):
        database_path = Path(database_path)

    # Startup
    logger.info("Starting Maestro Memory Dashboard")
    memory_service = MaestroMemoryService(database_path=database_path)
    await memory_service.initialize()

    # Store service in app state for access in routes
    app.state.memory_service = memory_service
    logger.info(f"Maestro Memory Dashboard started with database: {memory_service.database_path}")

    yield

    # Shutdown
    logger.info("Shutting down Maestro Memory Dashboard")
    if memory_service:
        await memory_service.close()
        memory_service = None


def create_dashboard_app(
    database_path: Optional[Path] = None,
    debug: bool = False
) -> FastAPI:
    """
    Create and configure the Maestro Memory Dashboard application.

    Args:
        database_path: Path to SQLite database file
        debug: Enable debug mode (verbose error messages)

    Returns:
        Configured FastAPI application instance
    """
    app = FastAPI(
        title="Maestro Memory Dashboard",
        description="Visual dashboard for Maestro unified development framework memory",
        version="2.0.0",
        docs_url="/api/docs",
        redoc_url="/api/redoc",
        openapi_url="/api/openapi.json",
        lifespan=lifespan
    )

    # Store database_path in app state for lifespan manager
    if database_path:
        app.state.database_path = database_path
    app.state.debug = debug

    # Issue 5: Fixed CORS - use environment variable, restrict with credentials
    # WARNING: allow_credentials=True requires specific origins, not "*"
    app.add_middleware(
        CORSMiddleware,
        allow_origins=ALLOWED_ORIGINS,  # From environment variable
        allow_credentials=True,  # OK when origins are specific
        allow_methods=["GET", "POST", "OPTIONS"],  # Restrict methods
        allow_headers=["Content-Type", "Authorization", "X-Requested-With"],  # Restrict headers
    )

    # GZip compression
    app.add_middleware(GZipMiddleware, minimum_size=1000)

    # Define exception handler functions
    async def http_exception_handler(request: Request, exc: StarletteHTTPException):
        """Handle HTTP exceptions with consistent error response"""
        return JSONResponse(
            status_code=exc.status_code,
            content={
                "success": False,
                "error": exc.detail,
                "status_code": exc.status_code
            }
        )

    async def general_exception_handler(request: Request, exc: Exception):
        """Handle uncaught exceptions"""
        logger.error(f"Unhandled exception: {exc}")
        return JSONResponse(
            status_code=500,
            content={
                "success": False,
                "error": "Internal server error",
                "detail": str(exc) if debug else "An error occurred processing your request"
            }
        )

    # Register exception handlers
    # Handle Starlette HTTPException (which includes FastAPI HTTPException and 404s)
    app.add_exception_handler(StarletteHTTPException, http_exception_handler)
    app.add_exception_handler(Exception, general_exception_handler)

    # Mount static files - prefer new built frontend, fallback to old static
    frontend_dist = Path(__file__).parent / "frontend" / "dist"
    old_static_dir = Path(__file__).parent.parent.parent / "static" / "memory_dashboard"
    local_static_dir = Path(__file__).parent / "static"

    # Try new built frontend first
    if frontend_dist.exists() and (frontend_dist / "index.html").exists():
        logger.info(f"Serving new frontend from: {frontend_dist}")
        app.mount("/static", StaticFiles(directory=str(frontend_dist)), name="static")
        static_dir = frontend_dist
    elif old_static_dir.exists():
        logger.info(f"Serving old static files from: {old_static_dir}")
        app.mount("/static", StaticFiles(directory=str(old_static_dir)), name="static")
        static_dir = old_static_dir
    elif local_static_dir.exists():
        logger.info(f"Serving local static files from: {local_static_dir}")
        app.mount("/static", StaticFiles(directory=str(local_static_dir)), name="static")
        static_dir = local_static_dir
    else:
        static_dir = None
        logger.warning("No static directory found")

    # =============================================================================
    # API Routes
    # =============================================================================

    @app.get("/", response_class=HTMLResponse)
    async def root():
        """Serve the dashboard HTML"""
        # Try the new built frontend first
        frontend_dist = Path(__file__).parent / "frontend" / "dist"
        old_static_dir = Path(__file__).parent.parent.parent / "static" / "memory_dashboard"
        local_static_dir = Path(__file__).parent / "static"

        # Priority 1: New built frontend
        index_path = frontend_dist / "index.html"
        if index_path.exists():
            with open(index_path, "r") as f:
                return HTMLResponse(content=f.read())

        # Priority 2: Old dashboard.html
        dashboard_path = old_static_dir / "dashboard.html"
        if dashboard_path.exists():
            with open(dashboard_path, "r") as f:
                return HTMLResponse(content=f.read())

        # Priority 3: Old index.html
        old_index_path = old_static_dir / "index.html"
        if old_index_path.exists():
            with open(old_index_path, "r") as f:
                return HTMLResponse(content=f.read())

        # Priority 4: Local static
        local_index_path = local_static_dir / "index.html"
        if local_index_path.exists():
            with open(local_index_path, "r") as f:
                return HTMLResponse(content=f.read())

        # Fallback: Error page
        return HTMLResponse(
            content="""
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Maestro Memory Dashboard</title>
                <style>
                    body { font-family: system-ui, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; background: #0a0a0a; color: #fff; }
                    h1 { color: #fff; }
                    .error { background: rgba(255, 100, 100, 0.1); border: 1px solid rgba(255, 100, 100, 0.3); padding: 20px; border-radius: 8px; }
                    .info { background: rgba(100, 150, 255, 0.1); border: 1px solid rgba(100, 150, 255, 0.3); padding: 20px; border-radius: 8px; margin-top: 20px; }
                    a { color: #6af; }
                </style>
            </head>
            <body>
                <h1>Maestro Memory Dashboard v2.0</h1>
                <div class="error">
                    <h2>Frontend Not Built</h2>
                    <p>The new frontend has not been built yet.</p>
                    <p><strong>To build the frontend:</strong></p>
                    <pre style="background: rgba(255,255,255,0.1); padding: 10px; border-radius: 4px; overflow-x: auto;">
cd /home/stan/Prod/maestro/maestro/memory/frontend
npm install
npm run build
                    </pre>
                </div>
                <div class="info">
                    <h3>Available Resources:</h3>
                    <ul>
                        <li><a href="/api/docs">API Documentation (Swagger UI)</a></li>
                        <li><a href="/api/redoc">API Documentation (ReDoc)</a></li>
                        <li><a href="/health">Health Check</a></li>
                    </ul>
                </div>
            </body>
            </html>
            """
        )

    @app.get("/health", tags=["health"])
    async def health_check():
        """
        Health check endpoint.

        Issue 19: Verify database connectivity in health check
        """
        health_status = {
            "status": "healthy",
            "timestamp": datetime.now(UTC).isoformat(),
            "version": "2.0.0",
            "service": "Maestro Memory Dashboard",
            "checks": {}
        }

        service: MaestroMemoryService = app.state.memory_service

        try:
            # Issue 19: Check if service is initialized
            if not service._initialized:
                health_status["status"] = "unhealthy"
                health_status["checks"]["initialization"] = {
                    "status": "failed",
                    "message": "Service not initialized"
                }
                return JSONResponse(status_code=503, content=health_status)

            health_status["checks"]["initialization"] = {
                "status": "passed",
                "message": "Service initialized"
            }

            # Issue 19: Verify database connectivity
            try:
                async with service.db_manager.get_async_session() as session:
                    from sqlalchemy import text
                    result = await session.execute(text("SELECT 1"))
                    if result.scalar() == 1:
                        health_status["checks"]["database"] = {
                            "status": "passed",
                            "message": "Database connection successful"
                        }
                    else:
                        health_status["status"] = "degraded"
                        health_status["checks"]["database"] = {
                            "status": "failed",
                            "message": "Database query returned unexpected result"
                        }
            except Exception as db_error:
                health_status["status"] = "unhealthy"
                health_status["checks"]["database"] = {
                    "status": "failed",
                    "message": f"Database connection failed: {str(db_error)}"
                }
                return JSONResponse(status_code=503, content=health_status)

            # Check memory manager
            if service.memory_manager is None:
                health_status["status"] = "degraded"
                health_status["checks"]["memory_manager"] = {
                    "status": "failed",
                    "message": "Memory manager not initialized"
                }
            else:
                health_status["checks"]["memory_manager"] = {
                    "status": "passed",
                    "message": "Memory manager initialized"
                }

            # Return appropriate status code based on overall health
            status_code = 200 if health_status["status"] == "healthy" else 503
            return JSONResponse(status_code=status_code, content=health_status)

        except Exception as e:
            logger.error(f"Health check failed: {e}")
            health_status["status"] = "unhealthy"
            health_status["checks"]["health_check"] = {
                "status": "failed",
                "message": str(e)
            }
            return JSONResponse(status_code=503, content=health_status)

    @app.get("/api/v1/context/project", response_model=MemoryListResponse)
    async def get_project_context(
        project_path: str = Query(..., description="Project path to retrieve memories for"),
        limit: int = Query(10, ge=1, le=100, description="Maximum number of memories to retrieve")
    ):
        """Retrieve all context for a specific project"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            # Issue 6: Validate project_path before passing to service
            validated_path = validate_project_path(project_path)

            memories = await service.retrieve_project_context(
                project_path=validated_path,
                limit=limit
            )

            # Sanitize memories to prevent XSS attacks
            sanitized_memories = [sanitize_dict_for_json(memory) for memory in memories]

            return MemoryListResponse(
                success=True,
                memories=sanitized_memories,
                total=len(sanitized_memories),
                project_path=project_path
            )
        except HTTPException:
            # Re-raise HTTP exceptions as-is
            raise
        except ValueError as e:
            raise HTTPException(status_code=400, detail=str(e))
        except Exception as e:
            logger.error(f"Error retrieving project context: {e}")
            raise HTTPException(status_code=500, detail="Failed to retrieve project context")

    @app.get("/api/v1/context/track", response_model=TrackContextResponse)
    async def get_track_context(
        track_id: str = Query(..., description="Track ID to retrieve memories for"),
        limit: int = Query(20, ge=1, le=100, description="Maximum number of memories to retrieve")
    ):
        """Retrieve all context for a specific track"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            memories = await service.retrieve_track_context(
                track_id=track_id,
                limit=limit
            )

            # Sanitize memories to prevent XSS attacks
            sanitized_memories = [sanitize_dict_for_json(memory) for memory in memories]

            return TrackContextResponse(
                success=True,
                track_id=track_id,
                memories=sanitized_memories,
                total=len(sanitized_memories)
            )
        except ValueError as e:
            raise HTTPException(status_code=400, detail=str(e))
        except Exception as e:
            logger.error(f"Error retrieving track context: {e}")
            raise HTTPException(status_code=500, detail="Failed to retrieve track context")

    @app.get("/api/v1/projects", response_model=ProjectListResponse)
    async def list_projects():
        """List all projects in memory"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            from .database.models import MaestroProject
            from sqlalchemy import select

            async with service.db_manager.get_async_session() as session:
                stmt = select(MaestroProject).order_by(MaestroProject.last_active.desc())
                result = await session.execute(stmt)
                projects = result.scalars().all()

                # Use to_dict() method for complete field mapping
                project_list = [p.to_dict() for p in projects]

                return ProjectListResponse(
                    success=True,
                    projects=project_list,
                    total=len(project_list)
                )
        except Exception as e:
            logger.error(f"Error listing projects: {e}")
            raise HTTPException(status_code=500, detail="Failed to list projects")

    @app.get("/api/v1/tracks", response_model=TrackListResponse)
    async def list_tracks(
        project_id: Optional[int] = Query(None, description="Filter by project ID")
    ):
        """List all tracks in memory"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            from .database.models import MaestroTrack
            from sqlalchemy import select

            async with service.db_manager.get_async_session() as session:
                stmt = select(MaestroTrack)
                if project_id:
                    stmt = stmt.filter_by(project_id=project_id)
                stmt = stmt.order_by(MaestroTrack.created_at.desc())

                result = await session.execute(stmt)
                tracks = result.scalars().all()

                # Use to_dict() method for complete field mapping
                track_list = [t.to_dict() for t in tracks]

                return TrackListResponse(
                    success=True,
                    tracks=track_list,
                    total=len(track_list)
                )
        except Exception as e:
            logger.error(f"Error listing tracks: {e}")
            raise HTTPException(status_code=500, detail="Failed to list tracks")

    @app.get("/api/v1/search", response_model=SearchResponse)
    async def search_memories(
        query: str = Query(..., description="Search query"),
        project_path: Optional[str] = Query(None, description="Filter by project path"),
        limit: int = Query(5, ge=1, le=50, description="Maximum results")
    ):
        """Search for similar command executions"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            results = await service.search_similar_commands(
                command=query,
                project_path=project_path,
                limit=limit
            )

            return SearchResponse(
                success=True,
                query=query,
                results=results,
                total=len(results)
            )
        except ValueError as e:
            raise HTTPException(status_code=400, detail=str(e))
        except Exception as e:
            logger.error(f"Error searching memories: {e}")
            raise HTTPException(status_code=500, detail="Failed to search memories")

    @app.get("/api/v1/stats", response_model=StatsResponse)
    async def get_statistics():
        """Get memory statistics"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            from .database.models import MaestroProject, MaestroTrack
            from sqlalchemy import select, func, text

            async with service.db_manager.get_async_session() as session:
                # Count total projects
                project_result = await session.execute(
                    select(func.count(MaestroProject.id))
                )
                total_projects = project_result.scalar() or 0

                # Count total tracks
                track_result = await session.execute(
                    select(func.count(MaestroTrack.id))
                )
                total_tracks = track_result.scalar() or 0

                # Count total memories
                memory_result = await session.execute(
                    select(func.count()).select_from(text("memories")).where(text("is_active = 1"))
                )
                total_memories = memory_result.scalar() or 0

                # Count memories by command
                command_result = await session.execute(
                    text("""
                        SELECT maestro_command, COUNT(*) as count
                        FROM memories
                        WHERE is_active = 1
                        GROUP BY maestro_command
                    """)
                )
                memories_by_command = {
                    row[0]: row[1] for row in command_result.fetchall() if row[0]
                }

                # Count memories by project (using project names)
                project_memories_result = await session.execute(
                    text("""
                        SELECT mp.project_name, COUNT(*) as count
                        FROM memories m
                        JOIN maestro_projects mp ON m.maestro_project_id = mp.id
                        WHERE m.is_active = 1
                        GROUP BY mp.project_name
                    """)
                )
                memories_by_project = {
                    row[0]: row[1] for row in project_memories_result.fetchall()
                }

                return StatsResponse(
                    success=True,
                    total_memories=total_memories,
                    total_projects=total_projects,
                    total_tracks=total_tracks,
                    memories_by_command=memories_by_command,
                    memories_by_project=memories_by_project
                )
        except Exception as e:
            logger.error(f"Error getting statistics: {e}")
            raise HTTPException(status_code=500, detail="Failed to get statistics")

    @app.get("/api/v1/memories")
    async def list_memories(
        project_id: Optional[int] = Query(None, description="Filter by project ID"),
        track_id: Optional[int] = Query(None, description="Filter by track ID"),
        limit: int = Query(50, ge=1, le=200, description="Maximum results"),
        offset: int = Query(0, ge=0, description="Offset for pagination")
    ):
        """
        List memories with optional filters.

        CRITICAL-4 FIX: Replaced raw SQL with SQLAlchemy ORM to prevent SQL injection.
        Uses parameterized queries through ORM's select() and filter_by() methods.
        """
        service: MaestroMemoryService = app.state.memory_service

        try:
            from sqlalchemy import select, desc, text
            from nexus.database.models import Memory

            async with service.db_manager.get_async_session() as session:
                # Build query using SQLAlchemy ORM (safe from SQL injection)
                stmt = select(Memory).filter_by(is_active=True)

                # Add project filter if provided
                if project_id is not None:
                    # Use text() for Maestro columns added via ALTER TABLE
                    # This is safe because project_id is validated by Query() type check
                    stmt = stmt.filter(text("maestro_project_id = :project_id")).params(project_id=project_id)

                # Add track filter if provided
                if track_id is not None:
                    stmt = stmt.filter(text("maestro_track_id = :track_id")).params(track_id=track_id)

                # Add ordering and pagination
                stmt = stmt.order_by(desc(Memory.created_at)).limit(limit).offset(offset)

                result = await session.execute(stmt)
                memories = result.scalars().all()

                memory_list = []
                for memory in memories:
                    # Get metadata from the ORM object
                    metadata = memory.extra_metadata if hasattr(memory, 'extra_metadata') else {}
                    if isinstance(metadata, str):
                        try:
                            metadata = json.loads(metadata)
                        except json.JSONDecodeError:
                            metadata = {}

                    # Get maestro_command from metadata
                    command = metadata.get("maestro_command", "unknown")

                    memory_list.append({
                        "id": memory.id,
                        "content": memory.content,
                        "category": memory.category,
                        "labels": memory.labels if memory.labels else [],
                        "created_at": memory.created_at.isoformat() if memory.created_at else None,
                        "command": command,
                        "metadata": metadata
                    })

                return {
                    "success": True,
                    "memories": memory_list,
                    "total": len(memory_list)
                }
        except Exception as e:
            logger.error(f"Error listing memories: {e}")
            raise HTTPException(status_code=500, detail="Failed to list memories")

    @app.post("/api/v1/store")
    async def store_memory(request: Request):
        """Store a new memory via the API"""
        service: MaestroMemoryService = app.state.memory_service

        try:
            # Parse request body
            body = await request.json()

            command = body.get("command")
            project_path = body.get("project_path")
            context = body.get("context", {})

            # Validate required fields
            if not command:
                raise HTTPException(status_code=400, detail="Missing required field: command")
            if not project_path:
                raise HTTPException(status_code=400, detail="Missing required field: project_path")

            # Validate project path for security
            validated_path = validate_project_path(project_path)

            # Store the memory
            await service.store_command_context(
                command=command,
                project_path=validated_path,
                context=context
            )

            return {
                "success": True,
                "message": "Memory stored successfully",
                "command": command,
                "project_path": validated_path
            }
        except HTTPException:
            # Re-raise HTTP exceptions as-is
            raise
        except ValueError as e:
            raise HTTPException(status_code=400, detail=str(e))
        except Exception as e:
            logger.error(f"Error storing memory: {e}")
            raise HTTPException(status_code=500, detail="Failed to store memory")

    @app.post("/api/v1/scan")
    async def scan_projects_endpoint(request: Request):
        """
        Scan filesystem for Maestro projects and import to database.

        Request body:
            {
                "base_dirs": ["/path/to/scan", ...],  # Optional, defaults to ~/Prod
                "max_depth": 5  # Optional, max directory depth
            }
        """
        from .scanner import MaestroScanner

        service: MaestroMemoryService = app.state.memory_service

        try:
            body = await request.json()
        except:
            body = {}

        base_dirs = body.get("base_dirs", [str(Path.home() / "Prod")])
        max_depth = body.get("max_depth", 5)

        try:
            scanner = MaestroScanner(service)
            results = await scanner.scan_directories(base_dirs, max_depth=max_depth)
            return results
        except Exception as e:
            logger.error(f"Error scanning projects: {e}")
            raise HTTPException(status_code=500, detail=f"Scan failed: {str(e)}")

    # WebSocket endpoint for real-time updates (optional, placeholder)
    @app.websocket("/ws/events")
    async def websocket_events(websocket: WebSocket):
        """WebSocket endpoint for real-time event streaming"""
        await websocket.accept()
        try:
            while True:
                # Keep connection alive
                data = await websocket.receive_text()
                # Echo back for now
                await websocket.send_json({"type": "echo", "data": data})
        except WebSocketDisconnect:
            logger.info("WebSocket disconnected")
        except Exception as e:
            logger.error(f"WebSocket error: {e}")
            await websocket.close()

    return app


# =============================================================================
# Convenience Functions
# =============================================================================


def get_dashboard_app(database_path: Optional[Path] = None, debug: bool = False) -> FastAPI:
    """
    Get or create the dashboard FastAPI application.

    Args:
        database_path: Path to SQLite database file
        debug: Enable debug mode

    Returns:
        FastAPI application instance
    """
    return create_dashboard_app(database_path=database_path, debug=debug)


__all__ = [
    "create_dashboard_app",
    "get_dashboard_app",
]
