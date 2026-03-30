"""
Maestro-specific hooks for memory extraction

This module provides hooks for extracting memory from Maestro command execution.
Each command (setup, newTrack, implement, status) has a context extractor.
"""

import re
import json
import os
from pathlib import Path
from typing import Callable, Awaitable, Optional, Dict, Any
from loguru import logger

# Placeholder - will import from Nexus when integrated
# from nexus.hooks.base import AgentHook, HookResult

class HookResult:
    """Placeholder for Nexus HookResult"""
    def __init__(self, success: bool, agent_type: str, source: str,
                 context: Optional[dict] = None, error: Optional[str] = None) -> None:
        self.success = success
        self.agent_type = agent_type
        self.source = source
        self.context = context
        self.error = error

class MaestroCommandHook:
    """
    Hook for extracting memory from Maestro command execution.

    This hook integrates with Maestro commands to automatically
    extract context when commands complete.
    """

    def __init__(self, memory_service: Optional[Any] = None) -> None:
        self.agent_type = "maestro"
        self.memory_service = memory_service

    async def on_command_complete(
        self,
        command: str,
        result: Dict[str, Any],
        project_path: str
    ) -> HookResult:
        """
        Called when a Maestro command completes.

        Args:
            command: Command that was executed (e.g., "/maestro:setup")
            result: Command execution result
            project_path: Project directory path

        Returns:
            HookResult indicating success/failure with extracted context
        """
        try:
            # Normalize command name
            command = command.strip()

            # Route to appropriate extractor
            if command == "/maestro:setup":
                context = await self._extract_setup_context(result, project_path)
            elif command == "/maestro:newTrack":
                context = await self._extract_newtrack_context(result, project_path)
            elif command == "/maestro:implement":
                context = await self._extract_implement_context(result, project_path)
            elif command == "/maestro:status":
                context = await self._extract_status_context(result, project_path)
            else:
                logger.warning(f"Unknown command for context extraction: {command}")
                return HookResult(
                    success=False,
                    agent_type="maestro",
                    source="maestro_hook",
                    error=f"Unknown command: {command}"
                )

            # Store extracted context if memory service is available
            if self.memory_service and context:
                try:
                    session_id = result.get("session_id") or os.environ.get("MAESTRO_SESSION_ID")
                    if session_id:
                        context["session_id"] = session_id
                    context["project_path"] = project_path
                    memory_id = await self.memory_service.store_command_context(
                        command=command,
                        project_path=project_path,
                        context=context,
                    )
                    logger.info(f"Stored command context for {command}: memory_id={memory_id}")
                except Exception as e:
                    logger.error(f"Failed to store command context: {e}")
                    # Still return success - extraction worked, storage failed

            return HookResult(
                success=True,
                agent_type="maestro",
                source="maestro_hook",
                context=context
            )

        except Exception as e:
            logger.error(f"Error extracting command context: {e}")
            return HookResult(
                success=False,
                agent_type="maestro",
                source="maestro_hook",
                error=str(e)
            )

    async def _extract_setup_context(
        self,
        result: Dict[str, Any],
        project_path: str
    ) -> Dict[str, Any]:
        """
        Extract context from /maestro:setup command.

        Expected context fields:
        - project_type: "greenfield" or "brownfield"
        - product_definition: From maestro/product.md (if greenfield)
        - tech_stack: From maestro/tech-stack.md
        - workflow_config: From maestro/workflow.md
        - setup_status: Success/failure of setup
        - files_created: List of files created during setup
        """
        context: Dict[str, Any] = {
            "setup_status": "unknown",
            "project_type": "unknown",
            "files_created": []
        }

        try:
            project_path_obj = Path(project_path)
            maestro_dir = project_path_obj / "maestro"

            # Determine project type
            if (maestro_dir / "product.md").exists():
                context["project_type"] = "greenfield"
            else:
                context["project_type"] = "brownfield"

            # Extract setup status from result
            if isinstance(result, dict):
                context["setup_status"] = "success" if result.get("success", False) else "failed"
                context["files_created"] = result.get("files_created", [])

            # Read product definition (greenfield projects)
            if context["project_type"] == "greenfield":
                product_file = maestro_dir / "product.md"
                if product_file.exists():
                    try:
                        content = product_file.read_text()
                        context["product_definition"] = self._extract_markdown_summary(content)
                    except Exception as e:
                        logger.warning(f"Failed to read product.md: {e}")

            # Read tech stack
            tech_stack_file = maestro_dir / "tech-stack.md"
            if tech_stack_file.exists():
                try:
                    content = tech_stack_file.read_text()
                    context["tech_stack"] = self._extract_markdown_summary(content)
                except Exception as e:
                    logger.warning(f"Failed to read tech-stack.md: {e}")

            # Read workflow config
            workflow_file = maestro_dir / "workflow.md"
            if workflow_file.exists():
                try:
                    content = workflow_file.read_text()
                    context["workflow_config"] = self._extract_markdown_summary(content)
                except Exception as e:
                    logger.warning(f"Failed to read workflow.md: {e}")

            logger.info(f"Extracted setup context for {project_path}")

        except Exception as e:
            logger.error(f"Error extracting setup context: {e}")
            context["error"] = str(e)

        return context

    async def _extract_newtrack_context(
        self,
        result: Dict[str, Any],
        project_path: str
    ) -> Dict[str, Any]:
        """
        Extract context from /maestro:newTrack command.

        Expected context fields:
        - track_id: Track identifier
        - track_title: Track title
        - track_description: Track description
        - tasks: List of tasks in the track
        - status: Track status ("new", "in_progress", etc.)
        """
        context: Dict[str, Any] = {
            "status": "new",
            "tasks": []
        }

        try:
            # Extract track info from result
            if isinstance(result, dict):
                context["track_id"] = result.get("track_id", "")
                context["track_title"] = result.get("track_title", "")
                context["track_description"] = result.get("track_description", "")
                raw_tasks = result.get("tasks", [])
                # Convert task dictionaries to task titles if expecting Sequence[str]
                context["tasks"] = [task.get("title", "") if isinstance(task, dict) else task for task in raw_tasks]
                context["status"] = result.get("status", "new")

                # If result contains track directory, read plan file
                track_dir = result.get("track_dir")
                if track_dir:
                    track_path = Path(track_dir)
                    plan_file = track_path / "plan.md"

                    if plan_file.exists():
                        try:
                            content = plan_file.read_text()
                            context["plan_content"] = self._extract_markdown_summary(content)
                            # Extract tasks from plan if not provided
                            if not context["tasks"]:
                                extracted_tasks = self._extract_tasks_from_plan(content)
                                # Convert task dictionaries to task titles if expecting Sequence[str]
                                context["tasks"] = [task.get("title", "") if isinstance(task, dict) else task for task in extracted_tasks]
                        except Exception as e:
                            logger.warning(f"Failed to read plan.md: {e}")

            logger.info(f"Extracted newTrack context: {context.get('track_id', 'unknown')}")

        except Exception as e:
            logger.error(f"Error extracting newTrack context: {e}")
            context["error"] = str(e)

        return context

    async def _extract_implement_context(
        self,
        result: Dict[str, Any],
        project_path: str
    ) -> Dict[str, Any]:
        """
        Extract context from /maestro:implement command.

        Expected context fields:
        - track_id: Track being implemented
        - tasks_completed: List of completed tasks
        - tasks_remaining: List of remaining tasks
        - commits_made: List of commit hashes
        - coverage: Test coverage percentage
        - implementation_status: Overall status
        """
        context: Dict[str, Any] = {
            "tasks_completed": [],
            "tasks_remaining": [],
            "commits_made": [],
            "coverage": 0.0,
            "implementation_status": "unknown"
        }

        try:
            # Extract from result
            if isinstance(result, dict):
                context["track_id"] = result.get("track_id", "")
                context["tasks_completed"] = result.get("tasks_completed", [])
                context["tasks_remaining"] = result.get("tasks_remaining", [])
                context["commits_made"] = result.get("commits_made", [])
                context["coverage"] = result.get("coverage", 0.0)
                context["implementation_status"] = result.get("status", "completed")

            # Try to read track plan to get remaining tasks
            track_id = context.get("track_id")
            if track_id:
                project_path_obj = Path(project_path)
                track_dir = project_path_obj / "maestro" / "tracks" / str(track_id)
                plan_file = track_dir / "plan.md"

                if plan_file.exists():
                    try:
                        content = plan_file.read_text()
                        # Parse tasks from plan
                        all_tasks = self._extract_tasks_from_plan(content)
                        completed_tasks = context.get("tasks_completed", [])
                        if not isinstance(completed_tasks, (list, tuple, set)):
                            completed_tasks = []

                        # Determine remaining tasks
                        if all_tasks and not context.get("tasks_remaining"):
                            context["tasks_remaining"] = [
                                task for task in all_tasks
                                if task not in completed_tasks
                            ]
                    except Exception as e:
                        logger.warning(f"Failed to read track plan: {e}")

            logger.info(f"Extracted implement context for track: {context.get('track_id', 'unknown')}")

        except Exception as e:
            logger.error(f"Error extracting implement context: {e}")
            context["error"] = str(e)

        return context

    async def _extract_status_context(
        self,
        result: Dict[str, Any],
        project_path: str
    ) -> Dict[str, Any]:
        """
        Extract context from /maestro:status command.

        Expected context fields:
        - active_track: Currently active track
        - current_phase: Current phase number/name
        - progress: Progress summary (e.g., "5/17 tasks")
        - blockers: List of blockers
        - next_actions: Recommended next actions
        """
        context: Dict[str, Any] = {
            "active_track": None,
            "current_phase": None,
            "progress": "0/0",
            "blockers": [],
            "next_actions": []
        }

        try:
            # Extract from result
            if isinstance(result, dict):
                context["active_track"] = result.get("active_track")
                context["current_phase"] = result.get("current_phase")
                context["progress"] = result.get("progress", "0/0")
                context["blockers"] = result.get("blockers", [])
                context["next_actions"] = result.get("next_actions", [])

            # If no track info in result, try reading from tracks.md
            if not context.get("active_track"):
                project_path_obj = Path(project_path)
                tracks_file = project_path_obj / "maestro" / "tracks.md"

                if tracks_file.exists():
                    try:
                        content = tracks_file.read_text()
                        # Find active track (first incomplete track)
                        track_match = re.search(r'\[~\]\s+Track:.*?\(([^)]+)\)', content)
                        if track_match:
                            track_id = str(track_match.group(1))
                            context["active_track"] = track_id

                            # Try to read track plan for progress
                            track_dir = project_path_obj / "maestro" / "tracks" / track_id
                            plan_file = track_dir / "plan.md"

                            if plan_file.exists():
                                plan_content = plan_file.read_text()
                                tasks = self._extract_tasks_from_plan(plan_content)
                                completed = len([t for t in tasks if t.get("completed", False)])
                                total = len(tasks)
                                context["progress"] = f"{completed}/{total}"

                    except Exception as e:
                        logger.warning(f"Failed to read tracks.md: {e}")

            logger.info(f"Extracted status context for {project_path}")

        except Exception as e:
            logger.error(f"Error extracting status context: {e}")
            context["error"] = str(e)

        return context

    def _extract_markdown_summary(self, content: str, max_lines: int = 20) -> str:
        """
        Extract a summary from markdown content.

        Args:
            content: Full markdown content
            max_lines: Maximum lines to include in summary

        Returns:
            Summary string
        """
        lines = content.split("\n")

        # Remove empty lines at start
        while lines and not lines[0].strip():
            lines.pop(0)

        # Take first N lines or until we hit a good stopping point
        summary_lines = []
        for i, line in enumerate(lines[:max_lines]):
            summary_lines.append(line)
            # Stop at major section break
            if line.startswith("##") and i > 3:
                break

        return "\n".join(summary_lines)

    def _extract_tasks_from_plan(self, plan_content: str) -> list[dict[str, Any]]:
        """
        Extract task list from a track plan markdown file.

        Args:
            plan_content: Content of plan.md

        Returns:
            List of task dictionaries with 'id', 'title', 'completed' keys
        """
        tasks = []
        current_phase = None

        for line in plan_content.split("\n"):
            # Track phase
            phase_match = re.match(r'^###\s+(.+)$', line)
            if phase_match:
                current_phase = phase_match.group(1).strip()
                continue

            # Extract task: "- [x] Task: Title" or "- [ ] Task: Title"
            task_match = re.match(r'^-\s+\[([ x])\]\s+Task:\s*(.+)$', line)
            if task_match:
                status_char = task_match.group(1)
                title = task_match.group(2).strip()

                tasks.append({
                    "title": title,
                    "completed": status_char == "x",
                    "phase": current_phase
                })

        return tasks
