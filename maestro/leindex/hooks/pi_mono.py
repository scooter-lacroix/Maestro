"""
Pi-Mono Hook for Nexus Memory System

This module provides the hook implementation for pi-mono, a TypeScript/Node.js
based coding agent with subagent support.

Pi-Mono CLI: /home/stan/pi-mono/pi
Main package: packages/coding-agent/
"""

import asyncio
import json
import os
import subprocess
from pathlib import Path
from typing import Optional, Dict, Any
from datetime import datetime
from loguru import logger

from .base import AgentHook, HookResult


class PiMonoHook(AgentHook):
    """
    Hook for extracting memory from pi-mono session execution.
    
    Pi-mono is a TypeScript/Node.js based coding agent that provides
    subagent workflows with parallel, chain, and single execution modes.
    
    Detection paths:
    - /home/stan/pi-mono/pi (development)
    - ~/.local/bin/pi
    - /usr/local/bin/pi
    - $PATH
    
    Session files:
    - .pi/sessions/ - Session history
    - .pi/logs/ - Execution logs
    """
    
    def __init__(self) -> None:
        super().__init__()
        self.agent_type = "pi-mono"
        self._executable_path: Optional[Path] = None
        self._session_dir: Optional[Path] = None
        self._active_session_data: Dict[str, Any] = {}
        self._detect_installation()
    
    def _detect_installation(self) -> None:
        """Detect pi-mono installation and set up paths."""
        # Check common locations for pi-mono
        possible_paths = [
            Path("/home/stan/pi-mono/pi"),  # Development path
            Path.home() / ".local/bin/pi",
            Path.home() / "bin/pi",
            Path("/usr/local/bin/pi"),
            Path("/usr/bin/pi"),
        ]
        
        for path in possible_paths:
            if path.exists() and os.access(path, os.X_OK):
                self._executable_path = path
                logger.info(f"Found pi-mono at: {path}")
                break
        
        if self._executable_path is None:
            # Try to find in PATH
            try:
                result = subprocess.run(
                    ["which", "pi"],
                    capture_output=True,
                    text=True,
                    timeout=5,
                )
                if result.returncode == 0:
                    self._executable_path = Path(result.stdout.strip())
                    logger.info(f"Found pi-mono in PATH: {self._executable_path}")
            except Exception as e:
                logger.warning(f"Failed to find pi-mono in PATH: {e}")
        
        # Set up session directory
        if self._executable_path:
            # Sessions are typically in .pi directory relative to home
            pi_dir = Path.home() / ".pi"
            if pi_dir.exists():
                self._session_dir = pi_dir
    
    async def install_session_end_hook(self) -> HookResult:
        """
        Install the session end hook for pi-mono.
        
        Sets up monitoring for pi-mono session completion by:
        1. Checking for active pi-mono processes
        2. Monitoring session files for completion
        3. Setting up callbacks for session end detection
        """
        try:
            if not self._executable_path:
                return HookResult(
                    success=False,
                    agent_type=self.agent_type,
                    source="pi_mono_hook",
                    error="pi-mono not detected on system"
                )
            
            # Start monitoring for session activity
            asyncio.create_task(self._monitor_sessions())
            
            logger.info("Pi-mono session end hook installed")
            return HookResult(
                success=True,
                agent_type=self.agent_type,
                source="pi_mono_hook",
                context={
                    "executable_path": str(self._executable_path),
                    "session_dir": str(self._session_dir) if self._session_dir else None,
                    "monitoring_active": True,
                }
            )
            
        except Exception as e:
            logger.error(f"Failed to install pi-mono session hook: {e}")
            return HookResult(
                success=False,
                agent_type=self.agent_type,
                source="pi_mono_hook",
                error=str(e)
            )
    
    async def _monitor_sessions(self) -> None:
        """Background task to monitor pi-mono sessions."""
        while True:
            try:
                activity = await self.detect_session_activity()
                if activity.get("active"):
                    self._active_session_data = activity
                    await self.notify_activity(activity)
                await asyncio.sleep(5)  # Check every 5 seconds
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.warning(f"Session monitoring error: {e}")
                await asyncio.sleep(5)
    
    async def detect_session_activity(self) -> Dict[str, Any]:
        """
        Detect if there is active pi-mono session activity.
        
        Checks:
        1. Running pi processes
        2. Recent session files
        3. Active subagent processes
        
        Returns:
            Dictionary with activity status and context
        """
        result = {
            "active": False,
            "session_id": None,
            "context": {},
            "timestamp": datetime.now().isoformat(),
        }
        
        try:
            # Check for running pi-mono processes
            proc_result = subprocess.run(
                ["pgrep", "-f", "pi"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if proc_result.returncode == 0 and proc_result.stdout.strip():
                result["active"] = True
                result["context"]["processes"] = proc_result.stdout.strip().split("\n")
            
            # Check session directory for recent activity
            if self._session_dir:
                sessions_dir = self._session_dir / "sessions"
                if sessions_dir.exists():
                    # Get most recent session
                    sessions = sorted(
                        sessions_dir.iterdir(),
                        key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                        reverse=True
                    )
                    
                    if sessions:
                        latest_session = sessions[0]
                        session_age = datetime.now().timestamp() - latest_session.stat().st_mtime
                        
                        # Consider active if modified in last 5 minutes
                        if session_age < 300:
                            result["active"] = True
                            result["session_id"] = latest_session.stem
                            
                            # Try to read session data
                            try:
                                with open(latest_session) as f:
                                    result["context"]["session_data"] = json.load(f)
                            except Exception:
                                pass
            
            # Check for active subagents
            subagent_result = subprocess.run(
                ["pgrep", "-f", "subagent|skill"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if subagent_result.returncode == 0 and subagent_result.stdout.strip():
                result["context"]["subagents"] = subagent_result.stdout.strip().split("\n")
                result["active"] = True
            
        except Exception as e:
            logger.warning(f"Error detecting pi-mono activity: {e}")
        
        return result
    
    async def extract_session_context(self) -> HookResult:
        """
        Extract context from pi-mono session.
        
        Extracts:
        - Session history and logs
        - Files modified during session
        - Subagent executions
        - Commands run
        - Agent role usage (scout, architect, critic, kraken)
        """
        context: Dict[str, Any] = {
            "sessions": [],
            "files_modified": [],
            "subagent_executions": [],
            "commands_run": [],
            "role_usage": {},
        }
        
        try:
            # Extract from session directory
            if self._session_dir:
                # Read session history
                sessions_dir = self._session_dir / "sessions"
                if sessions_dir.exists():
                    sessions = sorted(
                        sessions_dir.iterdir(),
                        key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                        reverse=True
                    )[:10]  # Last 10 sessions
                    
                    for session_file in sessions:
                        try:
                            with open(session_file) as f:
                                session_data = json.load(f)
                                
                            # Extract relevant context
                            session_info = {
                                "session_id": session_file.stem,
                                "timestamp": session_data.get("timestamp"),
                                "duration": session_data.get("duration"),
                                "tasks": session_data.get("tasks", []),
                                "files": session_data.get("files_modified", []),
                            }
                            
                            context["sessions"].append(session_info)
                            context["files_modified"].extend(session_info["files"])
                            
                            # Track role usage
                            for task in session_data.get("tasks", []):
                                role = task.get("role", "unknown")
                                context["role_usage"][role] = context["role_usage"].get(role, 0) + 1
                                
                        except Exception as e:
                            logger.warning(f"Failed to read session {session_file}: {e}")
                
                # Read logs
                logs_dir = self._session_dir / "logs"
                if logs_dir.exists():
                    recent_logs = sorted(
                        logs_dir.iterdir(),
                        key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                        reverse=True
                    )[:5]
                    
                    for log_file in recent_logs:
                        try:
                            with open(log_file) as f:
                                log_content = f.read()
                                
                            # Extract commands and errors
                            for line in log_content.split("\n"):
                                if "Executing:" in line or "Command:" in line:
                                    context["commands_run"].append(line.strip())
                                    
                        except Exception as e:
                            logger.warning(f"Failed to read log {log_file}: {e}")
            
            # Get git status if in a project
            try:
                git_result = subprocess.run(
                    ["git", "status", "--porcelain"],
                    capture_output=True,
                    text=True,
                    timeout=5,
                )
                
                if git_result.returncode == 0:
                    modified_files = [
                        line[3:] for line in git_result.stdout.strip().split("\n")
                        if line.strip()
                    ]
                    context["files_modified"].extend(modified_files)
                    
            except Exception:
                pass  # Not in a git repo or git not available
            
            # Deduplicate files
            context["files_modified"] = list(set(context["files_modified"]))
            
            logger.info(f"Extracted pi-mono context: {len(context['sessions'])} sessions, "
                       f"{len(context['files_modified'])} files")
            
            return HookResult(
                success=True,
                agent_type=self.agent_type,
                source="pi_mono_hook",
                context=context
            )
            
        except Exception as e:
            logger.error(f"Failed to extract pi-mono context: {e}")
            return HookResult(
                success=False,
                agent_type=self.agent_type,
                source="pi_mono_hook",
                error=str(e)
            )
    
    def get_version(self) -> Optional[str]:
        """Get pi-mono version if available."""
        if not self._executable_path:
            return None
            
        try:
            result = subprocess.run(
                [str(self._executable_path), "--version"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if result.returncode == 0:
                return result.stdout.strip()
        except Exception as e:
            logger.warning(f"Failed to get pi-mono version: {e}")
        
        return None
