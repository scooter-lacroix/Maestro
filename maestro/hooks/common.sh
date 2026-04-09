# Maestro hook bootstrap — sourced by all hook scripts.
#
# Usage: run_hook <phase> <class>
#   phase: hook phase name, e.g. "session-start", "pre-tool-use"
#   class: Python class name in entry_point.py, e.g. "SessionStart"

run_hook() {
    local phase="$1"
    local class="$2"

    MAESTRO_ROOT="$(cd "$(dirname "${BASH_SOURCE[1]}")/../.." && pwd)"
    unset PYTHONPATH
    PYTHON=$(command -v python3 || command -v python)
    if [[ -z "$PYTHON" ]]; then
        echo '{}'
        exit 0
    fi
    exec "$PYTHON" "$MAESTRO_ROOT/maestro/hooks/entry_point.py" "$phase" "$class"
}
