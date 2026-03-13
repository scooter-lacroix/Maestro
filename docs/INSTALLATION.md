# Installation

Maestro has a **single installer entrypoint**: `install.sh`.

`install.sh` launches the Rust **Conductor Wizard** (`maestro-setup`) which can configure multiple AI coding tools in one run (Claude Code, OpenCode, Codex, Gemini, Qwen, Amp, Droid), Pi-Mono integration for subagent workflows, and LSP server auto-installation.

## Quick Install

```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/main/install.sh | bash
```

Local (from a clone):
```bash
git clone --branch main https://github.com/scooter-lacroix/Maestro.git
cd Maestro
./install.sh
```

Local installs stay on your current checkout unless you explicitly set `MAESTRO_BRANCH`.

---

## Supported Platforms

The installer supports the following platforms with automatic distribution detection:

### Linux

| Distribution | Package Manager | Status |
|--------------|-----------------|--------|
| **Debian** | apt-get | ✅ Full Support |
| **Ubuntu** | apt-get | ✅ Full Support |
| **Linux Mint** | apt-get | ✅ Full Support |
| **Pop!_OS** | apt-get | ✅ Full Support |
| **Arch Linux** | pacman | ✅ Full Support |
| **CachyOS** | pacman | ✅ Full Support |
| **Manjaro** | pacman | ✅ Full Support |
| **EndeavourOS** | pacman | ✅ Full Support |
| **Fedora** | dnf | ✅ Full Support |
| **RHEL/CentOS** | dnf | ✅ Full Support |
| **AlmaLinux** | dnf | ✅ Full Support |
| **Rocky Linux** | dnf | ✅ Full Support |
| **Other** | Generic | ⚠️ Fallback Mode |

### macOS

| Package Manager | Status |
|-----------------|--------|
| **Homebrew** | ✅ Full Support |

### Windows

| Platform | Status |
|----------|--------|
| **WSL** | ✅ Full Support |
| **Native** | ❌ Not Supported |

---

## What Gets Installed

The wizard configures the following components (depending on which toggles you enable in the TUI):

| Component | Description |
|-----------|-------------|
| **Maestro protocols** | Canonical command protocols installed under `~/.maestro/integrations/commands/` (or your chosen install path) |
| **Tool command packs** | Installs the tool-specific command/prompt pack (Claude Code commands, OpenCode skill, Codex prompts, Gemini/Qwen commands) |
| **LeIndex MCP wiring** | Registers `leindex` in the Maestro MCP pool and points integrated CLI tools at `maestro mcp tool-search` |
| **Pi-Mono integration** | Subagent workflow orchestration via `crates/pi-mono/` |
| **LSP servers** | Auto-installs lsp-bridge and language servers for supported languages |
| **Optional search stack** | Go + Zoekt (if enabled) |
| **Rust CLI** | Installs the Rust `maestro` binary to `~/.local/bin/maestro` (after build) |

---

## Dependencies

### Required

- **Rust + Cargo** - Required to build Maestro's Rust core (installer will install via rustup if missing)
- **git** - For version control integration
- **build tools** - Platform-specific (see table below)
- **Turso/libsql** - Required for Pi-Mono persistent storage (auto-installed by wizard)

### Platform-Specific Build Dependencies

| Platform | Build Tools | SSL | Package Config |
|----------|-------------|-----|----------------|
| Debian/Ubuntu | `build-essential` | `libssl-dev` | `pkg-config` |
| Arch/CachyOS/Manjaro | `base-devel` | `openssl` | `pkgconf` |
| Fedora/RHEL | `@development-tools` | `openssl-devel` | `pkgconfig` |
| macOS | Xcode CLI | `openssl` (brew) | `pkg-config` (brew) |

### Optional (Recommended)

- **Node.js + npm** - Required to build the Memory Dashboard frontend step (`npm install && npm run build`)
- **Go** - Required if you enable Zoekt
- **lsp-bridge** - Enhanced LSP support (wizard provides installation guidance)

---

## Installation Walkthrough

### Step 1: Run `install.sh`

`install.sh` launches the Rust Conductor Wizard (TUI). In the configuration screen you can:

- Choose the Maestro home directory (default: `~/.maestro`)
- Select which tools to integrate:
  - Claude Code
  - OpenCode
  - Codex CLI
  - Gemini CLI
  - Qwen Code
  - Amp CLI
  - Droid CLI
- Enable Pi-Mono integration for subagent workflows
- Configure LSP server auto-installation (wizard provides guidance)
- Optionally enable Go/Zoekt, tmux tooling, etc

The TUI will display your detected distribution and package manager at startup.

### Step 2: What "first-class integration" means (per tool)

- **Claude Code**: installs commands to `~/.claude/commands/`, templates to `~/.claude/maestro-templates/`, and upserts `mcpServers.leindex` in `~/.claude/.mcp.json`
- **OpenCode**: installs the skill to `~/.config/opencode/skill/maestro/`, copies command protocols to `~/.config/opencode/commands/`, and updates `~/.config/opencode/opencode.json` (commands + MCP)
- **Codex**: installs custom prompts under `${CODEX_HOME:-~/.codex}/prompts/` and upserts `[mcp_servers.leindex]` in `${CODEX_HOME:-~/.codex}/config.toml`
- **Gemini**: installs TOML commands under `~/.gemini/commands/maestro/` and upserts `mcpServers.leindex` in `~/.gemini/settings.json`
- **iFlow CLI**: registers `mcpServers.leindex` inside `~/.iflow/settings.json` so the CLI routes pooled MCP access through `maestro mcp tool-search`.
- **Qwen**: installs TOML commands under `~/.qwen/commands/maestro/` and upserts `mcpServers.leindex` in `~/.qwen/settings.json`
- **Amp**: upserts `amp.mcpServers.leindex` in `~/.config/amp/settings.json`
- **Droid**: upserts `mcpServers.leindex` in `~/.factory/mcp.json` (with `type: "stdio"`)

The installer also registers the external `leindex mcp` server in the Maestro MCP pool, so Cockpit-launched CLI tools can all reach pooled MCP servers through the dynamic broker at `maestro mcp tool-search`.

---

## Post-Installation

### Configure PATH

Add the following to your `~/.bashrc` or `~/.zshrc`:

```bash
# Add user-local bin to PATH
export PATH="$HOME/.local/bin:$PATH"

# Add Go binaries to PATH (if using Zoekt)
export PATH="$PATH:$(go env GOPATH)/bin"
```

Then reload your shell:

```bash
source ~/.bashrc   # or source ~/.zshrc
```

### Verify Installation

```bash
# Canonical Maestro command protocols
ls ~/.maestro/integrations/commands/

# Claude Code (if enabled)
ls ~/.claude/commands/
ls ~/.claude/maestro-templates/
cat ~/.claude/.mcp.json

# OpenCode (if enabled)
ls ~/.config/opencode/skill/maestro/
ls ~/.config/opencode/commands/
cat ~/.config/opencode/opencode.json

# Codex (if enabled)
ls ${CODEX_HOME:-~/.codex}/prompts/
cat ${CODEX_HOME:-~/.codex}/config.toml

# Gemini / Qwen (if enabled)
ls ~/.gemini/commands/maestro/
cat ~/.gemini/settings.json
ls ~/.qwen/commands/maestro/
cat ~/.qwen/settings.json

# iFlow CLI (if enabled)
ls ~/.iflow
cat ~/.iflow/settings.json
```

### Optional: Configure Enhanced Agents

Run `/maestro:configure` to enable enhanced agent capabilities:

```
/maestro:configure
```

This checks for external CLI tools (gemini-cli, qwen-cli, codex-cli) and creates agent configurations for available tools. Maestro works without this step, but with expanded capabilities when configured.

### Optional: Configure Pi-Mono

Run `/maestro:configure --pi-mono` to enable Pi-Mono subagent workflows:

```
/maestro:configure --pi-mono
```

Verify Pi-Mono status:

```bash
maestro pi-status
```

---

## Manual Installation

If the one-line installer doesn't work for your environment:

### 1. Clone Repository

```bash
git clone --branch main https://github.com/scooter-lacroix/Maestro.git
cd Maestro
```

### 2. Run Installer Script

```bash
chmod +x install.sh
./install.sh
```

### 3. Validate the finalized build on `main`

```bash
bun install
bun run build:tracklens
cargo test --workspace
```

Concise validation steps:
- Confirm the remote installer pulls from `main` by default with `curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/main/install.sh | bash`
- Confirm local installs stay on your current branch unless you set `MAESTRO_BRANCH`, using `git branch --show-current` before and after `./install.sh`
- Confirm the finalized `main` build succeeds with `bun run build:tracklens && cargo test --workspace`

### 4. (Optional) Copy Claude Code assets manually

```bash
# Copy commands to Claude directory
cp -r claude-code/commands/* ~/.claude/commands/

# Copy templates
cp -r claude-code/templates/* ~/.claude/maestro-templates/
```

---

## Manual Dependency Installation

### Install Build Tools

**Debian/Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev
```

**Arch/CachyOS/Manjaro:**
```bash
sudo pacman -S --needed base-devel pkgconf openssl
```

**Fedora/RHEL:**
```bash
sudo dnf group install "Development Tools"
sudo dnf install pkgconfig openssl-devel
```

**macOS:**
```bash
xcode-select --install
brew install pkg-config openssl
```

### Install Go

**Debian/Ubuntu:**
```bash
sudo apt-get install golang-go
```

**Arch/CachyOS/Manjaro:**
```bash
sudo pacman -S --needed go
```

**Fedora/RHEL:**
```bash
sudo dnf install golang
```

**macOS:**
```bash
brew install go
```

**Or download from:** https://golang.org/dl/

### Install Tmux Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get install libncurses-dev libevent-dev tmux
```

**Arch/CachyOS/Manjaro:**
```bash
sudo pacman -S --needed ncurses libevent tmux
```

**Fedora/RHEL:**
```bash
sudo dnf install ncurses-devel libevent-devel tmux
```

### Install Zoekt

```bash
# Requires Go to be installed first
go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest
go install github.com/sourcegraph/zoekt/cmd/zoekt-indexer@latest
```

### Start Zoekt (Optional)

If you installed Zoekt, start the server:

```bash
# Create index directory
mkdir -p ~/.maestro/zoekt_index

# Start Zoekt server
zoekt-webserver -rpc -index ~/.maestro/zoekt_index
```

In another terminal, index your project:

```bash
zoekt-indexer -index ~/.maestro/zoekt_index -repo_name myproject /path/to/project
```

---

## Troubleshooting

### "command not found: maestro"

**Cause:** `~/.local/bin` is not in your PATH.

**Solution:** Add to `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

### "command not found: zoekt-webserver"

**Cause:** Go binaries are not in your PATH.

**Solution:** Add to `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$PATH:$(go env GOPATH)/bin"
```

### "Go not found" during Zoekt install

**Cause:** Zoekt requires Go to be installed.

**Solution:** Install Go first, then Zoekt:
```bash
# Install Go (Debian/Ubuntu)
sudo apt-get install golang-go

# Install Go (Arch)
sudo pacman -S go

# Install Go (Fedora)
sudo dnf install golang

# Install Go (macOS)
brew install go

# Then install Zoekt
go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest
```

### Frontend build failed

**Cause:** Node.js dependencies missing.

**Solution:** Install manually:
```bash
cd maestro/memory/frontend
npm install
npm run build
```

### TypeScript hooks not building

**Cause:** npm not available or package.json issues.

**Solution:** Ensure Node.js is installed and hooks contain valid package.json files.

### MCP servers not connecting

**Cause:** Invalid JSON in `.mcp.json` or paths not configured.

**Solution:** Validate the configuration:
```bash
python3 -c "import json; json.load(open('~/.claude/.mcp.json'))"
```

### Package not found on Arch/Fedora

**Cause:** Package name differs between distributions.

**Solution:** The installer handles most package name mappings automatically. If a package fails:
- **Arch:** Check AUR for the package
- **Fedora:** Try `sudo dnf search <package>`
- **Debian:** Package may have a different name on your distro

### Unknown distribution detected

**Cause:** Your distribution is not in the recognized list.

**Solution:** The installer will fall back to generic mode. You may need to install dependencies manually. Check the Manual Dependency Installation section above.

---

## Platform-Specific Notes

### Linux

All major distributions are supported with automatic detection:

- **Debian/Ubuntu:** Full support with apt-get
- **Arch-based (Arch, CachyOS, Manjaro, EndeavourOS):** Full support with pacman
- **Fedora-based (Fedora, RHEL, CentOS, AlmaLinux, Rocky):** Full support with dnf

The installer detects your distribution from `/etc/os-release` and uses the appropriate package manager.

### macOS

Homebrew is recommended for dependency installation. The installer will use `brew` if available.

### Windows/WSL

Full support via WSL (Windows Subsystem for Linux). Native Windows is not supported.

When using WSL, the installer will detect your Linux distribution (typically Ubuntu) and install accordingly.

---

## Uninstallation

To remove Maestro:

```bash
# Remove commands
rm -rf ~/.claude/commands/maestro*

# Remove templates
rm -rf ~/.claude/maestro-templates/

# Remove MCP configuration
rm ~/.claude/.mcp.json

# Remove Zoekt (optional)
rm -rf ~/.maestro/zoekt_index
```

---

## Next Steps

After installation:

1. **Initialize your project:** `/maestro:setup`
2. **Configure enhanced agents:** `/maestro:configure`
3. **Create your first track:** `/maestro:newTrack <description>`

For usage documentation, see:
- [Claude Code Guide](CLAUDE-CODE.md)
- [OpenCode Guide](OPENCODE.md)
- [Agent Reference](AGENTS.md)
