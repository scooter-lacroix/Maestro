#!/bin/bash
# Verify and load Maestro templates

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"

echo "Verifying Maestro templates..."

# Check if templates directory exists
if [ ! -d "$HOME/.claude/maestro-templates" ]; then
    echo "ERROR: Templates directory not found at ~/.claude/maestro-templates/"
    echo "Please install Maestro templates first."
    exit 1
fi

# Check for workflow template
if [ ! -f "$HOME/.claude/maestro-templates/workflow.md" ]; then
    echo "ERROR: workflow.md not found in templates directory"
    exit 1
fi

# Check for code styleguides
if [ ! -d "$HOME/.claude/maestro-templates/code_styleguides" ]; then
    echo "WARNING: code_styleguides directory not found"
fi

echo "✅ Templates verified successfully"
echo ""
echo "Available templates:"
echo "  - workflow.md"
ls -1 "$HOME/.claude/maestro-templates/code_styleguides/" 2>/dev/null | sed 's/^/  - /' || echo "  (no styleguides found)"
