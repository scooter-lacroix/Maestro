# Maestro Agent Usage

Complete guide to agents used by Maestro for automatic, proactive task execution.

## Overview

Maestro automatically selects and deploys specialized agents based on task complexity. **No manual configuration required** - this is a core feature that enables professional software engineering with AI assistance.

## Philosophy

**Proactive, Not Reactive**

- **You don't ask for agents** - Maestro automatically selects them
- **Task complexity drives selection** - Not user preference
- **Fallback mechanisms** - Graceful degradation if agents unavailable
- **Mandatory review** - Oracle reviews all implementation work

## Agent Selection Logic

### Complexity Assessment

Maestro assesses each task and selects the appropriate approach:

```
Task Complexity Analysis:
├── Trivial (1-5 lines, simple changes)
│   └── → Direct implementation (no agent)
│
├── Standard (5-50 lines, single file)
│   └── → explore agent (Claude Code)
│   └── → opencode-scaffolder (OpenCode)
│
├── Complex (multiple files, >50 lines)
│   ├── Design: oracle or librarian
│   └── Implementation: explore or opencode-scaffolder
│
├── Analysis (>100KB codebase)
│   └── → librarian (Claude Code)
│   └── → gemini-analyzer (OpenCode)
│
└── Spec-driven/ambiguous requirements
    └── → oracle for specification
```

### Mandatory Agents

**Oracle (Mandatory for Implementation)**
- **Role**: Architecture, code review, strategy
- **When Used**: ALL implementation work (non-negotiable)
- **Purpose**: Quality gate before commits

## Core Agents (Claude Code)

### Oracle

**Specialty**: Architecture, code review, strategic planning

**When Used**:
- All implementation work (MANDATORY)
- Spec-driven requirements
- Complex architectural decisions
- Code review before commits

**Strengths**:
- Deep reasoning for complex problems
- Strategic planning and architecture
- Thorough code review

**Quota**: Unlimited (built-in)

**Example**:
```
Task: Design authentication microservice
→ Oracle selected for architecture design
→ Explore selected for implementation
→ Oracle reviews implementation
```

### Librarian

**Specialty**: Multi-repo analysis, documentation lookup, implementation examples

**When Used**:
- Large codebase analysis (>100KB)
- Need for implementation patterns
- Documentation research
- Cross-repo reference

**Strengths**:
- Access to extensive code examples
- Documentation research
- Pattern discovery

**Quota**: 300 requests/day (use sparingly)

**Example**:
```
Task: Implement OAuth 2.0 flow
→ Librarian researches OAuth patterns
→ Oracle designs implementation
→ Explore implements
```

### Explore

**Specialty**: Fast codebase exploration and pattern matching

**When Used**:
- Standard implementation tasks (5-50 lines)
- Codebase exploration
- Pattern matching
- Quick refactoring

**Strengths**:
- Fast and efficient
- Good for standard tasks
- Pattern recognition

**Quota**: 2000 requests/day

**Example**:
```
Task: Add validation to user input
→ Explore implements directly
→ Oracle reviews
```

### Frontend UI/UX Engineer

**Specialty**: Designer turned developer, builds gorgeous UIs

**When Used**:
- Frontend features and components
- UI/UX implementation
- Prototyping interfaces
- Visual design

**Strengths**:
- Beautiful, functional UIs
- User experience focus
- Rapid prototyping

**Quota**: Unlimited free (use liberally)

**Example**:
```
Task: Create responsive navigation component
→ Frontend-ui-ux-engineer designs and implements
→ Oracle reviews accessibility
```

### Document Writer

**Specialty**: Technical writing expert, writes prose that flows

**When Used**:
- Documentation generation
- README creation
- API documentation
- Technical guides

**Strengths**:
- Clear, flowing prose
- Technical accuracy
- Audience adaptation

**Quota**: Separate credit pool (use to preserve main quotas)

**Example**:
```
Task: Document REST API endpoints
→ Document-writer creates comprehensive docs
→ Oracle reviews technical accuracy
```

### Multimodal Looker

**Specialty**: Visual content specialist, analyzes PDFs/images/diagrams

**When Used**:
- PDF document analysis
- Image/diagram understanding
- Visual specification review
- Screenshot analysis

**Strengths**:
- Visual content processing
- Diagram interpretation
- Document extraction

**Quota**: Context-dependent

**Example**:
```
Task: Implement feature from design mockup
→ Multimodal-looker analyzes mockup
→ Oracle creates spec
→ Explore implements
```

## Orchestrator Agents

### Kilocode Orchestrator

**Specialty**: Large-scale projects with persistent memory

**When Used**:
- Projects spanning multiple sessions
- Complex coordination needs
- Persistent context required

**Strengths**:
- Memory across sessions
- Large-scale coordination
- Persistent project state

**Example**:
```
Task: Refactor across 50 files over multiple days
→ Kilocode-orchestrator maintains context
→ Coordinates multiple implementation agents
```

### LLM Council Evaluator

**Specialty**: Meta-agent selection for high-risk decisions

**When Used**:
- High-risk architectural decisions
- Complex multi-agent coordination
- Critical path selection

**Strengths**:
- Meta-reasoning
- Agent selection optimization
- Risk assessment

**Example**:
```
Task: Choose between microservices vs monolith
→ LLM-council-evaluator analyzes trade-offs
→ Recommends optimal approach
→ Oracle executes
```

## OpenCode Agents

Maestro for OpenCode uses these specialized agents:

### AMP Code

**Specialty**: ETL/ELT data pipelines

**When Used**:
- Multi-stage data engineering
- Data validation and quality gates
- Enrichment and aggregation

**Example**:
```
Task: Build data processing pipeline
→ AMP-code designs and implements
```

### Gemini Analyzer

**Specialty**: OOP-heavy enterprise internals

**When Used**:
- Large codebase architecture analysis
- Enterprise system understanding
- OOP pattern analysis

**Example**:
```
Task: Understand Java enterprise system
→ Gemini-analyzer maps architecture
```

### OpenCode Scaffolder

**Specialty**: Fast scaffolds and prototypes

**When Used**:
- Quick MVP creation
- Initial project scaffolding
- Rapid prototyping

**Example**:
```
Task: Bootstrap new microservice
→ Opencode-scaffolder creates structure
```

### Qwen Coder

**Specialty**: Tests and documentation polish

**When Used**:
- Test writing and coverage
- Documentation generation
- Code polish

**Example**:
```
Task: Add comprehensive test suite
→ Qwen-coder implements tests
```

### Codex Reviewer

**Specialty**: Modularity and validation layers

**When Used**:
- Code review
- Refactoring
- API validation

**Example**:
```
Task: Validate API design
→ Codex-reviewer checks modularity
```

## Agent Selection Examples

### Example 1: Simple Feature

```
Task: Add email validation to user registration

Assessment:
- Single file
- ~10 lines of code
- Well-defined requirement

Selection:
→ Explore implements (standard task)
→ Oracle reviews (mandatory)
```

### Example 2: Complex Feature

```
Task: Implement real-time notifications with WebSockets

Assessment:
- Multiple files (frontend + backend)
- ~200 lines of code
- Architectural decisions needed

Selection:
→ Oracle designs architecture
→ Librarian researches WebSocket patterns
→ Explore implements backend
→ Frontend-ui-ux-engineer implements UI
→ Oracle reviews all changes
```

### Example 3: Large Codebase Analysis

```
Task: Refactor authentication across entire app

Assessment:
- Large codebase (>100KB)
- Multiple modules affected
- Complex dependencies

Selection:
→ Librarian analyzes codebase structure
→ Oracle designs refactoring strategy
→ Kilocode-orchestrator coordinates
→ Multiple explore instances implement
→ Oracle validates each module
```

## Quota Awareness

Maestro is quota-aware:

| Agent | Quota | Strategy |
|-------|-------|----------|
| librarian | 300/day | Use sparingly for large analysis |
| explore | 2000/day | Use for standard implementation |
| frontend-ui-ux-engineer | Unlimited | Use liberally for UI |
| document-writer | Separate pool | Use to preserve main quotas |
| oracle | Unlimited | Use for all reviews |

## Fallback Mechanisms

### External Tool Availability Check

Before using agents requiring external CLI tools, Maestro checks:

```bash
# Check if perspective CLI tools are available
which gemini-cli 2>/dev/null && which qwen-cli 2>/dev/null
```

### Graceful Degradation

If external agents unavailable:

1. **Recommend Installation**:
   ```
   Some specialized agents require external CLI tools.
   Would you like to:
   A) Install CLI tools (recommended)
   B) Use built-in agents (limited)
   ```

2. **Use Built-in Alternatives**:
   - Built-in Claude Code reasoning
   - Native subagent capability
   - Standard model capabilities

3. **Document Decision**:
   - Note user's choice
   - Apply to future tasks

## Mandatory Pre-Commit Review

Before marking any task complete:

1. **ALL code changes reviewed by oracle**
2. **Review findings addressed**
3. **Critical issues fixed before commit**
4. **Suggestions documented if deferred**

## Best Practices

### For Users

1. **Trust automatic selection** - Complexity assessment is reliable
2. **Don't specify agents** - Let Maestro decide
3. **Focus on requirements** - Agents handle implementation
4. **Review oracle feedback** - Quality gate is important
5. **Monitor quotas** - For external agents

### For Understanding Agent Behavior

1. **Check task complexity** - Determines agent selection
2. **Review oracle recommendations** - Strategic guidance
3. **Monitor librarian usage** - Preserve quota for large analysis
4. **Leverage frontend-ui-ux-engineer** - Unlimited for UI work
5. **Use document-writer** - Saves main quota for implementation

## Troubleshooting

### Agent Not Found

**Symptom**: Agent selection fails

**Solutions**:
1. Verify agent configuration
2. Check external CLI tools (if required)
3. Use built-in fallback
4. Check agent availability in config

### Quota Exhausted

**Symptom**: Agent unavailable due to quota

**Solutions**:
1. Use alternative agent
2. Implement directly (for trivial tasks)
3. Wait for quota reset
4. Use built-in capabilities

### Poor Agent Selection

**Symptom**: Wrong agent for task

**Solutions**:
1. Refine task description
2. Check complexity assessment
3. Review manual overrides (if available)
4. Provide clearer requirements

## Pi-Mono Integration

Pi-Mono enables Maestro to leverage subagent workflows with adaptive model selection. This integration provides parallel, chain, and single execution modes with automatic model discovery based on authenticated LLM providers.

### Agent Mappings

Pi-Mono maps Maestro roles to specialized subagents:

| Maestro Role | Pi-Mono Subagent | Purpose |
|--------------|------------------|---------|
| `scout` | `scout` | Fast reconnaissance and codebase exploration |
| `architect` | `planner` | Architecture design and strategic planning |
| `critic` | `reviewer` | Code review and quality validation |
| `kraken` | `worker` | Implementation and code generation |

### Workflow Presets

Pre-configured workflows for common development patterns:

#### `/implement` (Chain Mode)
Full implementation workflow with reconnaissance and architecture:
```
scout -> architect -> kraken
```
- Scout explores codebase context
- Architect designs the implementation strategy
- Kraken implements the solution

#### `/implement-and-review` (Chain Mode)
Implementation with iterative code review:
```
kraken -> critic -> kraken
```
- Kraken creates initial implementation
- Critic reviews and provides feedback
- Kraken applies improvements

#### `/parallel-review`
Parallel critic execution for multi-file review:
- Multiple critic instances review different files simultaneously
- Configurable concurrency limit (default: 4)

### Execution Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **single** | Single subagent execution | Simple tasks, targeted operations |
| **parallel** | Concurrent subagent execution | Multi-file processing, batch operations |
| **chain** | Sequential execution with output passing | Complex workflows, iterative refinement |

Chain mode supports the `{previous}` placeholder to pass output between agents.

### Configuration

Pi-Mono configuration is stored at `~/.maestro/config/pi-mono.yaml`:

```yaml
version: "1.0"
enabled: true
path: "/home/stan/pi-mono/pi"

role_assignments:
  scout:
    model_id: "claude-haiku-4-5"
    provider: "anthropic"
    fallback_models: ["gpt-4o-mini"]
  architect:
    model_id: "claude-sonnet-4-5"
    provider: "anthropic"
    use_reasoning: true
  critic:
    model_id: "claude-sonnet-4-5"
    provider: "anthropic"
  kraken:
    model_id: "claude-sonnet-4-5"
    provider: "anthropic"

settings:
  timeout: 300
  parallel_limit: 4
  chain_mode: true
  streaming: true
```

### Pi-Mono Commands

| Command | Description |
|---------|-------------|
| `/maestro:pi-status` | Display pi-mono configuration and provider status |
| `/maestro:pi-test` | Test subagent functionality and connectivity |
| `/maestro:pi-agents` | List available pi agents and their model assignments |

### Implement Command Flags

Enhanced `/maestro:implement` with pi-mono options:

```bash
# Single agent execution
/maestro:implement --pi-agent scout "Explore authentication module"

# Chain execution (comma-separated)
/maestro:implement --pi-chain scout,architect,kraken "Build user service"

# Parallel execution
/maestro:implement --pi-parallel critic "Review all changed files"
```

### Model Discovery

Pi-Mono automatically discovers available models from authenticated providers:
- **Anthropic**: Claude models (Opus, Sonnet, Haiku)
- **OpenAI**: GPT-4, GPT-4o, GPT-4o-mini
- **Google**: Gemini models
- **Groq**: Fast inference models
- **OpenRouter**: Multi-provider access

Model discovery results are cached for 24 hours.

### Fallback Behavior

When pi-mono is unavailable, Maestro gracefully degrades:

1. **Detection Failure**: Uses built-in agent selection
2. **Provider Unavailable**: Falls back to alternative models in `fallback_models`
3. **Subagent Error**: Retries with exponential backoff, then reports failure

## See Also

- [Claude Code Guide](CLAUDE-CODE.md) - Using Maestro with Claude Code
- [OpenCode Guide](OPENCODE.md) - Using Maestro with OpenCode
- [Pi-Mono Spec](../maestro/tracks/pi-mono_20260123/spec.md) - Full integration specification
