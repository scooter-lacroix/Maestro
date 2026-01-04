#!/bin/bash
# Maestro Installer for OpenCode v1.3.0
set -e

echo "🚀 Installing Maestro for OpenCode..."

# Create a temporary directory for downloading the repository
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

echo "📥 Downloading Maestro repository..."
REPO_URL="https://github.com/scooter-lacroix/Maestro"
REPO_BRANCH="master"

# Try git clone first, fallback to curl+tar
if command -v git &> /dev/null; then
    echo "   Using git to download..."
    git clone -q --depth 1 --branch "$REPO_BRANCH" "$REPO_URL" "$TMP_DIR" 2>/dev/null || {
        echo "   ⚠️  git clone failed, trying fallback method..."
        if command -v curl &> /dev/null; then
            curl -sSL "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
        elif command -v wget &> /dev/null; then
            wget -qO- "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
        else
            echo "❌ Error: Neither git nor curl/wget is available"
            exit 1
        fi
    }
else
    # Fallback: download tarball
    echo "   Downloading tarball..."
    if command -v curl &> /dev/null; then
        curl -sSL "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
    elif command -v wget &> /dev/null; then
        wget -qO- "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
    else
        echo "❌ Error: Neither curl nor wget is available"
        exit 1
    fi
fi

echo "✅ Download complete"
SCRIPT_DIR="$TMP_DIR"

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
cp "$SCRIPT_DIR/claude-code/templates/workflow.md" ~/.claude/maestro-templates/
mkdir -p ~/.claude/maestro-templates/code_styleguides
cp "$SCRIPT_DIR/claude-code/templates/code_styleguides"/*.md ~/.claude/maestro-templates/code_styleguides/

# Install Python CLI
echo "🐍 Installing Python CLI..."
if [ -d "$SCRIPT_DIR/maestro" ]; then
    # Check if pip is available
    if command -v pip &> /dev/null || command -v pip3 &> /dev/null; then
        # Install in development mode (editable)
        echo "   Installing maestro Python package in development mode..."
        cd "$SCRIPT_DIR"
        pip install -e . --quiet 2>/dev/null || pip3 install -e . --quiet 2>/dev/null || {
            echo "   ⚠️  Warning: pip install failed, installing manually..."
            # Fallback: create wrapper script
            mkdir -p ~/.local/bin
            cat > ~/.local/bin/maestro << 'EOF'
#!/bin/bash
# Maestro CLI wrapper
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAESTRO_ROOT="$SCRIPT_DIR"

# Handle 'tui' subcommand
if [ "$1" = "tui" ]; then
    # Check if maestro-tui binary exists
    if [ -f "$MAESTRO_ROOT/maestro/tui/build/maestro-tui" ]; then
        exec "$MAESTRO_ROOT/maestro/tui/build/maestro-tui" "${@:2}"
    else
        echo "Error: maestro-tui binary not found"
        echo "Please build it with: cd $MAESTRO_ROOT/maestro/tui && go build"
        exit 1
    fi
# Handle 'memory' subcommand
elif [ "$1" = "memory" ]; then
    # Delegate to Python CLI
    cd "$MAESTRO_ROOT"
    python3 -m maestro.memory.cli "${@:2}"
else
    # Delegate to main Python CLI
    cd "$MAESTRO_ROOT"
    python3 -m maestro.cli "$@"
fi
EOF
            chmod +x ~/.local/bin/maestro
            echo "   Created wrapper script at ~/.local/bin/maestro"
        }
    else
        echo "   ⚠️  Warning: pip not found, creating manual wrapper..."
        mkdir -p ~/.local/bin
        cat > ~/.local/bin/maestro << 'EOF'
#!/bin/bash
# Maestro CLI wrapper
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAESTRO_ROOT="$SCRIPT_DIR"

# Handle 'tui' subcommand
if [ "$1" = "tui" ]; then
    # Check if maestro-tui binary exists
    if [ -f "$MAESTRO_ROOT/maestro/tui/build/maestro-tui" ]; then
        exec "$MAESTRO_ROOT/maestro/tui/build/maestro-tui" "${@:2}"
    else
        echo "Error: maestro-tui binary not found"
        echo "Please build it with: cd $MAESTRO_ROOT/maestro/tui && go build"
        exit 1
    fi
# Handle 'memory' subcommand
elif [ "$1" = "memory" ]; then
    # Delegate to Python CLI
    cd "$MAESTRO_ROOT"
    python3 -m maestro.memory.cli "${@:2}"
else
    # Delegate to main Python CLI
    cd "$MAESTRO_ROOT"
    python3 -m maestro.cli "$@"
fi
EOF
        chmod +x ~/.local/bin/maestro
        echo "   Created wrapper script at ~/.local/bin/maestro"
    fi

    # Ensure ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo ""
        echo "⚠️  Note: ~/.local/bin is not in your PATH"
        echo "   Add the following to your ~/.bashrc or ~/.zshrc:"
        echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
    fi
else
    echo "   ⚠️  Warning: maestro/ directory not found, skipping CLI installation"
fi

# Install Go TUI binary if it exists
echo "🔷 Checking for Go TUI binary..."
TUI_BINARY="$SCRIPT_DIR/maestro/tui/build/maestro-tui"
if [ -f "$TUI_BINARY" ]; then
    mkdir -p ~/.local/bin
    cp "$TUI_BINARY" ~/.local/bin/maestro-tui
    chmod +x ~/.local/bin/maestro-tui
    echo "   Installed maestro-tui to ~/.local/bin/maestro-tui"
else
    echo "   ℹ️  maestro-tui binary not found (optional, requires Go build)"
fi

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
        cp "$SCRIPT_DIR/claude-code/commands/maestro:$cmd.md" ~/.claude/commands/"maestro:$cmd.md"
    fi
done

# Cleanup is handled by the trap at the top
echo ""
echo "✅ Maestro installed successfully for OpenCode!"
echo ""
echo "📚 Available OpenCode commands:"
echo "  /maestro setup      - Initialize Maestro environment"
echo "  /maestro newTrack   - Create new track"
echo "  /maestro implement  - Implement track tasks"
echo "  /maestro status     - View project progress"
echo "  /maestro revert     - Revert work"
echo "  /maestro configure - Configure Maestro settings"
echo ""
echo "🖥️  Available CLI commands (from terminal):"
echo "  maestro memory serve    - Start memory dashboard web server"
echo "  maestro memory status   - Show memory system status"
echo "  maestro memory migrate  - Migrate from Memori to Nexus"
echo "  maestro tui             - Launch Terminal UI"
echo ""
echo "🎯 Next steps:"
echo "  1. Ensure ~/.local/bin is in your PATH:"
echo "     export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "  2. Restart OpenCode"
echo "  3. Run /maestro setup in your project directory"
echo "  4. Run /maestro configure to customize settings"
echo ""
echo "📖 For more information, see:"
echo "  https://github.com/scooter-lacroix/Maestro"
echo ""
