#!/usr/bin/env python3
"""
Maestro TUI Launcher

Launches the Maestro TUI (Go binary) with proper error handling
and path detection.
"""

import sys
import subprocess
import shutil
from pathlib import Path
import os

# Version should match Maestro version
__version__ = "2.0.0"


def find_tui_binary() -> Path | None:
    """
    Find the maestro-tui binary.

    Search order:
    1. MAESTRO_TUI_BIN environment variable
    2. ~/.local/bin/maestro-tui
    3. ./maestro/tui/build/maestro-tui (dev)
    4. /usr/local/bin/maestro-tui
    """
    # Check environment variable
    if env_bin := os.environ.get("MAESTRO_TUI_BIN"):
        path = Path(env_bin)
        if path.exists():
            return path
        return None

    # Check common locations
    home = Path.home()
    candidates = [
        home / ".local/bin" / "maestro-tui",
        Path(__file__).parent.parent / "build" / "maestro-tui",  # Dev location
        Path("/usr/local/bin/maestro-tui"),
    ]

    for candidate in candidates:
        if candidate.exists() and candidate.is_file():
            return candidate

    return None


def main() -> int:
    """Main entry point for maestro-tui command."""

    # Find binary
    binary = find_tui_binary()
    if not binary:
        print("❌ Maestro TUI binary not found.", file=sys.stderr)
        print(file=sys.stderr)
        print("To install:", file=sys.stderr)
        print("  cd maestro/tui && go build -o ~/.local/bin/maestro-tui ./cmd/maestro-tui", file=sys.stderr)
        print("  make tui-install    # Build and install to ~/.local/bin", file=sys.stderr)
        print(file=sys.stderr)
        print("Or download from: https://github.com/scooter-lacroix/maestro/releases", file=sys.stderr)
        return 1

    # Verify executable
    if not os.access(binary, os.X_OK):
        print(f"❌ Maestro TUI binary exists but is not executable: {binary}", file=sys.stderr)
        print(f"Run: chmod +x {binary}", file=sys.stderr)
        return 1

    # Launch TUI, passing through all arguments
    args = [str(binary)] + sys.argv[1:]

    try:
        result = subprocess.run(args, check=False)
        return result.returncode
    except KeyboardInterrupt:
        # User interrupted - clean exit
        return 130
    except Exception as e:
        print(f"❌ Failed to launch Maestro TUI: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
