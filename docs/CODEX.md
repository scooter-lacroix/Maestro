# Maestro for Codex CLI

Complete guide to using Maestro with OpenAI's Codex CLI.

## What is Maestro for Codex?

Maestro is a spec-driven development framework that integrates seamlessly with Codex CLI's agent ecosystem, providing structured project planning, track-based development, and automatic agent selection based on task complexity.

## Installation

See [Installation Guide](INSTALLATION.md) for complete setup instructions.

Quick install:
```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash
```

In the Conductor Wizard, ensure **Codex CLI** is enabled.

## What Gets Installed

- **Custom Prompts**: Maestro command prompts at `${CODEX_HOME:-~/.codex}/prompts/`
- **MCP Configuration**: LeIndex MCP server entry in `${CODEX_HOME:-~/.codex}/config.toml`

## Post-Installation Configuration

**Recommended:** After installation, run `/prompts:maestro:configure` to set up enhanced agent capabilities.

```
/prompts:maestro:configure
```

This will:
- Check for external CLI tools (gemini, qwen, opencode)
- Create agent configurations for available tools
- Set up API key requirements
- Verify tool functionality
- Configure fallback behavior for missing tools

**Benefits of /prompts:maestro:configure:**
- Access to specialized agents for large codebase analysis (gemini-analyzer)
- Enhanced refactoring and test generation (qwen-coder)
- Production-quality code review (opencode-scaffolder)
- Automatic fallback to built-in agents if CLI tools unavailable

**Note:** Maestro works without `/prompts:maestro:configure`, but with reduced agent capabilities.

## Quick Start

### 1. Initialize Your Project

In your project directory, run:

```
/prompts:maestro:setup
```

Maestro will:
- Detect if this is a new or existing project
- For existing projects: Analyze your codebase
- Guide you through product definition
- Help select tech stack and workflow
- Set up initial track

### 2. Create a Track

```
/prompts:maestro:newTrack "Add user authentication with JWT tokens"
```

Maestro will:
- Ask 3-5 clarifying questions
- Generate a comprehensive specification
- Create a detailed implementation plan
- Register the track

### 3. Implement the Track

```
/prompts:maestro:implement user-auth-jwt
```

Maestro will:
- Load the specification and plan
- Assess task complexity
- Automatically select appropriate Codex agents
- Execute TDD workflow (test → implement → refactor)
- Track progress and commit changes

### 4. Check Progress

```
/prompts:maestro:status
```

## Command Reference

### `/prompts:maestro:setup`

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

### `/prompts:maestro:newTrack <description>`

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
/prompts:maestro:newTrack Add password reset flow with email verification
```

### `/prompts:maestro:implement [track_name]`

Execute a track's implementation plan.

**When to use**: Implementing a planned track

**What happens**:
1. Loads specification and plan
2. Iterates through tasks sequentially
3. For each task:
   - Assesses complexity
   - Selects appropriate Codex agent automatically
   - Executes TDD workflow
   - Commits changes
   - Updates plan.md
4. Stores progress to memory

**Automatic Agent Selection**:

| Task Type | Agent |
|-----------|-------|
| ETL/data pipelines | amp-code |
| Large codebase analysis | gemini-analyzer |
| Rapid prototyping | opencode-scaffolder |
| Refactoring/tests | qwen-coder |
| Security review | codex-reviewer |
| Complex orchestration | kilocode-orchestrator |

**Example**:
```
/prompts:maestro:implement password-reset
```

### `/prompts:maestro:status`

View current progress.

**When to use**: Checking project status

**Output includes**:
- Current phase and task
- Completion statistics
- Next pending actions
- Blockers and dependencies
- Memory context

### `/prompts:maestro:revert [track|phase|task]`

Revert previous work.

**When to use**: Undoing implementation work

## MCP Integration

### LeIndex Server

Maestro installs the LeIndex MCP server in your Codex configuration:

```toml
[mcp_servers.leindex]
command = "maestro"
args = ["mcp", "proxy", "leindex"]
```

This enables:
- **5-layer code analysis**: AST, call graph, CFG, DFG, program slicing
- **Multi-language support**: Python, JavaScript/TypeScript, Rust, Go, Java, C/C++
- **Token-efficient output**: Ultra mode for LLM consumption

### Using LeIndex in Codex

```
# Analyze current file
/maestro le-index analyze . --format ultra

# Run 5-phase analysis
/maestro le-index phase1 .    # Token-efficient AST
/maestro le-index phase2 .    # Call graph
/maestro le-index phase3 .    # Control flow graph
/maestro le-index phase4 .    # Data flow graph
/maestro le-index phase5 .    # Program slicing
```

<<<<<<< HEAD
## Pi-Mono Integration

Maestro v2.5 includes Pi-Mono integration for subagent workflows with adaptive model selection.

### Available Commands

| Command | Description |
|---------|-------------|
| `/prompts:maestro:pi-status` | Show Pi-Mono configuration |
| `/prompts:maestro:pi-test` | Test subagent functionality |
| `/prompts:maestro:pi-agents` | List available pi agents |

### Implementation Flags

When using `/prompts:maestro:implement`, you can specify Pi-Mono execution modes:

```bash
# Single agent execution
/prompts:maestro:implement my-track --pi-agent scout

# Chain execution (sequential)
/prompts:maestro:implement my-track --pi-chain scout,architect,critic

# Parallel execution
/prompts:maestro:implement my-track --pi-parallel scout,kraken
```

**Available Pi Agents**: `scout`, `architect`, `critic`, `kraken`

### Configuration

Pi-Mono settings are stored in: `~/.maestro/config/pi-mono.yaml`

=======
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
## Troubleshooting

### "Maestro prompt not found"
**Solution**: Ensure Maestro was installed for Codex CLI
```bash
ls ${CODEX_HOME:-~/.codex}/prompts/maestro:*.md
```

### "LeIndex not available"
**Solution**: Check MCP configuration
```bash
cat ${CODEX_HOME:-~/.codex}/config.toml | grep -A5 "\[mcp_servers.leindex\]"
```

### "Config file not found"
**Solution**: Set CODEX_HOME or ensure ~/.codex exists
```bash
export CODEX_HOME=~/.codex
mkdir -p $CODEX_HOME
```

## Best Practices

1. **Always run setup first** - Ensures proper context
2. **Be specific in descriptions** - Better specs = better plans
3. **Trust automatic agent selection** - Codex integration is smart
4. **Check status regularly** - Stay aligned with progress
5. **Use revert carefully** - Can undo significant work
6. **Leverage LeIndex analysis** - Get deep code insights

## Examples

### Example 1: ETL Pipeline

```bash
# 1. Initialize
mkdir etl-project && cd etl-project
/prompts:maestro:setup
# → Python, pandas, airflow
# → pytest, pytest-cov

# 2. Create track
/prompts:maestro:newTrack Build data validation pipeline

# 3. Implement (amp-code selected automatically)
/prompts:maestro:implement data-validation-pipeline
```

### Example 2: Microservice

```bash
# 1. Initialize
mkdir user-service && cd user-service
/prompts:maestro:setup
# → Node.js, Express, PostgreSQL
# → Jest, supertest

# 2. Create track
/prompts:maestro:newTrack Implement user CRUD operations

# 3. Implement (opencode-scaffolder for prototype)
/prompts:maestro:implement user-crud
```

### Example 3: Test Coverage

```bash
# 1. In existing project
cd existing-project
/prompts:maestro:setup

# 2. Create track
/prompts:maestro:newTrack Add comprehensive test coverage

# 3. Implement (qwen-coder selected automatically)
/prompts:maestro:implement test-coverage
```

## See Also

- [Claude Code Guide](CLAUDE-CODE.md) - Using Maestro with Claude Code
- [OpenCode Guide](OPENCODE.md) - Using Maestro with OpenCode
- [Agent Reference](AGENTS.md) - Detailed agent documentation
