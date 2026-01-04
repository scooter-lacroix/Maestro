# Maestro for OpenCode

Complete guide to using Maestro with OpenCode.

## What is Maestro for OpenCode?

Maestro is a spec-driven development framework that integrates seamlessly with OpenCode's agent ecosystem, providing structured project planning, track-based development, and automatic agent selection based on task complexity.

## Installation

### One-Line Installer

```bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/maestro/main/install-opencode.sh | bash
```

### Manual Installation

1. Clone the repository:
```bash
git clone https://github.com/scooter-lacroix/maestro.git
cd maestro
```

2. Run the installer:
```bash
./install-opencode.sh
```

### What Gets Installed

- **Skill**: OpenCode skill at `~/.opencode/skill/maestro/`
- **Commands**: Command files in `~/.claude/commands/`
- **Templates**: Project templates in `~/.claude/maestro-templates/`
- **Configuration**: OpenCode command entries in `~/.config/opencode/opencode.json`

## Quick Start

### 1. Initialize Your Project

In your project directory, run:

```
/maestro setup
```

Maestro will:
- Detect if this is a new or existing project
- For existing projects: Analyze your codebase
- Guide you through product definition
- Help select tech stack and workflow
- Set up initial track

### 2. Create a Track

```
/maestro newTrack "Add user authentication with JWT tokens"
```

Maestro will:
- Ask 3-5 clarifying questions
- Generate a comprehensive specification
- Create a detailed implementation plan
- Register the track

### 3. Implement the Track

```
/maestro implement user-auth-jwt
```

Maestro will:
- Load the specification and plan
- Assess task complexity
- Automatically select appropriate OpenCode agents
- Execute TDD workflow (test → implement → refactor)
- Track progress and commit changes

### 4. Check Progress

```
/maestro status
```

## Command Reference

### `/maestro setup`

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

### `/maestro newTrack <description>`

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
/maestro newTrack Add password reset flow with email verification
```

### `/maestro implement [track_name]`

Execute a track's implementation plan.

**When to use**: Implementing a planned track

**What happens**:
1. Loads specification and plan
2. Iterates through tasks sequentially
3. For each task:
   - Assesses complexity
   - Selects appropriate OpenCode agent automatically
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
/maestro implement password-reset
```

### `/maestro status`

View current progress.

**When to use**: Checking project status

**Output includes**:
- Current phase and task
- Completion statistics
- Next pending actions
- Blockers and dependencies
- Memory context

### `/maestro revert [track|phase|task]`

Revert previous work.

**When to use**: Undoing implementation work

## OpenCode Integration

### Agent Ecosystem

Maestro integrates with all OpenCode agents:

#### Specialized Agents

**amp-code**
- **Specialty**: ETL/ELT data pipelines
- **When Used**: Multi-stage data engineering, validation, enrichment
- **Example**: Building data processing pipelines

**gemini-analyzer**
- **Specialty**: OOP-heavy enterprise analysis
- **When Used**: Large codebase architecture analysis
- **Example**: Understanding enterprise Java systems

**opencode-scaffolder**
- **Specialty**: Fast prototyping
- **When Used**: Quick MVP, initial scaffolding
- **Example**: Bootstrapping a new microservice

**qwen-coder**
- **Specialty**: Tests and documentation polish
- **When Used**: Test writing, documentation generation
- **Example**: Adding comprehensive test coverage

**codex-reviewer**
- **Specialty**: Modularity and validation
- **When Used**: Code review, refactoring
- **Example**: Validating API design

**kilocode-orchestrator**
- **Specialty**: Large-scale orchestration
- **When Used**: Complex multi-file changes
- **Example**: Coordinating microservice updates

### Command Mappings

Maestro skill maps to command files:

```
/maestro setup → ~/.claude/commands/maestro:setup.md
/maestro newTrack → ~/.claude/commands/maestro:newTrack.md
/maestro implement → ~/.claude/commands/maestro:implement.md
/maestro status → ~/.claude/commands/maestro:status.md
/maestro revert → ~/.claude/commands/maestro:revert.md
```

### Configuration

Commands are registered in `~/.config/opencode/opencode.json`:

```json
{
  "command": {
    "maestro": {
      "template": "Load Maestro skill. Available: setup, newTrack, implement, status, revert.",
      "description": "Maestro spec-driven development framework"
    },
    "maestro:setup": {
      "template": "Read and execute from ~/.claude/commands/maestro:setup.md with args: $ARGUMENTS",
      "description": "Maestro setup command"
    }
  }
}
```

## Dependencies

### Required MCPs

**nexus-memory**
- Project context storage
- Track completion history
- Workflow preferences
- Categorized memory storage
- Agent-specific contexts
- Semantic search and retrieval

### Optional Enhancements

**prompt-enhancer**
- Context-aware question generation
- Improved specification quality

## Workflow

Maestro enforces a structured workflow with OpenCode integration:

### 1. Specification Phase
- Interactive requirements gathering
- Prompt enhancer integration
- Comprehensive spec.md generated

### 2. Planning Phase
- Detailed task breakdown
- OpenCode agent consideration
- TDD-compliant tasks

### 3. Implementation Phase
- Sequential task execution
- Automatic OpenCode agent selection
- TDD enforced (test → implement → refactor)
- Progress tracking in plan.md

### 4. Completion Phase
- Documentation synchronization
- Memory storage
- Track archival

## Memory Integration

### Nexus Memory
- Project context
- Track completion
- Workflow preferences
- Categorized storage
- Agent-specific contexts
- Semantic search and retrieval

Automatically updated during workflow.

## Troubleshooting

### "Maestro is not set up"
**Solution**: Run `/maestro setup` first

### "tracks.md not found"
**Solution**: Run `/maestro setup` to initialize

### Agent not found
**Solution**:
1. Check OpenCode agent configuration
2. Verify agent in `opencode.json`
3. Check agent availability

### Memory errors
**Solution**:
1. Verify Maestro memory service is running
2. Check memory configuration
3. Review memory logs for errors

### Commands not recognized
**Solution**:
```bash
# Verify command entries
cat ~/.config/opencode/opencode.json | grep maestro
```

### Templates not found
**Solution**:
```bash
ls ~/.claude/maestro-templates/
# Should show workflow.md and code_styleguides/
```

## Best Practices

1. **Always run setup first** - Ensures proper context
2. **Be specific in descriptions** - Better specs = better plans
3. **Trust automatic agent selection** - OpenCode integration is smart
4. **Check status regularly** - Stay aligned with progress
5. **Use revert carefully** - Can undo significant work
6. **Leverage agent specialties** - Each agent has unique strengths

## Examples

### Example 1: ETL Pipeline

```bash
# 1. Initialize
mkdir etl-project && cd etl-project
/maestro setup
# → Python, pandas, airflow
# → pytest, pytest-cov

# 2. Create track
/maestro newTrack Build data validation pipeline

# 3. Implement (amp-code selected automatically)
/maestro implement data-validation-pipeline
```

### Example 2: Microservice

```bash
# 1. Initialize
mkdir user-service && cd user-service
/maestro setup
# → Node.js, Express, PostgreSQL
# → Jest, supertest

# 2. Create track
/maestro newTrack Implement user CRUD operations

# 3. Implement (opencode-scaffolder for prototype)
/maestro implement user-crud
```

### Example 3: Test Coverage

```bash
# 1. In existing project
cd existing-project
/maestro setup

# 2. Create track
/maestro newTrack Add comprehensive test coverage

# 3. Implement (qwen-coder selected automatically)
/maestro implement test-coverage
```

## Advanced Features

### Prompt Enhancer Integration
Context-aware question generation for better specifications.

### Git-Aware Tracking
- Commits tracked alongside plan tasks
- Git notes for task summaries

### Multi-Project Support
Each directory has independent Maestro state.

### Resume Capability
Setup can resume from any step.

## Skill Structure

```
~/.opencode/skill/maestro/
├── SKILL.md              # Main skill definition
├── README.md             # Skill documentation
├── commands/             # Command symlinks
│   ├── maestro:setup.md
│   ├── maestro:newTrack.md
│   ├── maestro:implement.md
│   ├── maestro:status.md
│   └── maestro:revert.md
├── templates/            # Template symlinks
│   ├── workflow.md
│   └── code_styleguides/
└── scripts/              # Utility scripts
    ├── load_templates.sh
    └── fix_templates.sh
```

## See Also

- [Claude Code Guide](CLAUDE-CODE.md) - Using Maestro with Claude Code
- [Agent Usage](AGENTS.md) - Detailed agent documentation
- [Migration Guide](MIGRATION.md) - From gemini-cli
