#!/bin/bash
# Maestro Installer for Claude Code v1.3.0
set -e

echo "🚀 Installing Maestro for Claude Code..."

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

# Install Python CLI
echo "🐍 Installing Python CLI..."
if [ -d "$SCRIPT_DIR/maestro" ]; then
    # Check if pip is available
    if command -v pip &> /dev/null || command -v pip3 &> /dev/null; then
        # Install the package (regular install, not editable)
        echo "   Installing maestro Python package..."
        cd "$SCRIPT_DIR"
        pip install . --quiet 2>/dev/null || pip3 install . --quiet 2>/dev/null || {
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

# Cleanup is handled by the trap at the top
echo ""
echo "✅ Maestro installed successfully for Claude Code!"
echo ""
echo "📚 Available Claude Code slash commands:"
echo "  /maestro:setup      - Initialize Maestro environment"
echo "  /maestro:newTrack   - Create new track"
echo "  /maestro:implement  - Implement track tasks"
echo "  /maestro:status     - View project progress"
echo "  /maestro:revert     - Revert work"
echo "  /maestro:configure  - Configure Maestro settings"
echo "  /maestro:memory     - Interact with Memory System"
echo ""
echo "🖥️  Available CLI tools (from terminal):"
echo "  maestro memory serve    - Start memory dashboard web server"
echo "  maestro memory status   - Show memory system status"
echo "  maestro memory migrate  - Migrate from Memori to Nexus"
echo "  maestro tui             - Launch Terminal UI (requires Go build)"
echo ""
echo "🎯 Next steps:"
echo "  1. Ensure ~/.local/bin is in your PATH:"
echo "     export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "  2. Open Claude Code"
echo "  3. Run /maestro:setup in your project directory"
echo "  4. Run /maestro:configure to customize settings"
echo ""
echo "📖 For more information, see:"
echo "  https://github.com/scooter-lacroix/Maestro"
echo ""
