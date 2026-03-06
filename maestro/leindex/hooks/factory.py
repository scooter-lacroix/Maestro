"""
Hook Factory for Nexus Memory System

This module provides a factory for creating agent hooks based on the agent type.
It automatically detects available agents and creates the appropriate hook instances.
"""

from typing import Dict, Type, Optional, List
from loguru import logger

from .base import AgentHook, HookResult
from .pi_mono import PiMonoHook
from .oh_my_pi import OhMyPiHook
from .iflow import IFlowHook


# Registry of available hook classes
HOOK_REGISTRY: Dict[str, Type[AgentHook]] = {
    "pi-mono": PiMonoHook,
    "oh-my-pi": OhMyPiHook,
    "iflow": IFlowHook,
    # Legacy/alternative names
    "pi_mono": PiMonoHook,
    "oh_my_pi": OhMyPiHook,
}


def register_hook(agent_type: str, hook_class: Type[AgentHook]) -> None:
    """
    Register a new hook class for an agent type.
    
    Args:
        agent_type: Unique identifier for the agent
        hook_class: Hook class that implements AgentHook
    """
    HOOK_REGISTRY[agent_type.lower()] = hook_class
    logger.info(f"Registered hook for agent type: {agent_type}")


def create_hook(agent_type: str) -> Optional[AgentHook]:
    """
    Create a hook instance for the specified agent type.
    
    Args:
        agent_type: The type of agent (e.g., "pi-mono", "oh-my-pi", "iflow")
        
    Returns:
        AgentHook instance or None if agent type is not registered
    """
    agent_type_lower = agent_type.lower()
    
    if agent_type_lower in HOOK_REGISTRY:
        hook_class = HOOK_REGISTRY[agent_type_lower]
        return hook_class()
    
    logger.warning(f"No hook registered for agent type: {agent_type}")
    return None


def get_available_hooks() -> List[str]:
    """
    Get list of all registered agent types.
    
    Returns:
        List of agent type identifiers
    """
    return list(HOOK_REGISTRY.keys())


def create_all_hooks() -> Dict[str, AgentHook]:
    """
    Create hook instances for all registered agent types.
    
    Returns:
        Dictionary mapping agent types to hook instances
    """
    hooks: Dict[str, AgentHook] = {}
    
    for agent_type, hook_class in HOOK_REGISTRY.items():
        try:
            hook = hook_class()
            hooks[agent_type] = hook
        except Exception as e:
            logger.warning(f"Failed to create hook for {agent_type}: {e}")
    
    return hooks


class HookFactory:
    """
    Factory for creating and managing agent hooks.
    
    Provides methods for:
    - Creating hooks by agent type
    - Auto-detecting available agents
    - Batch hook operations
    """
    
    def __init__(self) -> None:
        self._hooks: Dict[str, AgentHook] = {}
        self._initialized: bool = False
    
    def create_hook(self, agent_type: str) -> Optional[AgentHook]:
        """Create and cache a hook for the specified agent type."""
        if agent_type in self._hooks:
            return self._hooks[agent_type]
        
        hook = create_hook(agent_type)
        if hook:
            self._hooks[agent_type] = hook
        
        return hook
    
    def get_hook(self, agent_type: str) -> Optional[AgentHook]:
        """Get a cached hook or None."""
        return self._hooks.get(agent_type)
    
    def get_all_hooks(self) -> Dict[str, AgentHook]:
        """Get all cached hooks."""
        return self._hooks.copy()
    
    def initialize_all(self) -> Dict[str, HookResult]:
        """
        Initialize all available hooks by installing session end hooks.
        
        Returns:
            Dictionary mapping agent types to their initialization results
        """
        results: Dict[str, HookResult] = {}
        
        for agent_type, hook_class in HOOK_REGISTRY.items():
            try:
                hook = hook_class()
                result = asyncio.get_event_loop().run_until_complete(
                    hook.install_session_end_hook()
                )
                results[agent_type] = result
                
                if result.success:
                    self._hooks[agent_type] = hook
                    
            except Exception as e:
                logger.error(f"Failed to initialize hook for {agent_type}: {e}")
                results[agent_type] = HookResult(
                    success=False,
                    agent_type=agent_type,
                    source="hook_factory",
                    error=str(e)
                )
        
        self._initialized = True
        return results
    
    async def initialize_all_async(self) -> Dict[str, HookResult]:
        """
        Async version of initialize_all.
        
        Returns:
            Dictionary mapping agent types to their initialization results
        """
        results: Dict[str, HookResult] = {}
        
        for agent_type, hook_class in HOOK_REGISTRY.items():
            try:
                hook = hook_class()
                result = await hook.install_session_end_hook()
                results[agent_type] = result
                
                if result.success:
                    self._hooks[agent_type] = hook
                    
            except Exception as e:
                logger.error(f"Failed to initialize hook for {agent_type}: {e}")
                results[agent_type] = HookResult(
                    success=False,
                    agent_type=agent_type,
                    source="hook_factory",
                    error=str(e)
                )
        
        self._initialized = True
        return results
    
    def is_initialized(self) -> bool:
        """Check if hooks have been initialized."""
        return self._initialized


# Global factory instance
_factory: Optional[HookFactory] = None


def get_hook_factory() -> HookFactory:
    """Get the global hook factory instance."""
    global _factory
    if _factory is None:
        _factory = HookFactory()
    return _factory


import asyncio
