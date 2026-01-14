#!/bin/bash
# Maestro Installer for OpenCode v1.5.0
# Enhanced with Go and Zoekt auto-installation
set -e

echo "🚀 Installing Maestro for OpenCode..."

# Function to detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if [ -f /etc/os-release ]; then
            . /etc/os-release
            OS=$ID
        else
            OS="linux"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
    else
        OS="unknown"
    fi
    echo "$OS"
}

# Function to check if command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Function to install Go
install_go() {
    local os=$(detect_os)
    echo ""
    echo "📦 Go not found. Installing Go..."

    case $os in
        ubuntu|debian|linuxmint|pop)
            echo "   Detected Debian-based system"
            if [ -w /usr/local ]; then
                echo "   Installing Go via apt..."
                sudo apt-get update -qq
                sudo apt-get install -y golang-go
            else
                echo "   ⚠️  Need sudo to install Go. Please enter your password."
                sudo apt-get update -qq
                sudo apt-get install -y golang-go
            fi
            ;;
        fedora|rhel|centos)
            echo "   Detected RedHat-based system"
            if [ -w /usr/local ]; then
                echo "   Installing Go via dnf..."
                sudo dnf install -y golang
            else
                echo "   ⚠️  Need sudo to install Go. Please enter your password."
                sudo dnf install -y golang
            fi
            ;;
        macos)
            echo "   Detected macOS"
            if command_exists brew; then
                echo "   Installing Go via Homebrew..."
                brew install go
            else
                echo "   ⚠️  Homebrew not found. Please install Homebrew first:"
                echo "   /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                echo ""
                echo "   Or install Go manually from: https://golang.org/dl/"
                return 1
            fi
            ;;
        *)
            echo "   ⚠️  Unsupported OS for auto-install. Please install Go manually:"
            echo "   - Download from: https://golang.org/dl/"
            echo "   - Or use your package manager"
            return 1
            ;;
    esac

    # Verify installation
    if command_exists go; then
        echo "   ✅ Go installed successfully: $(go version)"
        return 0
    else
        echo "   ❌ Go installation failed"
        return 1
    fi
}

# Function to install Rust
install_rust() {
    echo ""
    echo "🦀 Rust not found. Installing Rust..."
    if curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; then
        source "$HOME/.cargo/env"
        echo "   ✅ Rust installed successfully"
        return 0
    else
        echo "   ❌ Rust installation failed"
        return 1
    fi
}

# Function to install Zoekt
install_zoekt() {
    echo ""
    echo "🔍 Zoekt not found. Installing Zoekt..."

    # Ensure Go is installed first
    if ! command_exists go; then
        echo "   ⚠️  Go is required to install Zoekt"
        if ! install_go; then
            echo "   ❌ Cannot install Zoekt without Go"
            return 1
        fi
    fi

    echo "   Installing Zoekt via Go..."
    echo "   This may take a few minutes..."

    # Install Zoekt binaries
    if go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest 2>/dev/null && \
       go install github.com/sourcegraph/zoekt/cmd/zoekt-indexer@latest 2>/dev/null; then
        echo "   ✅ Zoekt installed successfully"

        # Add GOPATH to PATH if not already there
        GOPATH=$(go env GOPATH)
        if [[ ":$PATH:" != *":$GOPATH/bin:"* ]]; then
            echo ""
            echo "   ⚠️  Note: \$GOPATH/bin is not in your PATH"
            echo "   Add the following to your ~/.bashrc or ~/.zshrc:"
            echo "   export PATH=\"\$PATH:\$(go env GOPATH)/bin\""
            echo ""
            echo "   For now, adding to current session..."
            export PATH="$PATH:$(go env GOPATH)/bin"
        fi

        return 0
    else
        echo "   ❌ Zoekt installation failed"
        echo "   You can install manually with:"
        echo "   go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest"
        echo "   go install github.com/sourcegraph/zoekt/cmd/zoekt-indexer@latest"
        return 1
    fi
}

# Function to check and install dependencies
check_dependencies() {
    echo ""
    echo "🔧 Checking dependencies..."

    local go_installed=false
    local zoekt_installed=false
    local rust_installed=false

    # Check Rust
    if command_exists cargo; then
        echo "   ✅ Rust found: $(cargo --version)"
        rust_installed=true
    else
        echo "   ⚠️  Rust not found (required for Maestro v2)"
        read -p "   Install Rust now? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            if install_rust; then
                rust_installed=true
            fi
        fi
    fi

    # Check Go
    if command_exists go; then
        echo "   ✅ Go found: $(go version)"
        go_installed=true
    else
        echo "   ⚠️  Go not found"
        read -p "   Install Go now? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            if install_go; then
                go_installed=true
            fi
        else
            echo "   ℹ️  Skipping Go installation (Zoekt requires Go)"
        fi
    fi

    # Check Zoekt
    if command_exists zoekt-webserver && command_exists zoekt-indexer; then
        echo "   ✅ Zoekt found"
        zoekt_installed=true
    else
        echo "   ⚠️  Zoekt not found (optional but recommended)"
        if [ "$go_installed" = true ]; then
            read -p "   Install Zoekt now? (y/N) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                if install_zoekt; then
                    zoekt_installed=true
                fi
            else
                echo "   ℹ️  Skipping Zoekt installation"
            fi
        else
            echo "   ℹ️  Cannot install Zoekt without Go"
        fi
    fi

    echo ""
    echo "📋 Dependency Summary:"
    [ "$go_installed" = true ] && echo "   ✅ Go: Installed" || echo "   ⚠️  Go: Not installed"
    [ "$zoekt_installed" = true ] && echo "   ✅ Zoekt: Installed" || echo "   ⚠️  Zoekt: Not installed"
    echo ""

    if [ "$go_installed" = false ]; then
        echo "⚠️  Warning: Go is not installed. The Memory System code search feature requires Zoekt."
        echo "   You can install Go later: https://golang.org/dl/"
    fi

    if [ "$zoekt_installed" = false ]; then
        echo "ℹ️  Note: Zoekt is not installed. The Memory System will use fallback search mode."
        echo "   You can install Zoekt later with: go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest"
    fi
}

# Check dependencies before proceeding
check_dependencies

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

# Copy templates (use Claude Code versions for shared templates)
echo "📋 Copying templates..."
cp "$SCRIPT_DIR/claude-code/templates/workflow.md" ~/.claude/maestro-templates/
mkdir -p ~/.claude/maestro-templates/code_styleguides
cp "$SCRIPT_DIR/claude-code/templates/code_styleguides"/*.md ~/.claude/maestro-templates/code_styleguides/

# Build Memory Dashboard frontend FIRST (before pip install so dist files are included in package)
echo "🎨 Building Memory Dashboard frontend..."
FRONTEND_DIR="$SCRIPT_DIR/maestro/memory/frontend"
if [ -d "$FRONTEND_DIR" ]; then
    # Check if npm/node is available
    if command -v npm &> /dev/null && command -v node &> /dev/null; then
        echo "   Installing npm dependencies..."
        if cd "$FRONTEND_DIR" && npm install --quiet 2>/dev/null; then
            echo "   Building frontend..."
            if npm run build --quiet 2>/dev/null; then
                echo "   ✅ Frontend built successfully"
            else
                echo "   ⚠️  Warning: Frontend build failed"
                echo "   You can build it manually later:"
                echo "   cd $FRONTEND_DIR && npm install && npm run build"
            fi
        else
            echo "   ⚠️  Warning: npm install failed"
            echo "   You can install dependencies manually later:"
            echo "   cd $FRONTEND_DIR && npm install && npm run build"
        fi
    else
        echo "   ℹ️  npm/node not found, skipping frontend build"
        echo "   To enable the memory dashboard, install Node.js and build manually:"
        echo "   cd $FRONTEND_DIR && npm install && npm run build"
else
    echo "   ℹ️  Frontend directory not found, skipping build"
fi

# Build Maestro Core (Rust)
echo "🦀 Building Maestro Core (Rust)..."
if [ -d "$SCRIPT_DIR/maestro/leindex/rust" ]; then
    if command_exists cargo; then
        if cd "$SCRIPT_DIR/maestro/leindex/rust" && cargo build --release --quiet; then
            mkdir -p ~/.local/bin
            cp target/release/maestro ~/.local/bin/maestro
            chmod +x ~/.local/bin/maestro
            echo "   ✅ Maestro Core built and installed to ~/.local/bin/maestro"
        else
            echo "   ❌ Error: Maestro Core build failed"
        fi
    else
        echo "   ⚠️  Rust/Cargo not found, cannot build core"
    fi
else
    echo "   ⚠️  Maestro Core source not found"
fi

# Install Python CLI
 (after frontend is built so dist files are included in package)
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
        if .command.setup == null then
            .command.setup = {
                "template": "Read and execute from \($script_dir)/setup.md with args: $ARGUMENTS",
                "description": "Maestro setup command"
            }
        else
            .
        end |
        if .command.newTrack == null then
            .command.newTrack = {
                "template": "Read and execute from \($script_dir)/newTrack.md with args: $ARGUMENTS",
                "description": "Maestro newTrack command"
            }
        else
            .
        end |
        if .command.implement == null then
            .command.implement = {
                "template": "Read and execute from \($script_dir)/implement.md with args: $ARGUMENTS",
                "description": "Maestro implement command"
            }
        else
            .
        end |
        if .command.status == null then
            .command.status = {
                "template": "Read and execute from \($script_dir)/status.md with args: $ARGUMENTS",
                "description": "Maestro status command"
            }
        else
            .
        end |
        if .command.revert == null then
            .command.revert = {
                "template": "Read and execute from \($script_dir)/revert.md with args: $ARGUMENTS",
                "description": "Maestro revert command"
            }
        else
            .
        end |
        if .command.configure == null then
            .command.configure = {
                "template": "Read and execute from \($script_dir)/configure.md with args: $ARGUMENTS",
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
    if cmd not in config.get('command', {}):
        config.setdefault('command', {})[cmd] = {
            "template": f"Read and execute from {commands_dir}/{cmd}.md with args: $ARGUMENTS",
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
    "setup": {
      "template": "Read and execute from ~/.claude/commands/setup.md with args: $ARGUMENTS",
      "description": "Maestro setup command"
    },
    "newTrack": {
      "template": "Read and execute from ~/.claude/commands/newTrack.md with args: $ARGUMENTS",
      "description": "Maestro newTrack command"
    },
    "implement": {
      "template": "Read and execute from ~/.claude/commands/implement.md with args: $ARGUMENTS",
      "description": "Maestro implement command"
    },
    "status": {
      "template": "Read and execute from ~/.claude/commands/status.md with args: $ARGUMENTS",
      "description": "Maestro status command"
    },
    "revert": {
      "template": "Read and execute from ~/.claude/commands/revert.md with args: $ARGUMENTS",
      "description": "Maestro revert command"
    },
    "configure": {
      "template": "Read and execute from ~/.claude/commands/configure.md with args: $ARGUMENTS",
      "description": "Maestro configure command"
    }
  }
EOM
fi

# Create command symlinks
echo "🔗 Creating command symlinks..."
mkdir -p ~/.claude/commands
for cmd in setup newTrack implement status revert configure; do
    if [ ! -f ~/.claude/commands/"$cmd.md" ]; then
        cp "$SCRIPT_DIR/claude-code/commands/$cmd.md" ~/.claude/commands/"$cmd.md"
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
echo "🔍 Zoekt Code Search:"
if command_exists zoekt-webserver && command_exists zoekt-indexer; then
    echo "  ✅ Zoekt is installed and ready"
    echo "  Start Zoekt server: zoekt-webserver -rpc -index ~/.maestro/zoekt_index"
    echo "  Index code: zoekt-indexer -index ~/.maestro/zoekt_index -repo_name <name> <path>"
else
    echo "  ⚠️  Zoekt not installed (optional)"
    echo "  Install later: go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest"
fi
echo ""
echo "🎯 Next steps:"
echo "  1. Ensure ~/.local/bin is in your PATH:"
echo "     export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "  2. If you installed Go/Zoekt, ensure \$(go env GOPATH)/bin is in your PATH"
echo "  3. Restart OpenCode"
echo "  4. Run /maestro setup in your project directory"
echo "  5. Run /maestro configure to customize settings"
echo ""
echo "📖 For more information, see:"
echo "  https://github.com/scooter-lacroix/Maestro"
echo ""
