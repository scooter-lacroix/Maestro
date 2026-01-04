# Maestro CLI Commands

This document describes the Maestro CLI commands that are available after installation.

## Installation

After running the Maestro installer (`install-claude-code.sh` or `install-opencode.sh`), the following CLI commands will be available:

### Prerequisites

1. Ensure `~/.local/bin` is in your PATH:
   ```bash
   export PATH="$HOME/.local/bin:$PATH"
   ```

   Add this to your `~/.bashrc` or `~/.zshrc` to make it permanent.

2. The installer will:
   - Install the `maestro` Python CLI (via `pip install -e .`)
   - Copy the `maestro-tui` Go binary to `~/.local/bin/` (if pre-built)
   - Create a wrapper script at `~/.local/bin/maestro` if pip installation fails

## Available Commands

### Memory System Commands

#### `maestro memory serve`

Start the Maestro Memory Dashboard web server.

```bash
maestro memory serve [--port PORT] [--host HOST] [--db DATABASE] [--debug] [--quiet]
```

**Options:**
- `--port`, `-p`: Port to run the dashboard on (default: 8080)
- `--host`, `-H`: Host to bind the dashboard to (default: 127.0.0.1)
- `--db`, `-d`: Path to database file (default: ~/.maestro/maestro.db)
- `--debug`: Enable debug mode (verbose logging, auto-reload)
- `--quiet`, `-q`: Suppress access logs

**Examples:**
```bash
# Start on default port (8080)
maestro memory serve

# Start on custom port
maestro memory serve --port 3000

# Start with debug mode
maestro memory serve --debug

# Start on all interfaces
maestro memory serve --host 0.0.0.0 --port 8080
```

**Access the dashboard at:**
- http://localhost:8080 (default)
- http://localhost:3000 (if using --port 3000)

#### `maestro memory status`

Show Maestro memory system status and statistics.

```bash
maestro memory status [--db DATABASE]
```

**Options:**
- `--db`, `-d`: Path to database file (default: ~/.maestro/maestro.db)

**Example:**
```bash
maestro memory status
```

**Output:**
```
============================================================
  Maestro Memory System Status
============================================================
  Database: /home/user/.maestro/maestro.db
  Total Projects: 5
  Total Tracks: 12
  Total Memories: 234
============================================================
```

#### `maestro memory migrate`

Migrate memories from Memori database to Nexus.

```bash
maestro memory migrate <source> [--db DATABASE] [--backup BACKUP]
```

**Arguments:**
- `source`: Path to Memori database file to migrate

**Options:**
- `--db`, `-d`: Path to target Nexus database (default: ~/.maestro/maestro.db)
- `--backup`, `-b`: Path to backup directory (default: no backup)

**Example:**
```bash
maestro memory migrate ~/.memori/memori.db --backup ~/backups
```

### TUI Commands

#### `maestro tui`

Launch the Maestro Terminal UI for session management.

```bash
maestro tui [options]
```

This command launches the Go-based Terminal UI for managing AI coding agent sessions.

**Requirements:**
- `tmux` must be installed on your system

**Install tmux:**
```bash
# macOS
brew install tmux

# Ubuntu/Debian
sudo apt-get install tmux

# Fedora/CentOS
sudo dnf install tmux
```

**TUI Features:**
- Session management (add, remove, list sessions)
- Group management
- Profile management
- MCP server management
- Interactive session attachment
- Session status monitoring

**Keyboard Shortcuts (in TUI):**
- `n` - New session
- `g` - New group
- `Enter` - Attach to session
- `d` - Delete session/group
- `m` - Move session to group
- `R` - Rename session/group
- `/` - Search
- `Ctrl+Q` - Detach from session
- `q` - Quit

### Direct CLI Access

You can also invoke the CLI modules directly using Python:

```bash
# Memory CLI
python3 -m maestro.memory.cli serve
python3 -m maestro.memory.cli status
python3 -m maestro.memory.cli migrate <source>

# Main CLI
python3 -m maestro.cli memory serve
python3 -m maestro.cli tui
```

## Troubleshooting

### Command not found

If you get "command not found" errors:

1. Check if `~/.local/bin` is in your PATH:
   ```bash
   echo $PATH
   ```

2. If not, add it to your shell configuration:
   ```bash
   # For bash
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc

   # For zsh
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

3. Verify the commands are installed:
   ```bash
   ls -la ~/.local/bin/maestro*
   ```

### TUI binary not found

If the `maestro-tui` binary is not installed:

1. Check if it exists in the source:
   ```bash
   ls -la /path/to/maestro/maestro/tui/build/maestro-tui
   ```

2. If not, build it:
   ```bash
   cd /path/to/maestro/maestro/tui
   go build -o build/maestro-tui ./cmd/maestro-tui
   ```

3. Copy it manually:
   ```bash
   cp /path/to/maestro/maestro/tui/build/maestro-tui ~/.local/bin/
   chmod +x ~/.local/bin/maestro-tui
   ```

### Python package not found

If the Python package is not installed:

```bash
# Install in development mode
cd /path/to/maestro
pip install -e .
```

Or use the wrapper script created by the installer.

## Integration with Claude Code/OpenCode

The CLI commands work alongside the Claude Code/OpenCode slash commands:

- Use `/maestro:setup` to initialize a project
- Use `maestro memory serve` to start the dashboard
- Use `maestro tui` to manage sessions
- Use `/maestro:memory` in Claude Code to interact with memory

## API Documentation

When the memory dashboard is running, you can access the API documentation at:

- Swagger UI: http://localhost:8080/api/docs
- ReDoc: http://localhost:8080/api/redoc
- OpenAPI JSON: http://localhost:8080/api/openapi.json

## Examples

### Complete Workflow

```bash
# 1. Install Maestro
cd /path/to/maestro
./install-claude-code.sh

# 2. Ensure PATH is set
export PATH="$HOME/.local/bin:$PATH"

# 3. Start the memory dashboard
maestro memory serve --port 8080 &

# 4. Check memory status
maestro memory status

# 5. Launch TUI (in another terminal)
maestro tui

# 6. Access dashboard at http://localhost:8080
```

### Development Mode

```bash
# Start dashboard with debug mode and auto-reload
maestro memory serve --debug

# Build TUI from source
cd /path/to/maestro/maestro/tui
go build -o build/maestro-tui ./cmd/maestro-tui
cp build/maestro-tui ~/.local/bin/
```

## Support

For issues, questions, or contributions:
- GitHub: https://github.com/scooter-lacroix/Maestro
- Issues: https://github.com/scooter-lacroix/Maestro/issues
