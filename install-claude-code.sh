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
#                                    Claude Code Edition
#
#                          ✨🌟Every masterpiece needs a conductor🌟✨
# ═══════════════════════════════════════════════════════════════════════════════════════════════

set -e

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
    if [[ -n "$ZSH_VERSION" ]]; then
        echo "zsh"
    elif [[ -n "$BASH_VERSION" ]]; then
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
    local shell_rc=""
    local shell_detect=$(detect_shell)

    # Add to current session
    export PATH="${new_path}:${PATH}"

    # Add to appropriate RC file
    case "$shell_detect" in
        zsh)
            shell_rc="$HOME/.zshrc"
            if ! grep -q "export PATH=\".*${new_path}" "$shell_rc" 2>/dev/null; then
                echo "" >> "$shell_rc"
                echo "# Maestro PATH" >> "$shell_rc"
                echo "export PATH=\"${new_path}:\$PATH\"" >> "$shell_rc"
                echo -e "${C}  →${NC} Added to ${Y}~/.zshrc${NC}"
            fi
            ;;
        bash)
            shell_rc="$HOME/.bashrc"
            if ! grep -q "export PATH=\".*${new_path}" "$shell_rc" 2>/dev/null; then
                echo "" >> "$shell_rc"
                echo "# Maestro PATH" >> "$shell_rc"
                echo "export PATH=\"${new_path}:\$PATH\"" >> "$shell_rc"
                echo -e "${C}  →${NC} Added to ${Y}~/.bashrc${NC}"
            fi
            ;;
        *)
            shell_rc="$HOME/.profile"
            if ! grep -q "export PATH=\".*${new_path}" "$shell_rc" 2>/dev/null; then
                echo "" >> "$shell_rc"
                echo "# Maestro PATH" >> "$shell_rc"
                echo "export PATH=\"${new_path}:\$PATH\"" >> "$shell_rc"
                echo -e "${C}  →${NC} Added to ${Y}~/.profile${NC}"
            fi
            ;;
    esac
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
#                                    Claude Code Edition
#
#                          ✨🌟Every masterpiece needs a conductor🌟✨
# ═══════════════════════════════════════════════════════════════════════════════════════════════

EOF

    # Animated welcome
    echo -e "\n${BM}    Welcome, beautiful human!${NC} Let's make magic happen ✨"

    # Feature highlights with icons (properly aligned)
    echo -e "\n${C}  ╭────────────────────────────────────────────────────────────────╮${NC}"
    echo -e "${C}  │${NC} ${BW}You're about to experience:${NC}                                    ${C}│${NC}"
    echo -e "${C}  ├────────────────────────────────────────────────────────────────┤${NC}"
    echo -e "${C}  │${NC}  ${G}🎯${NC}  ${W}Smart Track Management${NC}          ${G}🤖${NC}  ${W}AI Agent Delegation${NC}        ${C}│${NC}"
    echo -e "${C}  │${NC}  ${G}📋${NC}  ${W}Auto-Generated Plans${NC}            ${G}🧠${NC}  ${W}Critical Think Framework${NC}     ${C}│${NC}"
    echo -e "${C}  │${NC}  ${G}🔍${NC}  ${W}Code Search (Zoekt)${NC}             ${G}💾${NC}  ${W}Memory Nexus System${NC}         ${C}│${NC}"
    echo -e "${C}  │${NC}  ${G}🎨${NC}  ${W}100+ Skills${NC}                   ${G}⚡${NC}  ${W}28 Specialized Agents${NC}        ${C}│${NC}"
    echo -e "${C}  ╰────────────────────────────────────────────────────────────────╯${NC}"
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
# 📦 INSTALLATION FUNCTIONS
# ─────────────────────────────────────────────────────────────────────────────

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
# 🎯 MAIN INSTALLATION FLOW
# ─────────────────────────────────────────────────────────────────────────────

main() {
    show_banner

    # Check for --restore flag
    if [[ "$1" == "--restore" ]]; then
        if [ -n "$2" ]; then
            restore_config "$2"
            exit 0
        else
            print_error "Usage: $0 --restore <backup_file>"
            exit 1
        fi
    fi

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
    # PHASE 4: INSTALL COMPONENTS
    # ─────────────────────────────────────────────────────────────────────

    local component=0
    local total_components=8

    install_component() {
        local name="$1"
        local icon="$2"
        local action="$3"

        component=$((component + 1))
        printf "\r${BM}  [$component/$total_components]${NC} ${icon}  ${BW}${name}...${NC}   " 2>/dev/null

        if eval "$action" > /dev/null 2>&1; then
            printf "\r${G}  ✓ [$component/$total_components]${NC} ${icon}  ${BW}${name}${NC}      \n" 2>/dev/null
        else
            printf "\r${Y}  ⚠ [$component/$total_components]${NC} ${icon}  ${BW}${name} (skipped)${NC}      \n" 2>/dev/null
        fi
    }

    print_header "🔧 Installing Components" "⚙️"

    # Commands (use /bin/cp to bypass interactive alias)
    install_component "Commands" "📋" "mkdir -p ~/.claude/commands && /bin/cp '$SCRIPT_DIR/claude-code/commands/maestro'*.md ~/.claude/commands/"

    # Templates (use /bin/cp to bypass interactive alias)
    install_component "Templates" "📝" "mkdir -p ~/.claude/maestro-templates && /bin/cp '$SCRIPT_DIR/claude-code/templates/workflow.md' ~/.claude/maestro-templates/ && mkdir -p ~/.claude/maestro-templates/code_styleguides && /bin/cp '$SCRIPT_DIR/claude-code/templates/code_styleguides/'*.md ~/.claude/maestro-templates/code_styleguides/"

    # Plugin (use /bin/cp to bypass interactive alias)
    install_component "Plugin" "🔌" "mkdir -p ~/.claude/plugins/maestro && [ -f '$SCRIPT_DIR/plugin.json' ] && /bin/cp '$SCRIPT_DIR/plugin.json' ~/.claude/plugins/maestro/"

    # Hooks (use /bin/cp to bypass interactive alias)
    install_component "Hooks" "🪝" "mkdir -p ~/.claude/plugins/maestro/hooks && [ -d '$SCRIPT_DIR/maestro/hooks' ] && /bin/cp -r '$SCRIPT_DIR/maestro/hooks/'* ~/.claude/plugins/maestro/hooks/"

    # Skills (use /bin/cp to bypass interactive alias)
    install_component "Skills" "🎓" "mkdir -p ~/.claude/plugins/maestro/skills && [ -d '$SCRIPT_DIR/maestro/skills' ] && /bin/cp -r '$SCRIPT_DIR/maestro/skills/'* ~/.claude/plugins/maestro/skills/"

    # Agents (use /bin/cp to bypass interactive alias)
    install_component "Agents" "🤖" "mkdir -p ~/.claude/plugins/maestro/agents && [ -d '$SCRIPT_DIR/maestro/agents' ] && /bin/cp -r '$SCRIPT_DIR/maestro/agents/'* ~/.claude/plugins/maestro/agents/"

    # Config (use /bin/cp to bypass interactive alias)
    install_component "Config Module" "⚙️" "mkdir -p ~/.claude/plugins/maestro/config && [ -d '$SCRIPT_DIR/maestro/config' ] && /bin/cp -r '$SCRIPT_DIR/maestro/config/'* ~/.claude/plugins/maestro/config/"

    # Critical Think (use /bin/cp to bypass interactive alias)
    install_component "Critical Think" "🧠" "mkdir -p ~/.claude/maestro-templates && [ -d '$SCRIPT_DIR/maestro/critical_think/templates' ] && /bin/cp '$SCRIPT_DIR/maestro/critical_think/templates/'*.md ~/.claude/maestro-templates/"

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 5: PYTHON CLI INSTALLATION & PATH SETUP
    # ─────────────────────────────────────────────────────────────────────

    print_header "🐍 Python CLI Installation" "🐍"

    if [ -d "$SCRIPT_DIR/maestro" ]; then
        print_info "Installing Maestro Python package..."

        # Create permanent location
        local install_dir="$HOME/.claude/plugins/maestro/lib"
        mkdir -p "$install_dir"
        rm -rf "$install_dir/maestro"

        echo -e "${C}  →${NC} Copying source to ${Y}${install_dir}${NC}..."
        /bin/cp -r "$SCRIPT_DIR/maestro" "$install_dir/"

        # Create wrapper
        mkdir -p ~/.local/bin
        cat > ~/.local/bin/maestro << 'WRAPPER_EOF'
#!/bin/bash
MAESTRO_ROOT="$HOME/.claude/plugins/maestro/lib"

if [ "$1" = "tui" ]; then
    if [ -f "$MAESTRO_ROOT/maestro/tui/build/maestro-tui" ]; then
        exec "$MAESTRO_ROOT/maestro/tui/build/maestro-tui" "${@:2}"
    else
        echo "Error: maestro-tui binary not found"
        exit 1
    fi
elif [ "$1" = "memory" ]; then
    cd "$MAESTRO_ROOT"
    python3 -m maestro.memory.cli "$@"
else
    cd "$MAESTRO_ROOT"
    python3 -m maestro.cli "$@"
fi
WRAPPER_EOF
        chmod +x ~/.local/bin/maestro
        print_success "CLI wrapper created: ~/.local/bin/maestro"

        # Add to PATH automatically for current session and RC file
        if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
            echo ""
            print_info "Adding ~/.local/bin to PATH..."
            add_to_path "$HOME/.local/bin"
            echo -e "${G}  ✅ PATH updated for current session and $(detect_shell)rc${NC}"
            echo ""
        fi
    else
        print_warning "maestro directory not found - CLI installation skipped"
    fi

    # ─────────────────────────────────────────────────────────────────────
    # PHASE 6: MCP CONFIGURATION
    # ─────────────────────────────────────────────────────────────────────

    print_header "🌐 MCP Configuration" "🌐"

    local mcp_config="$HOME/.claude/.mcp.json"
    mkdir -p "$HOME/.claude"

    if [ -f "$mcp_config" ]; then
        local timestamp=$(date +%Y%m%d_%H%M%S)
        cp "$mcp_config" "${mcp_config}.backup.${timestamp}"
        print_info "Backed up existing MCP config"
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

    print_success "MCP configuration created"

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

    echo -e "${C}  ╭─ ${BW}Claude Code Commands${NC} ${C}───────────────────────────────────────╮${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:setup${NC}      ${W}Initialize Maestro environment${NC}                 ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:newTrack${NC}   ${W}Create new track${NC}                                ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:implement${NC}  ${W}Implement track tasks${NC}                            ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:status${NC}     ${W}View project progress${NC}                             ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:configure${NC}  ${W}Configure Maestro settings${NC}                         ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:memory${NC}     ${W}Interact with Memory System${NC}                       ${C}│${NC}"
    echo -e "${C}  │${NC} ${BW}/maestro:tui${NC}        ${W}Launch Terminal UI${NC}                               ${C}│${NC}"
    echo -e "${C}  ╰──────────────────────────────────────────────────────────╯${NC}"
    echo ""

    echo -e "${M}  ╭─ ${BW}v2 Components Installed${NC} ${M}────────────────────────────────────╮${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Hooks${NC}      ${C}16${NC} ${W}event-driven automation hooks             ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Skills${NC}     ${C}100+${NC} ${W}specialized capabilities                 ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Agents${NC}     ${C}28${NC} ${W}task delegation agents                      ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Config${NC}     ${W}unified settings management                     ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}Memory${NC}     ${W}persistent context (Nexus) system                ${M}│${NC}"
    echo -e "${M}  │${NC} ${G}✅${NC} ${W}MCP${NC}        ${C}3${NC} ${W}pre-configured servers                       ${M}│${NC}"
    echo -e "${M}  ╰──────────────────────────────────────────────────────────╯${NC}"
    echo ""

    if command_exists zoekt-webserver && command_exists zoekt-indexer; then
        echo -e "${B}  ╭─ ${BW}Zoekt Code Search${NC} ${B}──────────────────────────────────────────╮${NC}"
        echo -e "${B}  │${NC} ${G}✓${NC} ${W}Zoekt is installed and ready!${NC}                          ${B}│${NC}"
        echo -e "${B}  │${NC}                                                                     ${B}│${NC}"
        echo -e "${B}  │${NC} ${W}Start server:${NC}                                                  ${B}│${NC}"
        echo -e "${B}  │${NC}   ${C}zoekt-webserver -rpc -index ~/.maestro/zoekt_index${NC}         ${B}│${NC}"
        echo -e "${B}  │${NC}                                                                     ${B}│${NC}"
        echo -e "${B}  │${NC} ${W}Index your code:${NC}                                                ${B}│${NC}"
        echo -e "${B}  │${NC}   ${C}zoekt-indexer -index ~/.maestro/zoekt_index -repo_name <name> <path>${NC} ${B}│${NC}"
        echo -e "${B}  ╰─────────────────────────────────────────────────────────╯${NC}"
        echo ""
    else
        echo -e "${Y}  ╭─ ${BW}Zoekt Code Search${NC} ${Y}──────────────────────────────────────────╮${NC}"
        echo -e "${Y}  │${NC} ${W}Zoekt not installed (optional)${NC}                                 ${Y}│${NC}"
        echo -e "${Y}  │${NC} ${W}Memory System will use fallback search mode${NC}                     ${Y}│${NC}"
        echo -e "${Y}  │${NC}                                                                     ${Y}│${NC}"
        echo -e "${Y}  │${NC} ${W}Install later:${NC}                                                   ${Y}│${NC}"
        echo -e "${Y}  │${NC}   ${C}go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest${NC} ${Y}│${NC}"
        echo -e "${Y}  ╰─────────────────────────────────────────────────────────╯${NC}"
        echo ""
    fi

    echo -e "${G}  ╭──────────────────────────────────────────────────────────────────╮${NC}"
    echo -e "${G}  │${NC} ${BW}Next Steps:${NC}                                                        ${G}│${NC}"
    echo -e "${G}  │${NC}                                                                     ${G}│${NC}"
    echo -e "${G}  │${NC} ${W}1.${NC} Open a ${Y}new terminal${NC} (or run: ${C}source ~/.$(detect_shell)rc${NC})     ${G}│${NC}"
    echo -e "${G}  │${NC}                                                                     ${G}│${NC}"
    echo -e "${G}  │${NC} ${W}2.${NC} Open Claude Code and run:                                       ${G}│${NC}"
    echo -e "${G}  │${NC}      ${C}/maestro:setup${NC}                                                ${G}│${NC}"
    echo -e "${G}  │${NC}                                                                     ${G}│${NC}"
    echo -e "${G}  │${NC} ${W}3.${NC} Run configuration to customize:                                   ${G}│${NC}"
    echo -e "${G}  │${NC}      ${C}/maestro:configure${NC}                                             ${G}│${NC}"
    echo -e "${G}  ╰──────────────────────────────────────────────────────────────────╯${NC}"
    echo ""

    echo -e "${M}  📖 Documentation:${NC} ${C}https://github.com/scooter-lacroix/Maestro${NC}"
    echo ""

    echo -e "${G}  ✨ Your AI orchestra awaits, Maestro! Let's create something beautiful. ✨${NC}"
    echo ""
}

# Run main
main "$@"
