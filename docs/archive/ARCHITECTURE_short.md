# Maestro Architecture

## Overview

Maestro is a unified spec-driven development framework combining three core systems into one cohesive platform.

## Core Systems

### 1. Spec-Driven Development Engine

The development engine provides structured workflow management:

- **Track System**: Feature/bug tracking with spec → plan → implement workflow
- **Agent Selection**: Automatic specialist assignment based on task complexity
- **TDD Enforcement**: Test-first development with 80%+ coverage goals
- **Progress Tracking**: Real-time status with rollback capability

**Key Components:**
- `maestro/tracks/` - Track definitions and metadata
- `claude-code/commands/` - CLI command implementations
- `claude-code/templates/` - Workflow and styleguide templates

### 2. Nexus Memory System

Built-in project context and memory management:

- **Agent Namespaces**: Isolated memory per agent type
- **Semantic Search**: Vector-based similarity with embeddings
- **LLM Enhancement**: Automatic context enrichment
- **Project Detection**: Automatic project-based isolation
- **Web Dashboard**: Visual memory browser

**Key Components:**
- `maestro/memory/nexus/` - Core memory system
- `maestro/memory/service.py` - Memory service API
- `maestro/memory/database/` - Data models and storage
- `maestro/memory/frontend/` - React dashboard
- `maestro/memory/cli.py` - CLI interface

### 3. Maestro TUI

Terminal-based session and MCP management:

- **Session Management**: Create, fork, group tmux sessions
- **Fuzzy Search**: Quick session navigation
- **MCP Pooling**: 50%+ memory reduction via socket pooling
- **Configuration**: TOML-based config system

**Key Components:**
- `maestro/tui/cmd/` - TUI commands
- `maestro/tui/mcppool/` - MCP socket pooling
- `~/.maestro/config.toml` - User configuration

### 4. Metacognitive Analysis Framework

Native Claude Code integration for quality assurance:

- **6-Step Analysis**: Core thesis, assumptions, logic, pitfalls, risks, synthesis
- **8 Integration Points**: Directive-based before/after analysis
- **Pitfall Detection**: Problem evasion, happy path, over-engineering, hallucination
- **Confidence Scoring**: Calibrated decision thresholds
- **Native Integration**: Uses Claude Code session model

**Key Components:**
- `maestro/critical_think/core.py` - Analysis engine
- `maestro/critical_think/native_integration.py` - Native session integration
- `maestro/critical_think/templates/` - Analysis prompt templates
- `maestro/critical_think/config_loader.py` - Configuration management

## Data Flow

```
User Input → CLI Command → Workflow Engine
                                ↓
                    ┌──────────┴──────────┐
                    ↓                     ↓
              Nexus Memory        Agent Selection
                    ↓                     ↓
                    └──────────┬──────────┘
                               ↓
                    Metacognitive Analysis
                               ↓
                    Implementation Execution
                               ↓
                    Progress Tracking + Git
```

## Configuration

### Global Config
- **Location**: `~/.claude/maestro.local.md`
- **Purpose**: Cross-project settings (models, analysis frequency, marketplace)

### Project Config
- **Location**: `.maestro/config.yaml`
- **Purpose**: Project-specific settings and state

### Memory Config
- **Location**: `.maestro/memory_config.yaml`
- **Purpose**: Memory system configuration

### TUI Config
- **Location**: `~/.maestro/config.toml`
- **Purpose**: Session and MCP management settings

## Agent Selection Logic

```python
if task.lines <= 5:
    return DirectImplementation
elif task.lines <= 50:
    return ExploreAgent
elif task.is_multi_file():
    return Oracle + Explore
elif task.context_size > 100KB:
    return Librarian
elif task.requires_specification:
    return Oracle
else:
    return ExploreAgent
```

## Memory Architecture

### Storage
- **Database**: SQLite with vector embeddings
- **Location**: `.maestro/memory.db`
- **Namespaces**: Per-agent-type isolation

### Retrieval
- **Vector Search**: Cosine similarity on embeddings
- **LLM Enhancement**: Context enrichment using stored memories
- **Project Isolation**: Automatic project-based filtering

### Dashboard
- **Framework**: React 18 + TypeScript + Vite
- **Access**: `maestro memory serve` → http://localhost:8000
- **Features**: Browse, search, visualize memories and tracks

## Integration Points

### Metacognitive Analysis Triggers

1. **Before Question** - Pre-Q&A validation
2. **After Question** - Post-Q&A verification
3. **Before Documentation** - Doc generation planning
4. **After Documentation** - Doc quality validation
5. **Before Implementation** - Plan analysis
6. **After Implementation** - Result validation
7. **Before Agent Delegation** - Delegation validation
8. **After Agent Delegation** - Result verification

## CLI Commands

### Claude Code
- `/maestro:setup` - Initialize environment
- `/maestro:newTrack` - Create track
- `/maestro:implement` - Execute plan
- `/maestro:status` - View progress
- `/maestro:revert` - Rollback work
- `/maestro:configure` - Settings management

### OpenCode
- `/maestro setup` - Initialize environment
- `/maestro newTrack` - Create track
- `/maestro implement` - Execute plan
- `/maestro status` - View progress
- `/maestro revert` - Rollback work
- `/maestro configure` - Settings management

### System Commands
- `maestro memory serve` - Web dashboard
- `maestro memory search` - Search memories
- `maestro memory stats` - Memory statistics
- `maestro tui` - Terminal interface

## Testing Strategy

- **Unit Tests**: Component-level testing
- **Integration Tests**: System integration validation
- **E2E Tests**: Complete workflow testing
- **Performance**: Benchmark regression detection

## Security

- **Input Validation**: All user inputs sanitized
- **Template Safety**: Safe substitution prevents injection
- **YAML Safety**: Safe loading only
- **Secrets Protection**: Sensitive data never logged
