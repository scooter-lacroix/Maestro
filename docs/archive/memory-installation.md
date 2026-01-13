# Maestro Memory Installation Guide

## Overview

Maestro 2.0 integrates Nexus Memory System for automatic context extraction and semantic search. Memory features are **optional** - Maestro works without them, but you get enhanced functionality with memory enabled.

## Installation Options

### Option 1: Core Only (No Memory)

Install Maestro without memory features:

```bash
pip install maestro-framework
# or
pip install maestro-framework[core]
```

This gives you basic Maestro functionality without memory extraction.

### Option 2: With Memory Support (Recommended)

Install Maestro with Nexus Memory System:

```bash
pip install maestro-framework[memory]
```

This installs all dependencies for Nexus Memory System including:
- SQLAlchemy (database)
- sentence-transformers (embeddings)
- sqlite-vec (vector search)
- FastAPI (web dashboard)
- uvicorn (server)
- websockets (real-time updates)
- psutil (system monitoring)

### Option 3: Development Installation

For development with all dependencies:

```bash
pip install maestro-framework[all]
# or
pip install maestro-framework[memory,dev]
```

This includes testing tools (pytest, coverage, linting).

## Verification

Verify installation:

```bash
# Check Maestro version
maestro --version

# Check if memory is available
python -c "from maestro.memory import MaestroMemoryService; print('Memory enabled')"

# Start memory dashboard (if memory installed)
maestro memory serve
```

## First-Time Setup

After installation, initialize the memory database:

```bash
# Create config directory
mkdir -p ~/.maestro

# Initialize database
python -c "from maestro.memory.service import MaestroMemoryService; MaestroMemoryService._init_db()"

# Start dashboard
maestro memory serve
```

Visit http://localhost:8000 for the memory dashboard.

## Memory Features

When memory is installed and enabled:

- **Automatic Context Extraction**: All Maestro commands automatically extract context
- **Semantic Search**: Find similar previous commands and implementations
- **Project Memory**: Each project maintains isolated memory context
- **Track History**: Track all decisions and context for each track
- **Web Dashboard**: Visual interface for browsing and searching memories

## Configuration

Memory is controlled by `~/.maestro/config.toml`:

```toml
[memory]
enabled = true
database_path = "~/.maestro/memory.db"
auto_extract = true

[memory.web]
enabled = true
host = "0.0.0.0"
port = 8000
```

## Troubleshooting

### Memory Not Working

1. Check installation:
   ```bash
   pip list | grep sentence-transformers
   ```

2. Reinstall with memory:
   ```bash
   pip install maestro-framework[memory] --force-reinstall
   ```

3. Check configuration:
   ```bash
   cat ~/.maestro/config.toml
   ```

### Embedding Model Download

On first use, sentence-transformers downloads the embedding model (~80MB). This happens automatically.

### Database Locked

If you see "database is locked" errors:
- Only one process can write to SQLite at a time
- Check for other running Maestro instances
- Consider using WAL mode for better concurrency

## Uninstalling Memory

To remove memory features:

```bash
pip uninstall sentence-transformers sqlite-vec fastapi uvicorn websockets
```

Maestro will continue to work without memory (it will be disabled automatically).

## Next Steps

- Read [Memory Architecture](./phase1-task2-integration-architecture.md)
- Configure [Memory Settings](../config/parser.py)
- Start using Maestro commands with automatic memory extraction
