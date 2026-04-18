"""
Standalone Nexus bridge for Maestro memory operations.

This module shells out to the installed `nexus` CLI instead of importing
Nexus Python packages directly. It also provides the compatibility helpers
needed while the remaining dashboard/admin paths migrate off the old service.
"""

from __future__ import annotations

import asyncio
import json
import os
import re
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Optional

import logging

try:
    from loguru import logger
except ImportError:  # pragma: no cover - optional dependency in minimal test envs
    logger = logging.getLogger(__name__)
from sqlalchemy import select, text

from maestro.memory.database.async_db import AsyncDatabaseManager
from maestro.memory.database.models import MaestroProject, MaestroTrack


def _json_dumps(value: Any) -> str:
    return json.dumps(value, ensure_ascii=True, default=str)


@dataclass(slots=True)
class NexusCommandResult:
    success: bool
    stdout: str
    stderr: str
    returncode: int
    parsed: Any = None


class StandaloneNexusClient:
    """CLI-backed Nexus bridge with lightweight hydration helpers."""

    def __init__(
        self,
        database_path: Optional[Path] = None,
        nexus_bin: Optional[str] = None,
        nexus_home: Optional[Path] = None,
        config_path: Optional[Path] = None,
        async_db: Optional[AsyncDatabaseManager] = None,
    ) -> None:
        self.database_path = database_path or (Path.home() / ".maestro" / "maestro.db")
        self.nexus_bin = nexus_bin or os.environ.get("NEXUS_BIN", "nexus")
        self.nexus_home = nexus_home or Path(os.environ.get("NEXUS_HOME", Path.home() / ".nexus"))
        default_config = Path.home() / ".config" / "nexus-memory-system" / "nexus.env"
        config_env = os.environ.get("NEXUS_CONFIG")
        self.config_path = config_path or (Path(config_env) if config_env else default_config)
        self.async_db = async_db or AsyncDatabaseManager(database_path=self.database_path)

    async def store_memory(
        self,
        content: str,
        agent: str = "maestro",
        category: str = "context",
        labels: Optional[Iterable[str]] = None,
        metadata: Optional[dict[str, Any]] = None,
        memory_lane_type: Optional[str] = None,
    ) -> dict[str, Any]:
        # Use temp files for sensitive data to avoid exposing via CLI argv
        content_file = None
        metadata_file = None

        try:
            # Write content to temp file with secure permissions
            with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix='nexus_content_') as f:
                f.write(content)
                content_file = f.name

            args = [
                "store",
                "--content-file",
                content_file,
                "--agent",
                agent,
                "--category",
                category,
            ]
            label_list = [label for label in (labels or []) if label]
            if label_list:
                args.extend(["--labels", ",".join(label_list)])

            # Write metadata to temp file if present
            if metadata:
                with tempfile.NamedTemporaryFile(mode='w', delete=False, prefix='nexus_metadata_') as f:
                    f.write(_json_dumps(metadata))
                    metadata_file = f.name
                    os.chmod(metadata_file, 0o600)
                args.extend(["--metadata-json-file", metadata_file])

            if memory_lane_type:
                args.extend(["--memory-lane-type", memory_lane_type])

            result = await self._run_nexus(args)
        finally:
            # Clean up temp files
            if content_file and os.path.exists(content_file):
                try:
                    os.unlink(content_file)
                except OSError:
                    pass
            if metadata_file and os.path.exists(metadata_file):
                try:
                    os.unlink(metadata_file)
                except OSError:
                    pass
        memory_id = self._extract_memory_id(result.stdout)
        if not result.success or memory_id is None:
            return {
                "success": False,
                "error": result.stderr or result.stdout or "nexus store failed",
                "stdout": result.stdout,
                "stderr": result.stderr,
                "returncode": result.returncode,
            }

        return {
            "success": True,
            "memory_id": memory_id,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "returncode": result.returncode,
        }

    async def store_command_context(
        self,
        command: str,
        project_path: str,
        context: dict[str, Any],
        project_id: Optional[int] = None,
        track_row_id: Optional[int] = None,
        track_id: Optional[str] = None,
        session_id: Optional[str] = None,
        agent: str = "maestro",
    ) -> dict[str, Any]:
        payload = {
            "command": command,
            "project_path": project_path,
            "context": context,
            "track_id": track_id,
            "session_id": session_id,
            "maestro_project_id": project_id,
            "maestro_track_id": track_row_id,
            "maestro_command": command,
            "maestro_command_context": context,
        }
        content = (
            f"Command: {command}\n"
            f"Project: {project_path}\n\n"
            f"Context:\n{_json_dumps(context)}"
        )
        result = await self.store_memory(
            content=content,
            agent=agent,
            category="context",
            labels=[command, "maestro"],
            metadata=payload,
        )
        if result.get("success") and result.get("memory_id") is not None:
            await self._sync_maestro_columns(
                memory_id=int(result["memory_id"]),
                project_id=project_id,
                track_row_id=track_row_id,
                command=command,
                context=context,
            )
        result["payload"] = payload
        return result

    async def search_similar_commands(
        self,
        query: str,
        agent: str = "maestro",
        project_path: Optional[str] = None,
        limit: int = 5,
        include_raw: bool = False,
    ) -> dict[str, Any]:
        include_raw_flag = ["--include-raw"] if include_raw else []
        recall = await self._run_nexus(
            [
                "recall",
                "--query",
                query,
                "--agent",
                agent,
                "--limit",
                str(limit),
                "--format",
                "json",
                *include_raw_flag,
            ]
        )

        parsed: list[dict[str, Any]] = []
        if recall.success:
            try:
                parsed = json.loads(recall.stdout or "[]")
                if not isinstance(parsed, list):
                    parsed = []
            except json.JSONDecodeError:
                parsed = self._parse_memory_list_output(recall.stdout)
        else:
            parsed = self._parse_memory_list_output(recall.stdout)

        if parsed:
            memory_ids = [item["id"] for item in parsed if isinstance(item, dict) and "id" in item]
            hydrated = await self.hydrate_memories(memory_ids) if memory_ids else []
            hydrated_by_id: dict[int, dict[str, Any]] = {}
            for item in hydrated:
                if not isinstance(item, dict) or item.get("id") is None:
                    continue
                try:
                    hydrated_by_id[int(item["id"])] = item
                except (TypeError, ValueError):
                    continue
            merged_results: list[dict[str, Any]] = []
            for item in parsed:
                if not isinstance(item, dict):
                    continue
                merged = dict(hydrated_by_id.get(int(item["id"]), {})) if item.get("id") is not None else {}
                merged.update(item)
                merged.setdefault("tags", merged.get("labels", []))
                merged.setdefault("stored_by", merged.get("metadata", {}).get("stored_by", agent))
                merged.setdefault(
                    "related_memory_ids",
                    merged.get("metadata", {}).get("related_memory_ids", []),
                )
                merged.setdefault(
                    "nexus_runtime_state",
                    merged.get("metadata", {}).get("nexus_runtime_state"),
                )
                merged.setdefault("nexus_scope", merged.get("metadata", {}).get("nexus_scope"))
                merged_results.append(merged)

            return {
                "success": True,
                "query": query,
                "agent": agent,
                "results": merged_results,
                "memory_ids": memory_ids,
                "stdout": recall.stdout,
                "stderr": recall.stderr,
                "returncode": recall.returncode,
            }

        if not parsed:
            fallback = await self._run_nexus(
                [
                    "search",
                    "--query",
                    query,
                    "--agent",
                    agent,
                    "--limit",
                    str(limit),
                    *include_raw_flag,
                ]
            )
            if fallback.success:
                parsed = self._parse_search_output(fallback.stdout)
            # Use parsed CLI results when available; fall back to SQL only if empty
            results = parsed if parsed else await self._query_command_fallback(query, project_path=project_path, limit=limit)
            return {
                "success": fallback.success,
                "query": query,
                "agent": agent,
                "results": results,
                "stdout": fallback.stdout,
                "stderr": fallback.stderr,
                "returncode": fallback.returncode,
            }

    async def retrieve_project_context(self, project_path: str, limit: int = 10) -> list[dict[str, Any]]:
        return await self._query_context_by_scope(
            "maestro_project_id = :scope_id",
            project_path=project_path,
            limit=limit,
        )

    async def retrieve_track_context(self, track_id: str, limit: int = 20) -> list[dict[str, Any]]:
        return await self._query_context_by_scope(
            "maestro_track_id = :scope_id",
            track_id=track_id,
            limit=limit,
        )

    async def retrieve_session_context(self, session_id: str, limit: int = 20) -> list[dict[str, Any]]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            stmt = text(
                """
                SELECT
                    id,
                    content,
                    created_at,
                    category,
                    labels,
                    metadata,
                    maestro_command,
                    maestro_command_context
                FROM memories
                WHERE is_active = 1
                  AND json_extract(metadata, '$.session_id') = :session_id
                ORDER BY created_at DESC
                LIMIT :limit_val
                """
            )
            result = await session.execute(
                stmt,
                {"session_id": session_id, "limit_val": limit},
            )
            return [self._row_to_context_dict(row) for row in result.fetchall()]

    async def list_projects(self) -> list[dict[str, Any]]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            result = await session.execute(select(MaestroProject).order_by(MaestroProject.last_active.desc()))
            return [project.to_dict() for project in result.scalars().all()]

    async def list_tracks(self, project_id: Optional[int] = None) -> list[dict[str, Any]]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            stmt = select(MaestroTrack).order_by(MaestroTrack.created_at.desc())
            if project_id is not None:
                stmt = stmt.filter_by(project_id=project_id)
            result = await session.execute(stmt)
            tracks = []
            for track in result.scalars().all():
                data = track.to_dict()
                data.setdefault("phase_count", 0)
                data.setdefault("current_phase", 0)
                data.setdefault("total_tasks", 0)
                data.setdefault("completed_tasks", 0)
                tracks.append(data)
            return tracks

    async def hydrate_memories(self, memory_ids: Iterable[int]) -> list[dict[str, Any]]:
        ids = [int(memory_id) for memory_id in memory_ids if memory_id is not None and str(memory_id).strip()]
        if not ids:
            return []

        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            placeholders = ", ".join(f":id_{index}" for index, _ in enumerate(ids))
            params = {f"id_{index}": memory_id for index, memory_id in enumerate(ids)}
            stmt = text(
                f"""
                SELECT
                    id,
                    content,
                    category,
                    created_at,
                    last_accessed,
                    labels,
                    metadata,
                    maestro_project_id,
                    maestro_track_id,
                    maestro_command,
                    maestro_command_context
                FROM memories
                WHERE id IN ({placeholders})
                ORDER BY created_at DESC
                """
            )
            result = await session.execute(stmt, params)
            return [self._row_to_memory_dict(row) for row in result.fetchall()]

    async def list_memories(
        self,
        limit: int = 50,
        offset: int = 0,
        project_path: Optional[str] = None,
        track_id: Optional[str] = None,
    ) -> list[dict[str, Any]]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            conditions = ["is_active = 1"]
            params: dict[str, Any] = {"limit_val": limit, "offset_val": offset}

            if project_path:
                conditions.append("json_extract(metadata, '$.project_path') = :project_path")
                params["project_path"] = project_path

            if track_id:
                conditions.append("json_extract(metadata, '$.track_id') = :track_id")
                params["track_id"] = track_id

            where_clause = " AND ".join(conditions)
            stmt = text(
                f"""
                SELECT
                    id,
                    content,
                    created_at,
                    category,
                    labels,
                    metadata,
                    maestro_project_id,
                    maestro_track_id,
                    maestro_command,
                    maestro_command_context,
                    last_accessed
                FROM memories
                WHERE {where_clause}
                ORDER BY created_at DESC
                LIMIT :limit_val OFFSET :offset_val
                """
            )
            result = await session.execute(stmt, params)
            return [self._row_to_memory_dict(row) for row in result.fetchall()]

    async def get_statistics(self, agent_type: Optional[str] = None) -> dict[str, Any]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            total_stmt = text(
                """
                SELECT COUNT(*) AS total_memories
                FROM memories
                WHERE is_active = 1
                  AND (:agent_type IS NULL OR json_extract(metadata, '$.agent_type') = :agent_type)
                """
            )
            total_result = await session.execute(total_stmt, {"agent_type": agent_type})
            total_memories = total_result.scalar() or 0

            command_stmt = text(
                """
                SELECT COALESCE(maestro_command, json_extract(metadata, '$.command'), 'unknown') AS command_name,
                       COUNT(*) AS count
                FROM memories
                WHERE is_active = 1
                  AND (:agent_type IS NULL OR json_extract(metadata, '$.agent_type') = :agent_type)
                GROUP BY command_name
                ORDER BY count DESC
                """
            )
            command_result = await session.execute(command_stmt, {"agent_type": agent_type})
            memories_by_command = {
                row[0]: int(row[1])
                for row in command_result.fetchall()
                if row[0]
            }

            project_stmt = text(
                """
                SELECT COALESCE(json_extract(metadata, '$.project_path'), 'unknown') AS project_path,
                       COUNT(*) AS count
                FROM memories
                WHERE is_active = 1
                  AND (:agent_type IS NULL OR json_extract(metadata, '$.agent_type') = :agent_type)
                GROUP BY project_path
                ORDER BY count DESC
                """
            )
            project_result = await session.execute(project_stmt, {"agent_type": agent_type})
            memories_by_project = {
                row[0]: int(row[1])
                for row in project_result.fetchall()
                if row[0]
            }
        return {
            "success": True,
            "statistics": {
                "total_memories": int(total_memories),
                "memories_by_command": memories_by_command,
                "memories_by_project": memories_by_project,
            },
        }

    async def close(self) -> None:
        await self.async_db.close()

    async def _query_context_by_scope(
        self,
        scope_clause: str,
        project_path: Optional[str] = None,
        track_id: Optional[str] = None,
        limit: int = 10,
    ) -> list[dict[str, Any]]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            scope_id: Optional[int] = None
            if project_path:
                project = await session.scalar(select(MaestroProject).where(MaestroProject.project_path == project_path))
                if project is None:
                    return []
                scope_id = int(project.id)
            if track_id:
                track = await session.scalar(select(MaestroTrack).where(MaestroTrack.track_id == track_id))
                if track is None:
                    return []
                scope_id = int(track.id)
            if scope_id is None:
                return []

            stmt = text(
                f"""
                SELECT
                    id,
                    content,
                    created_at,
                    category,
                    labels,
                    metadata,
                    maestro_command,
                    maestro_command_context
                FROM memories
                WHERE {scope_clause}
                  AND is_active = 1
                ORDER BY created_at DESC
                LIMIT :limit_val
                """
            )
            result = await session.execute(
                stmt,
                {"scope_id": scope_id, "limit_val": limit},
            )
            return [self._row_to_context_dict(row, project_path=project_path, track_id=track_id) for row in result.fetchall()]

    async def _sync_maestro_columns(
        self,
        memory_id: int,
        project_id: Optional[int],
        track_row_id: Optional[int],
        command: str,
        context: dict[str, Any],
    ) -> None:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            await session.execute(
                text(
                    """
                    UPDATE memories
                    SET maestro_project_id = :project_id,
                        maestro_track_id = :track_id,
                        maestro_command = :command,
                        maestro_command_context = :context_json
                    WHERE id = :memory_id
                    """
                ),
                {
                    "project_id": project_id,
                    "track_id": track_row_id,
                    "command": command,
                    "context_json": _json_dumps(context),
                    "memory_id": memory_id,
                },
            )
            await session.commit()

    async def _query_command_fallback(
        self,
        command: str,
        project_path: Optional[str],
        limit: int,
    ) -> list[dict[str, Any]]:
        await self.async_db.initialize()
        async with self.async_db.get_async_session() as session:
            stmt = text(
                """
                SELECT
                    id,
                    content,
                    created_at,
                    category,
                    labels,
                    metadata,
                    maestro_command,
                    maestro_command_context
                FROM memories
                WHERE is_active = 1
                  AND (
                    maestro_command = :command
                    OR json_extract(metadata, '$.maestro_command') = :command
                  )
                  AND (
                    :project_path IS NULL
                    OR json_extract(metadata, '$.project_path') = :project_path
                  )
                ORDER BY created_at DESC
                LIMIT :limit_val
                """
            )
            result = await session.execute(
                stmt,
                {
                    "command": command,
                    "project_path": project_path,
                    "limit_val": limit,
                },
            )
            return [
                self._row_to_context_dict(row, project_path=project_path)
                for row in result.fetchall()
            ]

    async def _run_nexus(self, args: list[str]) -> NexusCommandResult:
        env = os.environ.copy()
        env.setdefault("NEXUS_HOME", str(self.nexus_home))
        env.setdefault("NEXUS_DATABASE_PATH", str(self.database_path))
        env.setdefault("MAESTRO_NEXUS_PROVIDER", "standalone")

        command = [self.nexus_bin]
        if self.config_path:
            command.extend(["--config", str(self.config_path)])
        command.extend(args)
        # Log command without sensitive payload args (--content, --metadata-json, --content-file, --metadata-json-file)
        safe_preview = []
        skip_next = False
        for part in command:
            if skip_next:
                safe_preview.append("<redacted>")
                skip_next = False
                continue
            if part in ("--content", "--metadata-json", "--content-file", "--metadata-json-file"):
                skip_next = True
                safe_preview.append(part)
                continue
            safe_preview.append(part)
        logger.debug("Running nexus command: %s", " ".join(safe_preview))
        proc = await asyncio.create_subprocess_exec(
            *command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
        )
        try:
            stdout_b, stderr_b = await asyncio.wait_for(proc.communicate(), timeout=30.0)
        except asyncio.TimeoutError as e:
            proc.kill()
            await proc.wait()
            raise TimeoutError("Subprocess timed out after 30 seconds") from e
        stdout = stdout_b.decode("utf-8", errors="replace").strip()
        stderr = stderr_b.decode("utf-8", errors="replace").strip()
        return NexusCommandResult(
            success=proc.returncode == 0,
            stdout=stdout,
            stderr=stderr,
            returncode=proc.returncode or 0,
        )

    @staticmethod
    def _extract_memory_id(stdout: str) -> Optional[int]:
        for pattern in (
            r"ID:\s*(\d+)",
            r"Memory ID:\s*(\d+)",
            r"Stored memory.*?\b(\d+)\b",
        ):
            match = re.search(pattern, stdout, re.IGNORECASE | re.DOTALL)
            if match:
                try:
                    return int(match.group(1))
                except ValueError:
                    return None
        return None

    @staticmethod
    def _parse_memory_list_output(stdout: str) -> list[dict[str, Any]]:
        items: list[dict[str, Any]] = []
        current: dict[str, Any] | None = None
        for line in stdout.splitlines():
            stripped = line.strip()
            if not stripped:
                continue
            if stripped.startswith("ID:"):
                if current:
                    items.append(current)
                current = {"raw": []}
                match = re.search(r"ID:\s*(\d+)\s*\|\s*Category:\s*([^|]+)\|\s*(.+)$", stripped)
                if match:
                    current.update(
                        {
                            "id": int(match.group(1)),
                            "category": match.group(2).strip(),
                            "created_at": match.group(3).strip(),
                        }
                    )
                continue
            if current is not None:
                current.setdefault("raw", []).append(stripped)
                if "content" not in current:
                    current["content"] = stripped
        if current:
            items.append(current)
        return items

    @staticmethod
    def _parse_search_output(stdout: str) -> list[dict[str, Any]]:
        items: list[dict[str, Any]] = []
        current: dict[str, Any] | None = None
        for line in stdout.splitlines():
            stripped = line.strip()
            if not stripped:
                continue
            if stripped.startswith("ID:"):
                if current:
                    items.append(current)
                current = {"raw": []}
                match = re.search(r"ID:\s*(\d+)\s*\|\s*Category:\s*([^|]+)\|\s*(.+)$", stripped)
                if match:
                    current.update(
                        {
                            "id": int(match.group(1)),
                            "category": match.group(2).strip(),
                            "created_at": match.group(3).strip(),
                        }
                    )
                continue
            if current is not None:
                current.setdefault("raw", []).append(stripped)
                if "content" not in current:
                    current["content"] = stripped
        if current:
            items.append(current)
        return items

    @staticmethod
    def _row_to_context_dict(
        row: Any,
        project_path: Optional[str] = None,
        track_id: Optional[str] = None,
    ) -> dict[str, Any]:
        metadata = {}
        labels = []
        context = {}
        if getattr(row, "metadata", None):
            try:
                metadata = json.loads(row.metadata)
            except (TypeError, json.JSONDecodeError):
                metadata = {}
        if getattr(row, "labels", None):
            try:
                labels = json.loads(row.labels)
            except (TypeError, json.JSONDecodeError):
                labels = []
        if getattr(row, "maestro_command_context", None):
            try:
                context = json.loads(row.maestro_command_context)
            except (TypeError, json.JSONDecodeError):
                context = {}

        return {
            "id": row.id,
            "command": getattr(row, "maestro_command", None) or metadata.get("maestro_command", "unknown"),
            "project_path": project_path or metadata.get("project_path"),
            "track_id": track_id or metadata.get("track_id"),
            "session_id": metadata.get("session_id") or context.get("session_id"),
            "context": context or metadata.get("maestro_command_context", {}),
            "content": row.content,
            "created_at": row.created_at,
            "category": row.category,
            "labels": labels,
            "tags": labels,
            "metadata": metadata,
            "stored_by": metadata.get("stored_by") or metadata.get("agent") or metadata.get("agent_type") or "maestro",
            "related_memory_ids": metadata.get("related_memory_ids", []),
            "nexus_runtime_state": metadata.get("nexus_runtime_state"),
            "nexus_scope": metadata.get("nexus_scope"),
        }

    @staticmethod
    def _row_to_memory_dict(row: Any) -> dict[str, Any]:
        metadata = {}
        tags = []
        command_context = {}
        raw_metadata = getattr(row, "metadata", None)
        if raw_metadata:
            try:
                metadata = json.loads(raw_metadata)
            except (TypeError, json.JSONDecodeError):
                metadata = {}
        raw_tags = getattr(row, "labels", None)
        if raw_tags:
            try:
                tags = json.loads(raw_tags)
            except (TypeError, json.JSONDecodeError):
                tags = []
        raw_command_context = getattr(row, "maestro_command_context", None)
        if raw_command_context:
            try:
                command_context = json.loads(raw_command_context)
            except (TypeError, json.JSONDecodeError):
                command_context = {}

        return {
            "id": row.id,
            "content": row.content,
            "category": row.category,
            "summary": None,
            "importance": None,
            "source": None,
            "session_id": metadata.get("session_id"),
            "project_id": getattr(row, "maestro_project_id", None),
            "track_id": metadata.get("track_id") or getattr(row, "maestro_track_id", None),
            "command": getattr(row, "maestro_command", None) or metadata.get("maestro_command"),
            "command_context": command_context,
            "created_at": row.created_at,
            "expires_at": None,
            "last_accessed": getattr(row, "last_accessed", None),
            "metadata": metadata,
            "tags": tags,
            "labels": tags,
            "stored_by": metadata.get("stored_by") or metadata.get("agent") or metadata.get("agent_type") or "maestro",
            "related_memory_ids": metadata.get("related_memory_ids", []),
            "nexus_runtime_state": metadata.get("nexus_runtime_state"),
            "nexus_scope": metadata.get("nexus_scope"),
            "access_history": metadata.get("access_history", []),
        }


class CompatibilityMemoryManager:
    """Compatibility wrapper preserving the old async memory_manager surface."""

    def __init__(self, db_manager: AsyncDatabaseManager) -> None:
        self.client = StandaloneNexusClient(async_db=db_manager)

    async def store_memory(
        self,
        content: str,
        agent_type: str = "maestro",
        category: str = "context",
        labels: Optional[Iterable[str]] = None,
        metadata: Optional[dict[str, Any]] = None,
        memory_lane_type: Optional[str] = None,
        **_: Any,
    ) -> dict[str, Any]:
        if metadata and metadata.get("maestro_command"):
            return await self.client.store_command_context(
                command=str(metadata["maestro_command"]),
                project_path=str(metadata.get("project_path", "")),
                context=dict(metadata.get("maestro_command_context") or {}),
                project_id=metadata.get("maestro_project_id"),
                track_row_id=metadata.get("maestro_track_id"),
                track_id=metadata.get("track_id"),
                session_id=metadata.get("session_id"),
                agent=agent_type,
            )
        return await self.client.store_memory(
            content=content,
            agent=agent_type,
            category=category,
            labels=labels,
            metadata=metadata,
            memory_lane_type=memory_lane_type,
        )

    async def search_memories(
        self,
        query: str,
        agent_type: str = "maestro",
        limit: int = 5,
        category: Optional[str] = None,
        **_: Any,
    ) -> dict[str, Any]:
        result = await self.client.search_similar_commands(
            query=query,
            agent=agent_type,
            limit=limit,
            project_path=_.get("project_path"),
        )
        memory_ids = result.get("memory_ids") or []
        if memory_ids:
            hydrated = await self.client.hydrate_memories(memory_ids)
        else:
            hydrated = result.get("results", [])
        if category:
            hydrated = [item for item in hydrated if item.get("category") == category]
        result["results"] = hydrated[:limit]
        return result

    async def get_statistics(self, agent_type: Optional[str] = None) -> dict[str, Any]:
        return await self.client.get_statistics(agent_type=agent_type)


def get_database_manager() -> type[AsyncDatabaseManager]:
    return AsyncDatabaseManager


def get_memory_manager() -> type[CompatibilityMemoryManager]:
    return CompatibilityMemoryManager


def get_nexus_base() -> None:
    return None


__all__ = [
    "CompatibilityMemoryManager",
    "NexusCommandResult",
    "StandaloneNexusClient",
    "get_database_manager",
    "get_memory_manager",
    "get_nexus_base",
]
