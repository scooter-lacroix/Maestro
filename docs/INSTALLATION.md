# Installation

Complete guide to installing Maestro for Claude Code or OpenCode.

## Quick Install

### Claude Code

```bash
curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-claude-code.sh | bash
```

### OpenCode

```bash
curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-opencode.sh | bash
```

---

## What Gets Installed

The installer configures the following components:

| Component | Description |
|-----------|-------------|
| **Commands** | Slash commands for project setup, track management, and implementation |
| **Templates** | Project templates for workflow and style guides |
| **MCP Servers** | Pre-configured Model Context Protocol servers (3) |
| **Memory System** | Project context and semantic search |
| **Hooks** | TypeScript build support for hooks |
| **Zoekt** | Optional fast code search (requires Go) |

---

## Dependencies

### Required

- **Python 3.11+** - For core functionality
- **Node.js 18+** - For TypeScript hooks (if present)
- **git** - For version control integration

### Optional (Recommended)

- **Go 1.21+** - Required for Zoekt
- **Zoekt** - Fast indexed code search for Memory System

The installer will detect these dependencies and offer to install them if missing.

---

## Installation Walkthrough

### Step 1: Dependency Check

The installer first checks for Go and Zoekt:

```
🔧 Checking dependencies...
   ⚠️  Go not found
   Install Go now? (y/N)
```

**Type `y`** to install Go automatically, or `N` to skip.

### Step 2: Zoekt Installation (if Go installed)

```
   ⚠️  Zoekt not found (optional but recommended)
   Install Zoekt now? (y/N)
```

**Type `y`** to install Zoekt for fast code search, or `N` to skip.

### Step 3: Configuration Backup

If you have existing Maestro configuration, the installer creates a timestamped backup:

```
   ℹ️  Backed up existing config to ~/.claude/config.json.backup.20250113_120000
```

### Step 4: MCP Server Setup

The installer creates `~/.claude/.mcp.json` with three pre-configured servers:

| Server | Purpose |
|--------|---------|
| **filesystem** | Local file system access |
| **brave-search** | Web search capability |
| **github** | GitHub repository access |

### Step 5: Command Installation

Commands are installed to `~/.claude/commands/`:

| Command | Purpose |
|---------|---------|
| `maestro:setup` | Initialize project |
| `maestro:newTrack` | Create new feature/fix |
| `maestro:implement` | Execute track |
| `maestro:status` | View progress |
| `maestro:revert` | Undo work |
| `maestro:configure` | Configure agents |

### Step 6: Template Installation

Project templates are installed to `~/.claude/maestro-templates/`:

- `workflow.md` - Development workflow configuration
- `code_styleguides/` - Code style guide templates

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
# Check Maestro commands are installed
ls ~/.claude/commands/

# Check MCP configuration
cat ~/.claude/.mcp.json

# Check templates
ls ~/.claude/maestro-templates/
```

### Optional: Configure Enhanced Agents

Run `/maestro:configure` to enable enhanced agent capabilities:

```
/maestro:configure
```

This checks for external CLI tools (gemini-cli, qwen-cli, codex-cli) and creates agent configurations for available tools. Maestro works without this step, but with expanded capabilities when configured.

---

## Manual Installation

If the one-line installer doesn't work for your environment:

### 1. Clone Repository

```bash
git clone https://github.com/scooter-lacroix/Maestro.git
cd Maestro
```

### 2. Run Installer Script

```bash
chmod +x install-claude-code.sh   # or install-opencode.sh
./install-claude-code.sh
```

### 3. Copy Commands Manually

```bash
# Copy commands to Claude directory
cp -r claude-code/commands/* ~/.claude/commands/

# Copy templates
cp -r claude-code/templates/* ~/.claude/maestro-templates/
```

---

## Manual Dependency Installation

### Install Go

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get update
sudo apt-get install golang-go
```

**Linux (RedHat/Fedora):**
```bash
sudo dnf install golang
```

**macOS:**
```bash
brew install go
```

**Or download from:** https://golang.org/dl/

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
# Install Go
sudo apt-get install golang-go  # Linux
brew install go                   # macOS

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

---

## Platform-Specific Notes

### Linux

All distributions supported (Debian, Ubuntu, Fedora, RedHat, etc.). The installer detects your distribution and uses the appropriate package manager.

### macOS

Homebrew is recommended for dependency installation. The installer will use `brew` if available.

### Windows/WSL

Full support via WSL (Windows Subsystem for Linux). Native Windows is not supported.

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
