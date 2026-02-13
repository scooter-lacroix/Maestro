"""
iFlow Hook for Nexus Memory System

This module provides the hook implementation for iFlow, a configuration-based
system with MCP server integration.

iFlow details:
- Location: /home/stan/.iflow/
- Configuration based system
- MCP server integration (leindex)
- Uses minimax-m2.5 model
- Has agents in agents/ directory
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


class IFlowHook(AgentHook):
    """
    Hook for extracting memory from iFlow session execution.
    
    iFlow is a configuration-based AI coding system that:
    - Uses MCP server integration (leindex) for memory/context
    - Uses minimax-m2.5 model for AI responses
    - Has modular agent system in agents/ directory
    - Stores configuration in config files
    
    Detection paths:
    - /home/stan/.iflow/
    - ~/.iflow/
    - $IFLOW_HOME (environment variable)
    """
    
    def __init__(self) -> None:
        super().__init__()
        self.agent_type = "iflow"
        self._iflow_dir: Optional[Path] = None
        self._config_dir: Optional[Path] = None
        self._agents_dir: Optional[Path] = None
        self._session_data: Dict[str, Any] = {}
        self._detect_installation()
    
    def _detect_installation(self) -> None:
        """Detect iFlow installation and set up paths."""
        # Check common locations for iFlow
        possible_dirs = [
            Path("/home/stan/.iflow"),
            Path.home() / ".iflow",
            Path(os.environ.get("IFLOW_HOME", "")),
        ]
        
        for dir_path in possible_dirs:
            if dir_path.exists() and dir_path.is_dir():
                self._iflow_dir = dir_path
                logger.info(f"Found iFlow at: {dir_path}")
                break
        
        if self._iflow_dir:
            # Set up subdirectories
            self._config_dir = self._iflow_dir / "config"
            self._agents_dir = self._iflow_dir / "agents"
            
            # Also check for MCP integration
            self._mcp_config = self._iflow_dir / "mcp" / "config.json"
    
    async def install_session_end_hook(self) -> HookResult:
        """
        Install the session end hook for iFlow.
        
        Sets up monitoring for iFlow session completion by:
        1. Checking for active iflow processes
        2. Monitoring configuration changes
        3. Checking for MCP server activity
        """
        try:
            if not self._iflow_dir:
                return HookResult(
                    success=False,
                    agent_type=self.agent_type,
                    source="iflow_hook",
                    error="iFlow not detected on system"
                )
            
            # Start monitoring for session activity
            asyncio.create_task(self._monitor_sessions())
            
            logger.info("iFlow session end hook installed")
            return HookResult(
                success=True,
                agent_type=self.agent_type,
                source="iflow_hook",
                context={
                    "iflow_dir": str(self._iflow_dir),
                    "config_dir": str(self._config_dir) if self._config_dir else None,
                    "agents_dir": str(self._agents_dir) if self._agents_dir else None,
                    "monitoring_active": True,
                }
            )
            
        except Exception as e:
            logger.error(f"Failed to install iFlow session hook: {e}")
            return HookResult(
                success=False,
                agent_type=self.agent_type,
                source="iflow_hook",
                error=str(e)
            )
    
    async def _monitor_sessions(self) -> None:
        """Background task to monitor iFlow sessions."""
        while True:
            try:
                activity = await self.detect_session_activity()
                if activity.get("active"):
                    self._session_data = activity
                    await self.notify_activity(activity)
                await asyncio.sleep(5)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.warning(f"Session monitoring error: {e}")
                await asyncio.sleep(5)
    
    async def detect_session_activity(self) -> Dict[str, Any]:
        """
        Detect if there is active iFlow session activity.
        
        Checks:
        1. Running iflow processes
        2. MCP server activity
        3. Agent activity in agents directory
        
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
            # Check for running iflow processes
            # iFlow may run as "iflow", "node iflow", or similar
            proc_result = subprocess.run(
                ["pgrep", "-f", "iflow|minimax"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if proc_result.returncode == 0 and proc_result.stdout.strip():
                result["active"] = True
                result["context"]["processes"] = proc_result.stdout.strip().split("\n")
            
            # Check MCP server activity
            if self._iflow_dir:
                # Check for MCP server logs or activity files
                mcp_dir = self._iflow_dir / "mcp"
                if mcp_dir.exists():
                    # Check for recent activity
                    activity_file = mcp_dir / "activity.json"
                    if activity_file.exists():
                        try:
                            with open(activity_file) as f:
                                activity_data = json.load(f)
                            result["context"]["mcp_activity"] = activity_data
                            result["active"] = True
                        except Exception:
                            pass
                
                # Check config for active sessions
                config_file = self._iflow_dir / "config" / "sessions.json"
                if config_file.exists():
                    try:
                        with open(config_file) as f:
                            sessions = json.load(f)
                        
                        # Find active session
                        for session_id, session_info in sessions.items():
                            if session_info.get("status") == "active":
                                result["active"] = True
                                result["session_id"] = session_id
                                result["context"]["current_session"] = session_info
                                break
                    except Exception:
                        pass
                
                # Check agents directory for activity
                if self._agents_dir and self._agents_dir.exists():
                    # Check for running agent processes
                    agent_logs_dir = self._agents_dir / "logs"
                    if agent_logs_dir.exists():
                        recent_logs = sorted(
                            agent_logs_dir.iterdir(),
                            key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                            reverse=True
                        )[:3]
                        
                        if recent_logs:
                            latest_log = recent_logs[0]
                            log_age = datetime.now().timestamp() - latest_log.stat().st_mtime
                            
                            # Active if modified in last 5 minutes
                            if log_age < 300:
                                result["active"] = True
                                result["context"]["recent_agent_activity"] = latest_log.name
            
        except Exception as e:
            logger.warning(f"Error detecting iFlow activity: {e}")
        
        return result
    
    async def extract_session_context(self) -> HookResult:
        """
        Extract context from iFlow session.
        
        Extracts:
        - Session history
        - Configuration changes
        - MCP server interactions
        - Agent executions
        - Model usage (minimax-m2.5)
        """
        context: Dict[str, Any] = {
            "sessions": [],
            "config_changes": [],
            "mcp_interactions": [],
            "agent_executions": [],
            "model_usage": {},
        }
        
        try:
            if not self._iflow_dir:
                return HookResult(
                    success=False,
                    agent_type=self.agent_type,
                    source="iflow_hook",
                    error="iFlow directory not found"
                )
            
            # Extract session history
            sessions_file = self._iflow_dir / "config" / "sessions.json"
            if sessions_file.exists():
                try:
                    with open(sessions_file) as f:
                        sessions = json.load(f)
                    
                    # Get last 10 sessions
                    session_list = sorted(
                        sessions.items(),
                        key=lambda x: x[1].get("timestamp", ""),
                        reverse=True
                    )[:10]
                    
                    for session_id, session_info in session_list:
                        context["sessions"].append({
                            "session_id": session_id,
                            "timestamp": session_info.get("timestamp"),
                            "status": session_info.get("status"),
                            "duration": session_info.get("duration"),
                            "tasks": session_info.get("tasks", []),
                        })
                        
                except Exception as e:
                    logger.warning(f"Failed to read sessions: {e}")
            
            # Extract configuration changes
            config_dir = self._iflow_dir / "config"
            if config_dir.exists():
                # Check for config backup/history
                config_history_dir = config_dir / "history"
                if config_history_dir.exists():
                    recent_configs = sorted(
                        config_history_dir.iterdir(),
                        key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                        reverse=True
                    )[:5]
                    
                    for config_file in recent_configs:
                        try:
                            with open(config_file) as f:
                                config_data = json.load(f)
                            context["config_changes"].append({
                                "file": config_file.name,
                                "timestamp": config_data.get("timestamp"),
                                "changes": config_data.get("changes", []),
                            })
                        except Exception:
                            pass
            
            # Extract MCP interactions
            mcp_dir = self._iflow_dir / "mcp"
            if mcp_dir.exists():
                # Check leindex integration
                leindex_dir = mcp_dir / "leindex"
                if leindex_dir.exists():
                    interactions_file = leindex_dir / "interactions.json"
                    if interactions_file.exists():
                        try:
                            with open(interactions_file) as f:
                                interactions = json.load(f)
                            context["mcp_interactions"] = interactions[-20:]  # Last 20
                        except Exception:
                            pass
                
                # Check MCP logs
                mcp_logs_dir = mcp_dir / "logs"
                if mcp_logs_dir.exists():
                    recent_logs = sorted(
                        mcp_logs_dir.iterdir(),
                        key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                        reverse=True
                    )[:5]
                    
                    for log_file in recent_logs:
                        try:
                            with open(log_file) as f:
                                log_content = f.read()
                            context["mcp_interactions"].append({
                                "log": log_file.name,
                                "preview": log_content[:500],  # First 500 chars
                            })
                        except Exception:
                            pass
            
            # Extract agent executions
            if self._agents_dir and self._agents_dir.exists():
                agent_logs_dir = self._agents_dir / "logs"
                if agent_logs_dir.exists():
                    recent_logs = sorted(
                        agent_logs_dir.iterdir(),
                        key=lambda p: p.stat().st_mtime if p.is_file() else 0,
                        reverse=True
                    )[:10]
                    
                    for log_file in recent_logs:
                        try:
                            with open(log_file) as f:
                                log_data = json.load(f)
                            
                            context["agent_executions"].append({
                                "agent": log_data.get("agent_name"),
                                "timestamp": log_data.get("timestamp"),
                                "duration": log_data.get("duration"),
                                "status": log_data.get("status"),
                                "tasks": log_data.get("tasks", []),
                            })
                            
                            # Track model usage
                            model = log_data.get("model", "unknown")
                            context["model_usage"][model] = context["model_usage"].get(model, 0) + 1
                            
                        except Exception:
                            # Try to read as plain text
                            try:
                                with open(log_file) as f:
                                    content = f.read()
                                if "minimax" in content.lower():
                                    context["model_usage"]["minimax-m2.5"] = \
                                        context["model_usage"].get("minimax-m2.5", 0) + 1
                            except Exception:
                                pass
            
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
                    context["files_modified"] = modified_files
                    
            except Exception:
                pass
            
            logger.info(f"Extracted iFlow context: {len(context['sessions'])} sessions, "
                       f"{len(context['agent_executions'])} agent executions, "
                       f"model usage: {context['model_usage']}")
            
            return HookResult(
                success=True,
                agent_type=self.agent_type,
                source="iflow_hook",
                context=context
            )
            
        except Exception as e:
            logger.error(f"Failed to extract iFlow context: {e}")
            return HookResult(
                success=False,
                agent_type=self.agent_type,
                source="iflow_hook",
                error=str(e)
            )
    
    def get_config(self) -> Optional[Dict[str, Any]]:
        """Get iFlow configuration if available."""
        if not self._config_dir:
            return None
            
        config_file = self._config_dir / "config.json"
        if config_file.exists():
            try:
                with open(config_file) as f:
                    return json.load(f)
            except Exception as e:
                logger.warning(f"Failed to read iFlow config: {e}")
        
        return None
    
    def get_model_info(self) -> Optional[str]:
        """Get information about the model being used."""
        config = self.get_config()
        if config:
            return config.get("model", config.get("llm", {}).get("model"))
        return None
