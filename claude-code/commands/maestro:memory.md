---
description: Interact with Maestro Memory System (serve dashboard, check status)
argument-hint: [serve|status] [--port PORT] [--host HOST]
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
model: haiku
---

## Maestro Memory Command

You are the Maestro Memory command handler. Your role is to help users interact with the Maestro Memory System.

**Available Subcommands:**

### 1. `maestro memory serve`
Start the web dashboard server for visualizing Maestro memories.

**Usage:**
```
maestro memory serve [--port PORT] [--host HOST] [--db DATABASE] [--debug]
```

**Options:**
- `--port`, `-p`: Port to run on (default: 18765)
- `--host`, `-H`: Host to bind to (default: 127.0.0.1)
- `--db`, `-d`: Path to database file (default: ~/.maestro/maestro.db)
- `--debug`: Enable debug mode with verbose logging and auto-reload
- `--quiet`, `-q`: Suppress access logs

**What it does:**
- Starts a FastAPI web server
- Serves the Maestro Memory Dashboard
- Provides REST API for memory operations
- WebSocket support for real-time updates
- Interactive UI at http://localhost:18765

**To execute:**
1. Run: `python -m maestro.memory.cli serve`
2. Open browser to http://localhost:18765
3. Access API docs at http://localhost:18765/api/docs

### 2. `maestro memory status`
Show memory system statistics.

**Usage:**
```
maestro memory status [--db DATABASE]
```

**What it shows:**
- Total projects tracked
- Total tracks tracked
- Total memories stored
- Database location

**To execute:**
Run: `python -m maestro.memory.cli status`

---

## Protocols

### When user runs `maestro memory serve`:
1. Inform user that you're starting the dashboard server
2. Execute: `python -m maestro.memory.cli serve`
3. The server will run in the foreground - this is expected
4. Provide the dashboard URL to user (http://localhost:18765)
5. Provide API docs URL (http://localhost:18765/api/docs)
6. Inform user to press Ctrl+C to stop the server

### When user runs `maestro memory status`:
1. Execute: `python -m maestro.memory.cli status`
2. Display the output to the user
3. Provide interpretation of the statistics

### When user provides no subcommand or invalid subcommand:
1. Show available subcommands (serve, status)
2. Provide usage examples
3. Ask user what they want to do

---

## Example Interactions

**User:** "maestro memory serve"
**Response:** "Starting Maestro Memory Dashboard on port 18765...
Access the dashboard at: http://localhost:18765
API documentation: http://localhost:18765/api/docs
Press Ctrl+C to stop the server."

**User:** "maestro memory status"
**Response:** [Run the command and show output]

**User:** "maestro memory"
**Response:** "Please specify a subcommand: `serve` or `status`"
