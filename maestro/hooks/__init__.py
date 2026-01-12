"""
Maestro Hooks Package

This package contains all Maestro hooks organized by phase.
Hooks integrate with the UnifiedHookManager to provide
context capture, coordination, and memory management.

Hook Phases:
- session-start: Hooks that run when a session starts
- pre-tool-use: Hooks that run before tool execution
- post-tool-use: Hooks that run after tool execution
- pre-compact: Hooks that run before context compaction
- user-prompt-submit: Hooks that run when user submits a prompt
- subagent-stop: Hooks that run when a subagent completes
- session-end: Hooks that run when a session ends
"""

from typing import Any
from maestro.hooks.executor import (
    HookExecutor,
    get_hook_executor,
    execute_session_start,
    execute_pre_tool_use,
    execute_post_tool_use,
    execute_pre_compact,
    execute_user_prompt_submit,
    execute_subagent_stop,
    execute_session_end,
)

_UNIFIED_EXPORTS = {
    "UnifiedHookManager",
    "get_hook_manager",
    "shutdown_hook_manager",
    "Hook",
    "HookLayer",
    "NativeHookLayer",
    "ProcessMonitorLayer",
    "InactivityDetectorLayer",
    "PersistentBufferLayer",
}


def __getattr__(name: str) -> Any:  # noqa: ANN401
    if name in _UNIFIED_EXPORTS:
        try:
            import importlib

            module = importlib.import_module("maestro.memory.hooks.unified")
            return getattr(module, name)
        except Exception as exc:  # pragma: no cover - optional dependency
            raise AttributeError(
                f"maestro.memory.hooks.unified is unavailable; cannot load {name}"
            ) from exc
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__() -> list[str]:
    return sorted(list(globals().keys()) + list(_UNIFIED_EXPORTS))


__all__ = [
    # Unified Hook Manager
    "UnifiedHookManager",
    "get_hook_manager",
    "shutdown_hook_manager",
    # Base Hook Classes
    "Hook",
    "HookLayer",
    "NativeHookLayer",
    "ProcessMonitorLayer",
    "InactivityDetectorLayer",
    "PersistentBufferLayer",
    # Hook Executor
    "HookExecutor",
    "get_hook_executor",
    "execute_session_start",
    "execute_pre_tool_use",
    "execute_post_tool_use",
    "execute_pre_compact",
    "execute_user_prompt_submit",
    "execute_subagent_stop",
    "execute_session_end",
]

# Hook paths by phase
HOOK_PHASES = {
    "session-start": [
        "session-start/session-load.py",
        "session-start/session-register.py",
        "session-start/trace-start.py",
    ],
    "pre-tool-use": [
        "pre-tool-use/tldr-read.py",
        "pre-tool-use/smart-search.py",
        "pre-tool-use/file-claims.py",
        "pre-tool-use/tldr-context.py",
    ],
    "post-tool-use": [
        "post-tool-use/post-edit.py",
        "post-tool-use/handoff-index.py",
        "post-tool-use/edit-notify.py",
    ],
    "pre-compact": [
        "pre-compact/continuity.py",
    ],
    "user-prompt-submit": [
        "user-prompt-submit/skill-activation.py",
        "user-prompt-submit/memory-recall.py",
    ],
    "subagent-stop": [
        "subagent-stop/agent-report.py",
    ],
    "session-end": [
        "session-end/session-cleanup.py",
        "session-end/session-outcome.py",
    ],
}

TOTAL_HOOKS = sum(len(hooks) for hooks in HOOK_PHASES.values())
