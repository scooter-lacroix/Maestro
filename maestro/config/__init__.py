"""
Maestro Configuration Module

This module provides centralized configuration management for the Maestro framework.
"""

from .settings import (
    SettingsManager,
    get_settings,
    reload_settings,
    get_version,
    get_home_path,
    is_debug,
    is_memory_enabled,
    is_dashboard_enabled,
    get_cli_prefix,
    expand_path,
)

__all__ = [
    "SettingsManager",
    "get_settings",
    "reload_settings",
    "get_version",
    "get_home_path",
    "is_debug",
    "is_memory_enabled",
    "is_dashboard_enabled",
    "get_cli_prefix",
    "expand_path",
]
