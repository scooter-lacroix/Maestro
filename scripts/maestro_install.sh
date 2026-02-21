#!/bin/sh
# ════════════════════════════════════════════════════════════════════════════════════════════
#
#    ██████   ██████   █████████   ██████████  █████████  ███████████ ███████████      ███████
#   ▒▒██████ ██████   ███▒▒▒▒▒███ ▒▒███▒▒▒▒▒█ ███▒▒▒▒▒███▒█▒▒▒███▒▒▒█▒▒███▒▒▒▒▒███   ███▒▒▒▒▒███
#    ▒███▒█████▒███  ▒███    ▒███  ▒███  █ ▒ ▒███    ▒▒▒ ▒   ▒███  ▒  ▒███    ▒███  ███     ▒▒███
#    ▒███▒▒███ ▒███  ▒███████████  ▒██████   ▒▒█████████     ▒███     ▒██████████  ▒███      ▒███
#    ▒███ ▒▒▒  ▒███  ▒███▒▒▒▒▒███  ▒███▒▒█    ▒▒▒▒▒▒▒▒███    ▒███     ▒███▒▒▒▒▒███ ▒███      ▒███
#    ▒███      ▒███  ▒███    ▒███  ▒███ ▒   █ ███    ▒███    ▒███     ▒███    ▒███ ▒▒███     ███
#    █████     █████ █████   █████ ██████████▒▒█████████     █████    █████   █████ ▒▒▒███████▒
#   ▒▒▒▒▒     ▒▒▒▒▒ ▒▒▒▒▒   ▒▒▒▒▒ ▒▒▒▒▒▒▒▒▒▒  ▒▒▒▒▒▒▒▒▒     ▒▒▒▒▒    ▒▒▒▒▒   ▒▒▒▒▒    ▒▒▒▒▒▒▒
#
#
#                                      ✨ Maestro v2.5 ✨
#                                    THE CONDUCTOR WIZARD
#                              Cross-Platform Linux Installer
#
# ═══════════════════════════════════════════════════════════════════════════════════════════════
#
# Supported distributions:
#   - Debian/Ubuntu (apt-get)
#   - Arch/CachyOS/Manjaro (pacman)
#   - Fedora/RHEL/CentOS (dnf)
#   - macOS (brew)
#
# Shell compatibility: bash, zsh, fish, POSIX sh
#

set -e

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Color definitions (POSIX-compatible)
# ═══════════════════════════════════════════════════════════════════════════════════════════════

C='\033[0;36m'
Y='\033[0;33m'
G='\033[0;32m'
R='\033[0;31m'
NC='\033[0m'

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Distribution Detection
# ═══════════════════════════════════════════════════════════════════════════════════════════════

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case "$ID" in
            arch|cachyos|manjaro|endeavouros|arcolinux|garuda)
                DISTRO="arch"
                ;;
            fedora|rhel|centos|almalinux|rocky|rockylinux)
                DISTRO="fedora"
                ;;
            debian|ubuntu|linuxmint|pop|elementary|kali|raspbian|zorin)
                DISTRO="debian"
                ;;
            *)
                # Check ID_LIKE for derivatives
                case "$ID_LIKE" in
                    *arch*) DISTRO="arch" ;;
                    *fedora*|*rhel*|*centos*) DISTRO="fedora" ;;
                    *debian*|*ubuntu*) DISTRO="debian" ;;
                    *) DISTRO="unknown" ;;
                esac
                ;;
        esac
    elif [ "$(uname)" = "Darwin" ]; then
        DISTRO="macos"
    else
        DISTRO="unknown"
    fi
    echo "$DISTRO"
}

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Package Manager Detection
# ═══════════════════════════════════════════════════════════════════════════════════════════════

detect_package_manager() {
    if command -v apt-get >/dev/null 2>&1; then
        PKM="apt-get"
    elif command -v pacman >/dev/null 2>&1; then
        PKM="pacman"
    elif command -v dnf >/dev/null 2>&1; then
        PKM="dnf"
    elif command -v brew >/dev/null 2>&1; then
        PKM="brew"
    else
        PKM="unknown"
    fi
    echo "$PKM"
}

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Package Name Mapping (per distro)
# ═══════════════════════════════════════════════════════════════════════════════════════════════

get_package_name() {
    purpose="$1"
    distro="$2"
    
    case "$purpose" in
        build-tools)
            case "$distro" in
                debian) echo "build-essential" ;;
                arch) echo "base-devel" ;;
                fedora) echo "@development-tools" ;;
                *) echo "build-essential" ;;
            esac
            ;;
        pkg-config)
            case "$distro" in
                debian) echo "pkg-config" ;;
                arch) echo "pkgconf" ;;
                fedora) echo "pkgconfig" ;;
                *) echo "pkg-config" ;;
            esac
            ;;
        *)
            # Return the purpose as-is for common packages
            echo "$purpose"
            ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Install Package Function
# ═══════════════════════════════════════════════════════════════════════════════════════════════

install_package() {
    pkg="$1"
    pkm="$2"
    distro="$3"
    
    echo -e "${Y}  [!] Installing ${pkg}...${NC}"
    case "$pkm" in
        apt-get)
            sudo apt-get update 2>/dev/null || true
            sudo apt-get install -y "$pkg"
            ;;
        pacman)
            sudo pacman -S --noconfirm --needed "$pkg"
            ;;
        dnf)
            sudo dnf install -y "$pkg"
            ;;
        brew)
            brew install "$pkg"
            ;;
        *)
            echo -e "${R}      Cannot install ${pkg}: Unknown package manager.${NC}"
            echo -e "${Y}      Please install ${pkg} manually.${NC}"
            return 1
            ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Ensure Package Function
# ═══════════════════════════════════════════════════════════════════════════════════════════════

ensure_package() {
    cmd="$1"
    purpose="$2"
    
    if ! command -v "$cmd" >/dev/null 2>&1; then
        pkg=$(get_package_name "$purpose" "$DISTRO")
        # Special handling for build-tools group on Fedora
        if [ "$purpose" = "build-tools" ] && [ "$DISTRO" = "fedora" ]; then
            echo -e "${Y}  [!] Installing development tools group...${NC}"
            sudo dnf group install -y "Development Tools"
        else
            install_package "$pkg" "$PKM" "$DISTRO"
        fi
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════════════════════
# Main Script
# ═══════════════════════════════════════════════════════════════════════════════════════════════

clear
echo -e "${C}    Preparing the Overture...${NC}"

# Detect distribution and package manager
DISTRO=$(detect_distro)
PKM=$(detect_package_manager)

echo -e "${C}    Detected: ${DISTRO} (${PKM})${NC}"
echo -e "${C}    Revisiting system requirements...${NC}"

# Pre-flight Checklist
ensure_package "git" "git"
ensure_package "curl" "curl"
ensure_package "tmux" "tmux"
ensure_package "gcc" "build-tools"
ensure_package "pkg-config" "pkg-config"

# Check for Rust/Cargo
if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${Y}  [!] Rust not found.${NC} Rust is required to build the Conductor Wizard."
    echo -e "      Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    
    # Source cargo env (compatible with bash/zsh/fish)
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    elif [ -f "$HOME/.bashrc" ]; then
        # For bash, source bashrc which should have cargo
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# Determine the repo root directory
# POSIX-compatible way to get script directory
SCRIPT_DIR=""
if [ -n "${BASH_SOURCE:-}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "$BASH_SOURCE")" 2>/dev/null && pwd)"
elif [ -n "${ZSH_VERSION:-}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${(%):-%x}")" 2>/dev/null && pwd)"
else
    SCRIPT_DIR="$(cd "$(dirname "$0")" 2>/dev/null && pwd)"
fi
REPO_ROOT="$(cd "$SCRIPT_DIR/.." 2>/dev/null && pwd)"

if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT/maestro/leindex/rust" ]; then
    echo -e "${Y}  [!] Running from pipe — cloning Maestro repository...${NC}"
    TMPDIR="${TMPDIR:-/tmp}"
    REPO_TMP="$(mktemp -d "$TMPDIR/maestro-XXXXXX")"
    # Default branch is v2.5, but can be overridden with MAESTRO_BRANCH env var
    MAESTRO_BRANCH="${MAESTRO_BRANCH:-v2.5}"
    echo -e "${C}    Cloning branch: ${MAESTRO_BRANCH}${NC}"
    git clone --depth 1 --branch "$MAESTRO_BRANCH" https://github.com/scooter-lacroix/Maestro.git "$REPO_TMP/Maestro"
    REPO_ROOT="$REPO_TMP/Maestro"
fi

cd "$REPO_ROOT/maestro/leindex/rust"

# Build and Run the Conductor Wizard
echo -e "${G}    Launching Maestro Conductor Wizard...${NC}"
echo -e "    ${C}Please wait while the orchestra tunes (compiling setup tool)${NC}"
echo -e "    ${C}Detected distribution: ${DISTRO}${NC}"
echo -e "    ${C}Package manager: ${PKM}${NC}"

cargo run --release --bin maestro-setup
