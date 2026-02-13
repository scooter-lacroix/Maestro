"""
Oh-My-Pi (OMP) Hook for Nexus Memory System

This module provides the hook implementation for oh-my-pi (OMP), 
which is a fork of pi-mono with additional features.

OMP is similar to pi-mono but may have:
- Different session storage location
- Extended capabilities
- Modified subagent system

Detection paths:
- /home/stan/Prod/maestro/vendor/oh-my-pi/
- /tmp/oh-my-pi/
- $HOME/oh-my-pi/
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


class OhMyPiHook(AgentHook):
    """
    Hook for extracting memory from oh-my-pi (OMP) session execution.
    
    Oh-My-Pi is a fork of pi-mono with additional features and modifications.
    It maintains similar session management but may have different:
    - CLI name (omp instead of pi)
    - Session storage location
    - Configuration paths
    
    Detection paths checked:
    - /home/stan/Prod/maestro/vendor/oh-my-pi/
    - /tmp/oh-my-pi/
    - ~/.oh-my-pi/
    - $PATH (omp command)
    """
    
    def __init__(self) -> None:
        super().__init__()
        self.agent_type = "oh-my-pi"
        self._executable_path: Optional[Path] = None
        self._session_dir: Optional[Path] = None
        self._config_dir: Optional[Path] = None
        self._active_session_data: Dict[str, Any] = {}
        self._detect_installation()
    
    def _detect_installation(self) -> None:
        """Detect oh-my-pi installation and set up paths."""
        # Check common locations for oh-my-pi
        possible_paths = [
            Path("/home/stan/Prod/maestro/vendor/oh-my-pi/pi"),
            Path("/tmp/oh-my-pi/pi"),
            Path.home() / "oh-my-pi" / "pi",
            Path.home() / ".local/bin/omp",
            Path.home() / "bin/omp",
            Path("/usr/local/bin/omp"),
        ]
        
        for path in possible_paths:
            if path.exists() and os.access(path, os.X_OK):
                self._executable_path = path
                logger.info(f"Found oh-my-pi at: {path}")
                break
        
        if self._executable_path is None:
            # Try to find omp in PATH
            try:
                result = subprocess.run(
                    ["which", "omp"],
                    capture_output=True,
                    text=True,
                    timeout=5,
                )
                if result.returncode == 0:
                    self._executable_path = Path(result.stdout.strip())
                    logger.info(f"Found oh-my-pi in PATH: {self._executable_path}")
            except Exception as e:
                logger.warning(f"Failed to find oh-my-pi in PATH: {e}")
        
        # Set up directories
        if self._executable_path:
            # OMP typically uses .omp or .oh-my-pi directory
            omp_dir = Path.home() / ".omp"
            if not omp_dir.exists():
                omp_dir = Path.home() / ".oh-my-pi"
            
            if omp_dir.exists():
                self._config_dir = omp_dir
                self._session_dir = omp_dir / "sessions"
    
    async def install_session_end_hook(self) -> HookResult:
        """
        Install the session end hook for oh-my-pi.
        
        Sets up monitoring for OMP session completion by:
        1. Checking for active omp processes
        2. Monitoring session files for completion
        3. Checking for OMP-specific subagent activity
        """
        try:
            if not self._executable_path:
                return HookResult(
                    success=False,
                    agent_type=self.agent_type,
                    source="oh_my_pi_hook",
                    error="oh-my-pi not detected on system"
                )
            
            # Start monitoring for session activity
            asyncio.create_task(self._monitor_sessions())
            
            logger.info("Oh-my-pi session end hook installed")
            return HookResult(
                success=True,
                agent_type=self.agent_type,
                source="oh_my_pi_hook",
                context={
                    "executable_path": str(self._executable_path),
                    "session_dir": str(self._session_dir) if self._session_dir else None,
                    "config_dir": str(self._config_dir) if self._config_dir else None,
                    "monitoring_active": True,
                }
            )
            
        except Exception as e:
            logger.error(f"Failed to install oh-my-pi session hook: {e}")
            return HookResult(
                success=False,
                agent_type=self.agent_type,
                source="oh_my_pi_hook",
                error=str(e)
            )
    
    async def _monitor_sessions(self) -> None:
        """Background task to monitor oh-my-pi sessions."""
        while True:
            try:
                activity = await self.detect_session_activity()
                if activity.get("active"):
                    self._active_session_data = activity
                    await self.notify_activity(activity)
                await asyncio.sleep(5)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.warning(f"Session monitoring error: {e}")
                await asyncio.sleep(5)
    
    async def detect_session_activity(self) -> Dict[str, Any]:
        """
        Detect if there is active oh-my-pi session activity.
        
        Checks:
        1. Running omp processes
        2. Recent session files
        3. OMP-specific subagent processes
        
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
            # Check for running omp processes
            proc_result = subprocess.run(
                ["pgrep", "-f", "omp|oh-my-pi"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if proc_result.returncode == 0 and proc_result.stdout.strip():
                result["active"] = True
                result["context"]["processes"] = proc_result.stdout.strip().split("\n")
            
            # Check session directory for recent activity
            if self._session_dir and self._session_dir.exists():
                sessions = sorted(
                    self._session_dir.iterdir(),
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
            
            # Check for OMP-specific extensions
            ext_result = subprocess.run(
                ["pgrep", "-f", "omp-agent|oh-my-skill"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if ext_result.returncode == 0 and ext_result.stdout.strip():
                result["context"]["extensions"] = ext_result.stdout.strip().split("\n")
                result["active"] = True
            
        except Exception as e:
            logger.warning(f"Error detecting oh-my-pi activity: {e}")
        
        return result
    
    async def extract_session_context(self) -> HookResult:
        """
        Extract context from oh-my-pi session.
        
        Extracts:
        - Session history and logs
        - Files modified during session
        - OMP-specific extensions used
        - Commands run
        - Fork-specific features used
        """
        context: Dict[str, Any] = {
            "sessions": [],
            "files_modified": [],
            "extensions_used": [],
            "commands_run": [],
            "fork_features": {},
        }
        
        try:
            # Extract from session directory
            if self._session_dir and self._session_dir.exists():
                sessions = sorted(
                    self._session_dir.iterdir(),
                    key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                    reverse=True
                )[:10]
                
                for session_file in sessions:
                    try:
                        with open(session_file) as f:
                            session_data = json.load(f)
                        
                        session_info = {
                            "session_id": session_file.stem,
                            "timestamp": session_data.get("timestamp"),
                            "duration": session_data.get("duration"),
                            "tasks": session_data.get("tasks", []),
                            "files": session_data.get("files_modified", []),
                            "extensions": session_data.get("extensions_used", []),
                        }
                        
                        context["sessions"].append(session_info)
                        context["files_modified"].extend(session_info["files"])
                        context["extensions_used"].extend(session_info.get("extensions", []))
                        
                        # Track fork-specific features
                        for task in session_data.get("tasks", []):
                            feature = task.get("feature", task.get("role", "unknown"))
                            context["fork_features"][feature] = context["fork_features"].get(feature, 0) + 1
                            
                    except Exception as e:
                        logger.warning(f"Failed to read session {session_file}: {e}")
            
            # Extract from config directory if available
            if self._config_dir:
                # Read OMP-specific configuration
                config_file = self._config_dir / "config.json"
                if config_file.exists():
                    try:
                        with open(config_file) as f:
                            config = json.load(f)
                        context["config"] = config
                    except Exception:
                        pass
                
                # Read logs
                logs_dir = self._config_dir / "logs"
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
                                
                            for line in log_content.split("\n"):
                                if "Executing:" in line or "Command:" in line or "OMP:" in line:
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
                pass
            
            # Deduplicate
            context["files_modified"] = list(set(context["files_modified"]))
            context["extensions_used"] = list(set(context["extensions_used"]))
            
            logger.info(f"Extracted oh-my-pi context: {len(context['sessions'])} sessions, "
                       f"{len(context['files_modified'])} files, {len(context['extensions_used'])} extensions")
            
            return HookResult(
                success=True,
                agent_type=self.agent_type,
                source="oh_my_pi_hook",
                context=context
            )
            
        except Exception as e:
            logger.error(f"Failed to extract oh-my-pi context: {e}")
            return HookResult(
                success=False,
                agent_type=self.agent_type,
                source="oh_my_pi_hook",
                error=str(e)
            )
    
    def get_version(self) -> Optional[str]:
        """Get oh-my-pi version if available."""
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
            logger.warning(f"Failed to get oh-my-pi version: {e}")
        
        return None
