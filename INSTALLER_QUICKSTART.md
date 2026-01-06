# Maestro Installer Quick Start Guide

## Quick Installation

### Claude Code
```bash
curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-claude-code.sh | bash
```

### OpenCode
```bash
curl -fsSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-opencode.sh | bash
```

## What Gets Installed

The enhanced installer (v1.5.0) will:

1. ✅ **Check for Go** - Install if missing (optional)
2. ✅ **Check for Zoekt** - Install if missing (optional)
3. ✅ **Install Maestro** - Core framework and commands
4. ✅ **Build Frontend** - Memory Dashboard UI
5. ✅ **Install Python CLI** - Command-line tools
6. ✅ **Configure PATH** - Show warnings if needed

## Interactive Prompts

During installation, you'll be prompted:

```
🔧 Checking dependencies...
   ⚠️  Go not found
   Install Go now? (y/N)
```

**Type `y` and Enter** to install Go, or `N` to skip.

```
   ⚠️  Zoekt not found (optional but recommended)
   Install Zoekt now? (y/N)
```

**Type `y` and Enter** to install Zoekt, or `N` to skip.

## After Installation

### 1. Add to PATH

Add these lines to your `~/.bashrc` or `~/.zshrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
export PATH="$PATH:$(go env GOPATH)/bin"
```

Then reload your shell:
```bash
source ~/.bashrc   # or source ~/.zshrc
```

### 2. Verify Installation

```bash
# Check Maestro CLI
maestro --help

# Check Go (if installed)
go version

# Check Zoekt (if installed)
zoekt-webserver --help
```

### 3. Start Zoekt (Optional but Recommended)

If you installed Zoekt:

```bash
# Create index directory
mkdir -p ~/.maestro/zoekt_index

# Start Zoekt server (runs in foreground)
zoekt-webserver -rpc -index ~/.maestro/zoekt_index

# In another terminal, index your code
zoekt-indexer -index ~/.maestro/zoekt_index -repo_name maestro /path/to/project
```

## Manual Installation (If Needed)

### Install Go Manually

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

### Install Zoekt Manually

```bash
# Requires Go to be installed first
go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest
go install github.com/sourcegraph/zoekt/cmd/zoekt-indexer@latest
```

## Troubleshooting

### "command not found: maestro"

**Solution:** Add `~/.local/bin` to your PATH (see step 1 above)

### "command not found: zoekt-webserver"

**Solution:** Add `$(go env GOPATH)/bin` to your PATH (see step 1 above)

### "Go not found" during Zoekt install

**Solution:** Install Go first, then Zoekt
```bash
# Install Go
sudo apt-get install golang-go  # Linux
brew install go                   # macOS

# Then install Zoekt
go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@latest
```

### Frontend build failed

**Solution:** Install Node.js dependencies manually
```bash
cd /path/to/maestro/maestro/memory/frontend
npm install
npm run build
```

## Next Steps

1. **Open Claude Code or OpenCode**
2. **Run setup command:** `/maestro:setup`
3. **Configure settings:** `/maestro:configure`
4. **Start using Maestro!**

## Zoekt Benefits

If you install Zoekt, the Memory System will have:

- ⚡ **Fast indexed code search** (instant results)
- 🔍 **Powerful query syntax** (regex, file patterns, etc.)
- 📊 **Progressive disclosure UI** (expandable results)
- 🎯 **Project discovery** (find all Maestro projects)

**Without Zoekt**, the system uses filesystem fallback (slower but works).

## Support

- **Documentation:** https://github.com/scooter-lacroix/Maestro
- **Zoekt Integration:** See `maestro/memory/ZOEKT_INTEGRATION.md`
- **Issues:** https://github.com/scooter-lacroix/Maestro/issues

## Summary

The new installer makes it easy to get started with Maestro:

- ✅ **Auto-detects** Go and Zoekt
- ✅ **Auto-installs** missing dependencies (optional)
- ✅ **Clear prompts** guide you through installation
- ✅ **Graceful fallback** if you skip optional dependencies
- ✅ **Works everywhere** (Linux, macOS, various distributions)

Just run the installer and follow the prompts!
