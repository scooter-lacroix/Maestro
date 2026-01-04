"""
Maestro Memory Logging Configuration

Issue 17: Proper loguru configuration for production (levels, rotation, formatting).

This module configures structured logging for the Maestro memory system
with appropriate levels, rotation, and formatting for production use.
"""

import sys
import os
from pathlib import Path
from loguru import logger


def configure_logging(
    level: str = "INFO",
    log_file: Path = None,
    rotation: str = "500 MB",
    retention: str = "30 days",
    compression: str = "gz",
    enable_console: bool = True
):
    """
    Configure loguru logging for Maestro memory system.

    Issue 17: Production-ready logging configuration

    Args:
        level: Logging level (DEBUG, INFO, WARNING, ERROR, CRITICAL)
        log_file: Path to log file (optional, defaults to ~/.maestro/memory.log)
        rotation: Log rotation size (e.g., "500 MB", "100 MB", "1 GB")
        retention: Log retention period (e.g., "30 days", "1 week")
        compression: Compression algorithm (gz, zip, tar.gz)
        enable_console: Enable console output
    """
    # Remove default handler
    logger.remove()

    # Issue 17: Configure console logging with format
    if enable_console:
        logger.add(
            sys.stderr,
            format=(
                "<green>{time:YYYY-MM-DD HH:mm:ss.SSS}</green> | "
                "<level>{level: <8}</level> | "
                "<cyan>{name}</cyan>:<cyan>{function}</cyan>:<cyan>{line}</cyan> - "
                "<level>{message}</level>"
            ),
            level=level,
            colorize=True,
            backtrace=True,
            diagnose=True
        )

    # Issue 17: Configure file logging with rotation
    if log_file is None:
        log_dir = Path.home() / ".maestro" / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        log_file = log_dir / "memory.log"

    logger.add(
        log_file,
        format="{time:YYYY-MM-DD HH:mm:ss.SSS} | {level: <8} | {name}:{function}:{line} - {message}",
        level=level,
        rotation=rotation,
        retention=retention,
        compression=compression,
        encoding="utf-8",
        enqueue=True,  # Thread-safe logging
        backtrace=True,
        diagnose=True
    )

    # Issue 17: Add separate error log file
    error_log_file = log_file.parent / "memory_errors.log"
    logger.add(
        error_log_file,
        format="{time:YYYY-MM-DD HH:mm:ss.SSS} | {level: <8} | {name}:{function}:{line} - {message}",
        level="ERROR",
        rotation=rotation,
        retention=retention,
        compression=compression,
        encoding="utf-8",
        enqueue=True,
        backtrace=True,
        diagnose=True
    )

    logger.info(f"Logging configured: level={level}, file={log_file}")


def get_logger(name: str = None):
    """
    Get a logger instance.

    Args:
        name: Logger name (optional, defaults to module name)

    Returns:
        loguru logger instance
    """
    if name:
        return logger.bind(name=name)
    return logger


# Issue 17: Auto-configure logging on import
# Check environment variable for logging level
DEFAULT_LOG_LEVEL = os.environ.get("MAESTRO_LOG_LEVEL", "INFO")
DEFAULT_LOG_FILE = os.environ.get("MAESTRO_LOG_FILE")
ENABLE_CONSOLE = os.environ.get("MAESTRO_LOG_CONSOLE", "true").lower() == "true"

# Only configure if not already configured
if not logger._core.handlers:
    configure_logging(
        level=DEFAULT_LOG_LEVEL,
        log_file=Path(DEFAULT_LOG_FILE) if DEFAULT_LOG_FILE else None,
        enable_console=ENABLE_CONSOLE
    )


__all__ = [
    "configure_logging",
    "get_logger",
    "logger",
]
