#!/bin/bash
set -euo pipefail
MAESTRO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
unset PYTHONPATH
PYTHON=$(command -v python3 || command -v python); if [[ -z "$PYTHON" ]]; then echo '{}'; exit 0; fi
exec "$PYTHON" "$MAESTRO_ROOT/maestro/hooks/entry_point.py" "pre-tool-use" "PreToolUse"
