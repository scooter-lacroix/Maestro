# OpenCode Maestro Design - Executive Summary

## Overview

This document summarizes the comprehensive design for making the OpenCode variant of Maestro fully functional and independent from Claude Code dependencies.

**Full Design Document**: See `OPENCODE-INDEPENDENT-DESIGN.md`

---

## Current State Assessment

### Problems Identified

1. **Broken Symlinks**:
   - `templates/workflow.md` → Points to non-existent `~/.claude/conductor-templates/`
   - Should point to `~/.claude/maestro-templates/` (but should be independent anyway)

2. **Command Symlinks**:
   - All commands symlinked to Claude Code versions
   - Contain Claude Code agent references (oracle, librarian, explore)
   - Use Claude Code-specific delegation syntax

3. **Template Dependency**:
   - Templates expected to be in `~/.claude/maestro-templates/`
   - Code styleguides symlinked from Claude Code
   - No independent template storage

4. **Agent Mismatch**:
   - Workflow references Claude Code agents
   - Commands use Claude Code agent names
   - No OpenCode agent mapping configuration

### What Works

- SKILL.md is well-written for OpenCode
- README.md provides good OpenCode documentation
- Directory structure is sound
- Shell scripts have good functionality (wrong paths)

---

## Design Solution

### Core Principle

**Complete Independence**: No symlinks to Claude Code. All files are either:
- Direct copies (shared content)
- OpenCode-customized copies (adapted content)
- New OpenCode-specific files (unique content)

### Target Structure

```
~/.opencode/skill/maestro/
├── SKILL.md (existing, minor updates)
├── README.md (existing, minor updates)
├── MANIFEST.md (NEW - track file differences)
│
├── commands/ (BREAK SYMLINKS, customize)
│   ├── maestro:setup.md
│   ├── maestro:newTrack.md
│   ├── maestro:implement.md
│   ├── maestro:status.md
│   ├── maestro:revert.md
│   ├── maestro:configure.md (NEW)
│   ├── maestro:tui.md (NEW)
│   └── maestro:memory.md (NEW)
│
├── templates/ (INDEPENDENT STORAGE)
│   ├── workflow.md (COPY + CUSTOMIZE)
│   ├── README.md (NEW)
│   └── code_styleguides/ (DIRECT COPY)
│       └── *.md (15 files)
│
├── scripts/ (UPDATE PATHS)
│   ├── load_templates.sh (fix paths)
│   ├── fix_templates.sh (remove symlink logic)
│   ├── sync_templates.sh (NEW)
│   └── verify_installation.sh (NEW)
│
├── config/ (NEW - OpenCode config)
│   ├── agents.yaml
│   ├── workflow-config.yaml
│   └── opencode-integration.yaml
│
└── docs/ (NEW - OpenCode docs)
    ├── AGENT-MAPPINGS.md
    ├── WORKFLOW-CUSTOMIZATION.md
    └── TROUBLESHOOTING.md
```

---

## Critical Customizations

### 1. Agent Mappings

| Claude Code | OpenCode | Role |
|------------|----------|------|
| oracle | codex-reviewer | Architecture, review |
| librarian | gemini-analyzer | Analysis, docs |
| explore | opencode-scaffolder | Fast impl |
| explore (opus) | qwen-coder | Production impl |
| - | amp-code | ETL/data |

### 2. Files Requiring Heavy Customization

**workflow.md** (472 lines):
- Replace agent names throughout
- Update agent selection logic
- Fix delegation syntax
- Remove explicit model selection
- Update quota awareness

**maestro:implement.md**:
- Complete agent selection section rewrite
- Update delegation syntax
- Change task complexity assessment
- Fix agent fallback chain

**maestro:setup.md**:
- Update template paths
- Remove Claude Code plugin checks
- Add OpenCode agent availability checks

**maestro:newTrack.md**:
- Update agent references
- Fix delegation examples
- Update skill loading

### 3. Files Requiring Path Updates Only

- `scripts/load_templates.sh`: Change `~/.claude/maestro-templates/` to `~/.opencode/skill/maestro/templates/`
- `scripts/fix_templates.sh`: Remove symlink creation, verify local files

### 4. Files to Direct Copy (No Changes)

- All `templates/code_styleguides/*.md` (15 files)
- These are language-agnostic, completely independent of agent system

---

## Implementation Strategy

### Phase 1: Foundation (Critical Path)

**Make it functional**

1. Break command symlinks, copy from Claude Code versions
2. Apply agent name replacements (oracle → codex-reviewer, etc.)
3. Customize workflow.md with OpenCode agents
4. Copy code styleguides
5. Update shell script paths

**Deliverable**: Working OpenCode Maestro

**Testing**: Run full workflow (setup → newTrack → implement)

### Phase 2: Configuration & Documentation

**Make it maintainable**

1. Create `config/` YAML files with agent mappings
2. Create comprehensive documentation
3. Create sync and verification scripts
4. Create MANIFEST.md to track file differences

**Deliverable**: Maintainable system

**Testing**: Verify installation, test template sync

### Phase 3: Polish & Optimization

**Make it production-ready**

1. Update installer script
2. Comprehensive end-to-end testing
3. Performance optimization
4. Update SKILL.md and README.md

**Deliverable**: Production-ready OpenCode Maestro

### Phase 4: Maintenance & Sync

**Make it future-proof**

1. Document sync process from main repo
2. Automate customization application
3. Set up update monitoring

**Deliverable**: Easy to maintain

---

## Open Questions

### Critical Decisions Needed

1. **OpenCode Delegation Syntax**
   - What is the exact syntax?
   - Options: "Delegate to X", "Invoke agent X", "Use X"
   - **Impact**: All command files

2. **Model Selection**
   - Does OpenCode support `model:` field in frontmatter?
   - Or does agent system handle it?
   - **Impact**: Command file frontmatter

3. **Additional Agents**
   - Are there frontend-ui-ux-engineer, document-writer equivalents?
   - **Impact**: Agent mappings, workflow.md

4. **MCP Configuration Path**
   - Confirm `~/.config/opencode/opencode.jsonc`
   - **Impact**: Setup script

5. **Agent Quotas**
   - What are actual quotas for qwen-coder, amp-code, etc.?
   - **Impact**: Workflow.md guidance

---

## Success Criteria

### Must Have

✅ No broken symlinks
✅ All files independent
✅ Correct OpenCode agent mappings
✅ Full workflow functional
✅ Templates in opencode directory

### Should Have

✅ Comprehensive documentation
✅ Configuration files
✅ Verification scripts
✅ Easy sync from main repo

### Nice to Have

✅ Automated customization
✅ Update monitoring
✅ Performance optimization

---

## File Inventory

### Current (Symlinked)

```
commands/
├── conductor:setup.md → ~/.claude/commands/conductor:setup.md
├── conductor:newTrack.md → ~/.claude/commands/conductor:newTrack.md
├── conductor:implement.md → ~/.claude/commands/conductor:implement.md
├── conductor:status.md → ~/.claude/commands/conductor:status.md
└── conductor:revert.md → ~/.claude/commands/conductor:revert.md

templates/
├── workflow.md → ~/.claude/conductor-templates/workflow.md (BROKEN)
└── code_styleguides/ → ~/.claude/maestro-templates/code_styleguides/
```

**Note**: "conductor" is old name, should be "maestro"

### Source (Main Repo)

```
/home/stan/Prod/maestro/claude-code/
├── commands/
│   ├── maestro:setup.md
│   ├── maestro:newTrack.md
│   ├── maestro:implement.md
│   ├── maestro:status.md
│   └── maestro:revert.md
└── templates/
    ├── workflow.md
    └── code_styleguides/*.md (15 files)
```

---

## Next Steps

1. **Review design document** (`OPENCODE-INDEPENDENT-DESIGN.md`)
2. **Answer open questions** (delegation syntax, model selection, quotas)
3. **Begin Phase 1** (break symlinks, customize files)
4. **Test incrementally** (each command file as completed)
5. **Document deviations** from this design
6. **Update design** based on actual implementation

---

## Quick Reference

### Files to Customize Heavily

1. `templates/workflow.md` - 472 lines, agent references throughout
2. `commands/maestro:implement.md` - Agent selection logic
3. `commands/maestro:setup.md` - Template paths, agent checks
4. `commands/maestro:newTrack.md` - Agent references
5. `SKILL.md` - Agent descriptions

### Files to Copy Directly

1. `templates/code_styleguides/*.md` - All 15 files
2. `scripts/load_templates.sh` - Path updates only
3. `scripts/fix_templates.sh` - Path updates only

### New Files to Create

1. `config/agents.yaml` - Agent mappings
2. `config/workflow-config.yaml` - Workflow settings
3. `config/opencode-integration.yaml` - Integration config
4. `scripts/sync_templates.sh` - Sync utility
5. `scripts/verify_installation.sh` - Verification
6. `docs/AGENT-MAPPINGS.md` - Agent reference
7. `docs/WORKFLOW-CUSTOMIZATION.md` - Customization guide
8. `docs/TROUBLESHOOTING.md` - Troubleshooting
9. `MANIFEST.md` - File tracking

---

## Document References

- **Full Design**: `OPENCODE-INDEPENDENT-DESIGN.md` (comprehensive)
- **This Summary**: `OPENCODE-DESIGN-SUMMARY.md` (this file)
- **Current State**: `~/.opencode/skill/maestro/` (existing installation)
- **Source Files**: `/home/stan/Prod/maestro/claude-code/` (main repo)

---

**Status**: Design Complete, Awaiting Implementation
**Date**: 2026-01-05
**Version**: 1.0
