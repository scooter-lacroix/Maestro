"""
Maestro Memory System - Nexus Integration

This package integrates Nexus Memory System into Maestro for automatic
context extraction, storage, and retrieval across all Maestro operations.
"""

__version__ = "2.0.0"
__author__ = "Maestro Team"

from maestro.memory.service import MaestroMemoryService

__all__ = ["MaestroMemoryService"]
