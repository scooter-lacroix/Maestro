# Maestro for Qwen Code

Complete guide to using Maestro with QwenLM's Qwen Code CLI.

## What is Maestro for Qwen?

Maestro is a spec-driven development framework that integrates seamlessly with Qwen Code's agent ecosystem, providing structured project planning, track-based development, and automatic agent selection based on task complexity.

## Installation

See [Installation Guide](INSTALLATION.md) for complete setup instructions.

Quick install:
```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install.sh | bash
```

In the Conductor Wizard, ensure **Qwen Code** is enabled.

## What Gets Installed

- **Custom Commands**: TOML command files at `~/.qwen/commands/maestro/`
- **MCP Configuration**: LeIndex MCP server entry in `~/.qwen/settings.json`

## Post-Installation Configuration

**Recommended:** After installation, run `/maestro:configure` to set up enhanced agent capabilities.

```
/maestro:configure
```

This will:
- Check for external CLI tools (codex, gemini, opencode)
- Create agent configurations for available tools
- Set up API key requirements
- Verify tool functionality
- Configure fallback behavior for missing tools

**Benefits of /maestro:configure:**
- Access to specialized agents for large codebase analysis (codex-reviewer)
- Enhanced refactoring and test generation (gemini-analyzer)
- Production-quality code review (opencode-scaffolder)
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
/maestro:newTrack "Add user authentication with JWT tokens"
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
- Automatically select appropriate Qwen agents
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
   - Selects appropriate Qwen agent automatically
   - Executes TDD workflow
   - Commits changes
   - Updates plan.md
4. Stores progress to memory

**Automatic Agent Selection**:

| Task Type | Agent |
|-----------|-------|
| ETL/data pipelines | amp-code |
| Large codebase analysis | codex-reviewer |
| Rapid prototyping | opencode-scaffolder |
| Refactoring/tests | qwen-coder |
| Security review | gemini-analyzer |
| Complex orchestration | kilocode-orchestrator |

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

### `/maestro:revert [track|phase|task]`

Revert previous work.

**When to use**: Undoing implementation work

### `/maestro:leindex`

Access LeIndex code analysis capabilities.

**Subcommands**:
- `analyze` - Analyze source code files
- `phase1-5` - Run 5-layer analysis
- `context` - Generate context bundle for orchestration

## MCP Integration

### LeIndex Server

Maestro installs the LeIndex MCP server in your Qwen configuration:

```json
{
  "mcpServers": {
    "leindex": {
      "command": "maestro",
      "args": ["mcp", "proxy", "leindex"]
    }
  }
}
```

This enables:
- **5-layer code analysis**: AST, call graph, CFG, DFG, program slicing
- **Multi-language support**: Python, JavaScript/Typecript, Rust, Go, Java, C/C++
- **Token-efficient output**: Ultra mode for LLM consumption

### Using LeIndex in Qwen

```
# Analyze current file
maestro le-index analyze . --format ultra

# Run 5-phase analysis
maestro le-index phase1 .    # Token-efficient AST
maestro le-index phase2 .    # Call graph
maestro le-index phase3 .    # Control flow graph
maestro le-index phase4 .    # Data flow graph
maestro le-index phase5 .    # Program slicing
```

<<<<<<< HEAD
## Pi-Mono Integration

Maestro v2.5 includes Pi-Mono integration for subagent workflows with adaptive model selection.

### Available Commands

| Command | Description |
|---------|-------------|
| `/maestro:pi-status` | Show Pi-Mono configuration |
| `/maestro:pi-test` | Test subagent functionality |
| `/maestro:pi-agents` | List available pi agents |

### Implementation Flags

When using `/maestro:implement`, you can specify Pi-Mono execution modes:

```bash
# Single agent execution
/maestro:implement my-track --pi-agent scout

# Chain execution (sequential)
/maestro:implement my-track --pi-chain scout,architect,critic

# Parallel execution
/maestro:implement my-track --pi-parallel scout,kraken
```

**Available Pi Agents**: `scout`, `architect`, `critic`, `kraken`

### Configuration

Pi-Mono settings are stored in: `~/.maestro/config/pi-mono.yaml`

=======
>>>>>>> 5e3f2afb (feat(v2.5-phase5): Extract state types to dedicated module)
## Troubleshooting

### "Command not found: /maestro:*"
**Solution**: Ensure Maestro was installed for Qwen Code
```bash
ls ~/.qwen/commands/maestro/*.toml
```

### "LeIndex not available"
**Solution**: Check MCP configuration
```bash
cat ~/.qwen/settings.json | grep -A5 "mcpServers"
```

### "Settings file not found"
**Solution**: Ensure ~/.qwen exists
```bash
mkdir -p ~/.qwen
```

## Best Practices

1. **Always run setup first** - Ensures proper context
2. **Be specific in descriptions** - Better specs = better plans
3. **Trust automatic agent selection** - Qwen integration is smart
4. **Check status regularly** - Stay aligned with progress
5. **Use revert carefully** - Can undo significant work
6. **Leverage LeIndex analysis** - Get deep code insights

## Examples

### Example 1: Test Generation

```bash
# 1. Initialize
cd python-project
/maestro:setup

# 2. Create track
/maestro:newTrack Add comprehensive test coverage

# 3. Implement (qwen-coder for tests and documentation)
/maestro:implement test-coverage
```

### Example 2: Refactoring

```bash
# 1. Initialize
cd legacy-project
/maestro:setup

# 2. Create track
/maestro:newTrack Refactor to clean architecture

# 3. Implement (qwen-coder for refactoring)
/maestro:implement clean-arch-refactor
```

### Example 3: Documentation Polish

```bash
# 1. In existing project
cd api-project
/maestro:setup

# 2. Create track
/maestro:newTag Improve API documentation

# 3. Implement (qwen-coder for documentation generation)
/maestro:implement api-docs-improvement
```

## See Also

- [Claude Code Guide](CLAUDE-CODE.md) - Using Maestro with Claude Code
- [OpenCode Guide](OPENCODE.md) - Using Maestro with OpenCode
- [Agent Reference](AGENTS.md) - Detailed agent documentation
