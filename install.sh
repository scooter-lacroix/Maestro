#!/bin/bash
# ════════════════════════════════════════════════════════════════════════════════════════════
#
#    ██████   ██████   █████████   ██████████  █████████  ███████████ ███████████      ███████
#   ▒▒██████ ██████   ███▒▒▒▒▒███ ▒▒███▒▒▒▒▒█ ███▒▒▒▒▒███▒█▒▒▒███▒▒▒█▒▒███▒▒▒▒▒███   ███▒▒▒▒▒███
#    ▒███▒█████▒███  ▒███    ▒███  ▒███  █ ▒ ▒███    ▒▒▒ ▒   ▒███  ▒  ▒███    ▒███  ███     ▒▒███
#    ▒███▒▒███ ▒███  ▒███████████  ▒██████   ▒▒█████████     ▒███     ▒██████████  ▒███      ▒███
#    ▒███ ▒▒▒  ▒███  ▒███▒▒▒▒▒███  ▒███▒▒█    ▒▒▒▒▒▒▒▒███    ▒███     ▒███▒▒▒▒▒███ ▒███      ▒███
#    ▒███      ▒███  ▒███    ▒███  ▒███ ▒   █ ███    ▒███    ▒███     ▒███    ▒███ ▒▒███     ███
#    █████     █████ █████   █████ ██████████▒▒█████████     █████    █████   █████ ▒▒▒███████▒
#   ▒▒▒▒▒     ▒▒▒▒▒ ▒▒▒▒▒   ▒▒▒▒▒ ▒▒▒▒▒▒▒▒▒  ▒▒▒▒▒▒▒▒▒     ▒▒▒▒▒    ▒▒▒▒▒   ▒▒▒▒▒    ▒▒▒▒▒▒▒
#
#
#                                      ✨ Maestro v2 ✨
#                                    THE CONDUCTOR WIZARD
#
# ═══════════════════════════════════════════════════════════════════════════════════════════════════════

set -e

# Colors
C='\033[0;36m'
Y='\033[0;33m'
G='\033[0;32m'
R='\033[0;31m'
NC='\033[0m'

clear
echo -e "${C}    Preparing the Overture...${NC}"

# Default remote installs to main. Local installs stay on the current checkout
# unless MAESTRO_BRANCH was explicitly provided by the caller.
BRANCH_EXPLICIT=0
if [[ -n "${MAESTRO_BRANCH:-}" ]]; then
    BRANCH_EXPLICIT=1
fi
MAESTRO_BRANCH="${MAESTRO_BRANCH:-main}"
REPO_URL="${REPO_URL:-https://github.com/scooter-lacroix/Maestro.git}"

# Determine if we're running from a cloned repo or a remote install
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)"
INSTALL_DIR=""

# Check if we're in a valid Maestro repo
if [[ -f "$SCRIPT_DIR/install.sh" && -d "$SCRIPT_DIR/src/leindex" ]]; then
    # Running from a cloned repo
    INSTALL_DIR="$SCRIPT_DIR"
    echo -e "${G}    [Local Install]${NC} Installing from: $INSTALL_DIR"
else
    # Running from remote or invalid location - need to clone
    INSTALL_DIR="$HOME/.maestro/install-temp"
    echo -e "${G}    [Remote Install]${NC} Cloning from: $REPO_URL (branch: $MAESTRO_BRANCH)"

    # Remove temp dir if it exists from a previous failed install
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
    fi

    mkdir -p "$HOME/.maestro"
    git clone --depth 1 --branch "$MAESTRO_BRANCH" "$REPO_URL" "$INSTALL_DIR"
fi

# Change to the Rust directory
cd "$INSTALL_DIR"

# Respect the current local checkout unless a branch was explicitly requested.
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
if [[ "$BRANCH_EXPLICIT" -eq 1 && "$CURRENT_BRANCH" != "$MAESTRO_BRANCH" ]]; then
    echo -e "${Y}    [Branch Override] Current branch is '$CURRENT_BRANCH'; switching to '$MAESTRO_BRANCH' as requested.${NC}"
    git fetch origin "$MAESTRO_BRANCH" || git fetch origin
    git checkout "$MAESTRO_BRANCH"
elif [[ "$BRANCH_EXPLICIT" -eq 0 && "$CURRENT_BRANCH" != "HEAD" && "$CURRENT_BRANCH" != "unknown" ]]; then
    echo -e "${C}    [Local Install] Using current branch: $CURRENT_BRANCH${NC}"
fi

cd "$INSTALL_DIR/src/leindex"

# Check for basic build tools and dependencies
echo -e "${C}    Revising system requirements...${NC}"

# Detect Package Manager
PKM=""
if command -v apt-get &> /dev/null; then PKM="apt-get"; fi
if command -v pacman &> /dev/null; then PKM="pacman"; fi
if command -v dnf &> /dev/null; then PKM="dnf"; fi
if command -v brew &> /dev/null; then PKM="brew"; fi

# Function to install if missing
ensure_package() {
    local cmd=$1
    local pkg=$2
    if ! command -v $cmd &> /dev/null; then
        echo -e "${Y}  [!] $cmd not found.${NC} Attempting to install $pkg..."
        case $PKM in
            "apt-get") sudo apt-get update && sudo apt-get install -y $pkg ;;
            "pacman") sudo pacman -Sy --noconfirm $pkg ;;
            "dnf") sudo dnf install -y $pkg ;;
            "brew") brew install $pkg ;;
            *) echo -e "${R}      Dynamic installation failed: Unknown package manager.${NC}" ;;
        esac
    fi
}

# Pre-flight Checklist
ensure_package "git" "git"
ensure_package "curl" "curl"
ensure_package "tmux" "tmux"
ensure_package "gcc" "build-essential"
ensure_package "pkg-config" "pkg-config"

# Check for Bun (required for TrackLens)
if ! command -v bun &> /dev/null; then
    echo -e "${Y}  [!] Bun not found.${NC} Installing Bun..."
    curl -fsSL https://bun.sh/install | bash
    export BUN_INSTALL="$HOME/.bun"
    export PATH="$BUN_INSTALL/bin:$PATH"
fi

# Check for Rust/Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${Y}  [!] Rust not found.${NC} Rust is required to build the Conductor Wizard."
    echo -e "      Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Build TrackLens packages
echo -e "${C}    Building TrackLens packages...${NC}"
cd "$INSTALL_DIR"
if ! command -v bun &> /dev/null; then
    echo -e "${R}  [!] Bun is required to install the TrackLens workspace.${NC}"
    exit 1
fi

echo -e "${C}    Installing TrackLens workspace dependencies with Bun...${NC}"
bun install
echo -e "${C}    Building TrackLens workspace (packages and apps)...${NC}"
bun run build:tracklens

# Install TrackLens Claude Code Plugin
echo -e "${C}    Installing TrackLens Claude Code Plugin...${NC}"
mkdir -p "$HOME/.claude/plugins/tracklens"
cp -r "$INSTALL_DIR/apps/tracklens-hook/"* "$HOME/.claude/plugins/tracklens/"

# Install OpenCode Skill
echo -e "${C}    Installing OpenCode Skill...${NC}"
mkdir -p "$HOME/.opencode/skill/maestro"
cp -r "$INSTALL_DIR/opencode/skill/maestro/"* "$HOME/.opencode/skill/maestro/" 2>/dev/null || true

# Install Claude Hooks
echo -e "${C}    Installing Claude Hooks...${NC}"
if [ -f "$HOME/.claude/settings.json" ]; then
    # Backup existing settings
    cp "$HOME/.claude/settings.json" "$HOME/.claude/settings.json.bak"
    
    # Add hooks if not already present
    if ! grep -q "tracklens" "$HOME/.claude/settings.json" 2>/dev/null; then
        echo -e "${Y}  [!] Adding TrackLens hooks to settings.json...${NC}"
        # Note: Claude Code hooks are handled by the plugin system
    fi
fi

# Build and Run the Conductor Wizard
echo -e "${G}    Launching Maestro Conductor Wizard...${NC}"
echo -e "    ${C}Please wait while the orchestra tunes (compiling setup tool)${NC}"

# Change to leindex Rust directory
cd "$INSTALL_DIR/src/leindex"

# Check for headless mode (non-interactive install)
if [[ "${MAESTRO_HEADLESS:-}" == "1" || "${MAESTRO_HEADLESS:-}" == "true" ]]; then
    echo -e "${C}    Running in headless mode (skipping interactive TUI)...${NC}"
    echo -e "${Y}  [!] Headless install not yet fully implemented. Please run interactively.${NC}"
    echo -e "${Y}      Or manually build and install the components.${NC}"
    exit 1
fi

# Check if we have a TTY for the TUI (need both stdin and stdout)
SETUP_SUCCESS=0
if [[ -t 0 && -t 1 ]]; then
    # Both stdin and stdout are TTYs, run directly
    echo -e "${C}    Launching interactive setup wizard...${NC}"
    if cargo run --release --bin maestro-setup; then
        SETUP_SUCCESS=1
    fi
else
    echo -e "${Y}  [!] No TTY detected. Attempting to provide pseudo-terminal...${NC}"
    
    # Try different methods to provide a pseudo-TTY
    if command -v script &> /dev/null; then
        # Try script command with different options
        if script -q -c "cargo run --release --bin maestro-setup" /dev/null 2>/dev/null; then
            SETUP_SUCCESS=1
        elif script -qec "cargo run --release --bin maestro-setup" /dev/null 2>/dev/null; then
            SETUP_SUCCESS=1
        elif script "cargo run --release --bin maestro-setup" /dev/null 2>/dev/null; then
            SETUP_SUCCESS=1
        fi
    fi
    
    # If script command failed or isn't available, try expect
    if [[ $SETUP_SUCCESS -eq 0 ]] && command -v expect &> /dev/null; then
        echo -e "${C}    Trying expect for pseudo-terminal...${NC}"
        if expect -c "spawn cargo run --release --bin maestro-setup; interact"; then
            SETUP_SUCCESS=1
        fi
    fi
fi

# If all TTY methods failed, provide helpful error message
if [[ $SETUP_SUCCESS -eq 0 ]]; then
    echo ""
    echo -e "${R}  [!] Setup wizard failed to run.${NC}"
    echo -e "${Y}      This installer requires an interactive terminal.${NC}"
    echo ""
    echo -e "${C}    Possible solutions:${NC}"
    echo -e "      1. Run directly in a terminal: ${Y}bash install.sh${NC}"
    echo -e "      2. If using SSH, ensure TTY allocation: ${Y}ssh -t user@host 'bash install.sh'${NC}"
    echo -e "      3. If running in a container, allocate TTY: ${Y}docker run -it ...${NC}"
    echo -e "      4. For CI/automation, install components manually (see docs)"
    echo ""
    echo -e "${C}    Manual installation steps:${NC}"
    echo -e "      1. Build CLI: ${Y}cargo build --release --manifest-path crates/cli/Cargo.toml${NC}"
    echo -e "      2. Copy binary: ${Y}cp target/release/maestro ~/.local/bin/${NC}"
    echo -e "      3. See README.md for full manual setup instructions"
    echo ""
    exit 1
fi

# Clean up temp install directory if we cloned
if [[ "$INSTALL_DIR" == "$HOME/.maestro/install-temp" ]]; then
    echo -e "${C}    Cleaning up temporary install directory...${NC}"
    rm -rf "$INSTALL_DIR"
fi

echo -e "${G}    Installation complete!${NC}"
echo -e "    Run 'maestro' to get started."
