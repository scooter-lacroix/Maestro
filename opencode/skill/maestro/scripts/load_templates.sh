#!/bin/bash
# Verify and load Maestro templates

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"

echo "Verifying Maestro templates..."

# OpenCode-first: templates ship with the skill itself.
TEMPLATES_DIR="$SKILL_DIR/templates"

# Check for workflow template
if [ ! -f "$TEMPLATES_DIR/workflow.md" ]; then
    echo "ERROR: workflow.md not found in skill templates directory: $TEMPLATES_DIR"
    exit 1
fi

# Check for code styleguides
if [ ! -d "$TEMPLATES_DIR/code_styleguides" ]; then
    echo "WARNING: code_styleguides directory not found in $TEMPLATES_DIR"
fi

echo "✅ Templates verified successfully"
echo ""
echo "Available templates:"
echo "  - workflow.md"
ls -1 "$TEMPLATES_DIR/code_styleguides/" 2>/dev/null | sed 's/^/  - /' || echo "  (no styleguides found)"
