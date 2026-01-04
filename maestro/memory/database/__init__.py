"""
Maestro Memory Database

Maestro-specific database models and extensions to Nexus memory schema.
"""

from maestro.memory.database.models import (
    MaestroProject,
    MaestroTrack,
    Base,
    create_maestro_extension_tables,
    add_maestro_columns_to_memories
)

__all__ = [
    "MaestroProject",
    "MaestroTrack",
    "Base",
    "create_maestro_extension_tables",
    "add_maestro_columns_to_memories",
]
