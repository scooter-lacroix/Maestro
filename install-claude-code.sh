#!/bin/bash
# Maestro Installer for Claude Code v1.1.0
set -e

echo "🚀 Installing Maestro for Claude Code..."

# Detect script directory for relative path resolution
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Create commands directory
echo "📁 Creating commands directory..."
mkdir -p ~/.claude/commands

# Copy commands
echo "📋 Copying commands..."
cp "$SCRIPT_DIR/claude-code/commands/maestro"*.md ~/.claude/commands/

# Create templates directory
echo "📁 Creating templates directory..."
mkdir -p ~/.claude/maestro-templates

# Copy templates
echo "📋 Copying templates..."
cp "$SCRIPT_DIR/claude-code/templates/workflow.md" ~/.claude/maestro-templates/
mkdir -p ~/.claude/maestro-templates/code_styleguides
cp "$SCRIPT_DIR/claude-code/templates/code_styleguides"/*.md ~/.claude/maestro-templates/code_styleguides/

# Verify installation
echo ""
echo "✅ Maestro installed successfully for Claude Code!"
echo ""
echo "📚 Available commands:"
echo "  /maestro:setup      - Initialize Maestro environment"
echo "  /maestro:newTrack   - Create new track"
echo "  /maestro:implement  - Implement track tasks"
echo "  /maestro:status     - View project progress"
echo "  /maestro:revert     - Revert work"
echo "  /maestro:configure  - Configure Maestro settings"
echo "  /maestro:tui        - Launch Terminal UI"
echo "  /maestro:memory     - Interact with Memory System"
echo ""
echo "🎯 Next steps:"
echo "  1. Open Claude Code"
echo "  2. Run /maestro:setup in your project directory"
echo "  3. Run /maestro:configure to customize settings"
echo ""
echo "📖 For more information, see:"
echo "  https://github.com/scooter-lacroix/Maestro"
