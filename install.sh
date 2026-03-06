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

# Default branch (can be overridden with MAESTRO_BRANCH env var)
MAESTRO_BRANCH="${MAESTRO_BRANCH:-v2.5}"
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

# Verify we're on the correct branch (for local installs)
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
if [[ "$CURRENT_BRANCH" != "$MAESTRO_BRANCH" && "$CURRENT_BRANCH" != "HEAD" ]]; then
    echo -e "${Y}    [Warning] Current branch is '$CURRENT_BRANCH', but installing from branch '$MAESTRO_BRANCH'${NC}"
    echo -e "${Y}    Switching to branch: $MAESTRO_BRANCH${NC}"
    git fetch origin "$MAESTRO_BRANCH" || git fetch origin
    git checkout "$MAESTRO_BRANCH"
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
if command -v bun &> /dev/null; then
    bun install || true
    # Install tracklens-hook dependencies
    cd "$INSTALL_DIR/apps/tracklens-hook" && bun install 2>/dev/null || true
    cd "$INSTALL_DIR"
    bun run build:tracklens || echo -e "${Y}  [!] TrackLens build failed, continuing...${NC}"
    # Copy HTML files to tracklens-hook dist
    cd "$INSTALL_DIR/apps/tracklens-hook" && bun run copy:html 2>/dev/null || echo -e "${Y}  [!] HTML copy failed, continuing...${NC}"
else
    npm install && npm run build:tracklens || echo -e "${Y}  [!] TrackLens build failed, continuing...${NC}"
    # Install tracklens-hook dependencies
    cd "$INSTALL_DIR/apps/tracklens-hook" && npm install 2>/dev/null || true
    cd "$INSTALL_DIR"
fi

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

# Check if we have a TTY for the TUI
if [[ -t 0 ]]; then
    # stdin is a TTY, run directly
    cargo run --release --bin maestro-setup
else
    # No TTY available, use script to provide a pseudo-TTY
    # This is needed for the ratatui TUI to work
    script -qec "cargo run --release --bin maestro-setup" /dev/null
fi

# Clean up temp install directory if we cloned
if [[ "$INSTALL_DIR" == "$HOME/.maestro/install-temp" ]]; then
    echo -e "${C}    Cleaning up temporary install directory...${NC}"
    rm -rf "$INSTALL_DIR"
fi

echo -e "${G}    Installation complete!${NC}"
echo -e "    Run 'maestro' to get started."
