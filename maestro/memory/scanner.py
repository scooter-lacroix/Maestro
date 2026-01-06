"""
Maestro Project Scanner

Scans the filesystem for Maestro projects and imports them into the database.
This populates the dashboard with real project and track data.

Enhanced with Zoekt integration for fast indexed code search.
"""

import os
import re
import json
from pathlib import Path
from typing import List, Dict, Any, Optional
from datetime import datetime, UTC
from loguru import logger

from .service import MaestroMemoryService
from .database.models import MaestroProject, MaestroTrack
from .utils.detector import ProjectDetector, ProjectInfo
from .search.zoekt_client import ZoektClient, ZoektConfig, ZoektIndexer


class MaestroScanner:
    """
    Scans directories for Maestro projects and imports them to the database.
    """

    # File patterns to look for
    TRACK_FILE_PATTERN = re.compile(r'\[([x ~])\]\s+Track:\s*(.+?)(?:\(([^)]+)\))?$', re.MULTILINE)
    TASK_PATTERN = re.compile(r'^\s*-\s*\[([x ])\]', re.MULTILINE)

    def __init__(self, service: MaestroMemoryService, use_zoekt: bool = True):
        """
        Initialize scanner with a MaestroMemoryService instance.

        Args:
            service: Initialized MaestroMemoryService for database operations
            use_zoekt: Whether to use Zoekt for fast indexed search (default: True)
        """
        self.service = service
        self.detector = ProjectDetector()
        self.use_zoekt = use_zoekt
        self.zoekt_config = ZoektConfig()
        self.zoekt_client: Optional[ZoektClient] = None
        self.zoekt_indexer: Optional[ZoektIndexer] = None

    async def scan_directories(self, base_dirs: List[str], max_depth: int = 5) -> Dict[str, Any]:
        """
        Scan multiple directories for Maestro projects.

        Enhanced with Zoekt integration for fast indexed search when available.

        Args:
            base_dirs: List of directories to scan
            max_depth: Maximum directory depth to search

        Returns:
            Summary of discovered projects and tracks
        """
        discovered_projects = []
        discovered_tracks = []
        errors = []
        scan_method = "unknown"

        logger.info(f"Starting scan of {len(base_dirs)} directories with max_depth={max_depth}")
        for i, base_dir in enumerate(base_dirs):
            logger.info(f"  [{i+1}/{len(base_dirs)}] {base_dir}")

        try:
            # Try Zoekt-based scanning first (fast indexed search)
            if self.use_zoekt and await self._check_zoekt_available():
                logger.info("Using Zoekt for fast indexed project discovery")
                scan_method = "zoekt"
                zoekt_results = await self._scan_with_zoekt(base_dirs)

                # Import Zoekt-discovered projects
                for project_info in zoekt_results:
                    try:
                        project_path = Path(project_info["path"])
                        db_project = await self._import_project({
                            "path": str(project_path),
                            "name": project_info.get("name", project_path.name),
                            "type": project_info.get("type", "unknown"),
                            "description": None,
                            "tech_stack": []
                        })
                        discovered_projects.append({
                            "path": str(project_path),
                            "name": project_info.get("name", project_path.name),
                            "type": project_info.get("type", "unknown"),
                            "id": db_project.id,
                            "scan_method": "zoekt"
                        })

                        # Parse and import tracks
                        tracks = self._parse_tracks(project_path)
                        for track in tracks:
                            db_track = await self._import_track(db_project.id, track)
                            discovered_tracks.append({
                                "track_id": track["track_id"],
                                "title": track["title"],
                                "project_id": db_project.id
                            })

                    except Exception as e:
                        logger.error(f"Error importing Zoekt-discovered project {project_info.get('path')}: {e}")
                        errors.append(f"Error importing {project_info.get('path')}: {str(e)}")

            else:
                # Fall back to filesystem traversal
                if self.use_zoekt:
                    logger.info("Zoekt unavailable, falling back to filesystem scanning")
                else:
                    logger.info("Using filesystem traversal for project discovery")
                scan_method = "filesystem"

                for base_dir in base_dirs:
                    base_path = Path(base_dir).expanduser().resolve()
                    if not base_path.exists():
                        logger.warning(f"Directory does not exist: {base_dir}")
                        errors.append(f"Directory does not exist: {base_dir}")
                        continue

                    logger.info(f"Scanning directory: {base_path}")

                    # Find all potential project directories
                    for project_path in self._find_maestro_projects(base_path, max_depth):
                        try:
                            project_info = self._parse_project(project_path)
                            if project_info:
                                # Import to database
                                db_project = await self._import_project(project_info)
                                discovered_projects.append({
                                    "path": str(project_path),
                                    "name": project_info.get("name", project_path.name),
                                    "type": project_info.get("type", "unknown"),
                                    "id": db_project.id,
                                    "scan_method": "filesystem"
                                })

                                # Parse and import tracks
                                tracks = self._parse_tracks(project_path)
                                for track in tracks:
                                    db_track = await self._import_track(db_project.id, track)
                                    discovered_tracks.append({
                                        "track_id": track["track_id"],
                                        "title": track["title"],
                                        "project_id": db_project.id
                                    })

                        except Exception as e:
                            logger.error(f"Error processing project {project_path}: {e}")
                            errors.append(f"Error in {project_path}: {str(e)}")

        except Exception as e:
            logger.error(f"Scan failed: {e}")
            errors.append(f"Scan failed: {str(e)}")

        # Store a memory for this scan operation
        if discovered_projects:
            await self._store_scan_memory(discovered_projects, discovered_tracks)

        # Log comprehensive summary
        logger.info(f"Scan complete using method: {scan_method}")
        logger.info(f"Projects discovered: {len(discovered_projects)}")
        logger.info(f"Tracks discovered: {len(discovered_tracks)}")
        if errors:
            logger.warning(f"Errors during scan: {len(errors)}")
            for error in errors[:5]:  # Log first 5 errors
                logger.warning(f"  - {error}")

        return {
            "success": True,
            "projects_found": len(discovered_projects),
            "tracks_found": len(discovered_tracks),
            "projects": discovered_projects,
            "tracks": discovered_tracks,
            "errors": errors,
            "scan_method": scan_method,
            "directories_scanned": base_dirs,
            "max_depth": max_depth
        }

    async def _check_zoekt_available(self) -> bool:
        """
        Check if Zoekt server is available for indexed search.

        Returns:
            True if Zoekt is available and healthy
        """
        try:
            if not self.zoekt_client:
                self.zoekt_client = ZoektClient(self.zoekt_config)
            return await self.zoekt_client.health_check()
        except Exception as e:
            logger.debug(f"Zoekt health check failed: {e}")
            return False

    async def _scan_with_zoekt(self, base_dirs: List[str]) -> List[Dict[str, Any]]:
        """
        Use Zoekt for fast indexed project discovery.

        Args:
            base_dirs: Base directories to search

        Returns:
            List of discovered project information
        """
        if not self.zoekt_client:
            self.zoekt_client = ZoektClient(self.zoekt_config)

        try:
            # Find Maestro projects using Zoekt
            projects = await self.zoekt_client.find_maestro_projects(
                base_dirs=base_dirs,
                max_results=1000  # High limit for comprehensive discovery
            )

            logger.info(f"Zoekt discovered {len(projects)} projects")
            return projects

        except Exception as e:
            logger.error(f"Zoekt scan failed: {e}")
            return []

    def _find_maestro_projects(self, base_path: Path, max_depth: int) -> List[Path]:
        """
        Find all directories containing Maestro markers.

        Args:
            base_path: Starting directory
            max_depth: Maximum depth to search

        Returns:
            List of project paths
        """
        projects = []

        def is_real_maestro_project(path: Path) -> bool:
            """
            Check if this is a real Maestro project, not just a directory
            containing a `maestro/` Python package.

            A real Maestro project has:
            - maestro/product.md (greenfield)
            - maestro/tracks.md or maestro/tracks/ (any project)
            - .maestro/ directory (alternative marker)
            - product.md at root level (alternative structure)
            - tracks.md at root level (alternative structure)
            """
            maestro_dir = path / "maestro"
            dotmaestro_dir = path / ".maestro"

            # Check for .maestro config directory
            if dotmaestro_dir.exists() and dotmaestro_dir.is_dir():
                return True

            # Check for maestro/ with project files (not just a Python package)
            if maestro_dir.exists() and maestro_dir.is_dir():
                # Must have product.md, tracks.md, or workflow.md to be a project
                if (maestro_dir / "product.md").exists():
                    return True
                if (maestro_dir / "tracks.md").exists():
                    return True
                if (maestro_dir / "workflow.md").exists():
                    return True
                if (maestro_dir / "tracks").is_dir():
                    return True

            # Check for alternative structure: product.md and tracks.md at root level
            # This handles cases like Artificial_Labs/conductor where the project
            # is in a subdirectory without a maestro/ wrapper
            if (path / "product.md").exists() and (path / "tracks.md").exists():
                return True

            # Check for tracks directory at root level
            if (path / "tracks").is_dir():
                if (path / "product.md").exists() or (path / "tracks.md").exists():
                    return True

            return False

        def search(path: Path, depth: int):
            if depth > max_depth:
                return

            try:
                # Check if this directory is a real Maestro project
                if is_real_maestro_project(path):
                    # Check if we already found a parent project
                    is_nested = any(
                        path.is_relative_to(existing) or existing.is_relative_to(path)
                        for existing in projects
                    )
                    # Only add if not nested within an already-found project
                    if not is_nested:
                        projects.append(path)
                    # Still recurse into subdirectories to find nested projects
                    else:
                        # This is a nested project, skip it
                        logger.debug(f"Skipping nested maestro project: {path}")
                        return

                # Recurse into subdirectories
                for item in path.iterdir():
                    if item.is_dir() and not item.name.startswith('.'):
                        # Skip common non-project directories
                        if item.name in ('node_modules', '__pycache__', 'venv', 'env', '.git', 'dist', 'build'):
                            continue
                        search(item, depth + 1)

            except PermissionError:
                pass  # Skip directories we can't read

        search(base_path, 0)
        return projects

    def _parse_project(self, project_path: Path) -> Optional[Dict[str, Any]]:
        """
        Parse project information from a Maestro project directory.

        Args:
            project_path: Path to the project root

        Returns:
            Project info dictionary or None if invalid
        """
        maestro_dir = project_path / "maestro"
        dotmaestro_dir = project_path / ".maestro"

        info = {
            "path": str(project_path),
            "name": project_path.name,
            "type": None,
            "description": None,
            "tech_stack": []
        }

        # Try to read product.md for description (check both maestro/ and root)
        product_file = maestro_dir / "product.md"
        if not product_file.exists():
            product_file = project_path / "product.md"

        if product_file.exists():
            try:
                content = product_file.read_text()
                # Extract first paragraph as description
                lines = content.strip().split('\n')
                for line in lines:
                    if line.strip() and not line.startswith('#'):
                        info["description"] = line.strip()[:500]
                        break
                info["type"] = "greenfield"
            except Exception as e:
                logger.warning(f"Failed to read product.md: {e}")

        # Check for .maestro/config.json
        config_file = dotmaestro_dir / "config.json"
        if config_file.exists():
            try:
                config = json.loads(config_file.read_text())
                info["name"] = config.get("project_name", info["name"])
                info["description"] = config.get("description", info["description"])
                info["tech_stack"] = config.get("tech_stack", [])
            except Exception as e:
                logger.warning(f"Failed to read .maestro/config.json: {e}")

        # Determine project type if not set
        if not info["type"]:
            if maestro_dir.exists():
                info["type"] = "brownfield"
            elif dotmaestro_dir.exists():
                info["type"] = "generic"
            elif (project_path / "product.md").exists():
                info["type"] = "greenfield"

        return info

    def _parse_tracks(self, project_path: Path) -> List[Dict[str, Any]]:
        """
        Parse track information from project's tracks.md file.

        Args:
            project_path: Path to the project root

        Returns:
            List of track dictionaries
        """
        tracks = []

        # Try both maestro/tracks.md and tracks.md at root
        tracks_file = project_path / "maestro" / "tracks.md"
        if not tracks_file.exists():
            tracks_file = project_path / "tracks.md"

        if not tracks_file.exists():
            return tracks

        try:
            content = tracks_file.read_text()

            # Find all track definitions
            for match in self.TRACK_FILE_PATTERN.finditer(content):
                status_char = match.group(1)
                title = match.group(2).strip()
                track_id = match.group(3).strip() if match.group(3) else self._generate_track_id(title)

                # Map status character to status string
                status_map = {'x': 'completed', ' ': 'new', '~': 'in_progress'}
                status = status_map.get(status_char, 'new')

                tracks.append({
                    "track_id": track_id,
                    "title": title,
                    "status": status,
                    "description": None,
                    "total_tasks": 0,
                    "completed_tasks": 0
                })

            # Try to parse individual track files for more details
            # Check both maestro/tracks/ and tracks/ at root
            tracks_dir = project_path / "maestro" / "tracks"
            if not tracks_dir.exists():
                tracks_dir = project_path / "tracks"

            if tracks_dir.exists():
                for track_file in tracks_dir.glob("*.md"):
                    track_content = track_file.read_text()
                    track_id = track_file.stem

                    # Count tasks
                    all_tasks = self.TASK_PATTERN.findall(track_content)
                    total_tasks = len(all_tasks)
                    completed_tasks = sum(1 for t in all_tasks if t == 'x')

                    # Update existing track or add new one
                    existing = next((t for t in tracks if t["track_id"] == track_id), None)
                    if existing:
                        existing["total_tasks"] = total_tasks
                        existing["completed_tasks"] = completed_tasks
                    else:
                        # Extract title from first heading
                        title_match = re.search(r'^#\s+(.+)$', track_content, re.MULTILINE)
                        title = title_match.group(1) if title_match else track_id

                        tracks.append({
                            "track_id": track_id,
                            "title": title,
                            "status": "in_progress" if completed_tasks < total_tasks else "completed",
                            "description": None,
                            "total_tasks": total_tasks,
                            "completed_tasks": completed_tasks
                        })

        except Exception as e:
            logger.error(f"Failed to parse tracks from {tracks_file}: {e}")

        return tracks

    def _generate_track_id(self, title: str) -> str:
        """Generate a track ID from title."""
        # Convert to lowercase, replace spaces with hyphens, remove special chars
        clean = re.sub(r'[^a-z0-9\s-]', '', title.lower())
        clean = re.sub(r'\s+', '-', clean.strip())
        timestamp = datetime.now(UTC).strftime("%Y%m%d")
        return f"{clean[:30]}-{timestamp}"

    async def _import_project(self, project_info: Dict[str, Any]) -> MaestroProject:
        """
        Import a project to the database.

        Args:
            project_info: Project information dictionary

        Returns:
            Created or updated MaestroProject
        """
        async with self.service.db_manager.get_async_session() as session:
            project = await MaestroProject.get_or_create(
                session,
                project_path=project_info["path"],
                project_name=project_info.get("name"),
                description=project_info.get("description"),
                project_type=project_info.get("type"),
                tech_stack=project_info.get("tech_stack", [])
            )
            await session.commit()
            return project

    async def _import_track(self, project_id: int, track_info: Dict[str, Any]) -> MaestroTrack:
        """
        Import a track to the database.

        Args:
            project_id: Parent project ID
            track_info: Track information dictionary

        Returns:
            Created or updated MaestroTrack
        """
        async with self.service.db_manager.get_async_session() as session:
            track = await MaestroTrack.get_or_create(
                session,
                track_id=track_info["track_id"],
                project_id=project_id,
                title=track_info["title"],
                description=track_info.get("description"),
                status=track_info.get("status", "new"),
                total_tasks=track_info.get("total_tasks", 0),
                completed_tasks=track_info.get("completed_tasks", 0)
            )
            await session.commit()
            return track

    async def _store_scan_memory(self, projects: List[Dict], tracks: List[Dict]) -> None:
        """Store a memory record for this scan operation."""
        try:
            await self.service.store_command_context(
                command="/maestro:scan",
                project_path=str(Path.home()),  # Use home as a neutral path
                context={
                    "action": "filesystem_scan",
                    "projects_discovered": len(projects),
                    "tracks_discovered": len(tracks),
                    "project_names": [p["name"] for p in projects[:10]],  # First 10
                    "timestamp": datetime.now(UTC).isoformat()
                }
            )
        except Exception as e:
            logger.warning(f"Failed to store scan memory: {e}")


async def scan_projects(
    base_dirs: Optional[List[str]] = None,
    service: Optional[MaestroMemoryService] = None
) -> Dict[str, Any]:
    """
    Convenience function to scan for Maestro projects.

    Args:
        base_dirs: Directories to scan. Defaults to comprehensive search across common project locations:
                   - ~/Prod (main production directory)
                   - ~/dev (development projects)
                   - ~/projects (general projects)
                   - ~/code (code projects)
                   - ~/work (work projects)
                   - ~ (home directory for root-level projects)
        service: MaestroMemoryService instance. Creates one if not provided.

    Returns:
        Scan results dictionary
    """
    if base_dirs is None:
        # Comprehensive default search across common project locations
        home = Path.home()
        base_dirs = [
            str(home / "Prod"),
            str(home / "dev"),
            str(home / "projects"),
            str(home / "code"),
            str(home / "work"),
            str(home),  # Home directory for root-level projects
        ]
        logger.info(f"Scanning default directories: {base_dirs}")

    if service is None:
        service = MaestroMemoryService()
        await service.initialize()
        own_service = True
    else:
        own_service = False

    try:
        scanner = MaestroScanner(service)
        results = await scanner.scan_directories(base_dirs)
        return results
    finally:
        if own_service:
            await service.close()
