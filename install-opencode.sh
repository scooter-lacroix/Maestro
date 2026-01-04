#!/bin/bash
# Maestro Installer for OpenCode v1.1.0
set -e

echo "🚀 Installing Maestro for OpenCode..."

# Detect script directory for relative path resolution
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Create skill directory
echo "📁 Creating skill directory..."
mkdir -p ~/.opencode/skill/maestro

# Copy skill files
echo "📋 Copying skill files..."
cp -r "$SCRIPT_DIR/opencode/skill/maestro/"* ~/.opencode/skill/maestro/

# Create templates directory
echo "📁 Creating templates directory..."
mkdir -p ~/.claude/maestro-templates

# Copy templates
echo "📋 Copying templates..."
cp "$SCRIPT_DIR/claude-code/templates/workflow.md ~/.claude/maestro-templates/"
mkdir -p ~/.claude/maestro-templates/code_styleguides
cp "$SCRIPT_DIR/claude-code/templates/code_styleguides"/*.md ~/.claude/maestro-templates/code_styleguides/

# Backup opencode.json
echo "💾 Backing up opencode.json..."
CONFIG_FILE="$HOME/.config/opencode/opencode.json"
if [ -f "$CONFIG_FILE" ]; then
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup"
    echo "   Backup saved to $CONFIG_FILE.backup"
fi

# Add commands to opencode.json
echo "🔧 Adding commands to opencode.json..."

# Try jq first, fallback to python
if command -v jq &> /dev/null; then
    echo "   Using jq for configuration..."
    # Create temporary file for jq processing
    tmp_file=$(mktemp)

    # Add maestro commands if not present
    jq --arg script_dir "$HOME/.claude/commands" '
        if .command.maestro == null then
            .command.maestro = {
                "template": "Load Maestro skill. Available: setup, newTrack, implement, status, revert.",
                "description": "Maestro spec-driven development framework"
            }
        else
            .
        end |
        if .command."maestro:setup" == null then
            .command."maestro:setup" = {
                "template": "Read and execute from \($script_dir)/maestro:setup.md with args: $ARGUMENTS",
                "description": "Maestro setup command"
            }
        else
            .
        end |
        if .command."maestro:newTrack" == null then
            .command."maestro:newTrack" = {
                "template": "Read and execute from \($script_dir)/maestro:newTrack.md with args: $ARGUMENTS",
                "description": "Maestro newTrack command"
            }
        else
            .
        end |
        if .command."maestro:implement" == null then
            .command."maestro:implement" = {
                "template": "Read and execute from \($script_dir)/maestro:implement.md with args: $ARGUMENTS",
                "description": "Maestro implement command"
            }
        else
            .
        end |
        if .command."maestro:status" == null then
            .command."maestro:status" = {
                "template": "Read and execute from \($script_dir)/maestro:status.md with args: $ARGUMENTS",
                "description": "Maestro status command"
            }
        else
            .
        end |
        if .command."maestro:revert" == null then
            .command."maestro:revert" = {
                "template": "Read and execute from \($script_dir)/maestro:revert.md with args: $ARGUMENTS",
                "description": "Maestro revert command"
            }
        else
            .
        end |
        if .command."maestro:configure" == null then
            .command."maestro:configure" = {
                "template": "Read and execute from \($script_dir)/maestro:configure.md with args: $ARGUMENTS",
                "description": "Maestro configure command"
            }
        else
            .
        end
    ' "$CONFIG_FILE" > "$tmp_file"

    mv "$tmp_file" "$CONFIG_FILE"

elif command -v python3 &> /dev/null; then
    echo "   Using python3 for configuration..."
    python3 << 'EOF'
import json
import os

config_path = os.path.expanduser('~/.config/opencode/opencode.json')
with open(config_path, 'r') as f:
    config = json.load(f)

# Add maestro commands if not present
if 'maestro' not in config.get('command', {}):
    config.setdefault('command', {})['maestro'] = {
        "template": "Load Maestro skill. Available: setup, newTrack, implement, status, revert.",
        "description": "Maestro spec-driven development framework"
    }

commands_dir = os.path.expanduser('~/.claude/commands')
for cmd in ['setup', 'newTrack', 'implement', 'status', 'revert', 'configure']:
    key = f'maestro:{cmd}'
    if key not in config.get('command', {}):
        config.setdefault('command', {})[key] = {
            "template": f"Read and execute from {commands_dir}/maestro:{cmd}.md with args: $ARGUMENTS",
            "description": f"Maestro {cmd} command"
        }

with open(config_path, 'w') as f:
    json.dump(config, f, indent=2)
EOF
else
    echo "⚠️  Warning: Neither jq nor python3 found. Manual configuration required."
    echo "   Please add the following to $CONFIG_FILE:"
    echo ""
    cat << 'EOM'
  "command": {
    "maestro": {
      "template": "Load Maestro skill. Available: setup, newTrack, implement, status, revert.",
      "description": "Maestro spec-driven development framework"
    },
    "maestro:setup": {
      "template": "Read and execute from ~/.claude/commands/maestro:setup.md with args: $ARGUMENTS",
      "description": "Maestro setup command"
    },
    "maestro:newTrack": {
      "template": "Read and execute from ~/.claude/commands/maestro:newTrack.md with args: $ARGUMENTS",
      "description": "Maestro newTrack command"
    },
    "maestro:implement": {
      "template": "Read and execute from ~/.claude/commands/maestro:implement.md with args: $ARGUMENTS",
      "description": "Maestro implement command"
    },
    "maestro:status": {
      "template": "Read and execute from ~/.claude/commands/maestro:status.md with args: $ARGUMENTS",
      "description": "Maestro status command"
    },
    "maestro:revert": {
      "template": "Read and execute from ~/.claude/commands/maestro:revert.md with args: $ARGUMENTS",
      "description": "Maestro revert command"
    },
    "maestro:configure": {
      "template": "Read and execute from ~/.claude/commands/maestro:configure.md with args: $ARGUMENTS",
      "description": "Maestro configure command"
    }
  }
EOM
fi

# Create command symlinks
echo "🔗 Creating command symlinks..."
mkdir -p ~/.claude/commands
for cmd in setup newTrack implement status revert configure; do
    if [ ! -f ~/.claude/commands/"maestro:$cmd.md" ]; then
        ln -sf "$SCRIPT_DIR/claude-code/commands/maestro:$cmd.md" ~/.claude/commands/"maestro:$cmd.md" 2>/dev/null || \
            cp "$SCRIPT_DIR/claude-code/commands/maestro:$cmd.md" ~/.claude/commands/"maestro:$cmd.md"
    fi
done

echo ""
echo "✅ Maestro installed successfully for OpenCode!"
echo ""
echo "📚 Available commands:"
echo "  /maestro setup      - Initialize Maestro environment"
echo "  /maestro newTrack   - Create new track"
echo "  /maestro implement  - Implement track tasks"
echo "  /maestro status     - View project progress"
echo "  /maestro revert     - Revert work"
echo "  /maestro configure - Configure Maestro settings"
echo ""
echo "🎯 Next steps:"
echo "  1. Restart OpenCode"
echo "  2. Run /maestro setup in your project directory"
echo "  3. Run /maestro configure to customize settings"
echo ""
echo "📖 For more information, see:"
echo "  https://github.com/scooter-lacroix/Maestro"
