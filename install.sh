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
#   ▒▒▒▒▒     ▒▒▒▒▒ ▒▒▒▒▒   ▒▒▒▒▒ ▒▒▒▒▒▒▒▒▒▒  ▒▒▒▒▒▒▒▒▒     ▒▒▒▒▒    ▒▒▒▒▒   ▒▒▒▒▒    ▒▒▒▒▒▒▒
#
#
#
#                  █████████     ███████    ██████████   ██████████
#                 ███▒▒▒▒▒███  ███▒▒▒▒▒███ ▒▒███▒▒▒▒███ ▒▒███▒▒▒▒▒█
#                ███     ▒▒▒  ███     ▒▒███ ▒███   ▒▒███ ▒███  █ ▒
#               ▒███         ▒███      ▒███ ▒███    ▒███ ▒██████
#               ▒███         ▒███      ▒███ ▒███    ▒███ ▒███▒▒█
#               ▒▒███     ███▒▒███     ███  ▒███    ███  ▒███ ▒   █
#                ▒▒█████████  ▒▒▒███████▒   ██████████   ██████████
#                 ▒▒▒▒▒▒▒▒▒     ▒▒▒▒▒▒▒    ▒▒▒▒▒▒▒▒▒▒   ▒▒▒▒▒▒▒▒▒▒
#
#                                      ✨ Maestro v2 ✨
#                             Your AI-Powered Project Orchestrator
#                                    Unified Installer
#
#                          ✨🌟Every masterpiece needs a conductor🌟✨

# ═══════════════════════════════════════════════════════════════════════════════════════════════

set -e

# Ensure we are running in bash
if [ -z "$BASH_VERSION" ]; then
    echo "Error: This script must be run with bash."
    echo "Please run as: ./install.sh or bash install.sh"
    exit 1
fi

# ─────────────────────────────────────────────────────────────────────────────
# 🎨 COLOR PALETTE & VISUAL STYLING (zsh compatible)
# ─────────────────────────────────────────────────────────────────────────────
if [[ -n "$ZSH_VERSION" ]] || [[ -o interactive ]]; then
    # zsh compatibility
    R=$'\033[0;31m'    G=$'\033[0;32m'    Y=$'\033[0;33m'    B=$'\033[0;34m'
    M=$'\033[0;35m'    C=$'\033[0;36m'    W=$'\033[0;37m'    BR=$'\033[1;31m'
    BG=$'\033[1;32m'   BY=$'\033[1;33m'   BB=$'\033[1;34m'   BM=$'\033[1;35m'
    BC=$'\033[1;36m'   BW=$'\033[1;37m'   BD=$'\033[1;90m'   NC=$'\033[0m'
else
    # bash
    R='\033[0;31m'     G='\033[0;32m'     Y='\033[0;33m'     B='\033[0;34m'
    M='\033[0;35m'     C='\033[0;36m'     W='\033[0;37m'     BR='\033[1;31m'
    BG='\033[1;32m'    BY='\033[1;33m'    BB='\033[1;34m'    BM='\033[1;35m'
    BC='\033[1;36m'    BW='\033[1;37m'    BD='\033[1;90m'    NC='\033[0m'
fi

# ─────────────────────────────────────────────────────────────────────────────
# 🌟 ANIMATIONS & SPINNERS
# ─────────────────────────────────────────────────────────────────────────────
spinner_frames=("⠋" "⠙" "⠹" "⠸" "⠼" "⠴" "⠦" "⠧" "⠇" "⠏")
dots_frames=("⣾" "⣽" "⣻" "⢿" "⡿" "⣟" "⣯" "⣷")
moon_frames=("🌑" "🌒" "🌓" "🌔" "🌕" "🌖" "🌗" "🌘" "🌑")
star_frames=("✨" "💫" "⭐" "🌟" "✨" "💫" "⭐" "🌟")
music_frames=("♪" "♫" "♬" "♩" "♪" "♫" "♬" "♩")
pulse_frames=("▁" "▂" "▃" "▄" "▅" "▆" "▇" "█" "▇" "▆" "▅" "▄" "▃" "▂")

# Cursor hide/show
cursor_hide() { printf '\033[?25l'; }
cursor_show() { printf '\033[?25h'; }
trap cursor_show EXIT

# ─────────────────────────────────────────────────────────────────────────────
# 🎭 UTILITY FUNCTIONS
# ─────────────────────────────────────────────────────────────────────────────

# Detect shell
detect_shell() {
    # Check the user's login shell, not the current shell running the script
    local user_shell=$(basename "$SHELL")
    if [[ "$user_shell" == "zsh" ]]; then
        echo "zsh"
    elif [[ "$user_shell" == "bash" ]]; then
        echo "bash"
    else
        echo "unknown"
    fi
}

# Clear screen with style
clear_screen() {
    clear
    printf "${NC}"
}

# Print a section header (zsh compatible)
print_header() {
    local text="$1"
    local icon="${2:-┃}"
    local width=76
    local line=""
    for ((i=0; i<width; i++)); do line="${line}─"; done

    echo ""
    echo -e "${C}    ${line}${NC}"
    echo -e "${BM}  ${icon}  ${text}${NC}"
    echo -e "${C}    ${line}${NC}"
    echo ""
}

# Print success message
print_success() {
    local msg="$1"
    echo -e "${BG}  ✅ ${msg}${NC}"
}

# Print warning message
print_warning() {
    local msg="$1"
    echo -e "${BY}  ⚠️  ${msg}${NC}"
}

# Print error message
print_error() {
    local msg="$1"
    echo -e "${BR}  ❌ ${msg}${NC}"
}

# Print info message
print_info() {
    local msg="$1"
    echo -e "${BC}  ℹ️  ${msg}${NC}"
}

# Print step indicator
print_step() {
    local step="$1"
    local total="$2"
    local msg="$3"
    echo -e "${BM}  [${step}/${total}]${NC} ${BW}${msg}${NC}..."
}

# Animated spinner with message
show_spinner() {
    local msg="$1"
    local pid=$2
    local delay=0.1
    local spin_type="${3:-0}"

    local frames
    case $spin_type in
        0) frames=("${dots_frames[@]}") ;;
        1) frames=("${moon_frames[@]}") ;;
        2) frames=("${star_frames[@]}") ;;
        3) frames=("${music_frames[@]}") ;;
        4) frames=("${pulse_frames[@]}") ;;
        *) frames=("${dots_frames[@]}") ;;
    esac

    cursor_hide
    local i=0
    while kill -0 $pid 2>/dev/null; do
        printf "\r${C}  ${frames[$i]}${NC} ${BW}${msg}...${NC}    " 2>/dev/null
        i=$(( (i + 1) % ${#frames[@]} ))
        sleep $delay
    done
    printf "\r${G}  ✨${NC} ${BW}${msg}${NC}         \n"
    cursor_show
}

# Sleep with animation
sleep_anim() {
    local duration=$1
    local spin_type="${2:-4}"
    local duration_ns=$(awk "BEGIN {printf \"%.0f\", $duration * 1000000000}")
    local end_time=$(($(date +%s%N) + duration_ns))
    local frames=("${pulse_frames[@]}")
    local i=0

    cursor_hide
    while [ $(date +%s%N) -lt $end_time ]; do
        printf "\r${M}  ${frames[$i]}${NC} Working magic..." 2>/dev/null
        i=$(( (i + 1) % ${#frames[@]} ))
        sleep 0.05
    done
    printf "\r                         \r"
    cursor_show
}

# ─────────────────────────────────────────────────────────────────────────────
# 🔧 PATH MANAGEMENT (bash & zsh)
# ─────────────────────────────────────────────────────────────────────────────

add_to_path() {
    local new_path="$1"

    # Add to current session
    export PATH="${new_path}:${PATH}"

    # Add to both bashrc and zshrc if they exist
    for shell_rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
        if [[ -f "$shell_rc" ]]; then
            if ! grep -q "export PATH=\".*${new_path}" "$shell_rc" 2>/dev/null; then
                echo "" >> "$shell_rc"
                echo "# Maestro PATH" >> "$shell_rc"
                echo "export PATH=\"${new_path}:\$PATH\"" >> "$shell_rc"
                echo -e "${C}  →${NC} Added to ${Y}${shell_rc}${NC}"
            fi
        fi
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# 🎪 BANNER & INTRO
# ─────────────────────────────────────────────────────────────────────────────

show_banner() {
    clear_screen

    cat << 'EOF'

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
#
#                  █████████     ███████    ██████████   ██████████
#                 ███▒▒▒▒▒███  ███▒▒▒▒▒███ ▒▒███▒▒▒▒███ ▒▒███▒▒▒▒▒█
#                ███     ▒▒▒  ███     ▒▒███ ▒███   ▒▒███ ▒███  █ ▒
#               ▒███         ▒███      ▒███ ▒███    ▒███ ▒██████
#               ▒███         ▒███      ▒███ ▒███    ▒███ ▒███▒▒█
#               ▒▒███     ███▒▒███     ███  ▒███    ███  ▒███ ▒   █
#                ▒▒█████████  ▒▒▒███████▒   ██████████   ██████████
#                 ▒▒▒▒▒▒▒▒     ▒▒▒▒▒▒▒    ▒▒▒▒▒▒▒▒▒▒   ▒▒▒▒▒▒▒▒▒▒
#
#                                      ✨ Maestro v2 ✨
#                             Your AI-Powered Project Orchestrator
#                                    Unified Installer
#
#                          ✨🌟Every masterpiece needs a conductor🌟✨
# ═══════════════════════════════════════════════════════════════════════════════════════════════

EOF

    # Animated welcome
    echo -e "\n${BM}    Welcome, beautiful human!${NC} Let's make magic happen ✨"

    # Feature highlights with icons (properly aligned - 68 char inner width)
    echo -e "\n${C}  ╭──────────────────────────────────────────────────────────────────╮${NC}"
    echo -e "${C}  │${NC} ${BW}You're about to experience:${NC}                                      ${C}│${NC}"
    echo -e "${C}  ├──────────────────────────────────────────────────────────────────┤${NC}"
    echo -e "${C}  │${NC}  🎯 Smart Track Management        🤖 AI Agent Delegation          ${C}│${NC}"
    echo -e "${C}  │${NC}  📋 Auto-Generated Plans          🧠 Critical Think Framework     ${C}│${NC}"
    echo -e "${C}  │${NC}  🔍 Code Search (Zoekt)           💾 Memory Nexus System          ${C}│${NC}"
    echo -e "${C}  │${NC}  🎨 100+ Skills                   ⚡ 28 Specialized Agents         ${C}│${NC}"
    echo -e "${C}  ╰──────────────────────────────────────────────────────────────────╯${NC}"
    echo ""

    sleep_anim 0.5
}

# ─────────────────────────────────────────────────────────────────────────────
# 🔍 SYSTEM DETECTION
# ─────────────────────────────────────────────────────────────────────────────

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

command_exists() {
    command -v "$1" &> /dev/null
}

# Check if a binary exists at a specific path
binary_exists() {
    local binary="$1"
    [[ -f "$binary" && -x "$binary" ]]
}

# Find binary in common Go install locations
find_go_binary() {
    local binary_name="$1"
    local locations=(
        "$HOME/go/bin/$binary_name"
        "$(go env GOPATH 2>/dev/null)/bin/$binary_name"
        "$HOME/.local/bin/$binary_name"
        "/usr/local/bin/$binary_name"
    )

    for loc in "${locations[@]}"; do
        if binary_exists "$loc"; then
            echo "$loc"
            return 0
        fi
    done
    return 1
}

# ─────────────────────────────────────────────────────────────────────────────
# 🛠️ CLI TOOL CONFIGURATION
# ─────────────────────────────────────────────────────────────────────────────

# Global Maestro home for shared binaries and state
MAESTRO_HOME="$HOME/.maestro"
mkdir -p "$MAESTRO_HOME"

get_tool_config_dir() {
    local tool="$1"
    case "$tool" in
        claude)   echo "$HOME/.claude" ;;
        amp)      echo "$HOME/.amp" ;; # Sourcegraph Amp
        opencode) echo "$HOME/.opencode" ;;
        gemini)   echo "$HOME/.gemini" ;;
        codex)    echo "$HOME/.codex" ;;
        *)        echo "$HOME/.claude" ;;
    esac
}

install_go() {
    local os=$(detect_os)
    print_header "📦 Installing Go" "📦"

    case $os in
        ubuntu|debian|linuxmint|pop)
            print_info "Detected: ${BY}Debian-based system${NC}"
            print_step "1" "3" "Updating package cache..."
            if sudo apt-get update -qq > /dev/null 2>&1; then
                print_success "Package cache updated"
            else
                print_warning "Some packages may be outdated, continuing..."
            fi

            print_step "2" "3" "Installing Go..."
            if sudo apt-get install -y golang-go > /dev/null 2>&1; then
                print_success "Go installed successfully"
            else
                print_error "Go installation failed"
                return 1
            fi
            ;;
        fedora|rhel|centos)
            print_info "Detected: ${BY}RedHat-based system${NC}"
            if sudo dnf install -y golang > /dev/null 2>&1; then
                print_success "Go installed successfully"
            else
                print_error "Go installation failed"
                return 1
            fi
            ;;
        macos)
            print_info "Detected: ${BY}macOS${NC}"
            if command_exists brew; then
                print_step "1" "2" "Installing Go via Homebrew..."
                if brew install go > /dev/null 2>&1; then
                    print_success "Go installed successfully"
                else
                    print_error "Go installation failed"
                    return 1
                fi
            else
                print_error "Homebrew not found"
                echo -e "\n${Y}  💡 Install Homebrew first:${NC}"
                echo "     /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                return 1
            fi
            ;;
        *)
            print_error "Unsupported OS for auto-install"
            echo -e "\n${Y}  💡 Install Go manually:${NC} https://golang.org/dl/"
            return 1
            ;;
    esac

    print_step "3" "3" "Verifying installation..."
    if command_exists go; then
        local go_version=$(go version)
        print_success "Go verified: ${go_version}"
        return 0
    else
        print_error "Go verification failed"
        return 1
    fi
}

install_zoekt() {
    print_header "🔍 Installing Zoekt Code Search" "🔍"

    print_info "Zoekt provides blazing-fast code search for the Memory System"
    echo ""

    print_step "1" "4" "Checking Go installation..."
    if ! command_exists go; then
        print_error "Go is required for Zoekt installation"
        if install_go; then
            print_success "Go installed successfully"
        else
            return 1
        fi
    else
        print_success "Go is available"
    fi

    print_step "2" "4" "Setting up Go environment..."
    GOPATH=$(go env GOPATH)
    print_info "GOPATH: ${Y}${GOPATH}${NC}"

    # Add GOPATH/bin to PATH automatically
    local gopath_bin="$GOPATH/bin"
    if [[ ":$PATH:" != *":$gopath_bin:"* ]]; then
        add_to_path "$gopath_bin"
        echo ""
    fi

    print_step "3" "4" "Installing zoekt-webserver..."
    echo -e "${C}  →${NC} This may take a minute or two..."

    # Install with better error handling and progress indication
    {
        go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest 2>&1 | while IFS= read -r line; do
            if [[ "$line" =~ "go:" ]]; then
                printf "\r${C}  →${NC} Building: ${line:0:55}...      " 2>/dev/null
            fi
        done
    } &
    local web_pid=$!

    {
        go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@latest 2>&1 | while IFS= read -r line; do
            if [[ "$line" =~ "go:" ]]; then
                printf "\r${C}  →${NC} Building: ${line:0:55}...      " 2>/dev/null
            fi
        done
    } &
    local index_pid=$!

    # Wait for both installations
    local web_result=0
    local index_result=0

    wait $web_pid || web_result=$?
    wait $index_pid || index_result=$?

    if [ $web_result -eq 0 ] && [ $index_result -eq 0 ]; then
        print_success "Zoekt components installed!"

        print_step "4" "4" "Verifying installation..."

        # Get GOPATH
        local gopath_bin="$(go env GOPATH 2>/dev/null)/bin"
        [[ ! -d "$gopath_bin" ]] && gopath_bin="$HOME/go/bin"

        # Check if binaries exist at their install location
        local webserver_bin="$gopath_bin/zoekt-webserver"
        local indexer_bin="$gopath_bin/zoekt-git-index"

        local webserver_ok=false
        local indexer_ok=false

        if [[ -f "$webserver_bin" && -x "$webserver_bin" ]]; then
            webserver_ok=true
        else
            print_warning "zoekt-webserver not found at: ${webserver_bin}"
        fi

        if [[ -f "$indexer_bin" && -x "$indexer_bin" ]]; then
            indexer_ok=true
        else
            print_warning "zoekt-git-index not found at: ${indexer_bin}"
        fi

        if $webserver_ok && $indexer_ok; then
            # Binaries exist, now check if they're in PATH
            if command_exists zoekt-webserver && command_exists zoekt-indexer; then
                print_success "zoekt-webserver: ${Y}✓${NC}  zoekt-indexer: ${Y}✓${NC}"
                echo ""
                echo -e "${G}  🎉 Zoekt is ready to power your code searches!${NC}"
                return 0
            else
                # Create symlinks to ~/.local/bin for immediate access
                print_info "Creating symlinks to ~/.local/bin for immediate access..."
                mkdir -p ~/.local/bin

                ln -sf "$webserver_bin" ~/.local/bin/zoekt-webserver 2>/dev/null
                ln -sf "$indexer_bin" ~/.local/bin/zoekt-indexer 2>/dev/null

                # Export to current session
                export PATH="$HOME/.local/bin:$PATH"

                print_success "zoekt-webserver: ${Y}✓${NC}  zoekt-indexer: ${Y}✓${NC}"
                echo ""
                echo -e "${G}  🎉 Zoekt is ready to power your code searches!${NC}"
                echo -e "${C}  →${NC} Symlinks created in ~/.local/bin"
                echo -e "${C}  →${NC} Added to PATH for this session"
                return 0
            fi
        else
            print_error "Zoekt binaries not found after installation"
            return 1
        fi
    else
        print_error "Zoekt installation encountered issues"
        echo ""
        echo -e "${Y}  💡 Manual installation:${NC}"
        echo "     ${C}go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest${NC}"
        echo "     ${C}go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@latest${NC}"
        return 1
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# 📦 TMUX, YAZI & EDITOR INSTALLATION
# ─────────────────────────────────────────────────────────────────────────────

install_tmux() {
    local os=$(detect_os)
    print_header "📺 Installing tmux" "📺"
    
    if command_exists tmux; then
        print_success "tmux already installed: $(tmux -V)"
        return 0
    fi
    
    print_info "tmux provides the terminal multiplexer for Maestro TUI"
    
    local install_success=false
    case $os in
        ubuntu|debian|linuxmint|pop)
            print_step "1" "1" "Installing via apt..."
            if sudo apt-get install -y tmux > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
        fedora|rhel|centos)
            print_step "1" "1" "Installing via dnf..."
            if sudo dnf install -y tmux > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
        arch)
            print_step "1" "1" "Installing via pacman..."
            if sudo pacman -S --noconfirm tmux > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
        macos)
            print_step "1" "1" "Installing via homebrew..."
            if brew install tmux > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
        *)
            print_warning "Unknown OS, trying apt-get..."
            if sudo apt-get install -y tmux > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
    esac
    
    if [ "$install_success" = true ] || command_exists tmux; then
        print_success "tmux installed: $(tmux -V 2>/dev/null || echo 'ready')"
    else
        print_warning "tmux installation encountered issues"
        print_info "Manual installation recommended for TUI: https://github.com/tmux/tmux/wiki/Installing"
    fi
    return 0
}


install_yazi() {
    local os=$(detect_os)
    print_header "📁 Installing Yazi File Manager" "📁"
    
    if command_exists yazi; then
        print_success "Yazi already installed"
        return 0
    fi
    
    print_info "Yazi provides the file picker for the IDE layout"
    
    local install_success=false
    case $os in
        arch)
            if sudo pacman -S --noconfirm yazi > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
        macos)
            if brew install yazi > /dev/null 2>&1; then
                install_success=true
            fi
            ;;
        *)
            if command_exists cargo; then
                print_step "1" "1" "Installing via cargo (may take a few minutes)..."
                if cargo install --locked yazi-fm yazi-cli > /dev/null 2>&1; then
                    install_success=true
                fi
            fi
            
            if [ "$install_success" = false ]; then
                # Fallback to prebuilt
                print_step "1" "1" "Downloading prebuilt binary..."
                if curl -Lo /tmp/yazi.zip "https://github.com/sxyazi/yazi/releases/latest/download/yazi-x86_64-unknown-linux-gnu.zip" 2>/dev/null; then
                    if unzip -o /tmp/yazi.zip -d /tmp/yazi > /dev/null 2>&1; then
                        mkdir -p ~/.local/bin
                        if mv /tmp/yazi/yazi-x86_64-unknown-linux-gnu/yazi ~/.local/bin/ 2>/dev/null || mv /tmp/yazi/yazi ~/.local/bin/ 2>/dev/null; then
                            chmod +x ~/.local/bin/yazi
                            install_success=true
                        fi
                    fi
                fi
            fi
            ;;
    esac
    
    if [ "$install_success" = true ] || command_exists yazi; then
        print_success "Yazi installed"
    else
        print_warning "Yazi installation failed"
        print_info "Continuing without Yazi (IDE file picker will be limited)"
    fi
    return 0
}

select_editor() {
    print_header "✏️ Editor Selection" "✏️"
    
    echo -e "${BW}  Select your preferred editor for Maestro TUI:${NC}"
    echo ""
    echo -e "  ${G}1)${NC} ${BW}fresh${NC}    ${W}(recommended - AI-optimized, single command install)${NC}"
    echo -e "  ${G}2)${NC} ${BW}helix${NC}    ${W}(modern modal editor with built-in LSP)${NC}"
    echo -e "  ${G}3)${NC} ${BW}vim${NC}      ${W}(classic, usually pre-installed)${NC}"
    echo -e "  ${G}4)${NC} ${BW}neovim${NC}   ${W}(vim-based with modern features)${NC}"
    echo -e "  ${G}5)${NC} ${BW}skip${NC}     ${W}(keep current \$EDITOR)${NC}"
    echo ""
    read -p "  Enter choice [1-5] (default: 1): " editor_choice
    
    case "${editor_choice:-1}" in
        1) install_fresh ;;
        2) install_helix ;;
        3) print_success "Using vim as editor"; export EDITOR=vim; add_editor_to_rc "vim" ;;
        4) install_neovim ;;
        5) print_info "Keeping current EDITOR: ${EDITOR:-not set}" ;;
        *) print_warning "Invalid choice, using fresh"; install_fresh ;;
    esac
}

install_fresh() {
    print_step "1" "1" "Installing fresh editor..."
    curl -sSL https://raw.githubusercontent.com/sinelaw/fresh/refs/heads/master/scripts/install.sh | sh
    print_success "fresh installed"
    export EDITOR=fresh
    add_editor_to_rc "fresh"
}

install_helix() {
    local os=$(detect_os)
    print_step "1" "1" "Installing helix editor..."
    
    local installed=false
    
    case $os in
        ubuntu|debian|linuxmint|pop|debian-*)
            # Try APT first
            sudo apt-get update -qq > /dev/null 2>&1 || true
            if sudo apt-get install -y helix > /dev/null 2>&1; then
                installed=true
            else
                # Fallback to binary installation as requested
                print_info "Helix not found in APT, installing from GitHub binary..."
                
                # Ensure xz-utils and curl are available
                sudo apt-get install -y xz-utils curl > /dev/null 2>&1 || true
                
                local helix_ver=$(curl -s https://api.github.com/repos/helix-editor/helix/releases/latest | grep -oP '"tag_name": "\K[^"]+')
                local helix_url="https://github.com/helix-editor/helix/releases/download/${helix_ver}/helix-${helix_ver}-x86_64-linux.tar.xz"
                
                print_info "Downloading Helix ${helix_ver}..."
                curl -Lo /tmp/helix.tar.xz "$helix_url"
                
                rm -rf /tmp/helix_extract
                mkdir -p /tmp/helix_extract
                tar -xf /tmp/helix.tar.xz -C /tmp/helix_extract
                
                local extract_dir=$(ls -d /tmp/helix_extract/helix-*)
                mkdir -p ~/.local/bin
                cp "$extract_dir/hx" ~/.local/bin/
                chmod +x ~/.local/bin/hx
                
                # Setup runtime
                mkdir -p ~/.config/helix
                rm -rf ~/.config/helix/runtime
                cp -r "$extract_dir/runtime" ~/.config/helix/
                
                # Check for health
                if ~/.local/bin/hx --version > /dev/null 2>&1; then
                    installed=true
                fi
            fi
            ;;
        fedora|rhel|centos)
            sudo dnf install -y helix > /dev/null 2>&1 && installed=true
            ;;
        arch)
            sudo pacman -S --noconfirm helix > /dev/null 2>&1 && installed=true
            ;;
        macos)
            brew install helix > /dev/null 2>&1 && installed=true
            ;;
        *)
            if command_exists snap; then
                sudo snap install --classic helix > /dev/null 2>&1 && installed=true
            fi
            ;;
    esac
    
    if [ "$installed" = true ] || command_exists hx || command_exists helix; then
        print_success "helix installed"
        # Determine command name
        if command_exists hx; then export EDITOR=hx; else export EDITOR=helix; fi
        add_editor_to_rc "$EDITOR"
    else
        print_error "Could not install Helix"
        exit 1
    fi
}

install_neovim() {
    local os=$(detect_os)
    print_step "1" "1" "Installing neovim..."
    
    case $os in
        ubuntu|debian|linuxmint|pop)
            sudo apt-get install -y neovim
            ;;
        fedora|rhel|centos)
            sudo dnf install -y neovim
            ;;
        arch)
            sudo pacman -S --noconfirm neovim
            ;;
        macos)
            brew install neovim
            ;;
        *)
            print_error "Cannot auto-install neovim on this OS"
            exit 1
            ;;
    esac
    
    print_success "neovim installed"
    export EDITOR=nvim
    add_editor_to_rc "nvim"
}

add_editor_to_rc() {
    local editor="$1"
    
    # Always add to both bashrc and zshrc if they exist
    for shell_rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
        if [[ -f "$shell_rc" ]]; then
            # Remove any existing Maestro EDITOR entry
            sed -i '/# Maestro EDITOR/d' "$shell_rc" 2>/dev/null
            sed -i '/export EDITOR=.*# set by Maestro/d' "$shell_rc" 2>/dev/null
            
            # Append the new one at the end to ensure it takes precedence
            echo "" >> "$shell_rc"
            echo "# Maestro EDITOR" >> "$shell_rc"
            echo "export EDITOR=$editor # set by Maestro" >> "$shell_rc"
            echo -e "${C}  →${NC} Configured EDITOR=$editor in ${Y}$shell_rc${NC}"
        fi
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# 🧹 BACKUP & RESTORE
# ─────────────────────────────────────────────────────────────────────────────


backup_config() {
    local config_dir="$HOME/.claude"
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_file="$HOME/.claude.backup.${timestamp}.tar.gz"

    if [ -d "$config_dir" ]; then
        print_header "📦 Creating Backup" "💾"
        print_info "Backing up your existing Claude Code configuration..."

        local size=$(du -sh "$config_dir" 2>/dev/null | cut -f1)
        echo -e "\n${C}  →${NC} Configuration size: ${Y}${size}${NC}"
        echo -e "${C}  →${NC} Creating backup archive (excluding projects/tmp/debug/cache)..."
        echo -e "${C}  →${NC} Note: Only backing up modified config files..."

        # Create backup using rsync for better exclude handling, then tar
        local temp_backup_dir=$(mktemp -d)
        mkdir -p "$temp_backup_dir/.claude"

        # Use rsync to copy only what we need (excludes work better with rsync)
        if command -v rsync &> /dev/null; then
            rsync -av --delete \
                --exclude="tmp" \
                --exclude="debug" \
                --exclude="file-history" \
                --exclude="projects" \
                --exclude="cache" \
                --exclude="sessions" \
                --exclude="logs" \
                --exclude=".state" \
                --exclude="*.pyc" \
                "$config_dir/" "$temp_backup_dir/.claude/" > /dev/null 2>&1
        else
            # Fallback without rsync
            cp -R "$config_dir/commands" "$temp_backup_dir/.claude/" 2>/dev/null || true
            cp -R "$config_dir/maestro-templates" "$temp_backup_dir/.claude/" 2>/dev/null || true
            cp -R "$config_dir/plugins" "$temp_backup_dir/.claude/" 2>/dev/null || true
        fi

        # Now tar the filtered content
        if tar -czf "$backup_file" -C "$temp_backup_dir" ".claude" 2>/dev/null; then
            local backup_size=$(du -sh "$backup_file" 2>/dev/null | cut -f1)
            print_success "Backup created: ${backup_file}"
            print_info "Compressed size: ${Y}${backup_size}${NC}"
            echo "$backup_file" > /tmp/maestro_last_backup
        else
            print_error "Backup creation failed"
            rm -rf "$temp_backup_dir"
            return 1
        fi

        # Cleanup temp dir
        rm -rf "$temp_backup_dir"
    fi
}

restore_config() {
    local backup_file="$1"
    local config_dir="$HOME/.claude"

    print_header "🔄 Restoring Backup" "🔄"

    if [ -f "$backup_file" ]; then
        print_info "Restoring from: ${Y}${backup_file}${NC}"

        if [ -d "$config_dir" ]; then
            local old_backup="${config_dir}.pre-restore.$(date +%Y%m%d_%H%M%S)"
            mv "$config_dir" "$old_backup"
            print_info "Backed up current config to: ${Y}${old_backup}${NC}"
        fi

        if tar -xzf "$backup_file" -C "$HOME"; then
            print_success "Restore complete!"
        else
            print_error "Restore failed"
            return 1
        fi
    else
        print_error "Backup file not found: ${backup_file}"
        return 1
    fi
}

# ─────────────────────────────────────────────────────────────────────────────
# 🎛️ CLI TOOL SELECTION  
# ─────────────────────────────────────────────────────────────────────────────

SELECTED_TOOLS=()

select_cli_tools() {
    print_header "🎛️ Select CLI Tools" "🎛️"
    
    echo -e "${BW}  Which CLI tools would you like to configure Maestro for?${NC}"
    echo ""
    echo -e "  ${G}1)${NC} ${BW}Claude Code${NC}      ${W}(Anthropic's AI coding assistant)${NC}"
    echo -e "  ${G}2)${NC} ${BW}OpenCode${NC}         ${W}(Open-source AI coding tool)${NC}"
    echo -e "  ${G}3)${NC} ${BW}Gemini CLI${NC}       ${W}(Google's Gemini for terminal)${NC}"
    echo -e "  ${G}4)${NC} ${BW}Codex${NC}            ${W}(OpenAI Codex CLI)${NC}"
    echo -e "  ${G}5)${NC} ${BW}Sourcegraph Amp${NC}  ${W}(Sourcegraph's agentic CLI)${NC}"
    echo -e "  ${G}6)${NC} ${BW}All${NC}              ${W}(Configure for all tools)${NC}"
    echo ""
    echo -e "  ${Y}Enter comma-separated numbers (e.g., 1,2,3) or 6 for all:${NC}"
    read -p "  > " tool_choice
    
    if [[ "$tool_choice" == "6" ]] || [[ "$tool_choice" == "all" ]]; then
        SELECTED_TOOLS=("claude" "opencode" "gemini" "codex" "amp")
        print_success "Installing for ALL tools"
    else
        IFS=',' read -ra choices <<< "$tool_choice"
        for choice in "${choices[@]}"; do
            choice=$(echo "$choice" | tr -d ' ')
            case "$choice" in
                1) SELECTED_TOOLS+=("claude") ;;
                2) SELECTED_TOOLS+=("opencode") ;;
                3) SELECTED_TOOLS+=("gemini") ;;
                4) SELECTED_TOOLS+=("codex") ;;
                5) SELECTED_TOOLS+=("amp") ;;
            esac
        done
    fi
    
    if [ ${#SELECTED_TOOLS[@]} -eq 0 ]; then
        print_warning "No tools selected, defaulting to Claude Code"
        SELECTED_TOOLS=("claude")
    fi
    
    echo ""
    print_info "Selected tools: ${Y}${SELECTED_TOOLS[*]}${NC}"
    echo ""
}

# ─────────────────────────────────────────────────────────────────────────────
# 🗑️ UNINSTALL FUNCTION
# ─────────────────────────────────────────────────────────────────────────────

uninstall_maestro() {
    clear_screen
    echo -e "${BR}"
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║                    🗑️  MAESTRO UNINSTALLER                       ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    echo -e "${BY}  This will remove Maestro and all its components.${NC}"
    echo ""
    echo -e "${W}  The following will be removed:${NC}"
    echo -e "    ${C}→${NC} ~/.local/bin/maestro"
    echo -e "    ${C}→${NC} ~/.local/bin/maestro-tui"
    echo -e "    ${C}→${NC} ~/.claude/plugins/maestro/"
    echo -e "    ${C}→${NC} ~/.claude/maestro-templates/"
    echo -e "    ${C}→${NC} ~/.claude/commands/maestro*.md"
    echo ""
    echo -e "${Y}  Continue with uninstall? [y/N] ${NC}"
    read -r response
    
    if [[ ! $response =~ ^[Yy]$ ]]; then
        print_info "Uninstall cancelled."
        exit 0
    fi
    
    echo ""
    print_header "🗑️ Removing Maestro Components" "🗑️"
    
    local removed=0
    
    # Remove binaries and global home
    if [[ -f "$HOME/.local/bin/maestro" ]]; then
        rm -f "$HOME/.local/bin/maestro"
        print_success "Removed ~/.local/bin/maestro"
        ((removed++))
    fi
    
    if [[ -f "$HOME/.local/bin/maestro-tui" ]]; then
        rm -f "$HOME/.local/bin/maestro-tui"
        print_success "Removed ~/.local/bin/maestro-tui"
        ((removed++))
    fi

    if [[ -d "$MAESTRO_HOME" ]]; then
        rm -rf "$MAESTRO_HOME"
        print_success "Removed $MAESTRO_HOME"
        ((removed++))
    fi
    
    # Remove from all possible tool directories
    local tools=("claude" "amp" "opencode" "gemini" "codex")
    for tool in "${tools[@]}"; do
        local tool_dir=$(get_tool_config_dir "$tool")
        if [[ -d "$tool_dir" ]]; then
            # Plugins
            if [[ -d "$tool_dir/plugins/maestro" ]]; then
                rm -rf "$tool_dir/plugins/maestro"
                print_success "Removed $tool_dir/plugins/maestro/"
                ((removed++))
            fi
            # Templates
            if [[ -d "$tool_dir/maestro-templates" ]]; then
                rm -rf "$tool_dir/maestro-templates"
                print_success "Removed $tool_dir/maestro-templates/"
                ((removed++))
            fi
            # Commands
            local cmd_count=$(ls "$tool_dir/commands/maestro"*.md 2>/dev/null | wc -l)
            if [[ $cmd_count -gt 0 ]]; then
                rm -f "$tool_dir/commands/maestro"*.md
                print_success "Removed $cmd_count files from $tool_dir/commands/"
                ((removed++))
            fi
        fi
    done
    
    echo ""
    if [[ $removed -gt 0 ]]; then
        echo -e "${BG}╔══════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${BG}║              ✅ MAESTRO UNINSTALLED SUCCESSFULLY                 ║${NC}"
        echo -e "${BG}╚══════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        print_info "Note: PATH/EDITOR entries in ~/.bashrc and ~/.zshrc were not removed."
        print_info "You can manually remove lines starting with '# Maestro' if desired."
    else
        print_warning "No Maestro components were found to remove."
    fi
    echo ""
    
    exit 0
}

# ─────────────────────────────────────────────────────────────────────────────
# ⭐ GITHUB STAR PROMPT
# ─────────────────────────────────────────────────────────────────────────────

show_star_prompt() {
    echo ""
    echo -e "${M}  ╭──────────────────────────────────────────────────────────────────╮${NC}"
    echo -e "${M}  │${NC}  ${BY}⭐ Enjoying Maestro?${NC}                                            ${M}│${NC}"
    echo -e "${M}  │${NC}                                                                    ${M}│${NC}"
    echo -e "${M}  │${NC}  If Maestro helped your workflow, consider starring the repo!     ${M}│${NC}"
    echo -e "${M}  │${NC}  It helps others discover the project and motivates development.  ${M}│${NC}"
    echo -e "${M}  ╰──────────────────────────────────────────────────────────────────╯${NC}"
    echo ""
    read -p "  Would you like to star the Maestro repo now? [y/N] " response
    
    if [[ $response =~ ^[Yy]$ ]]; then
        if command -v gh &> /dev/null; then
            echo ""
            # Use gh api for more reliable starring
            local star_result
            star_result=$(gh api -X PUT /user/starred/scooter-lacroix/Maestro 2>&1)
            local star_status=$?
            
            if [[ $star_status -eq 0 ]]; then
                print_success "Thank you for starring Maestro! 🌟"
                echo -e "  ${C}→${NC} You're now part of the Maestro community!"
            else
                # Check if already starred (204 is success, empty response)
                if gh api /user/starred/scooter-lacroix/Maestro 2>/dev/null; then
                    print_success "You've already starred Maestro! 🌟"
                else
                    print_warning "Could not star the repo automatically."
                    print_info "You may need to run: ${C}gh auth refresh -s read:user${NC}"
                    print_info "Or star manually: ${C}https://github.com/scooter-lacroix/Maestro${NC}"
                    
                    # Offer browser fallback
                    if command -v xdg-open &> /dev/null; then
                        echo -ne "  ${W}Open in browser to star? [y/N]${NC} "
                        read -r open_response
                        if [[ $open_response =~ ^[Yy]$ ]]; then
                            xdg-open "https://github.com/scooter-lacroix/Maestro" 2>/dev/null &
                        fi
                    fi
                fi
            fi
        else
            echo ""
            print_info "GitHub CLI (gh) not installed."
            print_info "Star the repo here: ${C}https://github.com/scooter-lacroix/Maestro${NC}"
            
            # Offer to open in browser
            if command -v xdg-open &> /dev/null; then
                echo -ne "  ${W}Open in browser? [y/N]${NC} "
                read -r open_response
                if [[ $open_response =~ ^[Yy]$ ]]; then
                    xdg-open "https://github.com/scooter-lacroix/Maestro" 2>/dev/null &
                fi
            fi
        fi
    fi
    echo ""

}

# ─────────────────────────────────────────────────────────────────────────────
# ❓ HELP FUNCTION
# ─────────────────────────────────────────────────────────────────────────────

show_help() {
    echo -e "${BC}Maestro Installer${NC} - Your AI-Powered Project Orchestrator"
    echo ""
    echo -e "${BW}Usage:${NC}"
    echo "  ./install.sh [OPTIONS]"
    echo ""
    echo -e "${BW}Options:${NC}"
    echo -e "  ${C}(no flags)${NC}       Fresh install of Maestro"
    echo -e "  ${C}--upgrade${NC}        Upgrade existing installation"
    echo -e "  ${C}--uninstall${NC}      Remove Maestro completely"
    echo -e "  ${C}--restore FILE${NC}   Restore from a backup file"
    echo -e "  ${C}--help${NC}           Show this help message"
    echo ""
    echo -e "${BW}Examples:${NC}"
    echo "  ./install.sh                    # Fresh install"
    echo "  ./install.sh --upgrade          # Upgrade keeping config"
    echo "  ./install.sh --uninstall        # Complete removal"
    echo "  ./install.sh --restore ~/.claude.backup.tar.gz"
    echo ""
    exit 0
}

# ─────────────────────────────────────────────────────────────────────────────
# 🎯 MAIN INSTALLATION FLOW
# ─────────────────────────────────────────────────────────────────────────────

main() {
    # Handle flags before showing banner
    case "$1" in
        --help|-h)
            show_help
            ;;
        --uninstall)
            uninstall_maestro
            ;;
        --upgrade)
            export MAESTRO_UPGRADE=true
            ;;
        --restore)
            show_banner
            if [ -n "$2" ]; then
                restore_config "$2"
                exit 0
            else
                print_error "Usage: $0 --restore <backup_file>"
                exit 1
            fi
            ;;
    esac
    
    show_banner
    
    # Show upgrade notice if upgrading
    if [[ "$MAESTRO_UPGRADE" == "true" ]]; then
        echo -e "${BY}  📦 UPGRADE MODE${NC} - Updating existing installation...\n"
    fi

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 0: CLI TOOL SELECTION
    # ─────────────────────────────────────────────────────────────────────
    
    select_cli_tools

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 1: DEPENDENCY CHECK
    # ─────────────────────────────────────────────────────────────────────

    print_header "🔍 Dependency Check" "🔍"

    local go_installed=false
    local zoekt_installed=false

    # Check Go
    if command_exists go; then
        local go_version=$(go version)
        print_success "Go found: ${go_version}"
        go_installed=true
    else
        print_warning "Go not found"
        echo -e "\n${BM}  Go is recommended for Zoekt code search${NC}"
        echo -e "${W}  Would you like to install Go now? [y/N] ${NC}"
        read -r response
        echo
        if [[ $response =~ ^[Yy]$ ]]; then
            if install_go; then
                go_installed=true
            fi
        else
            print_info "Skipping Go installation (Zoekt requires Go)"
        fi
    fi

    # Check Zoekt - also check in Go install locations
    echo
    local zoekt_webserver_found=false
    local zoekt_indexer_found=false

    if command_exists zoekt-webserver; then
        zoekt_webserver_found=true
    elif [[ -f "$HOME/go/bin/zoekt-webserver" ]]; then
        zoekt_webserver_found=true
    elif command_exists go && [[ -f "$(go env GOPATH 2>/dev/null)/bin/zoekt-webserver" ]]; then
        zoekt_webserver_found=true
    fi

    if command_exists zoekt-indexer; then
        zoekt_indexer_found=true
    elif [[ -f "$HOME/go/bin/zoekt-indexer" ]]; then
        zoekt_indexer_found=true
    elif command_exists go && [[ -f "$(go env GOPATH 2>/dev/null)/bin/zoekt-indexer" ]]; then
        zoekt_indexer_found=true
    fi

    if $zoekt_webserver_found && $zoekt_indexer_found; then
        print_success "Zoekt found: ${Y}ready for code search${NC}"
        zoekt_installed=true
    else
        print_warning "Zoekt not found"
        echo -e "\n${BM}  Zoekt provides lightning-fast code search for Maestro's Memory System${NC}"
        echo -e "${W}  Would you like to install Zoekt now? [y/N] ${NC}"
        read -r response
        echo
        if [[ $response =~ ^[Yy]$ ]]; then
            if install_zoekt; then
                zoekt_installed=true
            fi
        else
            print_info "Skipping Zoekt - Memory System will use fallback search mode"
        fi
    fi

    # Check Rust
    echo
    local rust_installed=false
    if command_exists cargo; then
        local rust_version=$(cargo --version)
        print_success "Rust found: ${rust_version}"
        rust_installed=true
    else
        print_warning "Rust not found (required for Maestro v2 Core)"
        echo -e "\n${BM}  Maestro v2 requires Rust to build the high-performance core${NC}"
        echo -e "${W}  Would you like to install Rust now? [y/N] ${NC}"
        read -r response
        echo
        if [[ $response =~ ^[Yy]$ ]]; then
            if curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; then
                source "$HOME/.cargo/env"
                rust_installed=true
                print_success "Rust installed successfully"
            fi
        fi
    fi

    # Check tmux
    echo
    if command_exists tmux; then
        print_success "tmux found: $(tmux -V)"
    else
        print_warning "tmux not found (required for TUI)"
        echo -e "\n${BM}  tmux provides the terminal multiplexer for Maestro TUI${NC}"
        echo -e "${W}  Would you like to install tmux now? [Y/n] ${NC}"
        read -r response
        echo
        if [[ ! $response =~ ^[Nn]$ ]]; then
            install_tmux
        fi
    fi

    # Check Yazi
    echo
    if command_exists yazi; then
        print_success "Yazi found (file picker)"
    else
        print_warning "Yazi not found (required for IDE file picker)"
        echo -e "\n${BM}  Yazi provides the file picker for the IDE layout${NC}"
        echo -e "${W}  Would you like to install Yazi now? [Y/n] ${NC}"
        read -r response
        echo
        if [[ ! $response =~ ^[Nn]$ ]]; then
            install_yazi
        fi
    fi

    # Editor selection
    echo
    echo -e "${W}  Would you like to configure your preferred editor? [y/N] ${NC}"
    read -r response
    if [[ $response =~ ^[Yy]$ ]]; then
        select_editor
    fi

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 2: BACKUP
    # ─────────────────────────────────────────────────────────────────────

    backup_config

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 3: DOWNLOAD & INSTALL
    # ─────────────────────────────────────────────────────────────────────

    print_header "📥 Downloading Maestro" "⬇️"

    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR; cursor_show" EXIT

    print_info "Fetching Maestro v2.0.0 from GitHub..."

    REPO_URL="https://github.com/scooter-lacroix/Maestro"
    REPO_BRANCH="v2"

    if command -v git &> /dev/null; then
        echo -e "${C}  →${NC} Using ${BW}git${NC} to download..."

        {
            git clone -q --depth 1 --branch "$REPO_BRANCH" "$REPO_URL" "$TMP_DIR" 2>&1
        } &
        local git_pid=$!

        local spin_i=0
        while kill -0 $git_pid 2>/dev/null; do
            printf "\r${M}  ${music_frames[$spin_i]}${NC} Cloning repository...  " 2>/dev/null
            spin_i=$(( (spin_i + 1) % ${#music_frames[@]} ))
            sleep 0.15
        done
        wait $git_pid || {
            print_warning "git clone failed, trying download method..."
            if command -v curl &> /dev/null; then
                curl -sSL "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
            elif command -v wget &> /dev/null; then
                wget -qO- "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
            else
                print_error "Neither git nor curl/wget is available"
                exit 1
            fi
        }
        echo -e "\r${G}  ✓${NC} Repository cloned!                   "
    else
        echo -e "${C}  →${NC} Downloading via ${BW}curl${NC}..."
        if command -v curl &> /dev/null; then
            curl -sSL "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
        elif command -v wget &> /dev/null; then
            wget -qO- "$REPO_URL/archive/$REPO_BRANCH.tar.gz" | tar -xz -C "$TMP_DIR" --strip-components=1
        else
            print_error "Neither curl nor wget is available"
            exit 1
        fi
        print_success "Download complete!"
    fi

    SCRIPT_DIR="$TMP_DIR"

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 4: INSTALL COMPONENTS SPREAD ACROSS TOOLS
    # ─────────────────────────────────────────────────────────────────────

    local total_tools=${#SELECTED_TOOLS[@]}
    local current_tool_idx=0

    # Define install_component outside the tool loop to avoid re-declaration syntax issues
    install_component() {
        local name="$1"
        local icon="$2"
        local action="$3"
        local comp_idx="$4"
        local total_comp="$5"

        printf "\r${BM}  [$comp_idx/$total_comp]${NC} ${icon}  ${BW}${name}...${NC}   " 2>/dev/null

        if eval "$action" > /dev/null 2>&1; then
            printf "\r${G}  ✓ [$comp_idx/$total_comp]${NC} ${icon}  ${BW}${name}${NC}      \n" 2>/dev/null
        else
            printf "\r${Y}  ⚠ [$comp_idx/$total_comp]${NC} ${icon}  ${BW}${name} (skipped)${NC}      \n" 2>/dev/null
        fi
    }

    for tool in "${SELECTED_TOOLS[@]}"; do
        current_tool_idx=$((current_tool_idx + 1))
        local target_dir=$(get_tool_config_dir "$tool")
        
        print_header "🔧 Configuring for tool: ${Y}${tool}${NC} (${target_dir})" "⚙️"
        
        local component=0
        local total_components=8

        # Ensure tool config directory exists
        mkdir -p "$target_dir"

        # Commands
        component=$((component + 1))
        install_component "Commands" "📋" "mkdir -p '$target_dir/commands' && /bin/cp '$SCRIPT_DIR/claude-code/commands/maestro'*.md '$target_dir/commands/'" "$component" "$total_components"

        # Templates
        component=$((component + 1))
        install_component "Templates" "📝" "mkdir -p '$target_dir/maestro-templates' && /bin/cp '$SCRIPT_DIR/claude-code/templates/workflow.md' '$target_dir/maestro-templates/' && mkdir -p '$target_dir/maestro-templates/code_styleguides' && /bin/cp '$SCRIPT_DIR/claude-code/templates/code_styleguides/'*.md '$target_dir/maestro-templates/code_styleguides/'" "$component" "$total_components"

        # Plugin
        component=$((component + 1))
        install_component "Plugin" "🔌" "mkdir -p '$target_dir/plugins/maestro' && [ -f '$SCRIPT_DIR/plugin.json' ] && /bin/cp '$SCRIPT_DIR/plugin.json' '$target_dir/plugins/maestro/'" "$component" "$total_components"

        # Hooks
        component=$((component + 1))
        install_component "Hooks" "🪝" "mkdir -p '$target_dir/plugins/maestro/hooks' && [ -d '$SCRIPT_DIR/maestro/hooks' ] && /bin/cp -r '$SCRIPT_DIR/maestro/hooks/'* '$target_dir/plugins/maestro/hooks/'" "$component" "$total_components"

        # Skills
        component=$((component + 1))
        install_component "Skills" "🎓" "mkdir -p '$target_dir/plugins/maestro/skills' && [ -d '$SCRIPT_DIR/maestro/skills' ] && /bin/cp -r '$SCRIPT_DIR/maestro/skills/'* '$target_dir/plugins/maestro/skills/'" "$component" "$total_components"

        # Agents
        component=$((component + 1))
        install_component "Agents" "🤖" "mkdir -p '$target_dir/plugins/maestro/agents' && [ -d '$SCRIPT_DIR/maestro/agents' ] && /bin/cp -r '$SCRIPT_DIR/maestro/agents/'* '$target_dir/plugins/maestro/agents/'" "$component" "$total_components"

        # Config
        component=$((component + 1))
        install_component "Config Module" "⚙️" "mkdir -p '$target_dir/plugins/maestro/config' && [ -d '$SCRIPT_DIR/maestro/config' ] && /bin/cp -r '$SCRIPT_DIR/maestro/config/'* '$target_dir/plugins/maestro/config/'" "$component" "$total_components"

        # Critical Think
        component=$((component + 1))
        install_component "Critical Think" "🧠" "mkdir -p '$target_dir/maestro-templates' && [ -d '$SCRIPT_DIR/maestro/critical_think/templates' ] && /bin/cp '$SCRIPT_DIR/maestro/critical_think/templates/'*.md '$target_dir/maestro-templates/'" "$component" "$total_components"
    done

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 5: MAESTRO CORE BINARY & WRAPPERS (Shared ~/.maestro)
    # ─────────────────────────────────────────────────────────────────────

    print_header "🦀 Maestro Core & Global Setup" "🦀"

    # Maestro TUI (Rust)
    if [ "$rust_installed" = true ]; then
        local rust_dir=""
        if [[ -d "$SCRIPT_DIR/maestro/leindex/rust" ]]; then
            rust_dir="$SCRIPT_DIR/maestro/leindex/rust"
        elif [[ -d "$(dirname "$0")/maestro/leindex/rust" ]]; then
            rust_dir="$(dirname "$0")/maestro/leindex/rust"
        fi
        
        if [[ -n "$rust_dir" && -f "$rust_dir/Cargo.toml" ]]; then
            print_step "1" "3" "Building high-performance Rust Core"
            
            (cd "$rust_dir" && cargo build --release)
            
            mkdir -p "$MAESTRO_HOME/bin"
            if [ -f "$rust_dir/target/release/maestro" ]; then
                cp "$rust_dir/target/release/maestro" "$MAESTRO_HOME/bin/maestro-tui"
                print_success "Rust Core built: $MAESTRO_HOME/bin/maestro-tui"
            elif [ -f "$rust_dir/target/release/leindex-analyzers" ]; then
                cp "$rust_dir/target/release/leindex-analyzers" "$MAESTRO_HOME/bin/maestro-tui"
                print_success "Rust Core built: $MAESTRO_HOME/bin/maestro-tui"
            else
                print_error "Rust binary not found after build"
                exit 1
            fi
        else
            print_warning "Rust source not found - Core build skipped"
        fi
    fi

    # Python Support Library
    if [ -d "$SCRIPT_DIR/maestro" ]; then
        print_step "2" "3" "Installing Maestro Legacy Engine..."
        local lib_dir="$MAESTRO_HOME/lib"
        mkdir -p "$lib_dir"
        cp -r "$SCRIPT_DIR/maestro" "$lib_dir/"
        print_success "Legacy Engine installed: $lib_dir"

        # Global CLI wrapper
        mkdir -p "$HOME/.local/bin"
        cat > "$HOME/.local/bin/maestro" << 'WRAPPER_EOF'
#!/bin/bash
MAESTRO_HOME="$HOME/.maestro"
RUST_TUI="$MAESTRO_HOME/bin/maestro-tui"
PYTHON_LIB="$MAESTRO_HOME/lib"

# Ensure EDITOR is available
if [[ -f "$HOME/.bashrc" && -z "$EDITOR" ]]; then
    export EDITOR=$(grep -oP 'export EDITOR=\K[^ ]+' "$HOME/.bashrc" | tail -1)
fi
export EDITOR=${EDITOR:-fresh}

if [ "$1" = "tui" ]; then
    if [ -f "$RUST_TUI" ]; then
        exec "$RUST_TUI" tui "${@:2}"
    else
        echo "Error: maestro-tui binary not found at $RUST_TUI"
        exit 1
    fi
elif [ "$1" = "memory" ]; then
    if [ -f "$RUST_TUI" ]; then
        exec "$RUST_TUI" memory "${@:2}"
    else
        cd "$PYTHON_LIB"
        python3 -m maestro.memory.cli "$@"
    fi
else
    cd "$PYTHON_LIB"
    python3 -m maestro.cli "$@"
fi
WRAPPER_EOF
        chmod +x "$HOME/.local/bin/maestro"
        
        # Also provide maestro-tui symlink for direct access
        ln -sf "$MAESTRO_HOME/bin/maestro-tui" "$HOME/.local/bin/maestro-tui" 2>/dev/null
        
        print_success "Global wrappers created: ~/.local/bin/maestro"

        # Update PATH if needed
        if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
            add_to_path "$HOME/.local/bin"
        fi
    fi

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 6: MCP CONFIGURATION SYNC
    # ─────────────────────────────────────────────────────────────────────

    print_header "🌐 Syncing MCP Configs" "🌐"
    
    for tool in "${SELECTED_TOOLS[@]}"; do
        local tool_dir=$(get_tool_config_dir "$tool")
        local mcp_config="$tool_dir/.mcp.json"
        
        print_info "Configuring MCP for ${tool}..."
        mkdir -p "$tool_dir"

        if [ -f "$mcp_config" ]; then
            cp "$mcp_config" "${mcp_config}.backup.$(date +%Y%m%d_%H%M%S)"
        fi

        cat > "$mcp_config" << 'MCP_EOF'
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "type": "stdio"
    },
    "brave-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "type": "stdio"
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "type": "stdio"
    }
  }
}
MCP_EOF
    done
    print_success "MCP configurations synced across selected tools"

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 7: COMPLETION & SUMMARY
    # ─────────────────────────────────────────────────────────────────────

    sleep_anim 0.3

    cat << 'EOF'

╔══════════════════════════════════════════════════════════════════════════════╗
║                                                                              ║
║                        🎉 INSTALLATION COMPLETE! 🎉                         ║
║                                                                              ║
╚══════════════════════════════════════════════════════════════════════════════╝

EOF

    echo -e "${BM}  🚀 Ready to Compose Your Masterpiece!${NC}"
    echo ""

    echo -e "${C}  ╭─ ${BW}Claude Code Commands${NC} ${C}─────────────────────────────────────────────────────╮${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:setup${NC}      ${W}Initialize Maestro environment${NC}                         ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:newTrack${NC}   ${W}Create new track${NC}                                        ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:implement${NC}  ${W}Implement track tasks${NC}                                   ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:status${NC}     ${W}View project progress${NC}                                   ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:configure${NC}  ${W}Configure Maestro settings${NC}                                 ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:memory${NC}     ${W}Interact with Memory System${NC}                               ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:tui${NC}        ${W}Launch Terminal UI${NC}                                        ${C}│${NC}"
    echo -e "${C}  ╰────────────────────────────────────────────────────────────────────────────╯${NC}"
    echo ""

    echo -e "${M}  ╭─ ${BW}v2 Components Installed${NC} ${M}──────────────────────────────────────────────────╮${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Hooks${NC}      ${C}16${NC} ${W}event-driven automation hooks                             ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Skills${NC}     ${C}100+${NC} ${W}specialized capabilities                                 ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Agents${NC}     ${C}28${NC} ${W}task delegation agents                                      ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Config${NC}     ${W}unified settings management                                     ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Memory${NC}     ${W}persistent context (Nexus) system                               ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}MCP${NC}        ${C}3${NC} ${W}pre-configured servers                                       ${M}│${NC}"
    echo -e "${M}  ╰────────────────────────────────────────────────────────────────────────────╯${NC}"
    echo ""

    if command_exists zoekt-webserver && command_exists zoekt-indexer; then
        echo -e "${B}  ╭─ ${BW}Zoekt Code Search${NC} ${B}────────────────────────────────────────────────────────╮${NC}"
        echo -e "${B}  │${NC} ${G}✓${NC} ${W}Zoekt is installed and ready!${NC}                                            ${B}│${NC}"
        echo -e "${B}  │${NC}                                                                            ${B}│${NC}"
        echo -e "${B}  │${NC} ${W}Start server:${NC}                                                                 ${B}│${NC}"
        echo -e "${B}  │${NC}   ${C}zoekt-webserver -rpc -index ~/.maestro/zoekt_index${NC}                          ${B}│${NC}"
        echo -e "${B}  │${NC}                                                                            ${B}│${NC}"
        echo -e "${B}  │${NC} ${W}Index your code:${NC}                                                               ${B}│${NC}"
        echo -e "${B}  │${NC}   ${C}zoekt-git-index -index ~/.maestro/zoekt_index -repo_name <name> <path>${NC}  ${B}│${NC}"
        echo -e "${B}  ╰────────────────────────────────────────────────────────────────────────────╯${NC}"
        echo ""
    else
        echo -e "${Y}  ╭─ ${BW}Zoekt Code Search${NC} ${Y}────────────────────────────────────────────────────────╮${NC}"
        echo -e "${Y}  │${NC} ${W}Zoekt not installed (optional)${NC}                                              ${Y}│${NC}"
        echo -e "${Y}  │${NC} ${W}Memory System will use fallback search mode${NC}                                 ${Y}│${NC}"
        echo -e "${Y}  │${NC}                                                                            ${Y}│${NC}"
        echo -e "${Y}  │${NC} ${W}Install later:${NC}                                                                ${Y}│${NC}"
        echo -e "${Y}  │${NC}   ${C}go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest${NC}       ${Y}│${NC}"
        echo -e "${Y}  ╰────────────────────────────────────────────────────────────────────────────╯${NC}"
        echo ""
    fi
    echo -e "${G}  ╭──────────────────────────────────────────────────────────────────╮${NC}"
    echo -e "${G}  │${NC} ${BW}Next Steps:${NC}                                                    ${G}│${NC}"
    echo -e "${G}  │${NC}                                                                    ${G}│${NC}"
    echo -e "${G}  │${NC} 1. Open a new terminal (or run: source ~/.zshrc)                  ${G}│${NC}"
    echo -e "${G}  │${NC}                                                                    ${G}│${NC}"
    echo -e "${G}  │${NC} 2. Launch the TUI: ${C}maestro tui${NC}                                    ${G}│${NC}"
    echo -e "${G}  │${NC}                                                                    ${G}│${NC}"
    echo -e "${G}  │${NC} 3. Or in Claude Code: ${C}/maestro:setup${NC}                              ${G}│${NC}"
    echo -e "${G}  ╰──────────────────────────────────────────────────────────────────╯${NC}"
    echo ""

    echo -e "${M}  📖 Documentation:${NC} ${C}https://github.com/scooter-lacroix/Maestro${NC}"
    echo ""

    echo -e "${G}  ✨ Your AI orchestra awaits, Maestro! Let's create something beautiful. ✨${NC}"
    echo ""
    
    # Show star prompt
    show_star_prompt
}

# Run main
main "$@"
