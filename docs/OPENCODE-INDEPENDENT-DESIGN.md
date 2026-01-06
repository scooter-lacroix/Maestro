# OpenCode Maestro Independent Structure Design

## Executive Summary

This document defines the complete independent structure for the OpenCode variant of Maestro, making it fully functional and decoupled from Claude Code dependencies.

**Status**: Design Document
**Version**: 1.0
**Date**: 2026-01-05
**Author**: Design Analysis

---

## Table of Contents

1. [Current State Analysis](#current-state-analysis)
2. [Design Principles](#design-principles)
3. [Target Directory Structure](#target-directory-structure)
4. [File-by-File Specifications](#file-by-file-specifications)
5. [OpenCode-Specific Customizations](#opencode-specific-customizations)
6. [Agent Mapping Strategy](#agent-mapping-strategy)
7. [Implementation Phases](#implementation-phases)

---

## Current State Analysis

### Existing Structure Issues

**Current Opencode Skill Location**: `~/.opencode/skill/maestro/`

**Dependencies Identified**:
1. **Symlinked Templates**:
   - `templates/workflow.md` → `~/.claude/conductor-templates/workflow.md` (BROKEN - should be maestro-templates)
   - `templates/code_styleguides/` → `~/.claude/maestro-templates/code_styleguides/`

2. **Symlinked Commands**:
   - All commands symlinked to `~/.claude/commands/maestro:*.md`
   - These commands contain Claude Code-specific references
   - Agent mappings reference Claude Code agents (oracle, librarian, explore)

3. **Missing Independent Content**:
   - No OpenCode-specific workflow.md
   - No OpenCode-specific command implementations
   - No OpenCode agent configuration
   - Template loading depends on Claude Code directories

### What Works Well

1. **SKILL.md**: Good skill definition with OpenCode context
2. **README.md**: Clear documentation for OpenCode usage
3. **Shell Scripts**: Template management utilities are functional
4. **Directory Layout**: Basic structure is sound

---

## Design Principles

### 1. Complete Independence
- No symlinks to Claude Code directories
- All files are copies or OpenCode-specific originals
- Templates live in opencode skill directory

### 2. Agent Mapping Abstraction
- Workflow uses OpenCode agent names, not Claude Code
- Command files reference OpenCode agents
- Clear mapping between abstract agent roles and concrete implementations

### 3. Shared vs Independent Files
- **Code Styleguides**: COPY from main repo (language-agnostic)
- **Workflow**: CUSTOM for OpenCode (different agent system)
- **Commands**: CUSTOM for OpenCode (different delegation mechanisms)
- **SKILL.md**: Already OpenCode-specific

### 4. Maintainability
- Templates can be updated from main repo
- Clear separation between shared content and OpenCode customizations
- Version tracking for divergent files

---

## Target Directory Structure

```
~/.opencode/skill/maestro/
├── SKILL.md                          # ✅ Already exists, OpenCode-specific
├── README.md                         # ✅ Already exists, OpenCode-specific
├── MANIFEST.md                       # 🆕 Track file differences from main repo
│
├── commands/                         # OpenCode-specific command implementations
│   ├── maestro:setup.md              # 🔄 Replace symlink with OpenCode version
│   ├── maestro:newTrack.md           # 🔄 Replace symlink with OpenCode version
│   ├── maestro:implement.md          # 🔄 Replace symlink with OpenCode version
│   ├── maestro:status.md             # 🔄 Replace symlink with OpenCode version
│   ├── maestro:revert.md             # 🔄 Replace symlink with OpenCode version
│   ├── maestro:configure.md          # 🆕 OpenCode-specific configuration
│   ├── maestro:tui.md                # 🆕 TUI integration
│   └── maestro:memory.md             # 🆕 Memory system integration
│
├── templates/                        # Independent template storage
│   ├── workflow.md                   # 🔄 COPY + CUSTOMIZE for OpenCode
│   ├── README.md                     # 🆕 Template documentation
│   │
│   └── code_styleguides/             # ✅ COPY from main repo (shared)
│       ├── README.md
│       ├── general.md
│       ├── python.md
│       ├── javascript.md
│       ├── typescript.md
│       ├── go.md
│       ├── rust.md
│       ├── java.md
│       ├── shell.md
│       ├── docker.md
│       ├── react.md
│       ├── vue.md
│       ├── nextjs.md
│       ├── nodejs.md
│       ├── graphql.md
│       └── html-css.md
│
├── scripts/                          # ✅ Already exists
│   ├── load_templates.sh             # ✅ Update to use local templates
│   ├── fix_templates.sh              # ✅ Update to use local templates
│   ├── sync_templates.sh             # 🆕 Sync from main repo
│   └── verify_installation.sh        # 🆕 Verify OpenCode integration
│
├── config/                           # 🆕 OpenCode-specific configuration
│   ├── agents.yaml                   # 🆕 OpenCode agent mappings
│   ├── workflow-config.yaml          # 🆕 Workflow defaults
│   └── opencode-integration.yaml     # 🆕 Integration settings
│
└── docs/                             # 🆕 OpenCode-specific documentation
    ├── AGENT-MAPPINGS.md             # 🆕 Agent reference guide
    ├── WORKFLOW-CUSTOMIZATION.md     # 🆕 Workflow adaptation guide
    └── TROUBLESHOOTING.md            # 🆕 OpenCode-specific issues
```

---

## File-by-File Specifications

### A. Core Skill Files (Already Exist, Minor Updates)

#### 1. SKILL.md
**Status**: ✅ Exists, needs minor updates
**Action**: Update agent mappings, remove Claude Code references

**Required Changes**:
```markdown
# Change agent references from:
- **oracle**: Architecture, code review, strategy
- **librarian**: Multi-repo analysis, doc lookup
- **explore**: Fast codebase exploration

# To OpenCode agents:
- **codex-reviewer**: Architecture, code review, strategy
- **gemini-analyzer**: Multi-repo analysis, doc lookup
- **opencode-scaffolder**: Fast prototyping and scaffolding
- **qwen-coder**: Production implementation and testing
- **amp-code**: ETL/data pipeline specialist
```

#### 2. README.md
**Status**: ✅ Exists, needs updates
**Action**: Remove broken symlink references, update agent mappings

**Required Changes**:
- Update "Files" section to reflect independent structure
- Update "Templates" section (no longer symlinks)
- Update agent examples to use OpenCode agents

---

### B. Command Files (Replace Symlinks)

#### Strategy: Copy from Claude Code + OpenCode Customization

**For each command file** (`maestro:setup.md`, `maestro:newTrack.md`, etc.):

1. **Copy from**: `~/.claude/commands/maestro:*.md` OR `/home/stan/Prod/maestro/claude-code/commands/`

2. **Customizations Required**:

##### Common Replacements Across All Commands

**Agent References**:
```yaml
# BEFORE (Claude Code):
agents:
  - oracle
  - librarian
  - explore

# AFTER (OpenCode):
agents:
  - codex-reviewer
  - gemini-analyzer
  - opencode-scaffolder
  - qwen-coder
```

**Command References**:
```yaml
# BEFORE:
description: "Use /maestro:command"

# AFTER:
description: "Use maestro:command"
# (OpenCode doesn't use / prefix)
```

**Model Selection**:
```yaml
# BEFORE (Claude Code models):
model: sonnet  # or opus, haiku

# AFTER (OpenCode - use agent-specific models):
# Remove model specification - let OpenCode agent system handle it
# Or specify if agent supports model selection
```

##### Command-Specific Customizations

###### 1. maestro:setup.md
**Additional Changes**:
- Update template paths from `~/.claude/maestro-templates/` to `~/.opencode/skill/maestro/templates/`
- Remove Claude Code plugin checks
- Add OpenCode agent availability checks
- Update workflow.md copy destination

**Critical Section - Template Installation**:
```markdown
# CHANGE FROM:
cp ~/.claude/maestro-templates/workflow.md ./maestro/

# TO:
cp ~/.opencode/skill/maestro/templates/workflow.md ./maestro/
```

###### 2. maestro:newTrack.md
**Additional Changes**:
- Update prompt enhancer integration (if different for OpenCode)
- Update agent delegation examples
- Change skill loading references

**Agent Selection Logic**:
```markdown
# CHANGE FROM:
"If task complexity > threshold, use oracle for design"

# TO:
"If task complexity > threshold, use codex-reviewer for design"
```

###### 3. maestro:implement.md
**Major Changes**: This is the most critical file

**Agent Selection Table** (replace entire section):
```markdown
## OpenCode Agent Selection

### Task Complexity → Agent Mapping

**Trivial Tasks (1-5 lines, simple changes)**:
- Agent: None (direct implementation)
- Model: Default

**Standard Tasks (5-50 lines, single file)**:
- Agent: opencode-scaffolder
- Fallback: qwen-coder
- Use: Fast implementation, pattern matching

**Complex Tasks (multiple files, >50 lines)**:
- Design: codex-reviewer
- Implementation: qwen-coder or amp-code (domain-specific)
- Review: codex-reviewer (mandatory)

**Large Codebase Analysis (>100KB)**:
- Agent: gemini-analyzer
- Quota: 300 requests/day (use sparingly)

**ETL/Data Pipelines**:
- Agent: amp-code
- Specialty: Multi-stage data processing

**Security/Architecture Review**:
- Agent: codex-reviewer
- Mandatory: All implementation work
```

**Delegation Syntax**:
```markdown
# CHANGE FROM (Claude Code):
Use the Task tool to delegate to oracle agent

# TO (OpenCode):
Delegate to codex-reviewer subagent
# OR (if OpenCode uses different mechanism):
Invoke opencode agent: codex-reviewer
```

###### 4. maestro:status.md
**Minor Changes**:
- Update agent availability checks
- Change MCP server references

###### 5. maestro:revert.md
**Minor Changes**:
- Update git command handling (if different)
- Update agent references in context display

###### 6. maestro:configure.md (New Command)
**Purpose**: Configure OpenCode-specific Maestro settings

**Content Needed**:
```yaml
---
description: Configure Maestro settings for OpenCode
argument-hint: [setting] [value]
model: sonnet
---

## Configuration Options

### Agent Preferences
- Default review agent: codex-reviewer
- Default analysis agent: gemini-analyzer
- Default implementation agent: qwen-coder

### Workflow Mode
- autonomous: Automatic agent selection and progression
- manual: Require confirmation for agent usage
- checkpoint: Pause at phase boundaries

### Model Selection
- Let OpenCode agent system handle model selection
- Or override per-agent if supported
```

---

### C. Template Files

#### 1. workflow.md (COPY + HEAVY CUSTOMIZATION)

**Source**: `/home/stan/Prod/maestro/claude-code/templates/workflow.md`

**Major Customization Required**:

**Agent Usage Section** (complete rewrite):
```markdown
## Agent Usage Requirements

**CRITICAL SYSTEM DIRECTIVE: PROACTIVE AUTOMATIC AGENT USAGE**

### OpenCode Agent Selection

**Core Agents**:
- **codex-reviewer**: Architecture, code review, strategy. (MANDATORY for all implementation)
- **gemini-analyzer**: Multi-repo analysis, doc lookup, implementation examples.
- **opencode-scaffolder**: Fast codebase exploration and pattern matching.
- **qwen-coder**: Production implementation, refactoring, testing.
- **amp-code**: ETL/data pipeline specialist.

**Agent Selection Criteria** (Execute Automatically):
- **Trivial tasks (1-5 lines)**: Implement directly
- **Standard tasks (5-50 lines)**: Use opencode-scaffolder
- **Complex tasks (multiple files, >50 lines)**: Use codex-reviewer for design + qwen-coder for implementation
- **Large codebase analysis (>100KB)**: Use gemini-analyzer
- **ETL/data pipelines**: Use amp-code

**Quota Awareness**:
- gemini-analyzer: 300 requests/day
- qwen-coder: Check quota limits
- amp-code: Check quota limits
- opencode-scaffolder: Check quota limits
```

**Task Workflow Section** (update agent references):
```markdown
### 3. Assess Complexity and Select Agent (AUTOMATIC):
   - **Trivial (1-5 lines)**: Implement directly
   - **Standard (5-50 lines)**: Use opencode-scaffolder
   - **Complex (multiple files)**: Use codex-reviewer for design + qwen-coder/amp-code for implementation
   - **Analysis (>100KB)**: Use gemini-analyzer
```

**Tzar of Excellence Review** (update agent name):
```markdown
4. **Conduct "Tzar of Excellence" Review (MANDATORY):**
   - **CRITICAL**: Before creating checkpoint commit, MUST conduct review using codex-reviewer agent
   - **Deploy Review Agent**: Invoke codex-reviewer with "Tzar of Excellence" directive
```

**Other Sections**:
- Keep all TDD workflow (unchanged)
- Keep commit strategy (unchanged)
- Keep testing requirements (unchanged)
- Keep quality gates (unchanged)
- These are project-agnostic

#### 2. code_styleguides/* (COPY FROM MAIN REPO)

**Source**: `/home/stan/Prod/maestro/claude-code/templates/code_styleguides/*.md`

**Action**: Direct copy, no customization needed

**Reason**: These are language-specific style guides, completely independent of agent system

**Files to Copy**:
- `README.md`
- `general.md`
- `python.md`
- `javascript.md`
- `typescript.md`
- `go.md`
- `rust.md`
- `java.md` (if exists)
- `shell.md`
- `docker.md`
- `react.md`
- `vue.md`
- `nextjs.md`
- `nodejs.md`
- `graphql.md`
- `html-css.md`

---

### D. Configuration Files (NEW)

#### 1. config/agents.yaml

**Purpose**: Define OpenCode agent mappings and capabilities

```yaml
---
# OpenCode Agent Mappings for Maestro

# Core Review Agent (Mandatory)
review_agent:
  name: codex-reviewer
  role: Architecture, code review, strategy
  mandatory: true
  quota: unlimited
  use_cases:
    - All implementation work (mandatory review)
    - Spec-driven requirements
    - Complex architectural decisions
    - Pre-commit validation

# Analysis Agents
analysis_agents:
  gemini-analyzer:
    role: Multi-repo analysis, documentation lookup
    quota: 300/day
    specialty:
      - Large codebase analysis (>100KB)
      - Implementation patterns
      - Documentation research
      - Cross-repo reference
    use_when: "Large codebase analysis needed"

# Implementation Agents
implementation_agents:
  opencode-scaffolder:
    role: Fast prototyping and scaffolding
    quota: TBD
    specialty:
      - Standard tasks (5-50 lines)
      - Quick MVP
      - Initial scaffolding
      - Pattern matching
    use_when: "Standard implementation tasks"

  qwen-coder:
    role: Production implementation and testing
    quota: TBD
    specialty:
      - Complex tasks (multiple files, >50 lines)
      - Test writing
      - Documentation polish
      - Refactoring
    use_when: "Complex implementation, tests, docs"

  amp-code:
    role: ETL/data pipeline specialist
    quota: TBD
    specialty:
      - ETL/ELT data pipelines
      - Multi-stage data engineering
      - Data validation
      - Data enrichment
    use_when: "ETL/data pipeline work"

# Task Complexity Mapping
complexity_thresholds:
  trivial:
    max_lines: 5
    agent: none
    description: "Simple changes, implement directly"

  standard:
    min_lines: 5
    max_lines: 50
    agent: opencode-scaffolder
    fallback: qwen-coder
    description: "Single-file implementation"

  complex:
    min_lines: 50
    design_agent: codex-reviewer
    implementation_agent: qwen-coder
    fallback: amp-code  # for ETL tasks
    description: "Multi-file implementation"

  analysis:
    threshold_kb: 100
    agent: gemini-analyzer
    description: "Large codebase analysis"

# Agent Fallback Chain
fallback_order:
  - opencode-scaffolder
  - qwen-coder
  - gemini-analyzer
  # Always last resort
  - direct_implementation
```

#### 2. config/workflow-config.yaml

**Purpose**: Default workflow settings for OpenCode

```yaml
---
# Maestro Workflow Configuration for OpenCode

workflow_mode: autonomous  # autonomous | manual | checkpoint

checkpoint_interval: 3  # Pause every N phases (autonomous mode)

# Mandatory pre-commit review
mandatory_review: true
review_agent: codex-reviewer

# TDD enforcement
tdd_required: true
test_coverage_threshold: 95

# Commit strategy
commit_strategy: aggressive  # aggressive | conservative
# aggressive: Stage changes, commit logical phases
# conservative: Commit after each task

# Auto-proceed settings
auto_proceed: true
confidence_threshold: 7

# Critical Think integration
critical_think_enabled: true
analysis_frequency:
  before_question: true
  after_question: false
  documentation: true
  implementation: true
  agent_delegation: true

# Quality gates
quality_gates:
  - tests_pass
  - coverage_threshold
  - no_lint_errors
  - documentation_complete
  - security_check
  - review_complete

# Phase completion
phase_verification:
  test_coverage: true
  tzar_review: true
  checkpoint_commit: true
```

#### 3. config/opencode-integration.yaml

**Purpose**: OpenCode-specific integration settings

```yaml
---
# OpenCode Integration Configuration

# Command prefix (OpenCode may not use /)
command_prefix: ""  # Empty or "/" depending on OpenCode

# Skill loading
skill_path: ~/.opencode/skill/maestro/

# Template paths
templates_path: ~/.opencode/skill/maestro/templates/

# Command path (if OpenCode has separate command directory)
commands_path: ~/.opencode/skill/maestro/commands/

# Agent system
agent_system: opencode
agent_config: ~/.config/opencode/opencode.jsonc

# MCP servers
required_mcps:
  - nexus-memory
  - memori-memory-mcp

# Optional enhancements
optional_mcps:
  - prompt-enhancer

# Model handling
model_selection: agent_based  # agent_based | explicit | auto
# agent_based: Let OpenCode agents handle model selection
# explicit: Specify model in commands
# auto: Automatic selection based on task complexity
```

---

### E. Shell Scripts (Updates Required)

#### 1. scripts/load_templates.sh

**Current Issue**: Checks `~/.claude/maestro-templates/`

**Required Changes**:
```bash
# CHANGE FROM:
if [ ! -d "$HOME/.claude/maestro-templates" ]; then
    echo "ERROR: Templates directory not found"

# TO:
TEMPLATE_DIR="$HOME/.opencode/skill/maestro/templates"
if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "ERROR: Templates directory not found at $TEMPLATE_DIR"
```

**Complete Rewrite**:
```bash
#!/bin/bash
# Verify and load Maestro templates for OpenCode

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
TEMPLATE_DIR="$SKILL_DIR/templates"

echo "Verifying Maestro templates..."

# Check if templates directory exists
if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "ERROR: Templates directory not found at $TEMPLATE_DIR"
    exit 1
fi

# Check for workflow template
if [ ! -f "$TEMPLATE_DIR/workflow.md" ]; then
    echo "ERROR: workflow.md not found in templates directory"
    exit 1
fi

# Check for code styleguides
if [ ! -d "$TEMPLATE_DIR/code_styleguides" ]; then
    echo "WARNING: code_styleguides directory not found"
fi

echo "✅ Templates verified successfully"
echo ""
echo "Available templates:"
echo "  - workflow.md"
ls -1 "$TEMPLATE_DIR/code_styleguides/" 2>/dev/null | sed 's/^/  - /' || echo "  (no styleguides found)"
```

#### 2. scripts/fix_templates.sh

**Current Issue**: Creates symlinks to `~/.claude/maestro-templates/`

**Required Changes**: Remove symlink logic, templates are now local files

**Complete Rewrite**:
```bash
#!/bin/bash
# Verify Maestro template integrity for OpenCode

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
TEMPLATE_DIR="$SKILL_DIR/templates"

echo "Checking Maestro template integrity..."

# Check if templates directory exists
if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "ERROR: Templates directory not found at $TEMPLATE_DIR"
    echo "Templates should be located at: $TEMPLATE_DIR"
    echo ""
    echo "To restore templates, run:"
    echo "  $SKILL_DIR/scripts/sync_templates.sh"
    exit 1
fi

# Check for workflow template
if [ ! -f "$TEMPLATE_DIR/workflow.md" ]; then
    echo "ERROR: workflow.md not found"
    echo "Run: $SKILL_DIR/scripts/sync_templates.sh"
    exit 1
fi

# Check for code styleguides
if [ ! -d "$TEMPLATE_DIR/code_styleguides" ]; then
    echo "WARNING: code_styleguides directory not found"
else
    GUIDE_COUNT=$(ls -1 "$TEMPLATE_DIR/code_styleguides"/*.md 2>/dev/null | wc -l)
    echo "Found $GUIDE_COUNT code styleguides"
fi

echo ""
echo "✅ Template integrity check passed"
```

#### 3. scripts/sync_templates.sh (NEW)

**Purpose**: Sync templates from main Maestro repo

```bash
#!/bin/bash
# Sync templates from Maestro main repository

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(dirname "$SCRIPT_DIR")"
TEMPLATE_DIR="$SKILL_DIR/templates"

echo "Syncing Maestro templates from main repository..."

# Determine source
# Option 1: From local git repo
MAESTRO_REPO="$HOME/Prod/maestro"
if [ -d "$MAESTRO_REPO" ]; then
    SOURCE_DIR="$MAESTRO_REPO/claude-code/templates"
    echo "Using local repository: $MAESTRO_REPO"
else
    # Option 2: From GitHub (not implemented yet)
    echo "ERROR: Maestro repository not found at $MAESTRO_REPO"
    echo "Please clone repository or specify source"
    exit 1
fi

# Sync workflow.md (needs customization after sync)
echo "Syncing workflow.md..."
cp "$SOURCE_DIR/workflow.md" "$TEMPLATE_DIR/workflow.md"
echo "⚠️  NOTE: workflow.md needs OpenCode customization"
echo "   Edit: $TEMPLATE_DIR/workflow.md"
echo "   Update agent references to OpenCode agents"

# Sync code styleguides (direct copy, no customization)
echo "Syncing code_styleguides..."
mkdir -p "$TEMPLATE_DIR/code_styleguides"
cp "$SOURCE_DIR/code_styleguides"/*.md "$TEMPLATE_DIR/code_styleguides/"

echo ""
echo "✅ Templates synced successfully"
echo ""
echo "⚠️  IMPORTANT: Next steps"
echo "1. Customize workflow.md for OpenCode agents"
echo "2. Review agent mappings in commands/"
echo "3. Run: $SKILL_DIR/scripts/verify_installation.sh"
```

#### 4. scripts/verify_installation.sh (NEW)

**Purpose**: Verify OpenCode Maestro installation

```bash
#!/bin/bash
# Verify OpenCode Maestro installation

set -e

echo "Verifying OpenCode Maestro installation..."
echo ""

SKILL_DIR="$HOME/.opencode/skill/maestro"
ERRORS=0

# Check skill directory
if [ ! -d "$SKILL_DIR" ]; then
    echo "❌ Skill directory not found: $SKILL_DIR"
    ERRORS=$((ERRORS + 1))
else
    echo "✅ Skill directory exists"
fi

# Check core files
for file in SKILL.md README.md; do
    if [ -f "$SKILL_DIR/$file" ]; then
        echo "✅ $file exists"
    else
        echo "❌ $file not found"
        ERRORS=$((ERRORS + 1))
    fi
done

# Check commands
echo ""
echo "Checking commands..."
for cmd in setup newTrack implement status revert configure; do
    if [ -f "$SKILL_DIR/commands/maestro:$cmd.md" ]; then
        echo "✅ maestro:$cmd.md exists"
    else
        echo "❌ maestro:$cmd.md not found"
        ERRORS=$((ERRORS + 1))
    fi
done

# Check templates
echo ""
echo "Checking templates..."
if [ -f "$SKILL_DIR/templates/workflow.md" ]; then
    echo "✅ workflow.md exists"
    # Check for OpenCode agent references
    if grep -q "codex-reviewer\|opencode-scaffolder\|qwen-coder" "$SKILL_DIR/templates/workflow.md"; then
        echo "✅ workflow.md contains OpenCode agent references"
    else
        echo "⚠️  workflow.md may need OpenCode customization"
    fi
else
    echo "❌ workflow.md not found"
    ERRORS=$((ERRORS + 1))
fi

if [ -d "$SKILL_DIR/templates/code_styleguides" ]; then
    GUIDE_COUNT=$(ls -1 "$SKILL_DIR/templates/code_styleguides"/*.md 2>/dev/null | wc -l)
    echo "✅ code_styleguides exists ($GUIDE_COUNT guides)"
else
    echo "❌ code_styleguides not found"
    ERRORS=$((ERRORS + 1))
fi

# Check configuration
echo ""
echo "Checking configuration..."
for config in agents.yaml workflow-config.yaml opencode-integration.yaml; do
    if [ -f "$SKILL_DIR/config/$config" ]; then
        echo "✅ $config exists"
    else
        echo "⚠️  $config not found (optional)"
    fi
done

# Summary
echo ""
if [ $ERRORS -eq 0 ]; then
    echo "✅ Installation verification passed"
    exit 0
else
    echo "❌ Installation verification failed ($ERRORS errors)"
    exit 1
fi
```

---

### F. Documentation Files (NEW)

#### 1. docs/AGENT-MAPPINGS.md

```markdown
# OpenCode Agent Mappings for Maestro

## Agent Mapping Strategy

Maestro uses abstract agent roles that map to concrete OpenCode agents.

## Core Mappings

### Oracle (Claude Code) → codex-reviewer (OpenCode)
- **Role**: Architecture, code review, strategic planning
- **Usage**: Mandatory for all implementation work
- **Quota**: Unlimited

### Librarian (Claude Code) → gemini-analyzer (OpenCode)
- **Role**: Multi-repo analysis, documentation lookup
- **Usage**: Large codebase analysis (>100KB)
- **Quota**: 300 requests/day

### Explore (Claude Code) → opencode-scaffolder (OpenCode)
- **Role**: Fast prototyping and scaffolding
- **Usage**: Standard implementation tasks (5-50 lines)
- **Quota**: TBD

### Additional OpenCode Agents

#### qwen-coder
- **Role**: Production implementation, testing, documentation
- **Usage**: Complex tasks, test writing, refactoring
- **Quota**: TBD

#### amp-code
- **Role**: ETL/data pipeline specialist
- **Usage**: Multi-stage data engineering
- **Quota**: TBD

## Selection Logic

```
Task Assessment:
│
├── Trivial (1-5 lines)
│   └── Direct implementation
│
├── Standard (5-50 lines)
│   └── opencode-scaffolder
│
├── Complex (multiple files, >50 lines)
│   ├── Design: codex-reviewer
│   └── Implementation: qwen-coder or amp-code
│
├── Analysis (>100KB)
│   └── gemini-analyzer
│
└── ETL/Data Pipeline
    └── amp-code
```

## Usage Examples

### Example 1: Feature Implementation
```
Task: Add user authentication
→ Design: codex-reviewer (architecture)
→ Implementation: qwen-coder (coding)
→ Review: codex-reviewer (validation)
```

### Example 2: Bug Fix
```
Task: Fix login bug
→ Analysis: opencode-scaffolder (quick fix)
→ Review: codex-reviewer (validate)
```

### Example 3: Large Refactor
```
Task: Refactor payment module
→ Analysis: gemini-analyzer (understand codebase)
→ Design: codex-reviewer (plan refactor)
→ Implementation: qwen-coder (execute)
→ Review: codex-reviewer (quality check)
```

## Quota Management

- **gemini-analyzer**: 300/day - use sparingly
- **codex-reviewer**: Unlimited - use liberally
- **opencode-scaffolder**: TBD - monitor usage
- **qwen-coder**: TBD - monitor usage
- **amp-code**: TBD - domain-specific use

## Fallback Strategy

If primary agent unavailable:
1. Inform user of unavailable agent
2. Suggest alternative
3. Ask for confirmation
4. Proceed with fallback if approved

## Configuration

Agent mappings configured in:
- `config/agents.yaml` - Mappings and capabilities
- `config/workflow-config.yaml` - Selection logic
- `templates/workflow.md` - Usage requirements
```

#### 2. docs/WORKFLOW-CUSTOMIZATION.md

```markdown
# Workflow Customization for OpenCode

## Overview

Maestro's workflow.md template has been customized for OpenCode's agent system.

## Key Differences from Claude Code Version

### 1. Agent References

**Claude Code**:
- oracle, librarian, explore

**OpenCode**:
- codex-reviewer, gemini-analyzer, opencode-scaffolder, qwen-coder, amp-code

### 2. Delegation Syntax

**Claude Code**:
```markdown
Use the Task tool to delegate to oracle agent
```

**OpenCode**:
```markdown
Delegate to codex-reviewer subagent
# OR
Invoke opencode agent: codex-reviewer
```

### 3. Model Selection

**Claude Code**: Explicit model selection (haiku, sonnet, opus)

**OpenCode**: Agent-based model selection (let agent system handle)

### 4. Quota Awareness

Different quota limits require updated guidance:
- gemini-analyzer: 300/day (use sparingly)
- codex-reviewer: Unlimited (use liberally)

## Customization Checklist

When updating workflow.md from Claude Code version:

- [ ] Replace agent names throughout
- [ ] Update agent selection criteria
- [ ] Change delegation syntax examples
- [ ] Remove explicit model selection
- [ ] Update quota awareness section
- [ ] Fix Tzar of Excellence review agent
- [ ] Update agent fallback chain

## Maintaining Customization

To sync with upstream workflow.md:

1. Copy new version from main repo
2. Apply agent mapping replacements
3. Update delegation syntax
4. Test with sample track
5. Commit to opencode skill

## Testing

Verify workflow customization:

```bash
# Create test track
maestro newTrack "Test agent selection"

# Implement and observe agent selection
maestro implement test-agent-selection

# Verify correct agents used
```
```

#### 3. docs/TROUBLESHOOTING.md

```markdown
# OpenCode Maestro Troubleshooting

## Common Issues

### 1. "Agent not found"

**Symptoms**: Command fails with "agent not available" error

**Diagnosis**:
```bash
# Check if agent configured in opencode.jsonc
cat ~/.config/opencode/opencode.jsonc | grep agent-name
```

**Solutions**:
1. Verify agent in OpenCode configuration
2. Check agent availability
3. Use fallback agent if configured

### 2. "Workflow.md not found"

**Symptoms**: Setup fails to find workflow template

**Diagnosis**:
```bash
# Check template exists
ls -l ~/.opencode/skill/maestro/templates/workflow.md
```

**Solutions**:
```bash
# Run template sync
cd ~/.opencode/skill/maestro
./scripts/sync_templates.sh
```

### 3. "Command not recognized"

**Symptoms**: OpenCode doesn't recognize maestro commands

**Diagnosis**:
```bash
# Check command registration
cat ~/.config/opencode/opencode.jsonc | grep maestro
```

**Solutions**:
1. Verify opencode.jsonc contains maestro entries
2. Restart OpenCode
3. Re-run installer: `./install-opencode.sh`

### 4. "Agent quota exceeded"

**Symptoms**: Agent stops responding, quota error

**Diagnosis**:
```bash
# Check agent usage in logs
# (OpenCode-specific log location)
```

**Solutions**:
1. Use alternative agent with available quota
2. Wait for quota reset
3. Break task into smaller chunks

### 5. "Wrong agent selected"

**Symptoms**: Maestro uses unexpected agent for task

**Diagnosis**:
```bash
# Check workflow.md agent selection logic
grep -A 10 "Agent Selection" ~/.opencode/skill/maestro/templates/workflow.md
```

**Solutions**:
1. Verify workflow.md has correct agent mappings
2. Check config/agents.yaml
3. Adjust complexity thresholds if needed

## Getting Help

### Logs
Check OpenCode logs: `~/.opencode/logs/`

### Verification
Run installation verification:
```bash
~/.opencode/skill/maestro/scripts/verify_installation.sh
```

### Debug Mode
Enable verbose mode in workflow-config.yaml:
```yaml
verbose_mode: true
```

## Known Issues

### 1. Template Symlinks
**Issue**: Old installations use symlinks to Claude Code
**Fix**: Run `scripts/sync_templates.sh` to create local copies

### 2. Agent Name Changes
**Issue**: Agent names in workflow don't match OpenCode config
**Fix**: Update agent mappings in `config/agents.yaml`

### 3. Quota Limits
**Issue**: gemini-analyzer quota exceeded frequently
**Fix**: Use only for genuine large-scale analysis (>100KB)
```

---

### G. MANIFEST.md (NEW)

**Purpose**: Track file differences from main Maestro repo

```markdown
# OpenCode Maestro File Manifest

## Purpose

Track which files are copied, customized, or created new for OpenCode variant.

## File Categories

### 1. Direct Copies (No Customization)

Files copied exactly from main Maestro repo, no changes needed:

- `templates/code_styleguides/*.md` - All language style guides
- `scripts/load_templates.sh` - Updated paths only
- `scripts/fix_templates.sh` - Updated paths only

**Sync Strategy**: Safe to overwrite with upstream updates

### 2. Copied + OpenCode Customization

Files copied from main repo then customized for OpenCode:

#### workflow.md
**Source**: `claude-code/templates/workflow.md`
**Customizations**:
- Agent names (oracle → codex-reviewer, etc.)
- Agent selection logic
- Delegation syntax
- Quota awareness
- Model selection approach

**Sync Strategy**: Copy upstream, then re-apply customizations

**Customization Markers**: Search for "OpenCode" comments in file

#### Command Files (maestro:*.md)
**Source**: `claude-code/commands/maestro:*.md`
**Customizations**:
- Agent references
- Command paths
- MCP server references
- Model selection (remove or adapt)

**Sync Strategy**: Copy upstream, then re-apply agent mappings

**Customization Markers**: Search for "OpenCode" comments in file

### 3. OpenCode-Specific (New Files)

Files created specifically for OpenCode, no upstream equivalent:

#### Configuration
- `config/agents.yaml` - Agent mappings
- `config/workflow-config.yaml` - Workflow defaults
- `config/opencode-integration.yaml` - Integration settings

#### Documentation
- `docs/AGENT-MAPPINGS.md` - Agent reference
- `docs/WORKFLOW-CUSTOMIZATION.md` - Customization guide
- `docs/TROUBLESHOOTING.md` - OpenCode issues

#### Scripts
- `scripts/sync_templates.sh` - Template sync utility
- `scripts/verify_installation.sh` - Installation verifier

#### Core
- `MANIFEST.md` - This file

**Sync Strategy**: Never overwrite from upstream

### 4. Shared (Already OpenCode-Specific)

Files that are already OpenCode-specific, no action needed:

- `SKILL.md` - Already customized
- `README.md` - Already customized

**Sync Strategy**: Review upstream changes, apply if relevant

## Syncing from Upstream

### Workflow

```bash
# 1. Fetch upstream changes
cd ~/Prod/maestro
git pull origin master

# 2. Sync direct copies
cp claude-code/templates/code_styleguides/*.md ~/.opencode/skill/maestro/templates/code_styleguides/

# 3. Sync customizable files
cp claude-code/templates/workflow.md ~/.opencode/skill/maestro/templates/workflow.md
# Then re-apply OpenCode customizations (see docs/WORKFLOW-CUSTOMIZATION.md)

# 4. Review shared files
# Compare SKILL.md, README.md with upstream
# Apply relevant changes

# 5. Verify installation
~/.opencode/skill/maestro/scripts/verify_installation.sh
```

### Sync Frequency

- **After major Maestro releases**: Review all files
- **After minor releases**: Review customizable files only
- **Continuous**: Monitor GitHub for changes

## Version Tracking

**Current Maestro Version**: 2.0.0
**OpenCode Variant Version**: 1.0.0
**Last Sync**: 2026-01-05

## Contributing

When adding new files to OpenCode variant:

1. Determine category (copy/customize/new)
2. Document in this manifest
3. Add sync instructions if applicable
4. Update version tracking
```

---

## OpenCode-Specific Customizations

### Critical Customization Points

#### 1. Agent Name Replacements

**Global Find/Replace**:

```yaml
# Claude Code → OpenCode
oracle → codex-reviewer
librarian → gemini-analyzer
explore → opencode-scaffolder
frontend-ui-ux-engineer → [Check OpenCode equivalent]
document-writer → [Check OpenCode equivalent]
multimodal-looker → [Check OpenCode equivalent]
```

**Note**: Some Claude Code agents may not have direct OpenCode equivalents. Document these in `config/agents.yaml`.

#### 2. Delegation Mechanism

**Claude Code**:
```markdown
Use the Task tool with subagent oracle to [task]
```

**OpenCode** (exact syntax TBD):
```markdown
Delegate to codex-reviewer subagent
# OR
Invoke opencode agent: codex-reviewer
# OR
Use opencode delegation mechanism with agent: codex-reviewer
```

**Action Required**: Verify exact OpenCode delegation syntax and update all command files accordingly.

#### 3. Model Selection

**Claude Code**: Explicit model specification in frontmatter
```yaml
---
model: sonnet  # or opus, haiku
---
```

**OpenCode**: Two approaches:

**Option A**: Remove model specification, let agent system handle
```yaml
---
# No model field
---
```

**Option B**: Keep if OpenCode supports it
```yaml
---
model: sonnet  # Check if OpenCode agents support this
---
```

**Action Required**: Determine OpenCode's model selection approach and update command files.

#### 4. MCP Integration

**Claude Code**: MCP servers configured in Claude Code settings

**OpenCode**: MCP servers configured in opencode.jsonc

**Files to Update**:
- Command files that check for MCP servers
- Setup script that configures MCP
- Documentation references

**Example MCP Check**:
```yaml
# CHANGE FROM:
Check if nexus-memory MCP is running in Claude Code

# TO:
Check if nexus-memory MCP is running in OpenCode
# Update config path reference
```

---

## Agent Mapping Strategy

### Abstract vs Concrete Agents

Maestro uses **abstract agent roles** that map to **concrete implementations**:

#### Abstract Role: Architecture/Review
**Purpose**: Design, validation, quality assurance
- **Claude Code**: oracle
- **OpenCode**: codex-reviewer

#### Abstract Role: Analysis
**Purpose**: Codebase understanding, documentation research
- **Claude Code**: librarian
- **OpenCode**: gemini-analyzer

#### Abstract Role: Implementation (Fast)
**Purpose**: Quick prototyping, scaffolding
- **Claude Code**: explore
- **OpenCode**: opencode-scaffolder

#### Abstract Role: Implementation (Production)
**Purpose**: Quality code, testing, documentation
- **Claude Code**: explore (with opus)
- **OpenCode**: qwen-coder

#### Abstract Role: Domain Specialist
**Purpose**: Specialized domains (ETL, ML, etc.)
- **Claude Code**: [Various]
- **OpenCode**: amp-code (ETL)

### Configuration-Based Mapping

Store mappings in `config/agents.yaml`:

```yaml
abstract_roles:
  architecture_review:
    claude-code: oracle
    opencode: codex-reviewer

  analysis:
    claude-code: librarian
    opencode: gemini-analyzer

  implementation_fast:
    claude-code: explore
    opencode: opencode-scaffolder

  implementation_production:
    claude-code: explore
    opencode: qwen-coder
```

**Benefit**: Easy to add new platforms (future-proofing)

### Workflow Integration

Workflow.md uses abstract role names:

```markdown
## Agent Selection

For architecture tasks, use {{architecture_review}} agent
For analysis tasks, use {{analysis}} agent
```

**Resolution**: At setup time, replace placeholders with platform-specific agent names.

**Alternative**: Use concrete names directly (simpler, less flexible)

**Recommendation**: Use concrete names for OpenCode variant (simpler is better for now)

---

## Implementation Phases

### Phase 1: Foundation (Critical Path)

**Goal**: Make Maestro OpenCode variant functional

**Tasks**:

1. ✅ **Create independent directory structure**
   - Create `config/`, `docs/` directories
   - Verify existing structure

2. 🔄 **Copy and customize command files**
   - Break symlinks
   - Copy from Claude Code commands
   - Apply agent name replacements
   - Update delegation syntax
   - **Priority**: setup, newTrack, implement (critical)

3. 🔄 **Copy and customize workflow.md**
   - Copy from main repo
   - Replace agent names
   - Update agent selection logic
   - Fix delegation syntax
   - Remove explicit model selection

4. 🔄 **Copy code styleguides**
   - Direct copy from main repo
   - No customization needed

5. ✅ **Update shell scripts**
   - Fix paths in load_templates.sh
   - Fix paths in fix_templates.sh
   - Remove symlink logic

**Deliverable**: Functional OpenCode Maestro

**Testing**:
```bash
# Test basic workflow
maestro setup
maestro newTrack "Test track"
maestro implement test-track
```

### Phase 2: Configuration & Documentation

**Goal**: Improve maintainability and usability

**Tasks**:

1. 🔄 **Create configuration files**
   - `config/agents.yaml` - Define agent mappings
   - `config/workflow-config.yaml` - Workflow defaults
   - `config/opencode-integration.yaml` - Integration settings

2. 🔄 **Create documentation**
   - `docs/AGENT-MAPPINGS.md` - Agent reference
   - `docs/WORKFLOW-CUSTOMIZATION.md` - Customization guide
   - `docs/TROUBLESHOOTING.md` - Troubleshooting

3. 🔄 **Create utility scripts**
   - `scripts/sync_templates.sh` - Sync from main repo
   - `scripts/verify_installation.sh` - Verify installation

4. 🔄 **Create MANIFEST.md**
   - Document file categorization
   - Sync instructions
   - Version tracking

**Deliverable**: Maintainable OpenCode Maestro

**Testing**:
```bash
# Verify installation
~/.opencode/skill/maestro/scripts/verify_installation.sh

# Test sync
~/.opencode/skill/maestro/scripts/sync_templates.sh
```

### Phase 3: Polish & Optimization

**Goal**: Production-ready OpenCode Maestro

**Tasks**:

1. 🔄 **Update installer script**
   - Modify `install-opencode.sh`
   - Copy independent files (not symlinks)
   - Run `verify_installation.sh` at end

2. 🔄 **Update SKILL.md and README.md**
   - Reflect independent structure
   - Update agent mappings
   - Remove symlink references

3. 🔄 **Test comprehensive workflow**
   - Greenfield project
   - Brownfield project
   - Multiple tracks
   - Revert operations

4. 🔄 **Performance optimization**
   - Check for unnecessary file reads
   - Optimize agent selection
   - Cache template lookups

**Deliverable**: Production-ready OpenCode Maestro

**Testing**: Comprehensive end-to-end testing

### Phase 4: Maintenance & Sync

**Goal**: Easy updates from main Maestro repo

**Tasks**:

1. 🔄 **Document sync process**
   - When to sync
   - How to sync
   - What to customize after sync

2. 🔄 **Automate where possible**
   - Auto-apply agent replacements
   - Detect conflicting changes
   - Version tracking

3. 🔄 **Set up monitoring**
   - Track upstream changes
   - Notify of updates
   - Changelog maintenance

**Deliverable**: Maintainable OpenCode variant

---

## Open Questions & Decisions Needed

### 1. OpenCode Delegation Syntax

**Question**: What is the exact syntax for delegating to OpenCode agents?

**Options**:
- A) `Delegate to <agent-name> subagent`
- B) `Invoke opencode agent: <agent-name>`
- C) `Use <agent-name> via OpenCode`
- D) Other (TBD)

**Impact**: All command files need correct delegation syntax

**Decision Required**: Test with actual OpenCode system

### 2. Model Selection in OpenCode

**Question**: Does OpenCode support model selection in command files?

**Options**:
- A) Yes, keep model field in frontmatter
- B) No, remove model field (let agent system handle)
- C) Partial (some agents support it)

**Impact**: Command file frontmatter

**Decision Required**: Check OpenCode documentation

### 3. Additional OpenCode Agents

**Question**: Are there OpenCode agents beyond those identified?

**Known**:
- codex-reviewer
- gemini-analyzer
- opencode-scaffolder
- qwen-coder
- amp-code

**Unknown**:
- frontend-ui-ux-engineer equivalent?
- document-writer equivalent?
- multimodal-looker equivalent?

**Impact**: Agent mappings in workflow.md and commands

**Decision Required**: Review OpenCode agent list

### 4. MCP Configuration Paths

**Question**: Where does OpenCode configure MCP servers?

**Assumption**: `~/.config/opencode/opencode.jsonc`

**Verification Required**: Confirm path

**Impact**: Setup script, documentation

### 5. Agent Quotas

**Question**: What are the actual quotas for OpenCode agents?

**Known**:
- gemini-analyzer: 300/day

**Unknown**:
- qwen-coder: ?
- amp-code: ?
- opencode-scaffolder: ?
- codex-reviewer: ?

**Impact**: Workflow.md quota awareness, usage guidance

**Decision Required**: Check OpenCode quota documentation

---

## Success Criteria

### Functional Requirements

✅ Maestro OpenCode variant is fully functional:
- `/maestro setup` works
- `/maestro newTrack` works
- `/maestro implement` works with correct agent selection
- `/maestro status` works
- `/maestro revert` works

✅ No dependencies on Claude Code:
- No broken symlinks
- All files are local copies or custom
- Templates in opencode skill directory

✅ Correct agent mappings:
- workflow.md uses OpenCode agents
- Commands use OpenCode agents
- Delegation syntax works

### Maintainability Requirements

✅ Easy to sync from main repo:
- Manifest tracks file differences
- Sync script automates process
- Clear customization documentation

✅ Clear documentation:
- Agent mappings documented
- Customization guide exists
- Troubleshooting guide exists

✅ Verification tools:
- Installation verification script
- Template integrity check
- Automated testing

### Quality Requirements

✅ Production-ready:
- Comprehensive testing completed
- All edge cases handled
- Performance optimized
- Documentation complete

---

## Next Steps

1. **Review this design document** with stakeholders
2. **Answer open questions** (delegation syntax, model selection, quotas)
3. **Begin Phase 1 implementation** (critical path)
4. **Test incrementally** (after each command file)
5. **Document deviations** from this design
6. **Update this document** as decisions are made

---

## Appendix A: File Inventory

### Current Files ( symlinked)

```
~/.opencode/skill/maestro/
├── commands/
│   ├── conductor:setup.md → ~/.claude/commands/conductor:setup.md
│   ├── conductor:newTrack.md → ~/.claude/commands/conductor:newTrack.md
│   ├── conductor:implement.md → ~/.claude/commands/conductor:implement.md
│   ├── conductor:status.md → ~/.claude/commands/conductor:status.md
│   └── conductor:revert.md → ~/.claude/commands/conductor:revert.md
└── templates/
    ├── workflow.md → ~/.claude/conductor-templates/workflow.md (BROKEN)
    └── code_styleguides/ → ~/.claude/maestro-templates/code_styleguides/
```

**Note**: Commands reference "conductor" (old name), should be "maestro"

### Source Files (Main Repo)

```
/home/stan/Prod/maestro/
├── claude-code/
│   ├── commands/
│   │   ├── maestro:setup.md
│   │   ├── maestro:newTrack.md
│   │   ├── maestro:implement.md
│   │   ├── maestro:status.md
│   │   └── maestro:revert.md
│   └── templates/
│       ├── workflow.md
│       └── code_styleguides/
│           └── *.md (15 files)
```

---

## Appendix B: Agent Comparison Table

| Abstract Role | Claude Code Agent | OpenCode Agent | Quota | Specialty |
|--------------|-------------------|----------------|-------|-----------|
| Architecture/Review | oracle | codex-reviewer | Unlimited | Design, validation, quality |
| Analysis | librarian | gemini-analyzer | 300/day | Large codebase, docs |
| Fast Implementation | explore | opencode-scaffolder | TBD | Quick prototyping |
| Production Implementation | explore (opus) | qwen-coder | TBD | Quality code, tests |
| ETL/Data | - | amp-code | TBD | Data pipelines |
| Frontend UI/UX | frontend-ui-ux-engineer | ? | ? | UI design & impl |
| Documentation | document-writer | ? | ? | Technical writing |
| Multimodal | multimodal-looker | ? | ? | Visual content |

**?**: Needs investigation

---

## Appendix C: Quick Reference

### Critical Files to Customize

1. `templates/workflow.md` - Agent names, selection logic, delegation
2. `commands/maestro:implement.md` - Agent selection, delegation syntax
3. `commands/maestro:setup.md` - Template paths, agent checks
4. `commands/maestro:newTrack.md` - Agent references
5. `SKILL.md` - Agent descriptions

### Files to Direct Copy

1. `templates/code_styleguides/*.md` - All 15 style guides
2. `scripts/load_templates.sh` - Update paths only
3. `scripts/fix_templates.sh` - Update paths only

### New Files to Create

1. `config/agents.yaml`
2. `config/workflow-config.yaml`
3. `config/opencode-integration.yaml`
4. `scripts/sync_templates.sh`
5. `scripts/verify_installation.sh`
6. `docs/AGENT-MAPPINGS.md`
7. `docs/WORKFLOW-CUSTOMIZATION.md`
8. `docs/TROUBLESHOOTING.md`
9. `MANIFEST.md`

---

## Document Control

**Version**: 1.0
**Status**: Design Document (awaiting implementation)
**Review Date**: 2026-01-05
**Next Review**: After Phase 1 completion

**Change Log**:
- 2026-01-05: Initial design document created
