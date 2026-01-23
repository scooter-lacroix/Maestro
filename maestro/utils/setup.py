"""
Maestro environment setup and directory structure management.
Ensures the ~/.maestro directory structure exists on first run.
"""

import os
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

MAESTRO_HOME = os.path.expanduser("~/.maestro")

def check_is_first_run() -> bool:
    """
    Check if this is the first run of Maestro.

    Returns:
        bool: True if MAESTRO_HOME does not exist.
    """
    return not os.path.exists(MAESTRO_HOME)

def check_permissions(path: str) -> bool:
    """
    Check if we have write permissions to the path.

    Args:
        path: Path to check

    Returns:
        bool: True if writable, False otherwise
    """
    try:
        if os.path.exists(path):
            return os.access(path, os.W_OK)

        # If path doesn't exist, check parent
        parent = os.path.dirname(path)
        if not os.path.exists(parent):
            return check_permissions(parent)

        return os.access(parent, os.W_OK)
    except Exception:
        return False

def ensure_maestro_dirs() -> bool:
    """
    Ensure all required Maestro directories exist.

    Returns:
        bool: True if this was a first run (directories created), False otherwise.
    """
    is_first_run = check_is_first_run()

    # Check permissions for MAESTRO_HOME creation
    if not os.path.exists(MAESTRO_HOME):
        parent = os.path.dirname(MAESTRO_HOME)
        if not os.access(parent, os.W_OK):
             logger.error(f"Permission denied: Cannot create {MAESTRO_HOME}. Please check permissions on {parent}.")
             raise PermissionError(f"Cannot create {MAESTRO_HOME}")

    directories = [
        MAESTRO_HOME,
        os.path.join(MAESTRO_HOME, "locks"),
        os.path.join(MAESTRO_HOME, "backups"),
        os.path.join(MAESTRO_HOME, "logs"),
        os.path.join(MAESTRO_HOME, "tracks"),
        os.path.join(MAESTRO_HOME, "sessions"),
    ]

    for directory in directories:
        if not os.path.exists(directory):
            try:
                os.makedirs(directory, exist_ok=True)
                logger.info(f"Created directory: {directory}")
            except PermissionError:
                logger.error(f"Permission denied: Cannot create {directory}")
                raise
            except Exception as e:
                logger.error(f"Failed to create directory {directory}: {e}")
                raise

    return is_first_run

def get_db_path():
    """Get the path to the main SQLite database."""
    return os.path.join(MAESTRO_HOME, "memory.db")

def get_duckdb_path():
    """Get the path to the DuckDB analytics database."""
    return os.path.join(MAESTRO_HOME, "analytics.duckdb")
