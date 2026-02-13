"""
Nexus Memory System Integration Hooks

This package provides hooks for integrating various coding agents
with the Nexus Memory System.

Available hooks:
- pi-mono: Hook for pi-mono coding agent
- oh-my-pi: Hook for oh-my-pi (OMP) fork of pi-mono
- iflow: Hook for iFlow configuration-based system

Usage:
    from maestro.leindex.hooks import create_hook, get_agent_detector
    
    # Detect available agents
    detector = get_agent_detector()
    agents = detector.detect_all()
    
    # Create a hook for a specific agent
    hook = create_hook("pi-mono")
    if hook:
        await hook.install_session_end_hook()
        context = await hook.extract_session_context()
"""

from .base import AgentHook, HookResult
from .pi_mono import PiMonoHook
from .oh_my_pi import OhMyPiHook
from .iflow import IFlowHook
from .factory import (
    HookFactory,
    create_hook,
    register_hook,
    get_available_hooks,
    create_all_hooks,
    get_hook_factory,
)
from .detector import (
    AgentDetector,
    DetectedAgent,
    detect_available_agents,
    is_agent_available,
    get_agent_detector,
)

__all__ = [
    # Base classes
    "AgentHook",
    "HookResult",
    # Hook implementations
    "PiMonoHook",
    "OhMyPiHook",
    "IFlowHook",
    # Factory
    "HookFactory",
    "create_hook",
    "register_hook",
    "get_available_hooks",
    "create_all_hooks",
    "get_hook_factory",
    # Detector
    "AgentDetector",
    "DetectedAgent",
    "detect_available_agents",
    "is_agent_available",
    "get_agent_detector",
]
