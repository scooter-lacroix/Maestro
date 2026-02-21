# Maestro v2.5 - The Unified Development Framework

<div align="center">

**Transform AI interactions into production-ready software**

[![Version](https://img.shields.io/badge/version-2.5.0-blue.svg)](VERSION)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude_Code-supported-purple.svg)](docs/CLAUDE-CODE.md)
[![OpenCode](https://img.shields.io/badge/OpenCode-supported-orange.svg)](docs/OPENCODE.md)
[![Tests](https://img.shields.io/badge/tests-250%2B_passing-brightgreen.svg)](maestro/tracks/maestro-v2_20260110/)

</div>

## Overview

Maestro v2 is a major architectural evolution that unifies three powerful systems into a cohesive, spec-driven development orchestration framework:

- **Maestro Core** - Spec-driven development with automatic agent selection and TDD enforcement
- **Unified Memory System** - Built-in project context with semantic search and coordination patterns
- **109 Repurposed Skills** - Complete workflow, analysis, research, and quality skills from Maestro namespace
- **28 Specialized Agents** - Orchestrators, planners, explorers, implementers, debuggers, and more
- **16 Integrated Hooks** - Session start, tool use, coordination, and session end hooks
- **LeIndex Code Analysis** - 5-layer code analysis with semantic indexing (TLDR compatibility alias)
- **Conductor Engine** - Token-efficient track-based automation (inspired by Ralph TUI)
- **Pi-Mono Integration** - Unified agent discovery, detection, and execution across mono-repos
- **Cockpit TUI** - Rust-based Terminal UI for session, MCP, and orchestration management

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

### Cockpit TUI

Rust-based (ratatui) Terminal UI for complete project management:

- **7 Tabs**: Dashboard, Sessions, Projects, Analysis, LSP, Memory, Settings
- **Session Management**: Create, fork, and group tmux sessions by project
- **Conductor Module**: Track-based task automation with live output (replaces Orchestrate)
- **LSP Management**: Status, toggle, restart, and log viewing
- **MCP Pooling**: Efficient socket pooling reduces memory usage by 50%+
- **Configuration**: TOML-based config at `~/.config/maestro/config.toml`

Access via:
```bash
maestro tui
```

### Conductor Engine

Token-efficient track-based automation (inspired by [subsy/ralph-tui](https://github.com/subsy/ralph-tui)):

- **Track/Task Model**: Lossless parsing of tracks.md and plan.md
- **LeIndex Integration**: 5-phase context bundles for minimal token usage
- **Iteration Loop**: Select → Prompt → Run → Detect → Update
- **Error Strategies**: Retry, Skip, Abort with exponential backoff
- **Session Persistence**: Lock files, crash recovery, journal logs
- **Multi-Agent Support**: claude, gemini, qwen, opencode runners

Access via TUI or CLI:
```bash
maestro conductor start <track> --mode building
maestro conductor status
maestro conductor pause
```

### Pi-Mono Integration

Unified agent discovery and execution for mono-repo environments:

- **Detection**: Automatic detection of Pi-enabled projects and configurations
- **Discovery**: Scan and catalog available agents across the workspace
- **Config Management**: TOML-based configuration for agent settings
- **Agent Execution**: Unified interface for running detected agents
- **Cross-Project Coordination**: Manage agents across multiple sub-projects

Located at `crates/pi-mono/` with modules:
- `detection.rs` - Pi project detection
- `discovery.rs` - Agent discovery scanning
- `config/` - Configuration management
- `agents/` - Agent definitions and registry
- `execution/` - Agent execution runtime

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

### LeIndex Code Analysis

Powerful Rust-based code analysis (TLDR is now a compatibility alias):

- **5-Phase Analysis System**: Progressive codebase understanding
  - Phase 1: Structural scan (files, directories, dependencies)
  - Phase 2: Dependency mapping (import/export relationships)
  - Phase 3: Targeted file context (signatures, call edges)
  - Phase 4: CFG/DFG analysis (control flow, data flow)
  - Phase 5: Program slicing (backward/forward slicing)

- **5-Layer Code Analysis**:
  - AST: Extract functions, classes, imports
  - Call Graph: Who calls what
  - CFG: Code complexity and decision points
  - DFG: Where data goes
  - Slicing: What affects a line

- **Token-Efficient Output**:
  - `ultra` mode: ~2500 chars (exploration)
  - `balanced` mode: ~6000 chars (implementation-ready)
  - `json` mode: machine-readable

- **Multi-Language**: Python, TypeScript, JavaScript, Rust, Go, Java, C, C++
- **Automatic Hooks**: Context injection during your sessions

Access via slash commands (TLDR is a compatibility alias):
```bash
/maestro:tldr ast src/auth.py           # Analyze structure (delegates to LeIndex)
/maestro:tldr callers authenticate      # See who calls a function (delegates to LeIndex)
/maestro:leindex search "auth"          # Search code
```

Or via CLI (direct LeIndex access):
```bash
maestro analyze src/auth.py --mode ast
maestro le-index phase1 . --mode ultra
maestro le-index phase2 . --mode balanced
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
8. **Rust-First Architecture**: Native performance, memory safety, modular crates
9. **Conductor Engine**: Token-efficient automation with LeIndex context
10. **Session Management**: Cockpit TUI for managing complex workflows
11. **Visual Dashboard**: Web interface for memory and project exploration

## Architecture

Maestro v2.5 is built with a **Rust-first architecture**:

### Modular Crates

```
maestro/
├── crates/
│   ├── cli/              # maestro-cli (produces "maestro" binary)
│   ├── cockpit/          # maestro-cockpit (ratatui TUI library)
│   │                     # Tabs: Dashboard, Sessions, Projects, Analysis, LSP, Memory, Settings
│   ├── pi-mono/          # Pi-Mono integration (detection, discovery, agents, execution)
│   └── lsp-bridge/       # maestro-lsp-mcp-bridge (LSP protocol bridge)
│
└── leindex/
    └── rust/             # leindex-core (core library + analysis engine, Turso/libsql)
```

### Dependency Rules (One-Way)

```
cli → cockpit + leindex-core + pi-mono
cockpit → leindex-core
pi-mono → (standalone)
leindex-core ↛ cockpit (forbidden)
```

### Legacy Code Archive

- `archive/legacy-python-cli/` - Historical reference only (Python `cli.py`)
- `archive/tui-go/` - Historical reference only (Go TUI)

**Note**: CI gates prevent any runtime imports from archived code.

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

# Check memory system status
maestro memory status

# Scan directories for Maestro projects
maestro memory scan . --depth 3

# Store a memory entry
maestro memory store \
  --content "authentication flow validated" \
  --category decision \
  --importance high
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
| `maestro memory scan <path> --depth <n>` | Scan directories for Maestro projects |
| `maestro memory store --content <text> --category <type> --importance <level>` | Store a memory entry |

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
- [Memory Integration Guide](maestro/memory/docs/memory_integration.md) - Current CLI memory command surface and architecture
- [Memory Quick Start](maestro/memory/docs/quick_start.md) - 5-minute validation flow for memory commands
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
- **Cockpit TUI**: Rust-based terminal interface (ratatui)

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

# Check status from CLI
maestro memory status

# Scan current directory for projects
maestro memory scan . --depth 2

# Store from CLI
maestro memory store \
  --content "JWT implementation details captured" \
  --category observation \
  --importance normal
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

### Conductor Module (Ralph-Style Autonomous Execution)

The Cockpit TUI includes a Conductor module for autonomous track execution, inspired by [Ralph TUI](https://github.com/subsy/ralph-tui). The Conductor replaces the legacy Orchestrate pane.

**Key Features:**
- **Track/Task Tree**: Left panel shows all tracks with expandable task hierarchies
- **Live Output**: Right panel displays real-time iteration output with scrolling
- **Session Management**: Start, pause, resume, or abort conductor loops
- **LeIndex Integration**: Token-efficient context injection using 5-phase analysis
- **Crash-Safe Persistence**: Session state saved to `~/.maestro/conductor/` with lock files

**Modes:**
- **Planning Mode**: Generate/update plans without implementation. Focus on analysis and architecture.
- **Building Mode**: Execute tasks iteratively with auto-commit and completion detection.

**Keybindings:**
- `O` / `Shift+O`: Cycle through tracks
- `Space`: Expand/collapse task nodes
- `s`: Start conductor loop
- `p`: Pause conductor loop
- `r`: Resume paused loop
- `x`: Abort conductor loop
- `?`: Show help overlay

**Safety Notes:**
- **Session Locks**: Each track has a lock file to prevent concurrent execution. Stale locks (>1 hour) are automatically cleaned.
- **Crash Recovery**: If the conductor process crashes, session state is preserved. Resume with `r` key.
- **Dangerous Mode**: When using auto-approval agents, consider enabling sandbox mode (future enhancement) for file isolation.
- **Context Budget**: LeIndex context budget is configurable (default: 50K tokens). Ultra mode (<50K) uses minimal context; Balanced mode (>50K) provides full analysis.

**State Directory:**
```
~/.maestro/conductor/
├── locks/           # Per-track lock files
├── sessions/        # Session state JSON
└── logs/            # Iteration logs (JSONL)
```

**Example Workflow:**

1. Launch Cockpit: `maestro tui`
2. Navigate to Analysis tab and select a track
3. Select a track using `O` key
4. Press `s` to start conductor loop
5. Monitor progress in live output panel
6. Press `p` to pause if needed
7. Press `r` to resume
8. Press `x` to abort when complete

**Completion Detection:**
The conductor engine detects task completion through:
- Plan.md status marker updates (`[~]` → `[x]`)
- Git commits with descriptive messages
- `<promise>COMPLETE</promise>` token in agent output
- Backpressure validation (tests passing)

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

Maestro v2.5 includes comprehensive testing infrastructure:

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
- **Conductor Module**: Inspired by [subsy/ralph-tui](https://github.com/subsy/ralph-tui) (MIT License) - Terminal UI for autonomous task execution
- **Ralph Methodology**: Inspired by [ghuntley/how-to-ralph-wiggum](https://github.com/ghuntley/how-to-ralph-wiggum) - Autonomous AI development patterns

The Conductor module in Maestro Cockpit implements concepts from Ralph TUI, providing autonomous task execution with track-based planning, LeIndex-powered context injection, and crash-safe session persistence.

## Support

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/scooter-lacroix/Maestro/issues)
- Discussions: [GitHub Discussions](https://github.com/scooter-lacroix/Maestro/discussions)

---

<div align="center">

**Transform your AI-assisted development today**

[Get Started](docs/CLAUDE-CODE.md) · [Features](#key-features) · [Documentation](docs/) · [Web Dashboard](#nexus-memory-system)

**Maestro v2.5 - The Unified Development Framework**

</div>
