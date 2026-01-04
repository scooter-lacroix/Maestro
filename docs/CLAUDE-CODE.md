# Maestro for Claude Code

Complete guide to using Maestro with Claude Code.

## What is Maestro for Claude Code?

Maestro is a spec-driven development framework that transforms Claude Code into a professional software engineering environment. It provides structured project planning, track-based development, automatic agent selection, and TDD workflow enforcement.

## Installation

### One-Line Installer

```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/maestro/master/install-claude-code.sh | bash
```

### Manual Installation

1. Clone the repository:
```bash
git clone https://github.com/scooter-lacroix/maestro.git
cd maestro
```

2. Run the installer:
```bash
./install-claude-code.sh
```

### What Gets Installed

- **Commands**: 5 slash commands in `~/.claude/commands/`
  - `maestro:setup.md`
  - `maestro:newTrack.md`
  - `maestro:implement.md`
  - `maestro:status.md`
  - `maestro:revert.md`

- **Templates**: Project templates in `~/.claude/maestro-templates/`
  - `workflow.md`
  - `code_styleguides/*.md`

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
