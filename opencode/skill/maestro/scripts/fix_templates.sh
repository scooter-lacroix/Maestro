#!/bin/bash
# Repair broken template symlinks in Maestro skill

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"

echo "Fixing Maestro template symlinks..."

# OpenCode-first: templates ship with the skill itself.
TEMPLATES_DIR="$SKILL_DIR/templates"

# Remove broken symlinks
echo "Removing broken symlinks..."
find "$SKILL_DIR/templates" -type l ! -exec test -e {} \; -delete 2>/dev/null || true

echo "Validating template presence..."

if [ ! -f "$TEMPLATES_DIR/workflow.md" ]; then
    echo "ERROR: Missing workflow template at $TEMPLATES_DIR/workflow.md"
    echo "Reinstall the Maestro OpenCode skill to restore templates."
    exit 1
fi

if [ ! -d "$TEMPLATES_DIR/code_styleguides" ]; then
    echo "WARNING: Missing code_styleguides directory at $TEMPLATES_DIR/code_styleguides"
fi

echo ""
echo "✅ Templates validated successfully"
