"""
Visual Dashboard - Token savings visualization with progress bars and sparklines.

Implements Don Norman UX principles: visibility, feedback, mapping.
"""

from typing import Dict, List, Tuple, Optional
import math
from dataclasses import dataclass

# Lazy import of rich for optional functionality
try:
    from rich.console import Console
    from rich.table import Table
    from rich.progress import Progress, BarColumn, TextColumn
    from rich.panel import Panel
    from rich.text import Text
    RICH_AVAILABLE = True
except ImportError:
    Console = None
    Table = None
    Progress = None
    BarColumn = None
    TextColumn = None
    Panel = None
    Text = None
    RICH_AVAILABLE = False


class TokenSavingsCalculator:
    """Calculator for token savings metrics."""

    @staticmethod
    def calculate_savings(original_tokens: int, optimized_tokens: int) -> float:
        """
        Calculate token savings percentage.

        Args:
            original_tokens: Original token count
            optimized_tokens: Optimized token count

        Returns:
            Savings percentage (can be negative if optimization increased tokens)
        """
        return calculate_token_savings(original_tokens, optimized_tokens)

    @staticmethod
    def format_tokens(token_count: int) -> str:
        """
        Format token count for display (e.g., 1000 -> '1.00K').

        Args:
            token_count: Raw token count

        Returns:
            Formatted string representation
        """
        return format_token_display(token_count)


@dataclass
class TokenSavingsData:
    """Data class for token savings information."""
    original_tokens: int
    optimized_tokens: int
    savings_percentage: float
    savings_count: int


def calculate_token_savings(original_tokens: int, optimized_tokens: int) -> float:
    """
    Calculate token savings percentage.
    
    Args:
        original_tokens: Original token count
        optimized_tokens: Optimized token count
    
    Returns:
        Savings percentage (can be negative if optimization increased tokens)
    """
    if original_tokens <= 0:
        return 0.0
    
    savings = original_tokens - optimized_tokens
    return (savings / original_tokens) * 100


def format_token_display(token_count: int) -> str:
    """
    Format token count for display (e.g., 1000 -> '1.00K').
    
    Args:
        token_count: Raw token count
    
    Returns:
        Formatted string representation
    """
    if token_count >= 1_000_000:
        return f"{token_count / 1_000_000:.2f}M"
    elif token_count >= 1_000:
        return f"{token_count / 1_000:.2f}K"
    else:
        return str(token_count)


def generate_progress_bar(value: int, max_value: int, width: int = 20) -> str:
    """
    Generate a text-based progress bar.

    Args:
        value: Current value
        max_value: Maximum value
        width: Width of the progress bar in characters

    Returns:
        String representation of progress bar
    """
    if max_value <= 0:
        return " " * width

    filled_chars = round((value / max_value) * width)  # Use round instead of int
    filled_chars = min(filled_chars, width)  # Ensure we don't exceed width

    filled = "█" * filled_chars
    empty = " " * (width - filled_chars)

    return filled + empty


def generate_sparkline(data: List[float], width: Optional[int] = None, show_labels: bool = False) -> str:
    """
    Generate a text-based sparkline from numeric data.
    
    Args:
        data: List of numeric values
        width: Optional width limit for the sparkline
        show_labels: Whether to include min/max labels
    
    Returns:
        String representation of sparkline
    """
    if not data:
        return ""
    
    if len(data) == 1:
        # For single value, return middle character
        return "▆"
    
    min_val = min(data)
    max_val = max(data)
    
    if min_val == max_val:
        # If all values are the same, return flat line
        return "▆" * len(data)
    
    # Sparkline characters from Unicode
    spark_chars = "▁▂▃▄▅▆▇█"
    range_size = max_val - min_val
    
    sparkline_chars = []
    for value in data:
        # Normalize value to 0-1 range, then map to spark character index
        normalized = (value - min_val) / range_size
        char_index = int(normalized * (len(spark_chars) - 1))
        sparkline_chars.append(spark_chars[char_index])
    
    result = "".join(sparkline_chars)
    
    if width and len(result) > width:
        # Truncate if needed
        result = result[:width]
    
    if show_labels:
        result += f" Min:{min_val:.1f} Max:{max_val:.1f}"
    
    return result


class DashboardRenderer:
    """Renders visual dashboard components."""

    def __init__(self):
        if RICH_AVAILABLE:
            self.console = Console()
        else:
            self.console = None
    
    def render_token_savings_section(self, savings_data: Dict) -> str:
        """
        Render token savings visualization section.

        Args:
            savings_data: Dictionary containing token savings information

        Returns:
            Formatted string for token savings section
        """
        if not RICH_AVAILABLE:
            # Fallback plain text implementation
            original_tokens = savings_data.get("original_tokens", 0)
            optimized_tokens = savings_data.get("optimized_tokens", 0)
            savings_percentage = savings_data.get("savings_percentage", 0.0)
            savings_count = savings_data.get("savings_count", 0)

            progress_bar = generate_progress_bar(
                int(abs(savings_percentage)),
                100,
                width=30
            )

            return (
                f"Token Savings:\n"
                f"  Original Tokens: {format_token_display(original_tokens)}\n"
                f"  Optimized Tokens: {format_token_display(optimized_tokens)}\n"
                f"  Tokens Saved: {format_token_display(savings_count)}\n"
                f"  Savings Progress: {progress_bar}\n"
                f"  Savings Percentage: {savings_percentage:.2f}% "
                f"({format_token_display(savings_count)} tokens saved)\n"
            )

        original_tokens = savings_data.get("original_tokens", 0)
        optimized_tokens = savings_data.get("optimized_tokens", 0)
        savings_percentage = savings_data.get("savings_percentage", 0.0)
        savings_count = savings_data.get("savings_count", 0)

        # Create a rich table for token savings
        table = Table(title="Token Savings", show_header=True, header_style="bold magenta")
        table.add_column("Metric", style="dim")
        table.add_column("Value", justify="right")

        table.add_row("Original Tokens", format_token_display(original_tokens))
        table.add_row("Optimized Tokens", format_token_display(optimized_tokens))
        table.add_row("Tokens Saved", format_token_display(savings_count))

        # Add progress bar for savings percentage
        progress_bar = generate_progress_bar(
            int(abs(savings_percentage)),
            100,
            width=30
        )

        # Color code based on savings (green for positive, red for negative)
        savings_color = "green" if savings_percentage >= 0 else "red"
        savings_text = Text(f"{savings_percentage:.2f}%", style=savings_color)

        # Create a panel with the savings information
        savings_panel = Panel(
            f"[bold]Savings Progress:[/bold]\n"
            f"{progress_bar}\n"
            f"[{savings_color}]{savings_percentage:.2f}% ({format_token_display(savings_count)} tokens saved)[/{savings_color}]",
            title="Token Savings Visualization",
            border_style="blue"
        )

        # Properly render the panel to string
        from rich.console import Console
        from io import StringIO

        console = Console(width=80, force_terminal=True)
        with console.capture() as capture:
            console.print(savings_panel)
        return capture.get()
    
    def render_usage_trends_section(self, trend_data: Dict) -> str:
        """
        Render usage trends visualization section with sparklines.

        Args:
            trend_data: Dictionary containing usage trend information

        Returns:
            Formatted string for usage trends section
        """
        if not RICH_AVAILABLE:
            # Fallback plain text implementation
            daily_usage = trend_data.get("daily_usage", [])
            avg_daily_usage = trend_data.get("avg_daily_usage", 0)

            if not daily_usage:
                return "No usage data available"

            # Generate sparkline for daily usage
            sparkline = generate_sparkline(daily_usage, width=40, show_labels=True)

            return (
                f"Usage Trends:\n"
                f"  Daily Usage Trend: {sparkline}\n"
                f"  Average Daily Usage: {format_token_display(avg_daily_usage)} tokens\n"
            )

        daily_usage = trend_data.get("daily_usage", [])
        avg_daily_usage = trend_data.get("avg_daily_usage", 0)

        if not daily_usage:
            return "No usage data available"

        # Generate sparkline for daily usage
        sparkline = generate_sparkline(daily_usage, width=40, show_labels=True)

        # Create a panel with the trends information
        trends_panel = Panel(
            f"[bold]Daily Usage Trend:[/bold]\n"
            f"{sparkline}\n\n"
            f"Average Daily Usage: {format_token_display(avg_daily_usage)} tokens",
            title="Usage Trends",
            border_style="green"
        )

        # Properly render the panel to string
        from rich.console import Console
        from io import StringIO

        console = Console(width=80, force_terminal=True)
        with console.capture() as capture:
            console.print(trends_panel)
        return capture.get()
    
    def render_project_stats_section(self, stats_data: Dict) -> str:
        """
        Render project statistics section.

        Args:
            stats_data: Dictionary containing project statistics

        Returns:
            Formatted string for project stats section
        """
        if not RICH_AVAILABLE:
            # Fallback plain text implementation
            total_projects = stats_data.get("total_projects", 0)
            active_projects = stats_data.get("active_projects", 0)
            total_tokens_used = stats_data.get("total_tokens_used", 0)

            # Calculate active project percentage
            active_percentage = (active_projects / total_projects * 100) if total_projects > 0 else 0

            # Create progress bar for active projects
            active_progress = generate_progress_bar(active_projects, total_projects, width=30)

            return (
                f"Project Statistics:\n"
                f"  Total Projects: {total_projects}\n"
                f"  Active Projects: {active_projects} ({active_percentage:.1f}%)\n"
                f"  Progress: {active_progress}\n"
                f"  Total Tokens Used: {format_token_display(total_tokens_used)}\n"
            )

        total_projects = stats_data.get("total_projects", 0)
        active_projects = stats_data.get("active_projects", 0)
        total_tokens_used = stats_data.get("total_tokens_used", 0)

        # Calculate active project percentage
        active_percentage = (active_projects / total_projects * 100) if total_projects > 0 else 0

        # Create progress bar for active projects
        active_progress = generate_progress_bar(active_projects, total_projects, width=30)

        # Create a panel with the stats information
        stats_panel = Panel(
            f"[bold]Project Overview:[/bold]\n"
            f"Total Projects: {total_projects}\n"
            f"Active Projects: {active_projects} ({active_percentage:.1f}%)\n"
            f"Progress: {active_progress}\n\n"
            f"Total Tokens Used: {format_token_display(total_tokens_used)}",
            title="Project Statistics",
            border_style="yellow"
        )

        # Properly render the panel to string
        from rich.console import Console
        from io import StringIO

        console = Console(width=80, force_terminal=True)
        with console.capture() as capture:
            console.print(stats_panel)
        return capture.get()
    
    def render_complete_dashboard(self, dashboard_data: Dict) -> str:
        """
        Render complete dashboard with all sections.

        Args:
            dashboard_data: Dictionary containing all dashboard data

        Returns:
            Complete formatted dashboard string
        """
        # Extract sections from dashboard data
        token_savings = dashboard_data.get("token_savings", {})
        usage_trends = dashboard_data.get("usage_trends", {})
        project_stats = dashboard_data.get("project_stats", {})

        # Render each section
        savings_section = self.render_token_savings_section(token_savings)
        trends_section = self.render_usage_trends_section(usage_trends)
        stats_section = self.render_project_stats_section(project_stats)

        # Combine all sections
        if RICH_AVAILABLE:
            complete_dashboard = (
                f"[bold blue]Maestro Visual Dashboard[/bold blue]\n\n"
                f"{savings_section}\n\n"
                f"{trends_section}\n\n"
                f"{stats_section}\n\n"
                f"[dim]Dashboard generated with Don Norman UX principles: visibility, feedback, mapping[/dim]"
            )
        else:
            complete_dashboard = (
                f"Maestro Visual Dashboard\n\n"
                f"{savings_section}\n\n"
                f"{trends_section}\n\n"
                f"{stats_section}\n\n"
                f"Dashboard generated with Don Norman UX principles: visibility, feedback, mapping\n"
            )

        return complete_dashboard


def render_dashboard_for_maestro_status(dashboard_data: Dict) -> str:
    """
    Render dashboard specifically formatted for maestro:status command.
    
    Args:
        dashboard_data: Dictionary containing dashboard data
    
    Returns:
        Formatted dashboard string for maestro status
    """
    renderer = DashboardRenderer()
    return renderer.render_complete_dashboard(dashboard_data)


def render_dashboard_for_maestro_implement(dashboard_data: Dict) -> str:
    """
    Render dashboard specifically formatted for maestro:implement command output.
    
    Args:
        dashboard_data: Dictionary containing dashboard data
    
    Returns:
        Formatted dashboard string for maestro implement
    """
    renderer = DashboardRenderer()
    return renderer.render_complete_dashboard(dashboard_data)


# Example usage function
def example_dashboard_usage():
    """Example of how to use the dashboard functionality."""
    # Sample data
    sample_data = {
        "token_savings": {
            "original_tokens": 10000,
            "optimized_tokens": 7500,
            "savings_percentage": 25.0,
            "savings_count": 2500
        },
        "usage_trends": {
            "daily_usage": [1000, 1200, 900, 1500, 1100, 1300, 800],
            "avg_daily_usage": 1114
        },
        "project_stats": {
            "total_projects": 12,
            "active_projects": 8,
            "total_tokens_used": 45000
        }
    }

    renderer = DashboardRenderer()
    dashboard_output = renderer.render_complete_dashboard(sample_data)
    print(dashboard_output)


if __name__ == "__main__":
    example_dashboard_usage()