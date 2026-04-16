"""Detect LeIndex availability and provide installation guidance."""

import shutil
import subprocess
from dataclasses import dataclass
from enum import Enum
from typing import Optional


class LeIndexMode(Enum):
    UNAVAILABLE = "unavailable"
    CLI = "cli"
    MCP = "mcp"
    BOTH = "both"


@dataclass
class LeIndexStatus:
    mode: LeIndexMode
    cli_path: Optional[str] = None
    cli_version: Optional[str] = None
    mcp_available: bool = False

    @property
    def is_available(self) -> bool:
        return self.mode != LeIndexMode.UNAVAILABLE

    @property
    def install_instructions(self) -> str:
        return (
            "LeIndex can be installed via:\n"
            "\n"
            "1. Via cargo (recommended):\n"
            "   cargo install leindex\n"
            "\n"
            "2. Via install script:\n"
            "   curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/LeIndex/master/install.sh"
            " -o install-leindex.sh\n"
            "   bash install-leindex.sh\n"
            "\n"
            "3. Via PyPI bootstrap wrapper:\n"
            "   pip install leindex\n"
            "   leindex --version\n"
            "\n"
            "4. Via npm MCP wrapper (leanest, recommended for AI tools):\n"
            "   npm install -g @leindex/mcp\n"
            "\n"
            "5. From source:\n"
            "   git clone https://github.com/scooter-lacroix/LeIndex.git\n"
            "   cd LeIndex && cargo install --path ."
        )


def detect_leindex() -> LeIndexStatus:
    """Detect LeIndex installation status."""
    status = LeIndexStatus(mode=LeIndexMode.UNAVAILABLE)

    # Check CLI
    cli_path = shutil.which("leindex")
    if cli_path:
        status.cli_path = cli_path
        status.mode = LeIndexMode.CLI
        try:
            result = subprocess.run(
                ["leindex", "--version"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if result.returncode == 0:
                status.cli_version = result.stdout.strip()
        except Exception:
            pass  # mode already set above

    # Check MCP
    try:
        mcp_path = shutil.which("leindex-mcp")
        if mcp_path:
            status.mcp_available = True
            if status.mode == LeIndexMode.CLI:
                status.mode = LeIndexMode.BOTH
            else:
                status.mode = LeIndexMode.MCP
    except Exception:
        pass

    return status


def get_leindex_tool_list() -> Optional[str]:
    """Get the full list of LeIndex tools and their schemas via 'leindex help'."""
    try:
        result = subprocess.run(
            ["leindex", "help"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return result.stdout
    except Exception:
        pass
    return None
