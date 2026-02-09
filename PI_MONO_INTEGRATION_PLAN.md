# Maestro + Pi-Mono Integration Plan

**Version:** 1.0  
**Date:** January 22, 2026  
**Status:** Planning Complete - Implementation Pending

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Phase 1: Detection & Discovery System](#phase-1-detection--discovery-system)
4. [Phase 2: Adaptive Model Configuration](#phase-2-adaptive-model-configuration)
5. [Phase 3: Agent Role Mapping System](#phase-3-agent-role-mapping-system)
6. [Phase 4: Subagent Execution Engine](#phase-4-subagent-execution-engine)
7. [Phase 5: Interactive Configuration Workflow](#phase-5-interactive-configuration-workflow)
8. [Phase 6: Maestro Command Integration](#phase-6-maestro-command-integration)
9. [Phase 7: Testing & Validation](#phase-7-testing--validation)
10. [File Manifest](#file-manifest)
11. [Timeline & Milestones](#timeline--milestones)

---

## Executive Summary

Integrate pi-mono as a first-class CLI agent in Maestro's ecosystem, enabling:
- Dynamic discovery of available models in pi-mono instance
- Interactive assignment of Maestro agent roles to pi-mono subagents
- Flexible workflow execution via pi-mono's subagent system
- Adaptive configuration that updates based on authenticated providers

### Key Differentiators

- **Adaptive Model Selection**: Users select from their actually-authenticated models
- **Interactive Role Assignment**: maestro:configure offers visual agent mapping
- **Workflow Presets**: Pre-configured chains like /implement map to pi-mono workflows
- **Isolated Context Windows**: Each subagent gets fresh context
- **Streaming Intelligence**: Real-time progress from subagents in Maestro UI

---

## Architecture Overview

```
+----------------------------------------------------------------------+
|                        Maestro Framework                            |
+----------------------------------------------------------------------+
|  +---------------+    +---------------+    +-------------------+  |
|  | maestro:setup |    | maestro:confi |    | maestro:implement |  |
|  +-------+-------+    +-------+-------+    +---------+---------+  |
|          |                    |                        |            |
|          v                    v                        v            |
|  +----------------------------------------------------------------+  |
|  |              Pi-Mono Integration Layer                         |  |
|  +----------------------------------------------------------------+  |
|  |  +--------------+  +----------------+  +-----------------+   |  |
|  |  | Model        |  | Agent Role     |  | Subagent        |   |  |
|  |  | Discovery    |  | Mapping        |  | Runner          |   |  |
|  |  +--------------+  +----------------+  +-----------------+   |  |
|  +----------------------------------------------------------------+  |
|                              |                                      |
|                              v                                      |
|  +----------------------------------------------------------------+  |
|  |                    Pi-Mono CLI                                |  |
|  |  +--------------------------------------------------------+   |  |
|  |  | Subagent Extension (scout, planner, reviewer, worker)  |   |  |
|  |  +--------------------------------------------------------+   |  |
|  +----------------------------------------------------------------+  |
|                              |                                      |
|                              v                                      |
|  +----------------------------------------------------------------+  |
|  |                    LLM Providers                              |  |
|  |  Anthropic (Claude) | OpenAI (GPT) | Google (Gemini) | ...   |  |
|  +----------------------------------------------------------------+  |
+----------------------------------------------------------------------+
```

---

## Phase 1: Detection & Discovery System

### Objective
Automatically detect pi-mono installation, validate subagent extension, and discover available models from authenticated providers.

### Deliverables

#### 1.1 CLI Tool Detection (maestro/utils/cli_detection.py)

Key features:
- Search paths: /home/stan/pi-mono/pi, ~/.local/bin/pi, /usr/local/bin/pi
- Validation: Check for subagent extension and CLI
- Version detection
- Capability detection (subagent, streaming, parallel, chain)

#### 1.2 Model Discovery Service (maestro/services/pi_model_discovery.py)

Key features:
- Discovers models from pi-mono --list-models
- Validates authentication status per provider
- Caches results for 24 hours
- Supports providers: Anthropic, OpenAI, Google, Groq, OpenRouter

**Configuration:**
```python
PROVIDERS = {
    "anthropic": {
        "display_name": "Anthropic",
        "env_var": "ANTHROPIC_API_KEY",
        "models": ["claude-sonnet-4-5", "claude-haiku-4-5", "claude-opus-4-5"]
    },
    "openai": {
        "display_name": "OpenAI",
        "env_var": "OPENAI_API_KEY",  
        "models": ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"]
    },
    "google": {
        "display_name": "Google",
        "env_var": "GEMINI_API_KEY",
        "models": ["gemini-2.5-flash", "gemini-2.5-pro", "gemini-1.5-pro"]
    },
    # ... Groq, OpenRouter, etc.
}
```

---

## Phase 2: Adaptive Model Configuration

### Objective
Create an adaptive configuration system that allows users to select from their actually-authenticated models, with intelligent defaults and validation.

### Deliverables

#### 2.1 Model Configuration Schema (maestro/config/pi_models.py)

**Model Tiers:**
```python
class ModelTier(Enum):
    REASONING = "reasoning"    # Complex reasoning, planning
    FAST = "fast"              # Quick tasks, lookups
    BALANCED = "balanced"      # General purpose
    VISION = "vision"          # Image understanding
    CODING = "coding"          # Code-specific optimization
```

**Configuration Structure:**
```yaml
# ~/.maestro/config/pi-mono.yaml
version: "1.0"
enabled: true
path: "/home/stan/pi-mono/pi"
version_info: "0.49.3"

providers:
  anthropic:
    display_name: "Anthropic"
    is_configured: true
    env_var: "ANTHROPIC_API_KEY"
  openai:
    display_name: "OpenAI"
    is_configured: true
    env_var: "OPENAI_API_KEY"

model_preferences:
  - model_id: "claude-sonnet-4-5"
    provider: "anthropic"
    tier: "balanced"
    is_default: true
  - model_id: "claude-haiku-4-5"
    provider: "anthropic"
    tier: "fast"
    is_default: true

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

workflow_presets:
  /implement:
    description: "Full implementation workflow"
    chain:
      - scout
      - architect
      - kraken
  /implement-and-review:
    description: "Implement with code review"
    chain:
      - kraken
      - critic
      - kraken

settings:
  timeout: 300
  parallel_limit: 4
  chain_mode: true
  streaming: true
```

#### 2.2 Model Selection UI (maestro/ui/model_selector.py)

**Role Requirements Mapping:**
```python
DEFAULT_ROLE_MAPPINGS = {
    "scout": {
        "tier": ModelTier.FAST,
        "description": "Fast recon and exploration",
        "reasoning": False,
        "coding": False,
    },
    "architect": {
        "tier": ModelTier.REASONING,
        "description": "Planning and architecture design",
        "reasoning": True,
        "coding": False,
    },
    "critic": {
        "tier": ModelTier.BALANCED,
        "description": "Code review and quality assessment",
        "reasoning": False,
        "coding": True,
    },
    "kraken": {
        "tier": ModelTier.CODING,
        "description": "Implementation with TDD",
        "reasoning": False,
        "coding": True,
    },
    "oracle": {
        "tier": ModelTier.REASONING,
        "description": "Strategic analysis and research",
        "reasoning": True,
        "coding": False,
    },
    "librarian": {
        "tier": ModelTier.BALANCED,
        "description": "Documentation and external research",
        "reasoning": False,
        "coding": False,
    },
}
```

**Interactive Selection Features:**
- Shows only authenticated models
- Groups by provider
- Auto-filters based on role requirements (reasoning, coding)
- Supports fallback model selection
- Real-time preview of selections

#### 2.3 Configuration Wizard (maestro/ui/config_wizard.py)

**Wizard Steps:**
1. **Detection**: Verify pi-mono installation
2. **Provider Review**: Show authenticated/unauthenticated providers
3. **Model Selection**: Interactive model picker per tier
4. **Role Assignment**: Map Maestro roles to pi agents
5. **Workflow Config**: Configure workflow presets
6. **Confirmation**: Review and save

**Sample Session:**
```
🔍 Detecting pi-mono installation...
✅ Found pi-mono at: /home/stan/pi-mono/pi
   Version: 0.49.3
   Subagent extension: configured

🌐 Discovering available models...

📦 Provider Status:
------------------------
✅ Anthropic: 3 models authenticated
✅ OpenAI: 2 models authenticated  
❌ Google: Not authenticated

🎯 Model Selection
========================
Select models for different task tiers:

📌 FAST TIER
------------------------
   anthropic:
      👉 claude-haiku-4-5
         gpt-4o-mini

📌 BALANCED TIER  
------------------------
   anthropic:
      👉 claude-sonnet-4-5
         gpt-4o

🤖 Role Assignment
========================
Assign pi-mono agents to Maestro roles:

🔹 SCOUT
   Fast codebase recon
   Recommended tier: fast
   anthropic/claude-haiku-4-5 👈 SELECTED

🔹 ARCHITECT
   Planning and architecture
   Recommended tier: reasoning
   anthropic/claude-sonnet-4-5 👈 SELECTED
```

---

## Phase 3: Agent Role Mapping System

### Deliverables

#### 3.1 Agent Mapping Registry (maestro/core/agents/pi_mapping.py)

**Default Mappings:**
```python
DEFAULT_MAPPINGS = {
    "scout": {
        "pi_agent": "scout",
        "default_model": "claude-haiku-4-5",
        "tools": ["read", "grep", "find", "ls", "bash"],
        "complexity": "small",
    },
    "architect": {
        "pi_agent": "planner", 
        "default_model": "claude-sonnet-4-5",
        "tools": ["read", "grep", "find", "ls"],
        "complexity": "medium",
    },
    "critic": {
        "pi_agent": "reviewer",
        "default_model": "claude-sonnet-4-5", 
        "tools": ["read", "grep", "find", "ls", "bash"],
        "complexity": "small",
    },
    "kraken": {
        "pi_agent": "worker",
        "default_model": "claude-sonnet-4-5",
        "tools": ["*"],  # All default tools
        "complexity": "medium",
    },
    "oracle": {
        "pi_agent": "planner",
        "default_model": "claude-opus-4-5",
        "tools": ["read", "bash", "grep", "find"],
        "complexity": "medium",
    },
    "librarian": {
        "pi_agent": "scout",
        "default_model": "claude-sonnet-4-5",
        "tools": ["read", "grep", "find", "ls"],
        "complexity": "small",
    },
}
```

**Workflow Presets:**
```python
DEFAULT_WORKFLOWS = {
    "implement": {
        "description": "Full implementation workflow",
        "mode": "chain",
        "steps": [
            {"agent": "scout", "task": "Explore codebase for: {task}", "use_previous": False},
            {"agent": "architect", "task": "Create plan. Context: {previous}", "use_previous": True},
            {"agent": "kraken", "task": "Implement. Context: {previous}", "use_previous": True},
        ],
    },
    "scout-and-plan": {
        "description": "Research and plan workflow",
        "mode": "chain", 
        "steps": [
            {"agent": "scout", "task": "Find code related to: {task}", "use_previous": False},
            {"agent": "architect", "task": "Create plan. Context: {previous}", "use_previous": True},
        ],
    },
    "implement-and-review": {
        "description": "Implement with code review",
        "mode": "chain",
        "steps": [
            {"agent": "kraken", "task": "Implement: {task}", "use_previous": False},
            {"agent": "critic", "task": "Review. Context: {previous}", "use_previous": True},
            {"agent": "kraken", "task": "Fix issues. Context: {previous}", "use_previous": True},
        ],
    },
    "parallel-review": {
        "description": "Parallel code review",
        "mode": "parallel",
        "steps": [
            {"agent": "critic", "task": "Review models and data layer", "use_previous": False},
            {"agent": "critic", "task": "Review controllers and views", "use_previous": False},
            {"agent": "critic", "task": "Review utilities and helpers", "use_previous": False},
        ],
        "parallel_limit": 3,
    },
}
```

---

## Phase 4: Subagent Execution Engine

### Deliverables

#### 4.1 Subagent Runner (maestro/core/agents/pi_runner.py)

**Features:**
- Single execution mode
- Parallel execution (up to 4 concurrent)
- Chain execution with output passing ({previous} placeholder)
- Real-time streaming via callback
- Usage tracking
- Error handling with retries

**API Usage:**

```python
runner = PiSubagentRunner(
    pi_path="/home/stan/pi-mono/pi",
    config=config,
)

# Single execution
result = await runner.execute(
    agent="scout",
    task="Find authentication code",
    on_stream=lambda e: print(e.type, e.data),
)

# Parallel execution
results = await runner.execute_parallel([
    {"agent": "critic", "task": "Review models"},
    {"agent": "critic", "task": "Review controllers"},
    {"agent": "critic", "task": "Review views"},
], max_concurrent=3)

# Chain execution
result = await runner.execute_chain([
    {"agent": "scout", "task": "Find relevant code"},
    {"agent": "architect", "task": "Create implementation plan"},
    {"agent": "kraken", "task": "Implement the solution"},
])
```

**Result Structure:**
```python
@dataclass
class SubagentResult:
    success: bool
    task: str
    agent: str
    output: str = ""
    error: Optional[str] = None
    exit_code: int = 0
    duration_seconds: float = 0.0
    usage: Dict[str, int] = field(default_factory=dict)
    messages: List[Dict[str, Any]] = field(default_factory=list)
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
```

---

## Phase 5: Interactive Configuration Workflow

### Deliverables

#### 5.1 Configuration Skill (maestro/skills/maestro-pi-integration/configure.py)

**Usage:**
```bash
/maestro:configure --pi-mono
```

**Process:**
1. Detect pi-mono installation
2. Discover available models
3. Show interactive model selector
4. Allow role assignment
5. Configure workflow presets
6. Save configuration

---

## Phase 6: Maestro Command Integration

### Deliverables

#### 6.1 Enhanced Implement Command

**New Options:**
```bash
/maestro:implement user-auth --pi-agent scout
/maestro:implement user-auth --pi-chain scout,architect,kraken
/maestro:implement user-auth --pi-parallel critic,critic
```

#### 6.2 New Commands

```bash
/maestro:pi-status              # Show pi-mono configuration
/maestro:pi-test               # Test subagent functionality
/maestro:pi-agents             # List available pi agents
```

---

## Phase 7: Testing & Validation

### Test Suite Structure

```
maestro/tests/test_pi_integration/
├── test_detection.py           # CLI detection tests
├── test_model_discovery.py     # Model discovery tests
├── test_model_selection.py     # Model selection UI tests
├── test_agent_mapping.py       # Role mapping tests
├── test_subagent_runner.py     # Execution engine tests
├── test_workflows.py           # Workflow execution tests
├── test_config_wizard.py       # Configuration wizard tests
├── test_implement_command.py   # Enhanced implement tests
├── test_e2e.py                 # End-to-end integration tests
└── fixtures/
    ├── pi_installations/       # Mock pi installations
    ├── authenticated_providers.json
    └── model_responses.json
```

**Coverage Target:** 90%+

---

## File Manifest

### New Files
```
maestro/core/agents/
├── __init__.py
├── pi_mapping.py              # Agent role mapping system
└── pi_runner.py               # Subagent execution engine

maestro/config/
├── __init__.py
└── pi_models.py               # Model configuration schema

maestro/services/
└── pi_model_discovery.py      # Model discovery service

maestro/ui/
├── __init__.py
├── model_selector.py          # Adaptive model selector
└── config_wizard.py           # Configuration wizard

maestro/utils/
└── cli_detection.py           # CLI tool detection

maestro/skills/maestro-pi-integration/
├── __init__.py
├── configure.py               # Configuration skill
├── status.py                  # Status skill
└── test.py                    # Test skill
```

### Modified Files
```
maestro/core/agents/selector.py    # Add pi-aware selection
maestro/config/settings.py         # Add pi-mono config section
maestro/skills/implement.py       # Add pi-mono options
maestro/skills/setup.py           # Add pi-mono setup step
maestro/skills/configure.py       # Add pi-mono configure option
maestro/cli.py                    # Add pi-mono CLI commands
```

---

## Timeline & Milestones

| Phase | Duration | Key Deliverables | Week |
|-------|----------|------------------|------|
| 1. Detection & Discovery | Week 1-2 | CLI detection, model discovery | 1-2 |
| 2. Model Configuration | Week 2-3 | Adaptive selector, config wizard | 2-3 |
| 3. Agent Mapping | Week 3-4 | Role mappings, workflow presets | 3-4 |
| 4. Execution Engine | Week 4-5 | Subagent runner, streaming | 4-5 |
| 5. Command Integration | Week 5-6 | Enhanced commands | 5-6 |
| 6. Testing | Week 6-7 | Comprehensive tests | 6-7 |
| 7. Documentation | Week 7 | User guides, examples | 7 |

---

## Success Criteria

1. **Detection**: Automatically finds pi-mono installation
2. **Model Discovery**: Lists only authenticated models
3. **Role Mapping**: Flexible assignment of Maestro roles to pi agents
4. **Execution**: Successfully runs subagents with streaming
5. **Configuration**: Intuitive interactive setup
6. **Integration**: Seamless use in /maestro:implement
7. **Testing**: 90%+ code coverage

---

## Configuration File Example

```yaml
# ~/.maestro/config/pi-mono.yaml
version: "1.0"
enabled: true
path: "/home/stan/pi-mono/pi"
version_info: "0.49.3"

providers:
  anthropic:
    display_name: "Anthropic"
    is_configured: true
    env_var: "ANTHROPIC_API_KEY"
  openai:
    display_name: "OpenAI"
    is_configured: true
    env_var: "OPENAI_API_KEY"
  google:
    display_name: "Google"
    is_configured: false
    env_var: "GEMINI_API_KEY"

model_preferences:
  - model_id: "claude-sonnet-4-5"
    provider: "anthropic"
    tier: "balanced"
    is_default: true
  - model_id: "claude-haiku-4-5"
    provider: "anthropic"
    tier: "fast"
    is_default: true
  - model_id: "gpt-4o"
    provider: "openai"
    tier: "coding"
    is_default: false

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

workflow_presets:
  /implement:
    description: "Full implementation workflow"
    chain:
      - scout
      - architect
      - kraken
  /implement-and-review:
    description: "Implement with code review"
    chain:
      - kraken
      - critic
      - kraken

settings:
  timeout: 300
  parallel_limit: 4
  chain_mode: true
  streaming: true
```

---

## API Reference

### PiMonoConfig

```python
# Load configuration
config = PiMonoConfig.load()

# Get model for role
assignment = config.get_model_for_role("scout")
print(f"Scout uses: {assignment.model_id} ({assignment.provider})")

# Set model for role
config.set_model_for_role(
    "critic",
    model_id="gpt-4o",
    provider="openai",
    fallback_models=["claude-sonnet-4-5"]
)

# Save configuration
config.save()
```

### PiSubagentRunner

```python
runner = PiSubagentRunner(
    pi_path="/home/stan/pi-mono/pi",
    config=config,
)

# Single execution
result = await runner.execute(
    agent="scout",
    task="Find authentication code",
    on_stream=lambda e: print(e.type, e.data),
)

# Parallel execution
results = await runner.execute_parallel([
    {"agent": "critic", "task": "Review models"},
    {"agent": "critic", "task": "Review controllers"},
])

# Chain execution
result = await runner.execute_chain([
    {"agent": "scout", "task": "Find relevant code"},
    {"agent": "architect", "task": "Create implementation plan"},
    {"agent": "kraken", "task": "Implement"},
])
```

---

**Document Version:** 1.0  
**Last Updated:** January 22, 2026  
**Location:** /home/stan/Prod/maestro/PI_MONO_INTEGRATION_PLAN.md
