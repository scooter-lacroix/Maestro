"""
Unified Hook System for Maestro Memory

Provides a multi-layer hook system for capturing context throughout
the development workflow.
"""

from maestro.memory.hooks.unified import (
    Hook,
    HookLayer,
    NativeHookLayer,
    ProcessMonitorLayer,
    InactivityDetectorLayer,
    PersistentBufferLayer,
    UnifiedHookManager,
    get_hook_manager,
    shutdown_hook_manager,
)

from maestro.memory.hooks.maestro_hooks import MaestroCommandHook

__all__ = [
    # Unified hooks
    "Hook",
    "HookLayer",
    "NativeHookLayer",
    "ProcessMonitorLayer",
    "InactivityDetectorLayer",
    "PersistentBufferLayer",
    "UnifiedHookManager",
    "get_hook_manager",
    "shutdown_hook_manager",
    # Legacy hooks
    "MaestroCommandHook",
]
