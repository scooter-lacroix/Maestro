"""
Maestro Memory System - Nexus Integration

This package integrates Nexus Memory System into Maestro for automatic
context extraction, storage, and retrieval across all Maestro operations.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

__version__ = "2.0.0"
__author__ = "Maestro Team"

__all__ = ["MaestroMemoryService"]


if TYPE_CHECKING:  # pragma: no cover
    from maestro.memory.service import MaestroMemoryService as MaestroMemoryService


def __getattr__(name: str) -> Any:
    """
    Lazily import optional-heavy components.

    This avoids importing the full memory stack (and its optional dependencies)
    when only lightweight helpers (e.g., project detection) are needed.
    """
    if name == "MaestroMemoryService":
        try:
            from maestro.memory.service import MaestroMemoryService
        except ImportError as e:  # pragma: no cover
            raise ImportError(
                "Maestro memory dependencies are not installed. Install with `maestro[memory]`."
            ) from e
        return MaestroMemoryService

    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
