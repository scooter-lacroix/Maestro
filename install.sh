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
set -o pipefail

# Colors
C='\033[0;36m'
Y='\033[0;33m'
G='\033[0;32m'
R='\033[0;31m'
NC='\033[0m'

# ── Durable log to disk ──────────────────────────────────────────────────────
# Every significant action is written here so a failed install is debuggable
# after the fact.
INSTALL_LOG_DIR="$HOME/.maestro/logs"
mkdir -p "$INSTALL_LOG_DIR"
INSTALL_LOG="$INSTALL_LOG_DIR/install-$(date +%Y%m%d_%H%M%S).log"
# Also maintain stable symlinks for the most recent log.
# `setup-latest.log` stays as a compatibility alias for older wizard hints.
ln -sf "$INSTALL_LOG" "$INSTALL_LOG_DIR/install-latest.log"
ln -sf "$INSTALL_LOG" "$INSTALL_LOG_DIR/setup-latest.log"

log() {
    local ts
    ts="$(date '+%Y-%m-%d %H:%M:%S')"
    echo -e "$*" >> "$INSTALL_LOG"
    echo -e "${C}[$ts]${NC} $*"
}

log_raw() {
    # Write raw output (no timestamp prefix, no color) — used for command output
    echo "$1" >> "$INSTALL_LOG"
}

log_section() {
    local ts
    ts="$(date '+%Y-%m-%d %H:%M:%S')"
    echo "" >> "$INSTALL_LOG"
    echo "══════════════════════════════════════════════════════════════" >> "$INSTALL_LOG"
    echo "[$ts] $*" >> "$INSTALL_LOG"
    echo "══════════════════════════════════════════════════════════════" >> "$INSTALL_LOG"
    echo -e "${C}    $*${NC}"
}

log "Maestro install log: $INSTALL_LOG"
log "OS: $(uname -srm 2>/dev/null || echo unknown)"
log "Shell: $SHELL"
log "PATH: $PATH"
export MAESTRO_INSTALL_LOG="$INSTALL_LOG"
export MAESTRO_INSTALL_LOG_FILE="$INSTALL_LOG"
export MAESTRO_SETUP_LOG="$INSTALL_LOG"
export MAESTRO_SETUP_LOG_FILE="$INSTALL_LOG"

prepend_path_once() {
    local entry="$1"
    [[ -z "$entry" ]] && return 0
    case ":$PATH:" in
        *":$entry:"*) ;;
        *) PATH="$entry:$PATH" ;;
    esac
}

normalize_tool_paths() {
    prepend_path_once "$HOME/.cargo/bin"
    prepend_path_once "$HOME/.local/bin"
    export PATH
}

normalize_tool_paths
log "Normalized PATH: $PATH"

# On exit (success or failure), record the outcome so the log is self-contained.
trap 'rc=$?; ts=$(date "+%Y-%m-%d %H:%M:%S"); echo "" >> "$INSTALL_LOG"; if [[ $rc -eq 0 ]]; then echo "[$ts] INSTALL SUCCEEDED (exit 0)" >> "$INSTALL_LOG"; else echo "[$ts] INSTALL FAILED (exit $rc)" >> "$INSTALL_LOG"; fi' EXIT

clear
log_section "Preparing the Overture..."

# Default remote installs to master. Local installs stay on the current checkout
# unless MAESTRO_BRANCH was explicitly provided by the caller.
BRANCH_EXPLICIT=0
if [[ -n "${MAESTRO_BRANCH:-}" ]]; then
    BRANCH_EXPLICIT=1
fi
MAESTRO_BRANCH="${MAESTRO_BRANCH:-master}"
REPO_URL="${REPO_URL:-https://github.com/scooter-lacroix/Maestro.git}"

# Determine if we're running from a cloned repo or a remote install
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)"
INSTALL_DIR=""

# Check if we're in a valid Maestro repo
if [[ -f "$SCRIPT_DIR/install.sh" && -d "$SCRIPT_DIR/src/leindex" ]]; then
    # Running from a cloned repo
    INSTALL_DIR="$SCRIPT_DIR"
    log "[Local Install] Installing from: $INSTALL_DIR"
else
    # Running from remote or invalid location - need to clone
    INSTALL_DIR="$HOME/.maestro/install-temp"
    log "[Remote Install] Cloning from: $REPO_URL (branch: $MAESTRO_BRANCH)"

    # Remove temp dir if it exists from a previous failed install
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
    fi

    mkdir -p "$HOME/.maestro"
    log "Cloning $REPO_URL (branch: $MAESTRO_BRANCH) into $INSTALL_DIR ..."
    if ! git clone --depth 1 --branch "$MAESTRO_BRANCH" "$REPO_URL" "$INSTALL_DIR" 2>&1 | tee -a "$INSTALL_LOG"; then
        log "FATAL: git clone failed. Check network connectivity and repository access."
        exit 1
    fi
fi

# Change to the Rust directory
cd "$INSTALL_DIR"

# Respect the current local checkout unless a branch was explicitly requested.
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
if [[ "$BRANCH_EXPLICIT" -eq 1 && "$CURRENT_BRANCH" != "$MAESTRO_BRANCH" ]]; then
    log "[Branch Override] Current branch is '$CURRENT_BRANCH'; switching to '$MAESTRO_BRANCH' as requested."
    git fetch origin "$MAESTRO_BRANCH" || git fetch origin
    git checkout "$MAESTRO_BRANCH"
elif [[ "$BRANCH_EXPLICIT" -eq 0 && "$CURRENT_BRANCH" != "HEAD" && "$CURRENT_BRANCH" != "unknown" ]]; then
    log "[Local Install] Using current branch: $CURRENT_BRANCH"
fi

cd "$INSTALL_DIR/src/leindex"

# Check for basic build tools and dependencies
log_section "Revising system requirements..."

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
        log "[!] $cmd not found. Attempting to install $pkg via $PKM..."
        case $PKM in
            "apt-get") sudo apt-get update && sudo apt-get install -y $pkg ;;
            "pacman") sudo pacman -Sy --noconfirm $pkg ;;
            "dnf") sudo dnf install -y $pkg ;;
            "brew") brew install $pkg ;;
            *) log "[ERROR] Dynamic installation failed: Unknown package manager." ;;
        esac
        # Verify the install actually worked
        if ! command -v $cmd &> /dev/null; then
            log "[ERROR] $cmd still not found after attempting install of $pkg. PATH=$PATH"
            exit 1
        fi
        log "[OK] $cmd now available via $PKM install of $pkg"
    else
        log "[OK] $cmd already available"
    fi
}

# Pre-flight Checklist
ensure_package "git" "git"
ensure_package "curl" "curl"
ensure_package "tmux" "tmux"
ensure_package "gcc" "build-essential"
ensure_package "pkg-config" "pkg-config"

install_leindex_provider() {
    local method="${MAESTRO_LEINDEX_INSTALL_METHOD:-cargo}"
    log_section "LeIndex Provider (method=$method)"

    if command -v leindex &> /dev/null; then
        local ver
        ver="$(leindex --version 2>&1 || echo 'unknown')"
        log "[OK] Standalone LeIndex already available (version: $ver)"
        return 0
    fi

    log "Installing standalone LeIndex provider via ${method}..."
    local install_output
    local install_rc=0

    case "$method" in
        cargo)
            install_output="$(cargo install --force --locked leindex 2>&1)" || install_rc=$?
            log_raw "cargo install output (rc=$install_rc):"
            echo "$install_output" >> "$INSTALL_LOG"
            ;;
        install-script)
            log "Downloading LeIndex install script..."
            curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/LeIndex/master/install.sh -o /tmp/install-leindex.sh 2>>"$INSTALL_LOG"
            install_output="$(bash /tmp/install-leindex.sh 2>&1)" || install_rc=$?
            log_raw "install-script output (rc=$install_rc):"
            echo "$install_output" >> "$INSTALL_LOG"
            rm -f /tmp/install-leindex.sh
            ;;
        pypi)
            if command -v pip &> /dev/null; then
                install_output="$(pip install leindex 2>&1)" || install_rc=$?
            elif command -v pip3 &> /dev/null; then
                install_output="$(pip3 install leindex 2>&1)" || install_rc=$?
            else
                log "[ERROR] pip/pip3 not found for LeIndex PyPI install"
                return 1
            fi
            log_raw "pip install output (rc=$install_rc):"
            echo "$install_output" >> "$INSTALL_LOG"
            ;;
        skip)
            log "Skipping LeIndex installation by request"
            return 0
            ;;
        *)
            log "[ERROR] Unknown MAESTRO_LEINDEX_INSTALL_METHOD=${method}"
            return 1
            ;;
    esac

    # Post-install validation
    if [[ $install_rc -ne 0 ]]; then
        log "[ERROR] LeIndex install via ${method} failed (exit code $install_rc)"
        log "[DIAG] Check the full output above. Common causes:"
        log "[DIAG]   cargo: Rust toolchain issue, network timeout, or crate not published"
        log "[DIAG]   pypi: Python version mismatch or pip permission issue"
        log "[DIAG]   install-script: Network failure or script error"
        return 1
    fi

    if ! command -v leindex &> /dev/null; then
        log "[ERROR] LeIndex install reported success but 'leindex' not found on PATH"
        log "[DIAG] PATH=$PATH"
        log "[DIAG] Check if the install location is in PATH"
        log "[DIAG] cargo installs to: ~/.cargo/bin/"
        log "[DIAG] pip installs to: $(python3 -m site --user-base 2>/dev/null || echo '<unknown>')/bin/"
        return 1
    fi

    local ver
    ver="$(leindex --version 2>&1 || echo 'command failed')"
    log "[OK] LeIndex installed successfully via ${method} (version: $ver)"

    local analyze_help
    analyze_help="$(leindex analyze --help 2>&1)" || true
    if echo "$analyze_help" | grep -q "Analysis query"; then
        log "[OK] LeIndex analyze surface is available"
    else
        log "[WARN] LeIndex analyze help output was unexpected"
    fi

    local phase_help
    phase_help="$(leindex phase --help 2>&1)" || true
    if echo "$phase_help" | grep -q "5-phase analysis workflow"; then
        log "[OK] LeIndex phase-analysis surface is available"
    else
        log "[WARN] LeIndex phase help output was unexpected"
    fi
}

install_nexus_provider() {
    local method="${MAESTRO_NEXUS_INSTALL_METHOD:-git}"
    log_section "Nexus Provider (method=$method)"

    if command -v nexus &> /dev/null; then
        local ver
        ver="$(nexus --version 2>&1 || echo 'unknown')"
        log "[OK] Standalone Nexus already available (version: $ver)"
        local init_out
        init_out="$(nexus init 2>&1)" || true
        if [[ $? -ne 0 ]]; then
            log "[WARN] nexus init returned non-zero (already initialized?): $init_out"
        else
            log "[OK] nexus init succeeded"
        fi
        return 0
    fi

    log "Installing standalone Nexus provider via ${method}..."
    local install_output
    local install_rc=0

    case "$method" in
        git)
            local nexus_root="$HOME/.maestro/providers/Nexus-Memory-System"
            mkdir -p "$HOME/.maestro/providers"
            if [[ ! -d "$nexus_root/.git" ]]; then
                log "Cloning Nexus-Memory-System to $nexus_root..."
                install_output="$(git clone https://github.com/scooter-lacroix/Nexus-Memory-System.git "$nexus_root" 2>&1)" || install_rc=$?
                echo "$install_output" >> "$INSTALL_LOG"
                if [[ $install_rc -ne 0 ]]; then
                    log "[ERROR] git clone Nexus failed (exit code $install_rc)"
                    log "[DIAG] Check network connectivity and repository access"
                    return 1
                fi
            else
                log "[OK] Nexus repo already cloned at $nexus_root"
            fi
            log "Building Nexus from source (cargo build --release -p nexus-memory)..."
            install_output="$(cd "$nexus_root" && cargo build --release -p nexus-memory 2>&1)" || install_rc=$?
            log_raw "cargo build nexus output (rc=$install_rc):"
            echo "$install_output" >> "$INSTALL_LOG"
            if [[ $install_rc -ne 0 ]]; then
                log "[ERROR] Nexus cargo build failed (exit code $install_rc)"
                log "[DIAG] Check Rust toolchain: rustc --version"
                log "[DIAG] Check for missing system dependencies (openssl, pkg-config)"
                return 1
            fi
            log "Running Nexus install.sh..."
            install_output="$(cd "$nexus_root" && ./scripts/install.sh --binary ./target/release/nexus 2>&1)" || install_rc=$?
            log_raw "nexus install.sh output (rc=$install_rc):"
            echo "$install_output" >> "$INSTALL_LOG"
            ;;
        cargo)
            install_output="$(cargo install --force --locked nexus-memory 2>&1)" || install_rc=$?
            log_raw "cargo install nexus-memory output (rc=$install_rc):"
            echo "$install_output" >> "$INSTALL_LOG"
            ;;
        skip)
            log "Skipping Nexus installation by request"
            return 0
            ;;
        *)
            log "[ERROR] Unknown MAESTRO_NEXUS_INSTALL_METHOD=${method}"
            return 1
            ;;
    esac

    # Post-install validation
    if [[ $install_rc -ne 0 ]]; then
        log "[ERROR] Nexus install via ${method} failed (exit code $install_rc)"
        log "[DIAG] Check the full output above. Common causes:"
        log "[DIAG]   git: cargo build failure — check Rust version and system deps"
        log "[DIAG]   cargo: crate not published, network timeout, or an existing binary needed replacement"
        return 1
    fi

    if ! command -v nexus &> /dev/null; then
        log "[ERROR] Nexus install reported success but 'nexus' not found on PATH"
        log "[DIAG] PATH=$PATH"
        log "[DIAG] cargo installs to: ~/.cargo/bin/"
        log "[DIAG] git method installs to: via scripts/install.sh (check above output)"
        return 1
    fi

    local ver
    ver="$(nexus --version 2>&1 || echo 'command failed')"
    log "[OK] Nexus installed successfully via ${method} (version: $ver)"

    # Validate nexus init (no longer silently swallow errors)
    log "Running nexus init..."
    local init_out
    init_out="$(nexus init 2>&1)"
    local init_rc=$?
    log_raw "nexus init output (rc=$init_rc):"
    echo "$init_out" >> "$INSTALL_LOG"
    if [[ $init_rc -ne 0 ]]; then
        log "[WARN] nexus init returned non-zero (may already be initialized): $init_out"
    else
        log "[OK] nexus init succeeded"
    fi
}

# Check for Bun (required for TrackLens)
log_section "Checking for Bun..."
if ! command -v bun &> /dev/null; then
    log "[!] Bun not found. Installing Bun..."
    local bun_out
    bun_out="$(curl -fsSL https://bun.sh/install 2>&1 | bash 2>&1)" || true
    echo "$bun_out" >> "$INSTALL_LOG"
    export BUN_INSTALL="$HOME/.bun"
    export PATH="$BUN_INSTALL/bin:$PATH"
    if command -v bun &> /dev/null; then
        log "[OK] Bun installed successfully"
    else
        log "[ERROR] Bun install completed but 'bun' not found on PATH"
        log "[DIAG] Try adding: export PATH=\"\$HOME/.bun/bin:\$PATH\""
        exit 1
    fi
else
    log "[OK] Bun already available"
fi

# Check for Rust/Cargo
log_section "Checking for Rust/Cargo..."
if ! command -v cargo &> /dev/null; then
    log "[!] Rust not found. Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>>"$INSTALL_LOG"
    source $HOME/.cargo/env
    if cargo --version &> /dev/null; then
        log "[OK] Rust installed successfully"
    else
        log "[ERROR] Rust installation failed. 'cargo' could not execute natively."
        log "[DIAG] Try: source \$HOME/.cargo/env"
        exit 1
    fi
else
    log "[OK] Cargo already available ($(cargo --version 2>&1 | head -1))"
fi

install_leindex_provider
install_nexus_provider

# Build TrackLens packages
log_section "Building TrackLens packages"
# Remove old backup directories to prevent accumulation
rm -rf "$HOME/.maestro/tracklens.old" 2>/dev/null || true
rm -rf "$HOME/.maestro/tracklens.backup."* 2>/dev/null || true
rm -rf "$HOME/.claude/plugins/tracklens.old" 2>/dev/null || true
rm -rf "$HOME/.opencode/skill/maestro.old" 2>/dev/null || true
cd "$INSTALL_DIR"
if ! command -v bun &> /dev/null; then
    log "[ERROR] Bun is required to install the TrackLens workspace."
    exit 1
fi

log "Installing TrackLens workspace dependencies with Bun..."
local_bun_install_out="$(bun install 2>&1)" || { rc=$?; log "[ERROR] bun install failed (exit $rc)"; log_raw "$local_bun_install_out"; echo "$local_bun_install_out" >> "$INSTALL_LOG"; exit $rc; }
echo "$local_bun_install_out" >> "$INSTALL_LOG"
log "[OK] bun install succeeded"

log "Building TrackLens workspace (packages and apps)..."
local_bun_build_out="$(bun run build:tracklens 2>&1)" || { rc=$?; log "[ERROR] bun run build:tracklens failed (exit $rc)"; log_raw "$local_bun_build_out"; echo "$local_bun_build_out" >> "$INSTALL_LOG"; exit $rc; }
echo "$local_bun_build_out" >> "$INSTALL_LOG"
log "[OK] TrackLens build succeeded"

# Install TrackLens Claude Code Plugin
echo -e "${C}    Installing TrackLens Claude Code Plugin...${NC}"
mkdir -p "$HOME/.claude/plugins"
# Remove existing plugin to ensure no stale files
rm -rf "$HOME/.claude/plugins/tracklens.old"
if [[ -d "$HOME/.claude/plugins/tracklens" ]]; then
    echo -e "${Y}    Backing up existing TrackLens Claude Code Plugin...${NC}"
    mv "$HOME/.claude/plugins/tracklens" "$HOME/.claude/plugins/tracklens.old"
fi
mkdir -p "$HOME/.claude/plugins/tracklens"
cp -r "$INSTALL_DIR/apps/tracklens-hook/"* "$HOME/.claude/plugins/tracklens/"
echo -e "${G}    TrackLens Claude Code Plugin installed${NC}"

# Install TrackLens UI Bundle to ~/.maestro/tracklens
# This is the canonical location that the TrackLens server searches first
echo -e "${C}    Installing TrackLens UI Bundle...${NC}"
mkdir -p "$HOME/.maestro"
# Remove old bundle if exists to ensure we never have stale components
rm -rf "$HOME/.maestro/tracklens.old"
# Backup existing bundle if it exists
if [[ -d "$HOME/.maestro/tracklens" ]]; then
    echo -e "${Y}    Backing up existing TrackLens UI bundle...${NC}"
    mv "$HOME/.maestro/tracklens" "$HOME/.maestro/tracklens.old"
fi
# Copy the freshly built bundle from apps/tracklens-hook/dist
cp -r "$INSTALL_DIR/apps/tracklens-hook/dist" "$HOME/.maestro/tracklens"
echo -e "${G}    TrackLens UI bundle installed to: $HOME/.maestro/tracklens${NC}"

# Verify the bundle was installed correctly
if [[ ! -f "$HOME/.maestro/tracklens/index.html" ]]; then
    log "[ERROR] TrackLens UI bundle installation failed - index.html not found"
    log "[DIAG] Expected: $HOME/.maestro/tracklens/index.html"
    log "[DIAG] Check that bun run build:tracklens produced output in apps/tracklens-hook/dist/"
    exit 1
fi
if [[ ! -d "$HOME/.maestro/tracklens/assets" ]]; then
    log "[ERROR] TrackLens UI bundle installation failed - assets directory not found"
    exit 1
fi
# Count assets to ensure bundle is complete
ASSET_COUNT=$(find "$HOME/.maestro/tracklens/assets" -type f | wc -l)
if [[ $ASSET_COUNT -lt 10 ]]; then
    log "[ERROR] TrackLens UI bundle appears incomplete - only $ASSET_COUNT assets found"
    log "[DIAG] Expected at least 10 asset files in $HOME/.maestro/tracklens/assets/"
    exit 1
fi
echo -e "${G}    TrackLens UI bundle verified: $ASSET_COUNT assets${NC}"

# Install OpenCode Skill
echo -e "${C}    Installing OpenCode Skill...${NC}"
mkdir -p "$HOME/.opencode/skill"
# Remove existing skill to ensure no stale files
rm -rf "$HOME/.opencode/skill/maestro.old"
if [[ -d "$HOME/.opencode/skill/maestro" ]]; then
    echo -e "${Y}    Backing up existing OpenCode Skill...${NC}"
    mv "$HOME/.opencode/skill/maestro" "$HOME/.opencode/skill/maestro.old"
fi
mkdir -p "$HOME/.opencode/skill/maestro"
if [[ -d "$INSTALL_DIR/opencode/skill/maestro" ]]; then
    cp -r "$INSTALL_DIR/opencode/skill/maestro/"* "$HOME/.opencode/skill/maestro/" 2>/dev/null || true
    echo -e "${G}    OpenCode Skill installed${NC}"
else
    echo -e "${Y}    [!] OpenCode Skill directory not found, skipping...${NC}"
fi

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
log_section "Launching Maestro Conductor Wizard..."

# Change to leindex Rust directory
cd "$INSTALL_DIR/src/leindex"

# Clean previous build to ensure fresh binary
log "Cleaning previous build..."
cargo clean -p maestro-setup 2>/dev/null || true

log "Building maestro-setup with output preserved in $INSTALL_LOG..."
if ! cargo build --release --bin maestro-setup 2>&1 | tee -a "$INSTALL_LOG"; then
    build_rc=${PIPESTATUS[0]}
    log "[ERROR] Failed to build maestro-setup (exit $build_rc)"
    log "[DIAG] Build output was saved to $INSTALL_LOG"
    exit $build_rc
fi
SETUP_BIN="$INSTALL_DIR/target/release/maestro-setup"
log "[OK] Built setup binary at $SETUP_BIN"

SETUP_SUCCESS=0
SETUP_EXIT_CODE=0
SETUP_LAUNCHED=0

is_invocation_failure_output() {
    local output="$1"
    [[ "$output" == *"invalid option"* ]] \
        || [[ "$output" == *"unknown option"* ]] \
        || [[ "$output" == *"illegal option"* ]] \
        || [[ "$output" == *"usage: script"* ]] \
        || [[ "$output" == *"script: unexpected number of arguments"* ]] \
        || [[ "$output" == *"failed to create pseudo-terminal"* ]] \
        || [[ "$output" == *"openpty"* ]]
}

# Check for headless mode (non-interactive install)
if [[ "${MAESTRO_HEADLESS:-}" == "1" || "${MAESTRO_HEADLESS:-}" == "true" ]]; then
    log "Running in headless mode (skipping interactive TUI)..."
    SETUP_LAUNCHED=1
    if MAESTRO_HEADLESS=1 "$SETUP_BIN" --headless; then
        SETUP_SUCCESS=1
    else
        SETUP_EXIT_CODE=$?
        log "[ERROR] Headless installation failed (exit $SETUP_EXIT_CODE)"
        exit "$SETUP_EXIT_CODE"
    fi
# Check if we have a TTY for the TUI (need both stdin and stdout)
elif [[ -t 0 && -t 1 ]]; then
    # Both stdin and stdout are TTYs, run directly
    log "Launching interactive setup wizard..."
    SETUP_LAUNCHED=1
    if "$SETUP_BIN"; then
        SETUP_SUCCESS=1
    else
        SETUP_EXIT_CODE=$?
        log "[ERROR] Interactive setup wizard failed (exit $SETUP_EXIT_CODE)"
        exit "$SETUP_EXIT_CODE"
    fi
else
    log "[WARN] No TTY detected. Attempting pseudo-terminal fallbacks..."

    setup_env=(env
        MAESTRO_INSTALL_LOG="$INSTALL_LOG"
        MAESTRO_INSTALL_LOG_FILE="$INSTALL_LOG"
        MAESTRO_SETUP_LOG="$INSTALL_LOG"
        MAESTRO_SETUP_LOG_FILE="$INSTALL_LOG"
    )
    setup_cmd=("${setup_env[@]}" "$SETUP_BIN")
    setup_script_cmd="$(printf '%q ' "${setup_cmd[@]}")"

    try_script_command() {
        local script_args="$1"
        if [[ $SETUP_SUCCESS -eq 1 || $SETUP_LAUNCHED -eq 1 ]]; then
            return
        fi

        log "Trying 'script ${script_args}'..."
        setup_output="$(script ${script_args} "$setup_script_cmd" /dev/null 2>&1)"
        setup_rc=$?
        echo "$setup_output" >> "$INSTALL_LOG"
        if [[ $setup_rc -eq 0 ]]; then
            SETUP_LAUNCHED=1
            SETUP_SUCCESS=1
            log "[OK] Setup wizard launched successfully with 'script ${script_args}'"
        elif is_invocation_failure_output "$setup_output"; then
            log "[WARN] 'script ${script_args}' pseudo-terminal fallback is unavailable on this system"
        else
            SETUP_LAUNCHED=1
            SETUP_EXIT_CODE=$setup_rc
            log "[ERROR] Setup wizard failed while running under 'script ${script_args}' (exit $SETUP_EXIT_CODE)"
            exit "$SETUP_EXIT_CODE"
        fi
    }

    if command -v script &> /dev/null; then
        try_script_command "-q -c"
        try_script_command "-qec"
        try_script_command ""
    fi

    # If script command failed to launch the setup wizard, try expect
    if [[ $SETUP_SUCCESS -eq 0 && $SETUP_LAUNCHED -eq 0 ]] && command -v expect &> /dev/null; then
        log "Trying expect for pseudo-terminal..."
        setup_output="$(expect -c "set timeout -1; spawn ${setup_script_cmd}; interact" 2>&1)"
        setup_rc=$?
        echo "$setup_output" >> "$INSTALL_LOG"
        if [[ $setup_rc -eq 0 ]]; then
            SETUP_LAUNCHED=1
            SETUP_SUCCESS=1
        elif is_invocation_failure_output "$setup_output"; then
            log "[WARN] expect pseudo-terminal fallback could not launch the setup wizard"
            SETUP_EXIT_CODE=1
        else
            SETUP_LAUNCHED=1
            SETUP_EXIT_CODE=$setup_rc
            log "[ERROR] Setup wizard failed while running under expect (exit $SETUP_EXIT_CODE)"
            exit "$SETUP_EXIT_CODE"
        fi
    fi
fi

# If all TTY methods failed to launch the wizard, provide helpful error message
if [[ $SETUP_SUCCESS -eq 0 ]]; then
    log "[ERROR] Setup wizard could not be launched because no working TTY path was available"
    log "[DIAG] All pseudo-TTY fallbacks exhausted without starting the setup wizard"
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
    echo -e "      4. Full install log: ${Y}$INSTALL_LOG${NC}"
    echo ""
    exit 1
fi

# Copy live Python modules to plugin bundle (hooks/skills already copied by maestro-setup).
# Only copy modules that are actively imported — skip dead/orphaned code.
log_section "Installing Python modules..."
PY_PLUGIN_DIR="$MAESTRO_PLUGIN_DIR/maestro"
mkdir -p "$PY_PLUGIN_DIR"

# Live modules needed by hooks: memory/ (Nexus service, 11+ hook imports),
# utils/ (small utilities), config/ (settings manager),
# critical_think/ (metacognitive analysis at checkpoints).
for mod in memory utils config critical_think; do
    if [[ -d "$INSTALL_DIR/maestro/$mod" ]]; then
        cp -a "$INSTALL_DIR/maestro/$mod" "$PY_PLUGIN_DIR/$mod"
        log "[OK] Copied maestro/$mod/"
    else
        log "[WARN] maestro/$mod/ not found in source — skipping"
    fi
done

# Copy top-level __init__.py and non-deprecated .py files
for pyfile in "$INSTALL_DIR/maestro/"*.py; do
    [[ -f "$pyfile" ]] || continue
    fname="$(basename "$pyfile")"
    # handoffs.py is explicitly deprecated — skip
    [[ "$fname" == "handoffs.py" ]] && continue
    cp -a "$pyfile" "$PY_PLUGIN_DIR/$fname"
    log "[OK] Copied maestro/$fname"
done

# Verify critical Python modules are present
PY_VERIFY_OK=1
for check in \
    "maestro/memory/service.py:Nexus memory service" \
    "maestro/memory/hooks/unified.py:Hook manager" \
    "maestro/memory/nexus_client.py:Nexus client"; do
    fpath="${check%%:*}"
    flabel="${check##*:}"
    if [[ ! -f "$MAESTRO_PLUGIN_DIR/$fpath" ]]; then
        log "[VERIFY FAIL] $flabel not found at $MAESTRO_PLUGIN_DIR/$fpath"
        PY_VERIFY_OK=0
    fi
done
if [[ "$PY_VERIFY_OK" -eq 1 ]]; then
    log "[OK] Python modules installed and verified"
fi

# Clean up temp install directory if we cloned
if [[ "$INSTALL_DIR" == "$HOME/.maestro/install-temp" ]]; then
    echo -e "${C}    Cleaning up temporary install directory...${NC}"
    rm -rf "$INSTALL_DIR"
fi

# Final verification - ensure all components are installed
log_section "Cross-checking installed artifacts..."
VERIFY_FAILED=0

check_file() {
    local path="$1"
    local label="$2"
    if [[ ! -f "$path" ]]; then
        log "[VERIFY FAIL] ${label} not found at ${path}"
        VERIFY_FAILED=1
    fi
}

check_dir() {
    local path="$1"
    local label="$2"
    if [[ ! -d "$path" ]]; then
        log "[VERIFY FAIL] ${label} not found at ${path}"
        VERIFY_FAILED=1
    fi
}

check_command_surface() {
    local label="$1"
    shift
    local check_out
    if ! check_out="$("$@" 2>&1)"; then
        log "[VERIFY FAIL] ${label} failed to execute"
        log_raw "$check_out"
        VERIFY_FAILED=1
        return
    fi
}

check_output_contains() {
    local label="$1"
    local expected="$2"
    shift 2
    local output
    if ! output="$("$@" 2>/dev/null)"; then
        log "[VERIFY FAIL] ${label} failed to execute"
        VERIFY_FAILED=1
        return
    fi
    if [[ "$output" != *"$expected"* ]]; then
        log "[VERIFY FAIL] ${label} missing expected text: ${expected}"
        VERIFY_FAILED=1
    fi
}

check_command_surface "Maestro CLI binary" "$HOME/.local/bin/maestro" --help
check_command_surface "Maestro Cockpit binary" "$HOME/.local/bin/maestro-cockpit" --help
check_command_surface "Maestro Gateway binary" "$HOME/.local/bin/maestro-gateway" --help
check_command_surface "Maestro LSP bridge binary" "$HOME/.local/bin/maestro-lsp-mcp-bridge" --help

check_dir "$HOME/.maestro/integrations/commands" "Maestro command protocols"
check_dir "$HOME/.maestro/agents" "Maestro agent definitions"
check_dir "$HOME/.maestro/skills" "Maestro skill library"

for cmd in maestro:setup.md maestro:newTrack.md maestro:implement.md maestro:orchestrate.md maestro:status.md maestro:revert.md maestro:configure.md maestro:memory.md maestro:leindex.md maestro:tui.md maestro:tldr.md; do
    check_file "$HOME/.maestro/integrations/commands/$cmd" "Canonical Maestro command protocol"
done

check_file "$HOME/.maestro/tracklens/index.html" "TrackLens UI bundle"
check_file "$HOME/.claude/plugins/tracklens/package.json" "TrackLens Claude Code Plugin"
check_file "$HOME/.claude/plugins/maestro/plugin.json" "Maestro Claude Code Plugin"

check_output_contains "maestro --help" "track-lens" "$HOME/.local/bin/maestro" --help
check_output_contains "maestro --help" "orchestrate" "$HOME/.local/bin/maestro" --help
check_output_contains "maestro --help" "le-index" "$HOME/.local/bin/maestro" --help
check_output_contains "maestro track-lens --help" "TrackLens" "$HOME/.local/bin/maestro" track-lens --help
check_output_contains "maestro mcp --help" "tool-search" "$HOME/.local/bin/maestro" mcp --help
check_output_contains "maestro mcp --help" "serve" "$HOME/.local/bin/maestro" mcp --help
check_output_contains "maestro mcp --help" "proxy" "$HOME/.local/bin/maestro" mcp --help
check_output_contains "maestro-gateway --help" "Maestro Web Gateway" "$HOME/.local/bin/maestro-gateway" --help
check_output_contains "maestro-lsp-mcp-bridge --help" "Protocol translation" "$HOME/.local/bin/maestro-lsp-mcp-bridge" --help
check_command_surface "Standalone LeIndex provider" leindex --version
check_output_contains "Standalone LeIndex analyze surface" "Analysis query" leindex analyze --help
check_output_contains "Standalone LeIndex phase surface" "5-phase analysis workflow" leindex phase --help
check_command_surface "Standalone LeIndex MCP surface" leindex mcp --help
check_command_surface "Standalone Nexus provider" nexus --version
check_command_surface "Standalone Nexus init surface" nexus init --help
check_command_surface "Standalone Nexus session runtime surface" nexus session --help

if [[ -f "$HOME/.claude/.mcp.json" ]]; then
    check_output_contains "Claude MCP wiring" "\"leindex\"" grep -F "\"leindex\"" "$HOME/.claude/.mcp.json"
fi

if [[ $VERIFY_FAILED -eq 1 ]]; then
    log "[ERROR] Installation verification failed. Some components are missing."
    log "[DIAG] Full install log: $INSTALL_LOG"
    exit 1
fi

log "[OK] ✓ Maestro binary"
log "[OK] ✓ Runtime binaries (Cockpit/Gateway/LSP bridge)"
log "[OK] ✓ Maestro commands, agents, and skills"
log "[OK] ✓ TrackLens UI bundle"
log "[OK] ✓ Claude plugins (TrackLens + Maestro)"
log "[OK] ✓ Core command surfaces"
log "[OK] ✓ Maestro MCP pool surface"
log "[OK] ✓ Standalone LeIndex provider health"
log "[OK] ✓ Standalone LeIndex tools/MCP surface"
log "[OK] ✓ Standalone Nexus provider health"
log "[OK] ✓ Standalone Nexus init/session surface"

log_section "Installation complete!"
log "Run 'maestro' to get started."
log "Install log saved to: $INSTALL_LOG"
