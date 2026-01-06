"""
Maestro Memory Zoekt Integration

Fast code search integration using Zoekt for scanning and discovering Maestro projects.
Zoekt provides trigram-based indexed search for fast codebase-wide searches.

Architecture:
- ZoektClient: Python client for Zoekt JSON API
- ZoektIndexer: Manages indexing of Maestro projects
- ZoektScanner: Uses Zoekt for fast project discovery
"""

import asyncio
import json
import subprocess
import shutil
from pathlib import Path
from typing import List, Dict, Any, Optional, Set
from datetime import datetime, UTC
from dataclasses import dataclass, field
from loguru import logger

import httpx


@dataclass
class ZoektSearchResult:
    """Result from a Zoekt search query."""
    file_path: str
    repository: str
    branch: str
    line_matches: List[Dict[str, Any]]
    score: float = 0.0

    @classmethod
    def from_api_response(cls, result: Dict[str, Any]) -> "ZoektSearchResult":
        """Parse Zoekt API response into structured result."""
        file = result.get("File", "")
        repo = result.get("Repository", "")
        branch = result.get("Branch", "")
        matches = result.get("LineMatches", [])

        line_matches_parsed = []
        for match in matches:
            line_matches_parsed.append({
                "line_number": match.get("LineNumber", 0),
                "line": match.get("Line", ""),
                "before": match.get("Before", []),
                "after": match.get("After", []),
            })

        return cls(
            file_path=file,
            repository=repo,
            branch=branch,
            line_matches=line_matches_parsed,
            score=result.get("Score", 0.0)
        )


@dataclass
class ZoektConfig:
    """Configuration for Zoekt integration."""
    # Zoekt server configuration
    server_url: str = "http://127.0.0.1:6070"
    enabled: bool = True

    # Index configuration
    index_dir: Path = field(default_factory=lambda: Path.home() / ".maestro" / "zoekt_index")

    # Maestro-specific patterns
    maestro_patterns: List[str] = field(default_factory=lambda: [
        "maestro/product.md",      # Greenfield projects
        "maestro/tracks.md",       # Track definitions
        "maestro/tracks/",         # Track files
        "maestro/workflow.md",     # Workflow definitions
        ".maestro/config.json",    # Alternative marker
    ])

    # File patterns to exclude
    exclude_patterns: Set[str] = field(default_factory=lambda: {
        "node_modules/",
        "__pycache__/",
        ".git/",
        "venv/",
        "env/",
        "dist/",
        "build/",
        ".venv/",
    })

    # Search options
    max_results: int = 100
    context_lines: int = 3


class ZoektClientError(Exception):
    """Exception raised for Zoekt client errors."""
    pass


class ZoektClient:
    """
    Client for interacting with Zoekt search engine.

    Zoekt provides fast indexed code search through its JSON API.
    This client wraps the API and provides Maestro-specific search methods.
    """

    def __init__(self, config: Optional[ZoektConfig] = None):
        """
        Initialize Zoekt client.

        Args:
            config: Zoekt configuration. Uses defaults if not provided.
        """
        self.config = config or ZoektConfig()
        self.client = None
        self._search_url = f"{self.config.server_url}/api/search"

    async def __aenter__(self):
        """Async context manager entry."""
        self.client = httpx.AsyncClient(timeout=30.0)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        if self.client:
            await self.client.aclose()

    async def search(
        self,
        query: str,
        max_results: Optional[int] = None,
        repo_ids: Optional[List[int]] = None,
        context_lines: Optional[int] = None,
    ) -> List[ZoektSearchResult]:
        """
        Execute a search query against Zoekt.

        Args:
            query: Search query string (supports Zoekt query syntax)
            max_results: Maximum number of results to return
            repo_ids: Optional list of repository IDs to filter
            context_lines: Number of context lines to include

        Returns:
            List of search results

        Raises:
            ZoektClientError: If search fails
        """
        if not self.config.enabled:
            logger.debug("Zoekt search is disabled")
            return []

        if not self.client:
            self.client = httpx.AsyncClient(timeout=30.0)

        # Build request payload
        payload = {"Q": query}

        # Add search options
        opts = {}
        if max_results or self.config.max_results:
            opts["MaxNumResults"] = max_results or self.config.max_results
        if context_lines or self.config.context_lines:
            opts["NumContextLines"] = context_lines or self.config.context_lines

        if opts:
            payload["Opts"] = opts

        if repo_ids:
            payload["RepoIDs"] = repo_ids

        try:
            logger.debug(f"Executing Zoekt search: {query}")
            response = await self.client.post(
                self._search_url,
                json=payload,
                headers={"Content-Type": "application/json"}
            )

            if response.status_code != 200:
                raise ZoektClientError(
                    f"Zoekt search failed with status {response.status_code}: {response.text}"
                )

            data = response.json()

            # Parse results
            results = []
            for result in data.get("Result", {}).get("Files", []):
                results.append(ZoektSearchResult.from_api_response(result))

            logger.debug(f"Zoekt search returned {len(results)} results")
            return results

        except httpx.RequestError as e:
            raise ZoektClientError(f"HTTP request failed: {e}")
        except json.JSONDecodeError as e:
            raise ZoektClientError(f"Failed to parse JSON response: {e}")

    async def find_maestro_projects(
        self,
        base_dirs: Optional[List[str]] = None,
        max_results: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """
        Find all Maestro projects using Zoekt search.

        Uses Zoekt's powerful query capabilities to efficiently search for
        Maestro project markers across multiple directories.

        Args:
            base_dirs: Base directories to search (uses Zoekt path filters)
            max_results: Maximum number of projects to find

        Returns:
            List of discovered projects with metadata
        """
        if not self.config.enabled:
            logger.warning("Zoekt is disabled, cannot search for projects")
            return []

        # Build search query for Maestro markers
        # Search for any of the Maestro marker files
        pattern_queries = []
        for pattern in self.config.maestro_patterns:
            # Convert file pattern to zoekt query
            # "file:" pattern searches only filenames
            pattern_queries.append(f"file:{pattern}")

        # Combine with OR to find any marker
        query = " OR ".join(pattern_queries)

        # Add path restrictions if base_dirs provided
        if base_dirs:
            path_restrictions = " OR ".join([f"path:{d}" for d in base_dirs])
            query = f"({query}) ({path_restrictions})"

        try:
            results = await self.search(query, max_results=max_results)

            # Aggregate results by project directory
            projects_by_dir: Dict[str, Dict[str, Any]] = {}

            for result in results:
                # Extract project directory from file path
                file_path = Path(result.file_path)

                # Find project root (directory containing maestro/ or .maestro/)
                project_root = self._find_project_root(file_path)

                if not project_root:
                    continue

                project_key = str(project_root)

                if project_key not in projects_by_dir:
                    # Initialize project entry
                    projects_by_dir[project_key] = {
                        "path": project_key,
                        "name": project_root.name,
                        "type": self._detect_project_type(result.file_path),
                        "markers_found": [],
                        "files": [],
                    }

                # Add marker/file to project
                projects_by_dir[project_key]["markers_found"].append(Path(result.file_path).name)
                projects_by_dir[project_key]["files"].append(result.file_path)

            # Convert to list
            projects = list(projects_by_dir.values())

            logger.info(f"Zoekt discovered {len(projects)} Maestro projects")
            return projects

        except ZoektClientError as e:
            logger.error(f"Failed to search for Maestro projects: {e}")
            return []

    def _find_project_root(self, file_path: Path) -> Optional[Path]:
        """
        Find the project root directory from a file path.

        The project root is the directory containing either:
        - maestro/ directory (with project files)
        - .maestro/ directory

        Args:
            file_path: Path to a file within the project

        Returns:
            Project root directory or None
        """
        current = file_path.parent

        # Walk up the directory tree
        for _ in range(10):  # Max depth to search
            # Check for maestro/ directory
            maestro_dir = current / "maestro"
            if maestro_dir.exists() and maestro_dir.is_dir():
                # Verify it has project files
                if any((maestro_dir / f).exists() for f in ["product.md", "tracks.md", "workflow.md"]):
                    return current

            # Check for .maestro/ directory
            dot_maestro = current / ".maestro"
            if dot_maestro.exists() and dot_maestro.is_dir():
                return current

            # Move up one directory
            if current.parent == current:  # Reached root
                break
            current = current.parent

        return None

    def _detect_project_type(self, file_path: str) -> str:
        """
        Detect project type from file path.

        Args:
            file_path: Path to a Maestro marker file

        Returns:
            Project type: "greenfield", "brownfield", or "generic"
        """
        if "product.md" in file_path:
            return "greenfield"
        elif "tracks.md" in file_path or "tracks/" in file_path:
            return "brownfield"
        elif ".maestro" in file_path:
            return "generic"
        return "unknown"

    async def search_project_content(
        self,
        project_path: str,
        query: str,
        file_patterns: Optional[List[str]] = None,
    ) -> List[ZoektSearchResult]:
        """
        Search within a specific project's content.

        Args:
            project_path: Path to the project directory
            query: Search query
            file_patterns: Optional file patterns to filter (e.g., ["*.md", "*.py"])

        Returns:
            List of matching files with line matches
        """
        # Build query with path restriction
        path_query = f"path:{project_path} {query}"

        # Add file pattern restrictions if provided
        if file_patterns:
            file_restrictions = " OR ".join([f"file:{p}" for p in file_patterns])
            path_query = f"({path_query}) ({file_restrictions})"

        return await self.search(path_query)

    async def health_check(self) -> bool:
        """
        Check if Zoekt server is healthy and accessible.

        Returns:
            True if Zoekt is available, False otherwise
        """
        if not self.config.enabled:
            return False

        try:
            if not self.client:
                self.client = httpx.AsyncClient(timeout=5.0)

            response = await self.client.get(f"{self.config.server_url}/")

            if response.status_code == 200:
                logger.debug("Zoekt server is healthy")
                return True

            return False

        except Exception as e:
            logger.debug(f"Zoekt health check failed: {e}")
            return False


class ZoektIndexer:
    """
    Manages indexing of codebases with Zoekt.

    Provides methods to create and update Zoekt indexes for
    fast code search across Maestro projects.
    """

    def __init__(self, config: Optional[ZoektConfig] = None):
        """
        Initialize Zoekt indexer.

        Args:
            config: Zoekt configuration
        """
        self.config = config or ZoektConfig()
        self.index_dir = self.config.index_dir
        self.index_dir.mkdir(parents=True, exist_ok=True)

        # Check for zoekt-indexer binary
        self._indexer_cmd = self._find_indexer_command()

    def _find_indexer_command(self) -> Optional[str]:
        """Find the zoekt-indexer binary."""
        possible_paths = [
            shutil.which("zoekt-indexer"),
            Path("/home/stan/go/bin/zoekt-indexer"),
            Path.home() / "go" / "bin" / "zoekt-indexer",
            Path("/usr/local/bin/zoekt-indexer"),
        ]

        for path in possible_paths:
            if path and (Path(path) if isinstance(path, str) else path).exists():
                cmd = str(path)
                logger.debug(f"Found zoekt-indexer at: {cmd}")
                return cmd

        logger.warning("zoekt-indexer binary not found")
        return None

    async def index_directory(
        self,
        directory: str,
        repo_name: Optional[str] = None,
        repo_id: Optional[int] = None,
        force: bool = False,
    ) -> Dict[str, Any]:
        """
        Index a directory with Zoekt.

        Args:
            directory: Path to directory to index
            repo_name: Optional repository name
            repo_id: Optional repository ID
            force: Force re-index even if already indexed

        Returns:
            Indexing result with success status and metadata
        """
        if not self._indexer_cmd:
            return {
                "success": False,
                "error": "zoekt-indexer binary not found",
                "directory": directory
            }

        dir_path = Path(directory).expanduser().resolve()

        if not dir_path.exists():
            return {
                "success": False,
                "error": f"Directory does not exist: {directory}",
                "directory": directory
            }

        # Build command
        cmd = [
            self._indexer_cmd,
            "-index", str(self.index_dir),
            "-repo_name", repo_name or dir_path.name,
        ]

        if repo_id:
            cmd.extend(["-repo_id", str(repo_id)])

        if force:
            cmd.append("-force")

        cmd.append(str(dir_path))

        logger.info(f"Indexing directory with Zoekt: {directory}")

        try:
            # Run indexing command
            process = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )

            stdout, stderr = await process.communicate()

            if process.returncode == 0:
                logger.info(f"Successfully indexed: {directory}")
                return {
                    "success": True,
                    "directory": directory,
                    "repo_name": repo_name or dir_path.name,
                    "index_dir": str(self.index_dir),
                }
            else:
                logger.error(f"Indexing failed: {stderr.decode()}")
                return {
                    "success": False,
                    "error": stderr.decode(),
                    "directory": directory
                }

        except Exception as e:
            logger.error(f"Failed to run zoekt-indexer: {e}")
            return {
                "success": False,
                "error": str(e),
                "directory": directory
            }

    async def index_maestro_projects(
        self,
        base_dirs: List[str],
        force: bool = False,
    ) -> Dict[str, Any]:
        """
        Index all Maestro projects in the given directories.

        Args:
            base_dirs: List of base directories to scan and index
            force: Force re-indexing

        Returns:
            Summary of indexing results
        """
        results = {
            "success": True,
            "indexed": [],
            "failed": [],
            "total": 0,
        }

        for base_dir in base_dirs:
            base_path = Path(base_dir).expanduser().resolve()

            if not base_path.exists():
                results["failed"].append({
                    "directory": base_dir,
                    "error": "Directory does not exist"
                })
                continue

            # Index the entire directory tree
            # Zoekt will handle subdirectories
            result = await self.index_directory(
                str(base_path),
                repo_name=base_path.name,
                force=force,
            )

            results["total"] += 1

            if result["success"]:
                results["indexed"].append(result)
            else:
                results["failed"].append(result)

        if results["failed"]:
            results["success"] = False

        return results


# Convenience functions for common operations

async def search_maestro_projects(
    query: str,
    config: Optional[ZoektConfig] = None,
) -> List[Dict[str, Any]]:
    """
    Search for Maestro projects using Zoekt.

    Convenience function that creates a client and executes the search.

    Args:
        query: Search query
        config: Optional Zoekt configuration

    Returns:
        List of search results
    """
    client = ZoektClient(config)

    try:
        async with client:
            return await client.find_maestro_projects(max_results=100)
    except ZoektClientError as e:
        logger.error(f"Search failed: {e}")
        return []


async def index_projects_for_search(
    base_dirs: List[str],
    config: Optional[ZoektConfig] = None,
    force: bool = False,
) -> Dict[str, Any]:
    """
    Index Maestro projects for fast search.

    Convenience function that creates an indexer and indexes the directories.

    Args:
        base_dirs: Directories to index
        config: Optional Zoekt configuration
        force: Force re-indexing

    Returns:
        Indexing results
    """
    indexer = ZoektIndexer(config)
    return await indexer.index_maestro_projects(base_dirs, force=force)
