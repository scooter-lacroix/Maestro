# Maestro for Claude Code (v2.5)

Complete guide to using Maestro with Claude Code.

## What is Maestro for Claude Code?

Maestro is a spec-driven development framework that transforms Claude Code into a professional software engineering environment. It provides structured project planning, track-based development, automatic agent selection, and TDD workflow enforcement.

## Installation

See [Installation Guide](INSTALLATION.md) for complete setup instructions.

Quick install:
```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash
```

In the Conductor Wizard, ensure **Claude Code (by Anthropic)** is enabled.

## Post-Installation Configuration

**Recommended:** After installation, run `/maestro:configure` to set up enhanced agent capabilities.

```
/maestro:configure
```

This will:
- Check for external CLI tools (gemini-cli, qwen-cli, codex-cli)
- Create agent configurations for available tools
- Set up API key requirements
- Verify tool functionality
- Configure fallback behavior for missing tools

**Benefits of /maestro:configure:**
- Access to specialized agents for large codebase analysis (gemini-analyzer)
- Enhanced refactoring and test generation (qwen-coder)
- Production-quality code review (codex-reviewer)
- Automatic fallback to built-in agents if CLI tools unavailable

**Note:** Maestro works without `/maestro:configure`, but with reduced agent capabilities.

## Quick Start

### 1. Initialize Your Project

In your project directory, run:

```
/maestro:setup
```

Maestro will:
- Detect if this is a new or existing project
- For existing projects: Analyze your codebase
- Guide you through product definition
- Help select tech stack and workflow
- Set up initial track

### 2. Create a Track

```
/maestro:newTrack Add user authentication with JWT tokens
```

Maestro will:
- Ask 3-5 clarifying questions
- Generate a comprehensive specification
- Create a detailed implementation plan
- Register the track

### 3. Implement the Track

```
/maestro:implement user-auth-jwt
```

Maestro will:
- Load the specification and plan
- Assess task complexity
- Automatically select appropriate agents
- Execute TDD workflow (test → implement → refactor)
- Track progress and commit changes

### 4. Check Progress

```
/maestro:status
```

## Command Reference

### `/maestro:setup`

Initialize Maestro for a new or existing project.

**When to use**: First time in a project directory

**What happens**:
1. Detects greenfield (new) vs brownfield (existing)
2. For existing: Analyzes codebase
3. Interactive product definition
4. Tech stack selection
5. Workflow configuration
6. Code styleguide selection
7. Initial track generation

**Example output**:
```
Welcome to Maestro. I will guide you through the following steps:
1. Project Discovery
2. Product Definition
3. Configuration
4. Track Generation
```

### `/maestro:newTrack <description>`

Create a new feature, bug fix, or chore.

**When to use**: Starting any new work

**What happens**:
1. Loads project context
2. Interactive specification (3-5 questions)
3. Comprehensive spec.md generated
4. Detailed plan.md with task breakdown
5. Track registered in tracks.md

**Example**:
```
/maestro:newTrack Add password reset flow with email verification
```

### `/maestro:implement [track_name]`

Execute a track's implementation plan.

**When to use**: Implementing a planned track

**What happens**:
1. Loads specification and plan
2. Iterates through tasks sequentially
3. For each task:
   - Assesses complexity
   - Selects appropriate agent automatically
   - Executes TDD workflow
   - Commits changes
   - Updates plan.md
4. Stores progress to memory

**Pi-Mono Flags** (v2.5):
```
--pi-agent       Run with Pi-Mono agent orchestration
--pi-chain       Execute tasks in chain mode (sequential dependencies)
--pi-parallel    Execute independent tasks in parallel
```

**Automatic Agent Selection**:
```
Trivial (1-5 lines)      → Direct implementation
Standard (5-50 lines)    → explore agent
Complex (multi-file)     → oracle/librarian + explore
Analysis (>100KB)        → librarian
Spec-driven              → oracle for specification
```

**Example**:
```
/maestro:implement password-reset
/maestro:implement password-reset --pi-parallel
```

### `/maestro:status`

View current progress.

**When to use**: Checking project status

**Output includes**:
- Current phase and task
- Completion statistics
- Next pending actions
- Blockers and dependencies
- Memory context

**Example output**:
```
Project Status: On Track
Current Phase: Implementation
Current Task: Add password reset endpoint

Progress: 3/12 tasks (25%)

Next Action: Implement password reset token validation
Blockers: None
```

### `/maestro:pi-status`

View Pi-Mono agent orchestration status.

**When to use**: Checking Pi-Mono pipeline state

**Output includes**:
- Active Pi agents
- Chain/parallel execution state
- Agent task assignments
- Pipeline progress

### `/maestro:pi-test`

Run Pi-Mono integration tests.

**When to use**: Validating Pi-Mono setup and agent connectivity

### `/maestro:pi-agents`

List available Pi-Mono agents and their capabilities.

**When to use**: Discovering Pi agent ecosystem

### `/maestro:configure --pi-mono`

Configure Pi-Mono integration settings.

**What it configures**:
- Pi agent discovery
- Chain execution policies
- Parallel task limits
- Agent capability mapping

### `/maestro:revert [track|phase|task]`

Revert previous work.

**When to use**: Undoing implementation work

**Options**:
- No argument: Interactive selection menu
- `track`: Revert entire track
- `phase`: Revert specific phase
- `task`: Revert specific task

**Example**:
```
/maestro:revert password-reset
```

### `/maestro:tldr <command>`

Access Maestro's 5-layer code analysis system (TLDR) for intelligent code understanding.

**When to use**: Understanding code structure, complexity, and relationships

**Features**:
- **Layer 1 (AST)**: Extract functions, classes, imports
- **Layer 2 (Call Graph)**: Who calls what
- **Layer 3 (Control Flow)**: Code complexity and decision points
- **Layer 4 (Data Flow)**: Where data goes
- **Layer 5 (Program Slicing)**: What affects a line

**Commands**:
```bash
# Analyze file structure
/maestro:tldr ast src/auth.py

# See who calls a function
/maestro:tldr callers authenticate_user

# Analyze complexity
/maestro:tldr cfg src/utils.py

# Get LLM-ready context
/maestro:tldr context main.py

# Search by behavior
/maestro:tldr search "database connection"
```

**Automatic Integration**: TLDR hooks run automatically during your sessions, injecting relevant context before code edits.

### `/maestro:leindex <command>`

Access LeIndex - Maestro's powerful code indexing and search system.

**When to use**: Fast full-text and semantic code search

**Features**:
- Full-text search (Tantivy BM25)
- Semantic search (vector embeddings)
- 5-layer code analysis
- **Turso database backend** (v2.5) for distributed persistence
- File change tracking

**Commands**:
```bash
# Search code
/maestro:leindex search "authentication"

# RAG-style Q&A
/maestro:leindex answer "How is auth handled?"

# Analyze file
/maestro:leindex analyze src/auth.py

# Index status
/maestro:leindex status
```

**CLI Tools** (outside Claude Code):
```bash
leindex-search "pattern"
leindex stats
```

## TLDR & LeIndex Overview

Maestro includes powerful code analysis and search capabilities that work both **automatically** (via hooks) and **manually** (via commands).

### Automatic Features (Always Active)

TLDR and LeIndex work behind the scenes during your sessions:

1. **TLDR Context Injection**: Before you edit code, relevant context is automatically prepared
2. **Smart Search**: Code searches use semantic understanding, not just text matching
3. **File Read Optimization**: When you read files, TLDR provides optimized summaries

You don't need to do anything - these features are automatically available.

### Manual Features (When Needed)

Use slash commands for direct access:

| Your Question | Use This Command |
|---------------|------------------|
| "What functions are in this file?" | `/maestro:tldr ast <file>` |
| "Who calls this function?" | `/maestro:tldr callers <func>` |
| "How complex is this code?" | `/maestro:tldr cfg <file>` |
| "Where does this data go?" | `/maestro:tldr dfg <file>` |
| "What affects this line?" | `/maestro:tldr slice <file> <line>` |
| "Search for behavior" | `/maestro:leindex search "<query>"` |
| "Give Claude optimal context" | `/maestro:tldr context <target>` |

### Integration with llm-tldr

If you're familiar with [llm-tldr](https://github.com/parcadei/llm-tldr), Maestro TLDR provides equivalent functionality:

| llm-tldr Command | Maestro TLDR Equivalent |
|------------------|-------------------------|
| `tldr warm .` | `/maestro:tldr warm .` |
| `tldr context main --project .` | `/maestro:tldr context main.py` |
| `tldr impact helper_func` | `/maestro:tldr impact helper_func` |
| `tldr semantic "database"` | `/maestro:tldr search "database"` |

## Project Structure

After setup, your project will have:

```
.
├── maestro/
│   ├── product.md              # Product vision
│   ├── product-guidelines.md   # Brand voice, tone
│   ├── tech-stack.md           # Technology choices
│   ├── workflow.md             # Development workflow
│   ├── tracks.md               # Track registry
│   ├── setup_state.json        # Setup progress
│   └── tracks/
│       └── <track_id>/
│           ├── spec.md         # Track specification
│           ├── plan.md         # Implementation plan
│           └── metadata.json   # Track metadata
└── ... (your existing code)
```

## Workflow

Maestro enforces a structured workflow:

### 1. Specification Phase
- Interactive requirements gathering
- 3-5 targeted questions
- Comprehensive spec.md generated
- User approval required

### 2. Planning Phase
- Detailed task breakdown
- Follows workflow.md methodology
- TDD-compliant tasks
- Phase completion checkpoints
- User approval required

### 3. Implementation Phase
- Sequential task execution
- Automatic agent selection
- TDD enforced (test → implement → refactor)
- Progress tracking in plan.md
- Git commits with notes

### 4. Completion Phase
- Documentation synchronization
- Track archival/deletion option
- Memory storage

## Agent Usage

Maestro automatically uses agents based on task complexity:

### Core Agents

| Agent | Purpose | Trigger |
|-------|---------|---------|
| **oracle** | Architecture, review, strategy | All implementation (mandatory) |
| **librarian** | Multi-repo analysis, docs | Large codebase (>100KB) |
| **explore** | Fast exploration | Standard implementation |
| **frontend-ui-ux-engineer** | UI/UX design | Frontend features |
| **document-writer** | Technical writing | Documentation |
| **multimodal-looker** | Visual analysis | PDFs, images |

### Orchestrator Agents

| Agent | Purpose | Trigger |
|-------|---------|---------|
| **kilocode-orchestrator** | Large-scale projects | Persistent memory needed |
| **llm-council-evaluator** | Meta-selection | High-risk decisions |

### Automatic Selection

You don't need to specify agents. Maestro assesses task complexity and automatically selects the right specialist.

## TDD Workflow

Maestro enforces Test-Driven Development:

### 1. Red Phase
```bash
# Write failing test
/maestro:implement
# → Creates test file
# → Test fails as expected
```

### 2. Green Phase
```bash
# Implement to pass
# → Writes minimal code
# → All tests pass
```

### 3. Refactor Phase
```bash
# Refactor with safety
# → Improves quality
# → Tests still pass
```

### 4. Verify Coverage
```bash
# >80% coverage required
pytest --cov=app --cov-report=html
```

## Memory Integration

Maestro uses the Nexus Memory System:

### Nexus Memory
- Project context storage
- Track completion history
- Workflow preferences
- Agent-specific contexts with categorization
- Semantic search and retrieval

Automatically updated during workflow.

## Troubleshooting

### "Maestro is not set up"
**Solution**: Run `/maestro:setup` first

### "tracks.md not found"
**Solution**: Run `/maestro:setup` to initialize

### Agent not found
**Solution**:
1. Check agent is configured in Claude Code settings
2. Verify external CLI tools (if required)
3. Use built-in fallback

### Memory errors
**Solution**:
1. Verify Maestro memory service is running
2. Check memory configuration
3. Review memory logs for errors

### Template not found
**Solution**:
```bash
ls ~/.claude/maestro-templates/
# Should show workflow.md and code_styleguides/
```

## Best Practices

1. **Always run setup first** - Ensures proper context
2. **Be specific in descriptions** - Better specs = better plans
3. **Trust automatic agent selection** - Complexity assessment is reliable
4. **Check status regularly** - Stay aligned with progress
5. **Use revert carefully** - Can undo significant work
6. **Let TDD guide you** - Tests first, implementation second

## Advanced Features

### Prompt Enhancer Integration
Maestro integrates with prompt-enhancer for context-aware question generation.

### Git-Aware Tracking
- Commits tracked alongside plan tasks
- Git notes for task summaries
- Complete history

### Multi-Project Support
Each directory has independent Maestro state.

### Resume Capability
Setup can resume from any step using `setup_state.json`.

### LSP Integration (v2.5)
Maestro auto-starts language servers for enhanced code intelligence:
- **rust-analyzer** for Rust projects
- **ruff-lsp** for Python projects
- **typescript-language-server** for TypeScript/JavaScript projects

LSP provides real-time diagnostics, hover info, and go-to-definition during implementation.

### Conductor Module (v2.5)
The TUI now uses the **Conductor** module (replacing the legacy Orchestrate pane) for:
- Track visualization and navigation
- Agent task delegation
- Real-time progress monitoring
- Pi-Mono pipeline control

## Examples

### Example 1: New Web App

```bash
# 1. Initialize
mkdir my-app && cd my-app
/maestro:setup
# → React, TypeScript, Node.js
# → Jest, React Testing Library
# → ESLint, Prettier

# 2. Create track
/maestro:newTrack Build user registration with email verification

# 3. Implement
/maestro:implement user-registration

# 4. Check progress
/maestro:status
```

### Example 2: Existing Python Project

```bash
# 1. In existing project
cd existing-python-project
/maestro:setup
# → Detects brownfield
# → Analyzes codebase
# → Confirms tech stack

# 2. Add feature
/maestro:newTrack Add API rate limiting

# 3. Implement
/maestro:implement api-rate-limiting
```

## See Also

- [OpenCode Guide](OPENCODE.md) - Using Maestro with OpenCode
- [Agent Usage](AGENTS.md) - Detailed agent documentation
- [Migration Guide](MIGRATION.md) - From gemini-cli
