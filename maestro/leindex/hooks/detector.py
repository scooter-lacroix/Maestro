"""
Agent Detector for Nexus Memory System

This module provides functionality for detecting available coding agents
on the system. It checks for various agents like pi-mono, oh-my-pi, iflow,
and others to determine which hooks should be activated.
"""

import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field
from datetime import datetime
from loguru import logger

from .pi_mono import PiMonoHook
from .oh_my_pi import OhMyPiHook
from .iflow import IFlowHook


@dataclass
class DetectedAgent:
    """Represents a detected agent on the system."""
    agent_type: str
    name: str
    executable_path: Optional[str] = None
    version: Optional[str] = None
    config_dir: Optional[str] = None
    session_dir: Optional[str] = None
    is_active: bool = False
    last_activity: Optional[datetime] = None
    metadata: Dict[str, Any] = field(default_factory=dict)


class AgentDetector:
    """
    Detector for finding and identifying coding agents on the system.
    
    Searches common installation paths and uses process detection
    to find available agents like:
    - pi-mono
    - oh-my-pi
    - iflow
    - claude-code
    - opencode
    - Other agents
    """
    
    # Common agent configurations
    AGENT_CONFIGS = {
        "pi-mono": {
            "name": "Pi-Mono",
            "executable_names": ["pi"],
            "paths": [
                Path("/home/stan/pi-mono/pi"),
                Path.home() / ".local/bin/pi",
                Path("/usr/local/bin/pi"),
            ],
            "config_dir": ".pi",
            "session_dir": ".pi/sessions",
        },
        "oh-my-pi": {
            "name": "Oh-My-Pi",
            "executable_names": ["omp", "oh-my-pi"],
            "paths": [
                Path("/home/stan/Prod/maestro/vendor/oh-my-pi/pi"),
                Path("/tmp/oh-my-pi/pi"),
                Path.home() / "oh-my-pi/pi",
                Path.home() / ".local/bin/omp",
            ],
            "config_dir": ".omp",
            "session_dir": ".omp/sessions",
        },
        "iflow": {
            "name": "iFlow",
            "executable_names": ["iflow"],
            "paths": [
                Path("/home/stan/.iflow"),
                Path.home() / ".iflow",
            ],
            "config_dir": ".iflow/config",
            "session_dir": ".iflow/sessions",
        },
    }
    
    def __init__(self) -> None:
        self._detected_agents: Dict[str, DetectedAgent] = {}
        self._scan_completed: bool = False
    
    def detect_all(self) -> Dict[str, DetectedAgent]:
        """
        Detect all available agents on the system.
        
        Returns:
            Dictionary mapping agent types to detected agent info
        """
        if self._scan_completed:
            return self._detected_agents
        
        # Detect each agent type
        for agent_type, config in self.AGENT_CONFIGS.items():
            agent = self._detect_agent(agent_type, config)
            if agent:
                self._detected_agents[agent_type] = agent
        
        # Also try to detect running processes
        self._detect_running_agents()
        
        self._scan_completed = True
        return self._detected_agents
    
    def _detect_agent(self, agent_type: str, config: Dict[str, Any]) -> Optional[DetectedAgent]:
        """Detect a specific agent type."""
        executable_path = None
        config_dir = None
        session_dir = None
        
        # Check for executable
        for exe_name in config.get("executable_names", []):
            # Check configured paths
            for path in config.get("paths", []):
                if path.exists() and os.access(path, os.X_OK):
                    executable_path = str(path)
                    break
            
            # Check PATH if not found
            if not executable_path:
                path = shutil.which(exe_name)
                if path:
                    executable_path = path
        
        # Check for config directory
        config_path = config.get("config_dir")
        if config_path:
            config_dir_path = Path.home() / config_path
            if config_dir_path.exists():
                config_dir = str(config_dir_path)
                
                # Check for sessions
                session_path = config.get("session_dir")
                if session_path:
                    session_dir_path = Path.home() / session_path
                    if session_dir_path.exists():
                        session_dir = str(session_dir_path)
        
        # If we found something, create the detected agent
        if executable_path or config_dir:
            version = self._get_version(executable_path) if executable_path else None
            
            return DetectedAgent(
                agent_type=agent_type,
                name=config.get("name", agent_type),
                executable_path=executable_path,
                version=version,
                config_dir=config_dir,
                session_dir=session_dir,
            )
        
        return None
    
    def _get_version(self, executable_path: Optional[str]) -> Optional[str]:
        """Get the version of an executable."""
        if not executable_path:
            return None
        
        try:
            result = subprocess.run(
                [executable_path, "--version"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            
            if result.returncode == 0:
                return result.stdout.strip()
        except Exception as e:
            logger.debug(f"Failed to get version for {executable_path}: {e}")
        
        return None
    
    def _detect_running_agents(self) -> None:
        """Detect agents by checking running processes."""
        try:
            # Check for various agent processes
            agent_processes = {
                "pi-mono": ["pi", "subagent"],
                "oh-my-pi": ["omp", "oh-my-pi"],
                "iflow": ["iflow", "minimax"],
            }
            
            for agent_type, process_names in agent_processes.items():
                for proc_name in process_names:
                    result = subprocess.run(
                        ["pgrep", "-f", proc_name],
                        capture_output=True,
                        text=True,
                        timeout=5,
                    )
                    
                    if result.returncode == 0 and result.stdout.strip():
                        # Agent is running
                        if agent_type in self._detected_agents:
                            self._detected_agents[agent_type].is_active = True
                            self._detected_agents[agent_type].last_activity = datetime.now()
                        break
        
        except Exception as e:
            logger.warning(f"Error detecting running agents: {e}")
    
    def is_agent_available(self, agent_type: str) -> bool:
        """Check if a specific agent is available."""
        if not self._scan_completed:
            self.detect_all()
        return agent_type in self._detected_agents
    
    def get_agent(self, agent_type: str) -> Optional[DetectedAgent]:
        """Get detected agent info."""
        if not self._scan_completed:
            self.detect_all()
        return self._detected_agents.get(agent_type)
    
    def get_all_agents(self) -> Dict[str, DetectedAgent]:
        """Get all detected agents."""
        if not self._scan_completed:
            self.detect_all()
        return self._detected_agents.copy()
    
    def get_active_agents(self) -> List[DetectedAgent]:
        """Get list of currently active agents."""
        if not self._scan_completed:
            self.detect_all()
        return [agent for agent in self._detected_agents.values() if agent.is_active]
    
    def create_hook(self, agent_type: str) -> Optional[Any]:
        """
        Create a hook for the specified agent type if available.
        
        Args:
            agent_type: The type of agent to create a hook for
            
        Returns:
            AgentHook instance or None if agent not available
        """
        if not self.is_agent_available(agent_type):
            return None
        
        # Import here to avoid circular imports
        from .factory import create_hook
        return create_hook(agent_type)


# Global detector instance
_detector: Optional[AgentDetector] = None


def get_agent_detector() -> AgentDetector:
    """Get the global agent detector instance."""
    global _detector
    if _detector is None:
        _detector = AgentDetector()
    return _detector


def detect_available_agents() -> Dict[str, DetectedAgent]:
    """Convenience function to detect all available agents."""
    detector = get_agent_detector()
    return detector.detect_all()


def is_agent_available(agent_type: str) -> bool:
    """Convenience function to check if an agent is available."""
    detector = get_agent_detector()
    return detector.is_agent_available(agent_type)
