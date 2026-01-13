"""
Maestro Project Detection

This module provides automatic project detection for Maestro.
It identifies Maestro projects and provides memory isolation between projects.
"""

import os
import logging
from pathlib import Path
from typing import Optional, Dict, Any
from dataclasses import dataclass

# Dependency hygiene: allow this module to import even when loguru isn't installed.
try:
    from loguru import logger  # type: ignore
except ImportError:  # pragma: no cover
    logger = logging.getLogger(__name__)


@dataclass
class ProjectInfo:
    """Information about a detected Maestro project"""
    project_path: str
    project_name: str
    project_type: Optional[str] = None  # greenfield, brownfield
    has_maestro_dir: bool = False
    has_maestro_config: bool = False
    has_track: bool = False
    current_track_id: Optional[str] = None


class ProjectDetector:
    """
    Detects Maestro projects from directory markers.

    A Maestro project is identified by:
    1. A `maestro/` directory in the project root
    2. A `.maestro/` configuration directory
    3. Presence of track files in `maestro/tracks/`
    """

    # Marker files/directories that identify a Maestro project
    MAESTRO_MARKERS = [
        "maestro",           # maestro/ directory
        ".maestro",         # .maestro/ config directory
    ]

    # Files that indicate project type
    GREENFIELD_MARKERS = [
        "maestro/product.md",           # Product definition (greenfield projects)
        "maestro/workflow.md",          # Workflow config
    ]

    def __init__(self) -> None:
        self._cache: Dict[str, Optional[ProjectInfo]] = {}

    def detect_project(self, start_path: Optional[str] = None) -> Optional[ProjectInfo]:
        """
        Detect Maestro project from current or given directory.

        Args:
            start_path: Directory to start detection from. Defaults to current working directory.

        Returns:
            ProjectInfo if a Maestro project is detected, None otherwise
        """
        if start_path is None:
            start_path = os.getcwd()

        start_path = os.path.abspath(start_path)

        # Check cache first
        if start_path in self._cache:
            return self._cache[start_path]

        # Search for Maestro project markers
        project_path = self._find_project_root(start_path)
        if not project_path:
            logger.debug(f"No Maestro project found for path: {start_path}")
            self._cache[start_path] = None
            return None

        # Gather project information
        project_info = self._gather_project_info(project_path)
        self._cache[start_path] = project_info

        logger.info(f"Detected Maestro project: {project_info.project_name} at {project_path}")
        return project_info

    def _find_project_root(self, start_path: str) -> Optional[str]:
        """
        Find the root directory of the Maestro project.

        Searches upward from start_path until finding a directory with
        Maestro markers or reaching filesystem root.

        Args:
            start_path: Directory to start searching from

        Returns:
            Project root path, or None if not found
        """
        current = Path(start_path).absolute()

        while True:
            # Check if current directory has Maestro markers
            if self._has_maestro_markers(current):
                return str(current)

            # Check if we've reached the filesystem root
            parent = current.parent
            if parent == current:  # Can't go higher
                break
            current = parent

        return None

    def _has_maestro_markers(self, path: Path) -> bool:
        """Check if directory has Maestro project markers"""
        for marker in self.MAESTRO_MARKERS:
            if (path / marker).exists():
                return True
        return False

    def _gather_project_info(self, project_path: str) -> ProjectInfo:
        """Gather detailed information about the project"""
        path = Path(project_path)

        # Check for maestro/ directory
        has_maestro_dir = (path / "maestro").exists()
        has_maestro_config = (path / ".maestro").exists()

        # Determine project type
        project_type = None
        if has_maestro_dir:
            # Check if it's a greenfield project (has product.md)
            if (path / "maestro" / "product.md").exists():
                project_type = "greenfield"
            else:
                project_type = "brownfield"

        # Check for active track
        current_track_id = self._get_current_track_id(path)

        return ProjectInfo(
            project_path=project_path,
            project_name=path.name,
            project_type=project_type,
            has_maestro_dir=has_maestro_dir,
            has_maestro_config=has_maestro_config,
            has_track=current_track_id is not None,
            current_track_id=current_track_id
        )

    def _get_current_track_id(self, project_path: Path) -> Optional[str]:
        """
        Determine the currently active track for this project.

        Looks at maestro/tracks.md to find the first incomplete track.

        Returns:
            Track ID if found, None otherwise
        """
        tracks_file = project_path / "maestro" / "tracks.md"
        if not tracks_file.exists():
            return None

        try:
            content = tracks_file.read_text()
            # Look for track IDs in the file
            # Format: "## [ ] Track: Description (track-id)"
            import re
            pattern = r'\[[ ~x]\]\s+Track:.*?\(([^)]+)\)'
            matches: list[str] = re.findall(pattern, content)

            for track_id in matches:
                # Return the first track ID found
                return track_id.strip()
        except Exception as e:
            logger.warning(f"Failed to read tracks file: {e}")

        return None

    def get_isolated_project_path(self, start_path: Optional[str] = None) -> str:
        """
        Get the project path with memory isolation.

        For Maestro projects, returns the detected project path.
        For non-Maestro directories, returns the given path as-is.

        Args:
            start_path: Directory to check

        Returns:
            Project path to use for memory isolation
        """
        project_info = self.detect_project(start_path)
        if project_info:
            return project_info.project_path
        else:
            # Not a Maestro project - use the path as-is
            return os.path.abspath(start_path) if start_path else os.getcwd()


# Singleton instance for easy access
_detector_instance = None


def detect_project(start_path: Optional[str] = None) -> Optional[ProjectInfo]:
    """
    Detect Maestro project from current or given directory.

    This is a convenience function that uses a singleton detector instance.

    Args:
        start_path: Directory to start detection from. Defaults to current working directory.

    Returns:
        ProjectInfo if a Maestro project is detected, None otherwise

    Example:
        >>> from maestro.memory.utils.detector import detect_project
        >>> project = detect_project()
        >>> if project:
        ...     print(f"Found Maestro project: {project.project_name}")
        >>>     print(f"Type: {project.project_type}")
        >>>     print(f"Track: {project.current_track_id}")
    """
    global _detector_instance
    if _detector_instance is None:
        _detector_instance = ProjectDetector()
    return _detector_instance.detect_project(start_path)


def get_project_path(start_path: Optional[str] = None) -> str:
    """
    Get the project path for memory isolation.

    This is a convenience function that returns the appropriate project path
    to use for storing/retrieving memories, with automatic detection for Maestro projects.

    Args:
        start_path: Directory to check

    Returns:
        Project path to use for memory operations

    Example:
        >>> from maestro.memory.utils.detector import get_project_path
        >>> project_path = get_project_path()
        >>> # Use project_path with memory service
        >>> await service.store_command_context(command, project_path, context)
    """
    global _detector_instance
    if _detector_instance is None:
        _detector_instance = ProjectDetector()
    return _detector_instance.get_isolated_project_path(start_path)
