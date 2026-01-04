#!/bin/bash
# Repair broken template symlinks in Maestro skill

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"

echo "Fixing Maestro template symlinks..."

# Check if templates directory exists
if [ ! -d "$HOME/.claude/maestro-templates" ]; then
    echo "ERROR: Templates directory not found at ~/.claude/maestro-templates/"
    echo "Please install Maestro templates first."
    exit 1
fi

# Remove broken symlinks
echo "Removing broken symlinks..."
find "$SKILL_DIR/templates" -type l ! -exec test -e {} \; -delete 2>/dev/null || true

# Create symlinks if they don't exist
echo "Creating template symlinks..."

# Workflow template
if [ ! -f "$SKILL_DIR/templates/workflow.md" ]; then
    ln -sf "$HOME/.claude/maestro-templates/workflow.md" "$SKILL_DIR/templates/workflow.md"
    echo "  ✓ workflow.md"
fi

# Code styleguides
if [ ! -d "$SKILL_DIR/templates/code_styleguides" ]; then
    mkdir -p "$SKILL_DIR/templates/code_styleguides"
fi

for guide in "$HOME/.claude/maestro-templates/code_styleguides"/*.md; do
    if [ -f "$guide" ]; then
        guide_name=$(basename "$guide")
        if [ ! -f "$SKILL_DIR/templates/code_styleguides/$guide_name" ]; then
            ln -sf "$guide" "$SKILL_DIR/templates/code_styleguides/$guide_name"
            echo "  ✓ code_styleguides/$guide_name"
        fi
    fi
done

echo ""
echo "✅ Template symlinks fixed successfully"
