"""
Base Hook Classes for Nexus Memory System Integration

This module provides the base classes for agent hooks in the Nexus Memory System.
All agent-specific hooks should inherit from the AgentHook base class.
"""

import asyncio
import os
import json
import subprocess
from pathlib import Path
from typing import Optional, Dict, Any, Callable, Awaitable
from datetime import datetime
from dataclasses import dataclass, field
from abc import ABC, abstractmethod
from loguru import logger


@dataclass
class HookResult:
    """Result of a hook operation."""
    success: bool
    agent_type: str
    source: str
    context: Optional[Dict[str, Any]] = None
    error: Optional[str] = None
    timestamp: datetime = field(default_factory=datetime.now)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            "success": self.success,
            "agent_type": self.agent_type,
            "source": self.source,
            "context": self.context,
            "error": self.error,
            "timestamp": self.timestamp.isoformat(),
        }


class AgentHook(ABC):
    """
    Base class for all agent hooks in the Nexus Memory System.
    
    Each agent type (claude-code, pi-mono, oh-my-pi, iflow, etc.) should
    implement this interface to integrate with the Nexus Memory System.
    
    Attributes:
        agent_type: Unique identifier for the agent type
        session_id: Current session identifier (if active)
    """
    
    def __init__(self) -> None:
        self.agent_type: str = "unknown"
        self.session_id: Optional[str] = None
        self._session_end_hook: Optional[Callable[[], Awaitable[None]]] = None
        self._activity_callbacks: list[Callable[[Dict[str, Any]], Awaitable[None]]] = []
    
    @abstractmethod
    async def install_session_end_hook(self) -> HookResult:
        """
        Install the session end hook for the agent.
        
        This method should set up any necessary monitoring to detect
        when a session ends and trigger context extraction.
        
        Returns:
            HookResult indicating success/failure of hook installation
        """
        pass
    
    @abstractmethod
    async def detect_session_activity(self) -> Dict[str, Any]:
        """
        Detect if there is active session activity for this agent.
        
        This method should check if the agent is currently running
        and gathering context.
        
        Returns:
            Dictionary with keys:
                - active: bool - Whether session is active
                - session_id: Optional[str] - Current session ID if active
                - context: Optional[Dict] - Any context gathered so far
        """
        pass
    
    @abstractmethod
    async def extract_session_context(self) -> HookResult:
        """
        Extract context from the current or recent session.
        
        This method should gather all relevant context from the
        agent's session including:
        - Files modified
        - Commands executed
        - Agent decisions made
        - Any other relevant context
        
        Returns:
            HookResult with extracted context
        """
        pass
    
    def set_session_end_callback(self, callback: Callable[[], Awaitable[None]]) -> None:
        """Set a callback to be called when the session ends."""
        self._session_end_hook = callback
    
    def add_activity_callback(self, callback: Callable[[Dict[str, Any]], Awaitable[None]]) -> None:
        """Add a callback to be called on activity updates."""
        self._activity_callbacks.append(callback)
    
    async def notify_activity(self, activity_data: Dict[str, Any]) -> None:
        """Notify all activity callbacks of new activity."""
        for callback in self._activity_callbacks:
            try:
                await callback(activity_data)
            except Exception as e:
                logger.warning(f"Activity callback error: {e}")
    
    def _find_executable(self, *names: str) -> Optional[Path]:
        """Find an executable in PATH or common locations."""
        for name in names:
            # Check PATH
            path = shutil.which(name)
            if path:
                return Path(path)
            
            # Check common locations
            common_paths = [
                Path.home() / ".local/bin" / name,
                Path.home() / "bin" / name,
                Path("/usr/local/bin") / name,
                Path("/usr/bin") / name,
            ]
            for common_path in common_paths:
                if common_path.exists() and os.access(common_path, os.X_OK):
                    return common_path
        
        return None
    
    def _run_command(
        self, 
        cmd: list[str], 
        timeout: int = 30,
        cwd: Optional[Path] = None
    ) -> tuple[int, str, str]:
        """
        Run a command and return (returncode, stdout, stderr).
        
        Args:
            cmd: Command and arguments as list
            timeout: Timeout in seconds
            cwd: Working directory
            
        Returns:
            Tuple of (returncode, stdout, stderr)
        """
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=cwd,
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return -1, "", "Command timed out"
        except Exception as e:
            return -1, "", str(e)


import shutil
