"""
Maestro Dashboard Module

This module provides visual dashboard functionality for Maestro,
including token savings visualization, progress bars, and sparklines.
"""

from .visual_dashboard import (
    calculate_token_savings,
    format_token_display,
    generate_progress_bar,
    generate_sparkline,
    DashboardRenderer,
    render_dashboard_for_maestro_status,
    render_dashboard_for_maestro_implement
)

__all__ = [
    "calculate_token_savings",
    "format_token_display", 
    "generate_progress_bar",
    "generate_sparkline",
    "DashboardRenderer",
    "render_dashboard_for_maestro_status",
    "render_dashboard_for_maestro_implement"
]