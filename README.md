# Maestro v2 - The Unified Development Framework

<div align="center">

**Transform AI interactions into production-ready software**

[![Version](https://img.shields.io/badge/version-2.0.0-blue.svg)](VERSION)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude_Code-supported-purple.svg)](docs/CLAUDE-CODE.md)
[![OpenCode](https://img.shields.io/badge/OpenCode-supported-orange.svg)](docs/OPENCODE.md)
[![Tests](https://img.shields.io/badge/tests-250%2B_passing-brightgreen.svg)](maestro/tracks/maestro-v2_20260110/)

</div>

## Overview

Maestro v2 is a major architectural evolution that unifies three powerful systems into a cohesive, spec-driven development orchestration framework:

- **Maestro Core** - Spec-driven development with automatic agent selection and TDD enforcement
- **Unified Memory System** - Built-in project context with semantic search and coordination patterns
- **109 Rebranded Skills** - Complete workflow, analysis, research, and quality skills from Maestro namespace
- **28 Specialized Agents** - Orchestrators, planners, explorers, implementers, debuggers, and more
- **16 Integrated Hooks** - Session start, tool use, coordination, and session end hooks
- **TLDR Code Analysis** - 5-layer code analysis with semantic indexing
- **Maestro TUI** - Terminal User Interface for session and MCP management

Transform AI chat interactions into professional software engineering workflows with:

- **Structured project planning** with product definition, tech stack, and workflow configuration
- **Track-based development** where each feature/bug goes through spec → plan → implement
- **Automatic agent selection** based on task complexity (8+ specialized agents)
- **TDD workflow enforcement** with test-first development and 80%+ coverage goals
- **Built-in memory system** via integrated Nexus Memory (no external MCP required)
- **Git-aware tracking** for complete history and rollback capability
- **Web dashboard** for visualizing memory, tracks, and project context
- **TUI interface** for managing tmux sessions and MCP server connections

<p align="center">
  <img src="brain.jpeg" alt="Maestro" width="100%"/>
</p>

## Key Features

### Spec-Driven Development

Every feature goes through a structured workflow:

1. **Specification Generation** - Interactive Q&A creates comprehensive spec.md
2. **Task Breakdown** - Detailed plan.md with phased implementation
3. **Agent Selection** - Automatic specialist assignment based on complexity
4. **TDD Implementation** - Test-first development with coverage goals
5. **Progress Tracking** - Real-time status updates and rollback capability

### Proactive Agent Usage

Maestro automatically selects and deploys specialized agents based on task complexity:

| Agent | Specialty | When Used |
|-------|-----------|-----------|
| **oracle** | Architecture, code review, strategy | All implementation work (mandatory) |
| **librarian** | Multi-repo analysis, doc lookup | Large codebase analysis (>100KB) |
| **explore** | Fast codebase exploration | Standard implementation tasks |
| **frontend-ui-ux-engineer** | UI/UX design and implementation | Frontend features, prototypes |
| **document-writer** | Technical writing | Documentation generation |
| **multimodal-looker** | Visual content analysis | PDF, image, diagram analysis |
| **kilocode-orchestrator** | Large-scale projects | Persistent memory across sessions |
| **llm-council-evaluator** | Meta-agent selection | High-risk or complex decisions |

### Automatic Complexity Assessment

```
Trivial (1-5 lines)        → Direct implementation
Standard (5-50 lines)      → explore agent
Complex (multi-file, >50)  → oracle/librarian + explore
Analysis (>100KB)          → librarian
Spec-driven/ambiguous      → oracle for specification
```

### Nexus Memory System

Built directly into Maestro - no external MCP required:

- **Agent-Specific Namespaces**: Isolated memory per agent type
- **Semantic Search**: Vector-based similarity search with embeddings
- **LLM Enhancement**: Automatic context enrichment using stored memories
- **Project Detection**: Automatic project-based memory isolation
- **Web Dashboard**: Visual browser for all stored memories
- **Data Import**: Import from external memory systems

### Maestro TUI

Terminal-based session and MCP management:

- **Session Management**: Create, fork, and group tmux sessions by project
- **Fuzzy Search**: Quickly find and switch between sessions
- **MCP Pooling**: Efficient socket pooling reduces memory usage by 50%+
- **Configuration**: TOML-based config at `~/.maestro/config.toml`

### Web Dashboard

Modern React-based dashboard for memory and project visualization:

- **Memory Browser**: Browse, search, and filter all stored memories
- **Project Management**: View all projects with tracks and progress
- **Statistics**: Real-time metrics on memory usage and activity
- **Semantic Search**: Natural language search across memories
- **Visual Effects**: Modern brutalist design with advanced animations

Access via:
```bash
maestro memory serve
# Visit http://localhost:18765
```

### TLDR & LeIndex

Powerful code analysis and search capabilities (ported from llm-tldr):

- **5-Layer Code Analysis**:
  - Layer 1 (AST): Extract functions, classes, imports
  - Layer 2 (Call Graph): Who calls what
  - Layer 3 (Control Flow): Code complexity and decision points
  - Layer 4 (Data Flow): Where data goes
  - Layer 5 (Program Slicing): What affects a line

- **Automatic Hooks**: Context injection during your sessions
- **Full-Text + Semantic Search**: Fast code search with intelligent results
- **95% Token Reduction**: Optimized context for LLM consumption

Access via slash commands:
```bash
/maestro:tldr ast src/auth.py           # Analyze structure
/maestro:tldr callers authenticate      # See who calls a function
/maestro:tldr cfg src/utils.py          # Analyze complexity
/maestro:leindex search "auth"          # Search code
```

Or via CLI:
```bash
leindex-search "authentication pattern"
leindex stats
```

### Metacognitive Analysis

Native Claude Code integration for systematic analysis and quality assurance:

- **6-Step Analysis**: Core thesis, assumptions, logic check, pitfall analysis, risk assessment, and synthesis
- **8 Integration Points**: Directive-based analysis before/after questions, documentation, implementation, and agent delegation
- **AI Pitfall Detection**: Prevents problem evasion, happy path bias, over-engineering, and hallucination
- **Confidence Scoring**: Calibrated decision-making with go/no-go thresholds
- **Quality Validation**: Post-implementation validation ensures work matches specifications
- **Native Integration**: Uses Claude Code's session model - no separate API calls required

**Key Benefits**:
- Prevents common AI mistakes (over-confidence, unverified assumptions, ignoring edge cases)
- Improves decision quality through systematic analysis
- Ensures robust implementation with risk identification
- Maintains high code quality through validation checkpoints
- Configurable analysis frequency per integration point

## Why Maestro?

AI assistants are powerful, but unstructured conversations lead to:
- Inconsistent code quality
- Forgotten requirements
- No documentation
- Difficulty tracking progress
- Impossible to rollback

Maestro solves these problems by:

1. **Spec First**: Every feature starts with a comprehensive specification
2. **Plan Driven**: Detailed task breakdown before any code is written
3. **TDD Enforced**: Tests written before implementation
4. **Agent Smart**: Automatically selects the right specialist for each task
5. **Systematic Analysis**: Metacognitive analysis prevents AI pitfalls and ensures quality
6. **Memory Aware**: Built-in Nexus Memory learns your project context
7. **Git Integrated**: Tracks progress alongside commits for complete history
8. **Session Management**: TUI for managing complex multi-project workflows
9. **Visual Dashboard**: Web interface for memory and project exploration

## Quick Start

### Understanding Maestro Components

Maestro has two types of tools:

1. **Claude Code Slash Commands** - AI-assisted commands for development workflows
   - Installed via plugin marketplace (no additional setup required)
   - Work directly within Claude Code/OpenCode sessions
   - Examples: `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`

2. **CLI Tools** - Standalone terminal tools for advanced features
   - Require running the installer script
   - Run from your terminal outside of Claude Code
   - Examples: `maestro tui` (Terminal UI), `maestro memory serve` (Web Dashboard)

### Marketplace Installation (Slash Commands Only)

Install Maestro slash commands directly from the Claude Code plugin marketplace:

```bash
# Add the marketplace repository
/plugin marketplace add scooter-lacroix/maestro

# Install Maestro
/plugin install maestro
```

Then run the setup command in Claude Code:
```
/maestro:setup
```

**What you get:** All core slash commands for spec-driven development, track management, and AI-assisted workflows.

**What you don't get:** CLI tools (TUI, web dashboard) - see "Full Installation" below for those.

### Full Installation (Slash Commands + CLI Tools)

For the complete Maestro experience including the TUI and web dashboard for **Claude Code**, **Sourcegraph Amp**, **OpenCode**, **Gemini CLI**, and **Codex**:

```bash
# One-line installer
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash

# Or manually clone and install
git clone https://github.com/scooter-lacroix/Maestro.git
cd Maestro
./install.sh
```

Then in Claude Code or your selected tool:
```
/maestro:setup
```

**What you get:** Everything from marketplace installation PLUS:
- `maestro tui` - Terminal UI for session and MCP management
- `maestro memory serve` - Web dashboard for memory visualization
- `maestro memory status` - Memory system statistics
- Full CLI with all features

### For Other Agents (OpenCode, Gemini, etc.)

Maestro's unified installer automatically detects and configures all supported agents. Simply run:

```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash
```

Then in OpenCode:
```
/maestro setup
```

## Complete Workflow

### 1. Setup

Initialize the Maestro environment for a new or existing project.

```bash
# Claude Code
/maestro:setup

# OpenCode
/maestro setup
```

**Process:**
- Detects if project is greenfield (new) or brownfield (existing)
- For brownfield: Analyzes existing code to understand tech stack
- Interactive product definition (vision, guidelines, tech stack)
- Workflow and code styleguide selection
- Generates initial track

### 2. Create Track

Create a new track with interactive specification generation.

```bash
# Claude Code
/maestro:newTrack Add user authentication with JWT

# OpenCode
/maestro newTrack Add user authentication with JWT
```

**Process:**
- Loads project context from Nexus Memory
- Asks 3-5 clarifying questions
- Generates comprehensive spec.md
- Creates detailed plan.md with task breakdown
- Registers track in tracks.md

### 3. Implement

Execute the implementation plan for a specific track.

```bash
# Claude Code
/maestro:implement user-auth-jwt

# OpenCode
/maestro implement user-auth-jwt
```

**Process:**
- Loads track specification and plan
- Applies metacognitive analysis before/after key actions
- Identifies task complexity
- Automatically selects appropriate agent
- Executes TDD workflow (test → implement → refactor)
- Tracks progress in plan.md
- Stores context to Nexus Memory

### 4. Track Progress

Display current progress across all tracks.

```bash
# Claude Code
/maestro:status

# OpenCode
/maestro status
```

**Output includes:**
- Current phase and in-progress tasks
- Completion statistics
- Next pending actions
- Blockers and dependencies
- Memory context timestamp

### 5. Manage Memory

Interact with the Nexus Memory System.

```bash
# Browse memory via web dashboard
maestro memory serve

# Import data from external memory
maestro memory import <db_path>

# Search memory
maestro memory search "authentication flow"

# Get statistics
maestro memory stats
```

### 6. TUI Session Management

Launch the Terminal User Interface for session and MCP management.

```bash
maestro tui
```

**Features:**
- List and manage all tmux sessions
- Fork sessions for experimentation
- Group sessions by project
- Fuzzy search across sessions
- Manage MCP server connections
- Socket pooling for efficiency

### 7. Revert

Revert previous work at specified granularity.

```bash
# Claude Code
/maestro:revert [track|phase|task]

# OpenCode
/maestro revert [track|phase|task]
```

## Project Structure

```
maestro/
├── product.md              # Product vision and guidelines
├── tech-stack.md           # Technology stack choices
├── workflow.md             # Development workflow rules
├── tracks.md               # Track registry and overview
├── setup_state.json        # Setup progress tracking
├── critical_think/         # Metacognitive analysis framework
│   ├── core.py            # Analysis engine
│   ├── templates/         # Analysis prompt templates
│   └── tests/             # Analysis tests
├── memory/                 # Nexus Memory System
│   ├── nexus/             # Integrated Nexus code
│   ├── frontend/          # Web dashboard (React + TypeScript)
│   └── docs/              # Memory documentation
├── tui/                   # Terminal User Interface (Go)
│   ├── cmd/               # TUI commands
│   ├── mcppool/           # MCP socket pooling
│   └── docs/              # TUI documentation
└── tracks/
    └── <track_id>/
        ├── spec.md         # Track specification
        ├── plan.md         # Implementation plan
        └── metadata.json   # Track metadata
```

## Commands Reference

### Claude Code Slash Commands

These commands work within Claude Code/OpenCode sessions:

| Command | Description |
|---------|-------------|
| `/maestro:setup` | Initialize Maestro for new/existing projects |
| `/maestro:newTrack <desc>` | Create new track with interactive spec |
| `/maestro:implement [track]` | Execute implementation plan |
| `/maestro:status` | Display progress across all tracks |
| `/maestro:revert [track\|phase\|task]` | Revert previous work |
| `/maestro:configure` | Configure Maestro settings and features |
| `/maestro:tldr <command>` | 5-layer code analysis (AST, callgraph, CFG, DFG, slicing) |
| `/maestro:leindex <command>` | Code indexing and search (full-text + semantic) |
| `/maestro:memory serve` | Start memory dashboard server (requires CLI installation) |
| `/maestro:memory status` | Show memory system statistics (requires CLI installation) |

### CLI Tools (Terminal Commands)

These commands run in your terminal and require the full installation:

#### Memory Commands
| Command | Description |
|---------|-------------|
| `maestro memory serve` | Launch web dashboard (http://localhost:18765) |
| `maestro memory status` | Show memory system statistics |
| `maestro memory import <db>` | Import from external memory systems |
| `maestro memory search <query>` | Search memories by query |
| `maestro memory stats` | Display memory statistics |
| `maestro memory export <file>` | Export memories to JSON |
| `maestro memory import <file>` | Import memories from JSON |

#### TUI Commands
| Command | Description |
|---------|-------------|
| `maestro tui` | Launch Terminal User Interface for session management |

## Documentation

- [Quick Start Guide](docs/CLAUDE-CODE.md) - Claude Code specific documentation
- [OpenCode Guide](docs/OPENCODE.md) - OpenCode specific documentation
- [Marketplace](docs/MARKETPLACE.md) - Plugin marketplace and distribution
- [Agent Usage](docs/AGENTS.md) - All 8+ agents explained
- [Memory System](maestro/memory/docs/) - Nexus Memory documentation
- [TUI Configuration](maestro/tui/docs/CONFIG_FORMAT.md) - TUI setup guide
- [Testing](maestro/tracks/maestro-unified_20250101/TESTING_COVERAGE_ANALYSIS.md) - Test coverage details

## Dependencies

### Built-in (No Installation Required)
- **Unified Memory System**: 95-100% reliable memory capture via 4-layer hooks
- **109 Skills**: Workflow, analysis, research, quality, planning, math, and context skills
- **28 Agents**: Specialized agents for orchestration, planning, exploration, and more
- **16 Hooks**: Session, tool use, coordination, and event-driven automation
- **TLDR Analysis**: 5-layer code analysis with semantic indexing
- **Web Dashboard**: Built with React 18 + TypeScript + Vite
- **TUI**: Go-based terminal interface

### Optional Enhancements
- **tmux**: Required for TUI session management
- **Node.js 18+**: Required for web dashboard development
- **Python 3.11+**: Required for Maestro v2 core
- **UV Package Manager**: Required for Maestro v2 installation

## Examples

### Creating a New Feature

```bash
# Claude Code
/maestro:newTrack Add user authentication with JWT

# OpenCode
/maestro newTrack Add user authentication with JWT
```

Maestro will:
1. Load project context from Nexus Memory
2. Ask 3-5 clarifying questions
3. Generate a comprehensive spec.md
4. Create a detailed plan.md with task breakdown
5. Store decisions to memory for future reference
6. Register the track

### Implementing a Track

```bash
# Claude Code
/maestro:implement user-auth-jwt

# OpenCode
/maestro implement user-auth-jwt
```

Maestro will:
1. Load the specification and plan
2. Apply metacognitive analysis to validate approach
3. Assess each task's complexity
4. Automatically select appropriate agents
5. Execute TDD workflow for each task
6. Validate results with analysis
7. Store progress and decisions to memory
8. Track progress and commit changes

### Managing Memory

```bash
# Launch web dashboard
maestro memory serve

# In browser at http://localhost:18765:
# - Browse all memories by project
# - Semantic search across memories
# - View project and track progress
# - Visualize memory statistics

# Search from CLI
maestro memory search "JWT implementation details"
```

### TUI Session Management

```bash
# Launch TUI
maestro tui

# In TUI:
# - List all sessions (fuzzy search with /)
# - Create new session (Ctrl-n)
# - Fork session for experimentation (Ctrl-f)
# - Group sessions by project
# - Manage MCP server connections
# - View socket pooling statistics
```

## Development Philosophy

Maestro embodies these principles:

1. **The Plan is the Source of Truth**: All work tracked in plan.md
2. **The Tech Stack is Deliberate**: Changes documented before implementation
3. **Test-Driven Development**: Write tests before functionality
4. **High Code Coverage**: Aim for >80% coverage
5. **Systematic Analysis**: Metacognitive analysis before/after key decisions
6. **User Experience First**: Every decision prioritizes UX
7. **Non-Interactive & CI-Aware**: Prefer non-interactive commands
8. **Memory-Aware**: Learn from every interaction
9. **Session-Aware**: Manage complex workflows efficiently

## Testing

Maestro v2 includes comprehensive testing infrastructure:

- **250+ tests** across unit, integration, E2E, and performance suites
- **Target >98% code coverage** for critical paths
- **Unit tests** for skills, agents, hooks, memory, and tracks modules
- **Integration tests** for memory system, coordination patterns, and TLDR
- **E2E tests** for complete track workflows (newTrack, implement, status, revert)
- **Performance benchmarks** for memory operations and semantic search
- **CI/CD ready** with pytest, coverage, and automated regression detection

See [Maestro v2 Track](maestro/tracks/maestro-v2_20260110/) for implementation details.

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit pull requests to the main repository.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built for Claude Code and OpenCode ecosystems
- Inspired by test-driven development and spec-first methodologies
- Integrates Council of Agents framework

## Support

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/scooter-lacroix/Maestro/issues)
- Discussions: [GitHub Discussions](https://github.com/scooter-lacroix/Maestro/discussions)

---

<div align="center">

**Transform your AI-assisted development today**

[Get Started](docs/CLAUDE-CODE.md) · [Features](#key-features) · [Documentation](docs/) · [Web Dashboard](#nexus-memory-system)

**Maestro - The Unified Development Framework**

</div>
