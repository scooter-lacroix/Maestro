#!/bin/bash
# Maestro Installer for Claude Code v2.0.0
# Enhanced with hooks, skills, agents, and memory system
set -e

echo "🚀 Installing Maestro for Claude Code..."

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

# Function to backup existing configuration
backup_config() {
    local config_dir="$HOME/.claude"
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_file="$HOME/.claude.backup.${timestamp}.tar.gz"

    if [ -d "$config_dir" ]; then
        echo ""
        echo "📦 Backing up existing configuration..."
        if tar -czf "$backup_file" -C "$HOME" ".claude" 2>/dev/null; then
            echo "   ✅ Backup created: $backup_file"
        else
            echo "   ⚠️  Warning: Backup failed. Continuing without backup..."
        fi
    fi
}

# Function to restore configuration
restore_config() {
    local backup_file="$1"
    local config_dir="$HOME/.claude"

    if [ -f "$backup_file" ]; then
        echo ""
        echo "🔄 Restoring configuration from $backup_file..."
        # Create backup of current before restore if it exists
        if [ -d "$config_dir" ]; then
            mv "$config_dir" "${config_dir}.pre-restore.$(date +%Y%m%d_%H%M%S)"
        fi

        if tar -xzf "$backup_file" -C "$HOME"; then
            echo "   ✅ Restore complete"
        else
            echo "   ❌ Restore failed"
            return 1
        fi
    else
        echo "   ❌ Error: Backup file not found: $backup_file"
        return 1
    fi
}

# Handle arguments
if [[ "$1" == "--restore" ]]; then
    if [ -n "$2" ]; then
        restore_config "$2"
        exit 0
    else
        echo "Usage: $0 --restore <backup_file>"
        exit 1
    fi
fi

# Perform backup before installation
backup_config

# Create a temporary directory for downloading the repository
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

echo "📥 Downloading Maestro repository..."
REPO_URL="https://github.com/scooter-lacroix/Maestro"
REPO_BRANCH="v2"

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

# Create plugin directory and copy plugin files
echo "📁 Creating plugin directory..."
mkdir -p ~/.claude/plugins/maestro
if [ -f "$SCRIPT_DIR/plugin.json" ]; then
    echo "📋 Copying plugin files..."
    cp "$SCRIPT_DIR/plugin.json" ~/.claude/plugins/maestro/
    # Copy other plugin files if they exist
    [ -f "$SCRIPT_DIR/README.md" ] && cp "$SCRIPT_DIR/README.md" ~/.claude/plugins/maestro/ 2>/dev/null
    [ -f "$SCRIPT_DIR/LICENSE" ] && cp "$SCRIPT_DIR/LICENSE" ~/.claude/plugins/maestro/ 2>/dev/null
    echo "   ✅ Plugin configuration installed"
else
    echo "   ⚠️  Warning: plugin.json not found, skipping plugin configuration"
fi

# Copy hooks (v2 component)
echo "📋 Copying hooks..."
if [ -d "$SCRIPT_DIR/maestro/hooks" ]; then
    # Check for TypeScript hooks that need building
    if [ -f "$SCRIPT_DIR/maestro/hooks/package.json" ]; then
        echo "📦 Detected package.json in hooks directory. Building TypeScript hooks..."
        if command_exists npm; then
            echo "   Running npm install..."
            if (cd "$SCRIPT_DIR/maestro/hooks" && npm install --quiet 2>/dev/null); then
                echo "   Running npm run build..."
                if (cd "$SCRIPT_DIR/maestro/hooks" && npm run build --quiet 2>/dev/null); then
                    echo "   ✅ TypeScript hooks built successfully"
                else
                    echo "   ⚠️  Warning: TypeScript hook build failed"
                fi
            else
                echo "   ⚠️  Warning: npm install for hooks failed"
            fi
        else
            echo "   ⚠️  Warning: npm not found. Skipping TypeScript hook build."
            echo "   Please install Node.js and npm to build hooks manually."
        fi
    fi

    mkdir -p ~/.claude/plugins/maestro/hooks
    cp -r "$SCRIPT_DIR/maestro/hooks"/* ~/.claude/plugins/maestro/hooks/
    echo "   ✅ Hooks installed ($(find "$SCRIPT_DIR/maestro/hooks" -name "*.py" | wc -l) hook files)"
else
    echo "   ⚠️  Warning: hooks directory not found"
fi

# Copy skills (v2 component)
echo "📋 Copying skills..."
if [ -d "$SCRIPT_DIR/maestro/skills" ]; then
    mkdir -p ~/.claude/plugins/maestro/skills
    cp -r "$SCRIPT_DIR/maestro/skills"/* ~/.claude/plugins/maestro/skills/
    echo "   ✅ Skills installed ($(find "$SCRIPT_DIR/maestro/skills" -name "SKILL.md" | wc -l) skills)"
else
    echo "   ⚠️  Warning: skills directory not found"
fi

# Copy agents (v2 component)
echo "📋 Copying agents..."
if [ -d "$SCRIPT_DIR/maestro/agents" ]; then
    mkdir -p ~/.claude/plugins/maestro/agents
    cp -r "$SCRIPT_DIR/maestro/agents"/* ~/.claude/plugins/maestro/agents/
    echo "   ✅ Agents installed ($(find "$SCRIPT_DIR/maestro/agents" -name "*.md" | wc -l) agents)"
else
    echo "   ⚠️  Warning: agents directory not found"
fi

# Copy config module (v2 component)
echo "📋 Copying config module..."
if [ -d "$SCRIPT_DIR/maestro/config" ]; then
    mkdir -p ~/.claude/plugins/maestro/config
    cp -r "$SCRIPT_DIR/maestro/config"/* ~/.claude/plugins/maestro/config/
    echo "   ✅ Config module installed"
else
    echo "   ⚠️  Warning: config directory not found"
fi

# Copy critical_think module
echo "📋 Copying critical_think module..."
if [ -d "$SCRIPT_DIR/maestro/critical_think" ]; then
    cp -r "$SCRIPT_DIR/maestro/critical_think" ~/.claude/plugins/maestro/
    echo "   ✅ Critical Think module installed"
else
    echo "   ⚠️  Warning: critical_think module not found"
fi

# Copy critical_think templates
if [ -d "$SCRIPT_DIR/maestro/critical_think/templates" ]; then
    cp "$SCRIPT_DIR/maestro/critical_think/templates"/*.md ~/.claude/maestro-templates/
    echo "   ✅ Critical Think templates installed"
fi

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
    fi
else
    echo "   ℹ️  Frontend directory not found, skipping build"
fi

# Install Python CLI (after frontend is built so dist files are included in package)
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
echo "✅ Maestro v2.0.0 installed successfully for Claude Code!"
echo ""
echo "📚 Available Claude Code slash commands:"
echo "  /maestro:setup      - Initialize Maestro environment"
echo "  /maestro:newTrack   - Create new track"
echo "  /maestro:implement  - Implement track tasks"
echo "  /maestro:status     - View project progress"
echo "  /maestro:revert     - Revert work"
echo "  /maestro:configure  - Configure Maestro settings"
echo "  /maestro:memory     - Interact with Memory System"
echo "  /maestro:tui        - Launch Terminal UI"
echo ""
echo "🔌 v2 Components Installed:"
echo "  ✅ Hooks      - Event-driven automation (16 hooks)"
echo "  ✅ Skills     - Specialized capabilities (109+ skills)"
echo "  ✅ Agents     - Task delegation (28 agents)"
echo "  ✅ Config     - Unified settings management"
echo "  ✅ Memory     - Persistent context system"
echo ""
echo "🖥️  Available CLI tools (from terminal):"
echo "  maestro memory serve    - Start memory dashboard web server"
echo "  maestro memory status   - Show memory system status"
echo "  maestro memory migrate  - Migrate from Memori to Nexus"
echo "  maestro tui             - Launch Terminal UI (requires Go build)"
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
echo "  3. Open Claude Code"
echo "  4. Run /maestro:setup in your project directory"
echo "  5. Run /maestro:configure to customize settings"
echo ""
echo "📖 For more information, see:"
echo "  https://github.com/scooter-lacroix/Maestro"
echo ""
