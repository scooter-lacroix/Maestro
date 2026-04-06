#!/bin/bash
set -euo pipefail
MAESTRO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
unset PYTHONPATH
if command -v python3 >/dev/null 2>&1; then PYTHON=python3; elif command -v python >/dev/null 2>&1; then PYTHON=python; else echo '{}'; exit 0; fi
exec "$PYTHON" "$MAESTRO_ROOT/maestro/hooks/entry_point.py" "session-start" "SessionStart"
