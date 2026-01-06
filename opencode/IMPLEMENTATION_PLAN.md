# OpenCode Maestro Implementation Plan

**Status**: READY FOR IMPLEMENTATION
**Version**: 1.0.0
**Created**: 2026-01-05
**Priority**: HIGH

## Executive Summary

This plan details the steps needed to make the OpenCode variant of Maestro fully functional and independent from the Claude Code variant. The OpenCode variant will have its own skill definition, commands, templates, workflow, and agent mappings while sharing core functionality where appropriate.

## Current State Analysis

### What Exists
- [x] OpenCode skill directory structure: `/opencode/skill/maestro/`
- [x] Basic SKILL.md and README.md
- [x] Template symlink scripts (fix_templates.sh, load_templates.sh)
- [x] Documentation in `/docs/OPENCODE.md` and `/docs/AGENTS.md`

### What's Missing or Broken
- [ ] Commands directory contains symlinks to old "conductor" commands
- [ ] No actual command files (maestro:setup.md, maestro:newTrack.md, etc.)
- [ ] Templates symlink to wrong locations (conductor-templates vs maestro-templates)
- [ ] No independent workflow.md for OpenCode (references Claude Code agents)
- [ ] No opencode.json configuration snippet
- [ ] No agent mappings in opencode configuration
- [ ] No installer script (install-opencode.sh)
- [ ] Code styleguides may need OpenCode-specific adjustments

## Implementation Phases

### Phase 1: Foundation Setup (CRITICAL - Do First)
**Goal**: Establish independent file structure and configuration

#### 1.1 Create Command Files
**Priority**: CRITICAL
**Effort**: 2 hours
**Dependencies**: None

**Tasks**:
1. Copy all command files from `/claude-code/commands/` to `/opencode/skill/maestro/commands/`
2. Modify each command to replace Claude Code-specific references with OpenCode equivalents
3. Update command frontmatter if needed for OpenCode compatibility

**Files to Create**:
- `/opencode/skill/maestro/commands/maestro:setup.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:newTrack.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:implement.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:status.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:revert.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:configure.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:tui.md` (copy from claude-code)
- `/opencode/skill/maestro/commands/maestro:memory.md` (copy from claude-code)

**Verification**:
```bash
# Verify all command files exist
ls -1 /opencode/skill/maestro/commands/*.md
# Expected: 8 files

# Verify no symlinks remain
find /opencode/skill/maestro/commands -type l
# Expected: empty output

# Verify files have content
wc -l /opencode/skill/maestro/commands/*.md
# Expected: all files > 0 lines
```

#### 1.2 Create Independent Workflow
**Priority**: CRITICAL
**Effort**: 1 hour
**Dependencies**: None

**Tasks**:
1. Create `/opencode/skill/maestro/templates/workflow.md`
2. Adapt Claude Code workflow.md for OpenCode agents
3. Replace agent mappings:
   - oracle → codex-reviewer (or user-configurable)
   - librarian → gemini-analyzer
   - explore → opencode-scaffolder
   - macgyver → qwen-coder
   - dexter → amp-code or opencode-scaffolder
4. Maintain all other workflow content (TDD, Critical Think, etc.)

**Key Changes**:
```
Claude Code → OpenCode Agent Mappings:
- oracle → codex-reviewer (GPT-5, high-rigor review)
- librarian → gemini-analyzer (1M+ context, large codebase)
- explore → opencode-scaffolder (rapid prototyping)
- macgyver → qwen-coder (implementation specialist)
- dexter → amp-code (ETL/pipelines) or opencode-scaffolder
```

**Verification**:
```bash
# Verify workflow exists
test -f /opencode/skill/maestro/templates/workflow.md

# Verify OpenCode agent references
grep -E "codex-reviewer|gemini-analyzer|opencode-scaffolder|qwen-coder|amp-code" \
  /opencode/skill/maestro/templates/workflow.md
# Expected: multiple matches

# Verify no Claude Code agent references
grep -E "(?<!opencode-)explor|(?<!-ui-ux-engineer|oracle|librarian" \
  /opencode/skill/maestro/templates/workflow.md
# Expected: no Claude Code-only agents
```

#### 1.3 Create Code Styleguides
**Priority**: HIGH
**Effort**: 30 minutes
**Dependencies**: 1.2

**Tasks**:
1. Copy `/claude-code/templates/code_styleguides/` to `/opencode/skill/maestro/templates/code_styleguides/`
2. Review for any Claude Code-specific references
3. Add OpenCode-specific notes where needed (e.g., agent preferences)

**Decision Point**: Copy or Symlink?
- **Option A (Recommended)**: Copy files - allows OpenCode-specific customization
- **Option B**: Symlink to shared location - maintains single source of truth

**Verification**:
```bash
# Verify all styleguides exist
ls -1 /opencode/skill/maestro/templates/code_styleguides/*.md
# Expected: 10+ files (general, python, javascript, typescript, go, etc.)

# If copying: verify no symlinks
find /opencode/skill/maestro/templates/code_styleguides -type l
# Expected: empty output (if copying)
```

### Phase 2: Configuration & Integration
**Goal**: Integrate with OpenCode system

#### 2.1 Create OpenCode Configuration
**Priority**: CRITICAL
**Effort**: 1 hour
**Dependencies**: 1.1, 1.2

**Tasks**:
1. Create `/opencode/opencode-config.patch` or `/opencode/opencode.jsonc.example`
2. Define command mappings for OpenCode
3. Define agent mappings for automatic selection

**Configuration Structure**:
```json
{
  "commands": [
    {
      "name": "maestro",
      "description": "Maestro spec-driven development",
      "subcommands": {
        "setup": {
          "file": "~/.opencode/skill/maestro/commands/maestro:setup.md",
          "description": "Initialize Maestro project"
        },
        "newTrack": {
          "file": "~/.opencode/skill/maestro/commands/maestro:newTrack.md",
          "description": "Create new track"
        },
        "implement": {
          "file": "~/.opencode/skill/maestro/commands/maestro:implement.md",
          "description": "Implement track"
        },
        "status": {
          "file": "~/.opencode/skill/maestro/commands/maestro:status.md",
          "description": "View project status"
        },
        "revert": {
          "file": "~/.opencode/skill/maestro/commands/maestro:revert.md",
          "description": "Revert work"
        },
        "configure": {
          "file": "~/.opencode/skill/maestro/commands/maestro:configure.md",
          "description": "Configure Maestro settings"
        },
        "tui": {
          "file": "~/.opencode/skill/maestro/commands/maestro:tui.md",
          "description": "Launch TUI"
        },
        "memory": {
          "file": "~/.opencode/skill/maestro/commands/maestro:memory.md",
          "description": "Memory operations"
        }
      }
    }
  ],
  "agentMappings": {
    "maestro": {
      "review": "codex-reviewer",
      "analysis": "gemini-analyzer",
      "implementation": "opencode-scaffolder",
      "testing": "qwen-coder",
      "etl": "amp-code"
    }
  }
}
```

**Verification**:
```bash
# Verify config file exists
test -f /opencode/opencode.jsonc.example

# Verify JSON validity
jq empty /opencode/opencode.jsonc.example 2>/dev/null
# Expected: no errors

# Verify all commands listed
jq '.commands[0].subcommands | keys' /opencode/opencode.jsonc.example
# Expected: ["configure", "implement", "memory", "newTrack", "revert", "status", "setup", "tui"]
```

#### 2.2 Create Installer Script
**Priority**: HIGH
**Effort**: 2 hours
**Dependencies**: 2.1

**Tasks**:
1. Create `/install-opencode.sh`
2. Script should:
   - Copy skill to `~/.opencode/skill/maestro/`
   - Copy templates to `~/.claude/maestro-templates/`
   - Update `~/.config/opencode/opencode.json` with command entries
   - Set up agent mappings
   - Verify installation
   - Provide rollback option

**Script Outline**:
```bash
#!/bin/bash
set -e

INSTALL_DIR="$HOME/.opencode/skill/maestro"
TEMPLATE_DIR="$HOME/.claude/maestro-templates"
CONFIG_FILE="$HOME/.config/opencode/opencode.jsonc"

# Backup existing installation
backup_existing() {
  if [ -d "$INSTALL_DIR" ]; then
    mv "$INSTALL_DIR" "${INSTALL_DIR}.backup.$(date +%Y%m%d_%H%M%S)"
  fi
}

# Copy skill files
copy_skill() {
  mkdir -p "$INSTALL_DIR"
  cp -r opencode/skill/maestro/* "$INSTALL_DIR/"
  chmod +x "$INSTALL_DIR"/scripts/*.sh
}

# Copy templates
copy_templates() {
  mkdir -p "$TEMPLATE_DIR"
  cp -r opencode/skill/maestro/templates/* "$TEMPLATE_DIR/"
}

# Update opencode.jsonc
update_config() {
  # Backup config
  cp "$CONFIG_FILE" "${CONFIG_FILE}.backup"

  # Merge command entries
  # (Use jq or similar tool)
}

# Verify installation
verify() {
  # Check all files exist
  # Check config updated
  # Test command availability
}

# Main
backup_existing
copy_skill
copy_templates
update_config
verify

echo "✅ Maestro for OpenCode installed successfully"
```

**Verification**:
```bash
# Test installer in dry-run mode
bash -n /install-opencode.sh
# Expected: no syntax errors

# Test installation (in test environment)
./install-opencode.sh --dry-run
# Expected: shows what would be installed
```

### Phase 3: Command File Modifications
**Goal**: Adapt commands for OpenCode-specific behavior

#### 3.1 Modify maestro:setup.md
**Priority**: HIGH
**Effort**: 2 hours
**Dependencies**: 1.1, 1.2, 2.1

**Tasks**:
1. Update agent selection logic to use OpenCode agents
2. Replace workflow.md path reference
3. Update template paths
4. Add OpenCode-specific configuration options
5. Update examples to use `/maestro` prefix (not `/maestro:`)

**Key Changes**:
```
Find: ~/.claude/conductor-templates/workflow.md
Replace: ~/.claude/maestro-templates/workflow.md

Find: /maestro:
Replace: /maestro

Update agent mappings section to reference OpenCode agents
```

**Verification**:
```bash
# Verify no old references
grep -r "conductor" /opencode/skill/maestro/commands/maestro:setup.md
# Expected: no matches

# Verify new references
grep "maestro-templates" /opencode/skill/maestro/commands/maestro:setup.md
# Expected: multiple matches

# Verify command prefix
grep "/maestro " /opencode/skill/maestro/commands/maestro:setup.md | head -5
# Expected: examples with /maestro prefix
```

#### 3.2 Modify maestro:newTrack.md
**Priority**: HIGH
**Effort**: 1 hour
**Dependencies**: 3.1

**Tasks**:
1. Update agent delegation examples
2. Update template paths
3. Ensure OpenCode workflow reference

**Verification**:
```bash
# Verify workflow reference
grep "workflow.md" /opencode/skill/maestro/commands/maestro:newTrack.md
# Expected: references to maestro-templates/workflow.md
```

#### 3.3 Modify maestro:implement.md
**Priority**: HIGH
**Effort**: 2 hours
**Dependencies**: 3.1

**Tasks**:
1. Update agent selection logic for OpenCode
2. Replace Claude Code agents with OpenCode equivalents
3. Update agent delegation syntax
4. Add fallback mechanisms for missing agents

**Agent Mapping Updates**:
```python
# Old (Claude Code)
agents = {
    "oracle": "oracle",
    "librarian": "librarian",
    "explore": "explore"
}

# New (OpenCode)
agents = {
    "review": "codex-reviewer",
    "analysis": "gemini-analyzer",
    "implementation": "opencode-scaffolder",
    "testing": "qwen-coder",
    "etl": "amp-code"
}
```

**Verification**:
```bash
# Verify OpenCode agents mentioned
grep -E "codex-reviewer|gemini-analyzer|opencode-scaffolder|qwen-coder|amp-code" \
  /opencode/skill/maestro/commands/maestro:implement.md
# Expected: multiple matches

# Verify no Claude Code-only agents
grep -E "\boracle\b|\blibrarian\b|\bexplore\b(?=-)" \
  /opencode/skill/maestro/commands/maestro:implement.md
# Expected: no standalone references
```

#### 3.4 Modify Remaining Commands
**Priority**: MEDIUM
**Effort**: 2 hours
**Dependencies**: 3.1, 3.2, 3.3

**Files**:
- `maestro:status.md`
- `maestro:revert.md`
- `maestro:configure.md`
- `maestro:tui.md`
- `maestro:memory.md`

**Tasks**:
1. Update command prefix references
2. Update agent references
3. Update template paths
4. Test each command

**Verification**:
```bash
# Verify all commands
for cmd in status revert configure tui memory; do
  echo "Checking maestro:$cmd.md"
  grep -q "maestro-templates" "/opencode/skill/maestro/commands/maestro:$cmd.md"
done
# Expected: all pass
```

### Phase 4: Template & Script Updates
**Goal**: Ensure templates and scripts work correctly

#### 4.1 Update Template Scripts
**Priority**: MEDIUM
**Effort**: 1 hour
**Dependencies**: 1.2, 1.3

**Files**:
- `/opencode/skill/maestro/scripts/fix_templates.sh`
- `/opencode/skill/maestro/scripts/load_templates.sh`

**Tasks**:
1. Update to point to correct template locations
2. Add verification for OpenCode-specific files
3. Add error handling

**Key Changes**:
```bash
# Old
TEMPLATE_DIR="$HOME/.claude/conductor-templates"

# New
TEMPLATE_DIR="$HOME/.claude/maestro-templates"
```

**Verification**:
```bash
# Test scripts
bash -n /opencode/skill/maestro/scripts/fix_templates.sh
bash -n /opencode/skill/maestro/scripts/load_templates.sh
# Expected: no syntax errors

# Run verification script
/opencode/skill/maestro/scripts/load_templates.sh
# Expected: ✅ Templates verified successfully
```

#### 4.2 Create OpenCode-Specific Templates
**Priority**: LOW
**Effort**: 2 hours
**Dependencies**: 4.1

**Optional Tasks** (if customization needed):
1. Create OpenCode-specific critical think templates
2. Create OpenCode-specific agent delegation templates
3. Add OpenCode-specific examples

**Decision Point**: Are OpenCode-specific templates needed?
- If YES: Create separate template files
- If NO: Use shared templates with documentation

### Phase 5: Documentation Updates
**Goal**: Ensure documentation is accurate and complete

#### 5.1 Update SKILL.md
**Priority**: HIGH
**Effort**: 1 hour
**Dependencies**: All previous phases

**Tasks**:
1. Verify all command references are accurate
2. Update agent selection section
3. Update examples to use `/maestro` prefix
4. Add OpenCode-specific notes
5. Verify integration section

**Verification**:
```bash
# Check for accuracy
grep -E "/maestro (setup|newTrack|implement|status|revert)" \
  /opencode/skill/maestro/SKILL.md
# Expected: all commands documented

# Check agent references
grep -E "codex-reviewer|gemini-analyzer|opencode-scaffolder" \
  /opencode/skill/maestro/SKILL.md
# Expected: OpenCode agents mentioned
```

#### 5.2 Update README.md
**Priority**: HIGH
**Effort**: 1 hour
**Dependencies**: 5.1

**Tasks**:
1. Update installation instructions
2. Update agent mappings table
3. Add troubleshooting section
4. Update examples

**Verification**:
```bash
# Verify installation steps
grep -A5 "## Installation" /opencode/skill/maestro/README.md
# Expected: clear steps

# Verify agent mappings
grep -A10 "Agent Integration" /opencode/skill/maestro/README.md
# Expected: accurate table
```

#### 5.3 Update docs/OPENCODE.md
**Priority**: HIGH
**Effort**: 2 hours
**Dependencies**: 5.1, 5.2

**Tasks**:
1. Review and update all sections
2. Add detailed installation guide
3. Add agent selection examples
4. Add troubleshooting guide
5. Add migration guide from Claude Code variant

**Verification**:
```bash
# Verify all sections exist
grep -E "^## " /docs/OPENCODE.md
# Expected: Installation, Quick Start, Commands, Agents, Troubleshooting

# Verify command examples
grep -E "^/maestro " /docs/OPENCODE.md | wc -l
# Expected: 10+ examples
```

#### 5.4 Create Migration Guide
**Priority**: MEDIUM
**Effort**: 1 hour
**Dependencies**: 5.3

**Tasks**:
1. Create `/docs/MIGRATION_CLAUDE_TO_OPENCODE.md`
2. Document differences between variants
3. Provide migration steps
4. List incompatible features

**Content Outline**:
```markdown
# Migrating from Claude Code to OpenCode

## Key Differences
- Command prefix: `/maestro:` vs `/maestro`
- Agent ecosystem
- Configuration files
- Template locations

## Migration Steps
1. Install OpenCode variant
2. Copy project state
3. Update agent mappings
4. Test commands

## Feature Parity Matrix
| Feature | Claude Code | OpenCode |
|---------|-------------|----------|
| Commands | ✅ | ✅ |
| Templates | ✅ | ✅ |
| Agent Selection | ✅ | ✅ |
| TUI | ✅ | ⚠️ |
| Memory Dashboard | ✅ | ⚠️ |
```

### Phase 6: Testing & Validation
**Goal**: Ensure everything works correctly

#### 6.1 Unit Testing
**Priority**: CRITICAL
**Effort**: 3 hours
**Dependencies**: All previous phases

**Tasks**:
1. Test each command independently
2. Test agent selection logic
3. Test template loading
4. Test configuration integration

**Test Matrix**:
```bash
# Command Tests
test_setup() {
  /maestro setup
  # Verify: maestro/ directory created
  # Verify: product.md, tech-stack.md, workflow.md exist
  # Verify: tracks.md initialized
}

test_newTrack() {
  /maestro newTrack "Test track"
  # Verify: track directory created
  # Verify: spec.md, plan.md exist
  # Verify: registered in tracks.md
}

test_implement() {
  /maestro implement test-track
  # Verify: agent selection works
  # Verify: tasks executed
  # Verify: progress tracked
}

test_status() {
  /maestro status
  # Verify: displays current state
  # Verify: shows progress
}

test_revert() {
  /maestro revert task
  # Verify: task reverted
  # Verify: state updated
}
```

**Verification**:
```bash
# Run all tests
./opencode/tests/test_commands.sh
# Expected: all tests pass

# Check coverage
./opencode/tests/test_commands.sh --coverage
# Expected: >90% coverage
```

#### 6.2 Integration Testing
**Priority**: HIGH
**Effort**: 2 hours
**Dependencies**: 6.1

**Tasks**:
1. Test full workflow (setup → newTrack → implement → status)
2. Test agent delegation
3. Test memory integration
4. Test git integration

**Test Scenario**:
```bash
# Complete workflow test
cd /tmp/test-maestro
git init

# Setup
/maestro setup
# Choose: greenfield project
# Define: simple product

# Create track
/maestro newTrack "Add hello world function"
# Answer spec questions

# Implement
/maestro implement add-hello-world
# Verify: agent selection
# Verify: implementation

# Status
/maestro status
# Verify: shows progress

# Revert
/maestro revert task
# Verify: rollback
```

**Verification**:
```bash
# Test in clean environment
./opencode/tests/integration_test.sh
# Expected: all scenarios pass

# Test agent availability
for agent in codex-reviewer gemini-analyzer opencode-scaffolder qwen-coder; do
  echo "Testing $agent"
  # Verify agent can be invoked
done
```

#### 6.3 Documentation Testing
**Priority**: MEDIUM
**Effort**: 1 hour
**Dependencies**: 6.1, 6.2

**Tasks**:
1. Follow installation instructions literally
2. Test all examples in documentation
3. Verify all links work
4. Check for outdated information

**Verification**:
```bash
# Test installation steps
cat docs/OPENCODE.md | grep -A10 "## Installation" | \
  while read step; do
    # Execute each step
    # Verify success
  done

# Test examples
grep -E "^/maestro " docs/OPENCODE.md | \
  while read cmd; do
    # Test each command
  done
```

### Phase 7: Deployment & Release
**Goal**: Prepare for distribution

#### 7.1 Create Release Checklist
**Priority**: HIGH
**Effort**: 1 hour
**Dependencies**: All testing complete

**Checklist**:
- [ ] All command files created and tested
- [ ] Independent workflow.md created
- [ ] Code styleguides copied or symlinked
- [ ] OpenCode configuration created
- [ ] Installer script tested
- [ ] All documentation updated
- [ ] All tests passing
- [ ] Examples verified
- [ ] Migration guide created
- [ ] Release notes prepared

#### 7.2 Prepare Release Notes
**Priority**: MEDIUM
**Effort**: 30 minutes
**Dependencies**: 7.1

**Content**:
```markdown
# Maestro for OpenCode v1.0.0

## Features
- Full command parity with Claude Code variant
- OpenCode agent integration
- Independent workflow configuration
- Complete documentation
- One-line installer

## Installation
\`\`\`bash
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/maestro/master/install-opencode.sh | bash
\`\`\`

## What's New
- First stable OpenCode release
- Support for 8 OpenCode agents
- Automatic agent selection
- Complete TDD workflow

## Migration
See MIGRATION_CLAUDE_TO_OPENCODE.md for details.
```

#### 7.3 Tag Release
**Priority**: HIGH
**Effort**: 15 minutes
**Dependencies**: 7.2

**Tasks**:
1. Update VERSION file
2. Create git tag
3. Push to repository
4. Create GitHub release

**Commands**:
```bash
# Update version
echo "1.0.0" > VERSION
git add VERSION
git commit -m "chore: bump version to 1.0.0"

# Tag release
git tag -a opencode-v1.0.0 -m "OpenCode Maestro v1.0.0"

# Push
git push origin master --tags
```

## File Operations Summary

### Files to Create
```
/opencode/skill/maestro/commands/
  ├── maestro:setup.md           (copy & modify from claude-code)
  ├── maestro:newTrack.md        (copy & modify from claude-code)
  ├── maestro:implement.md       (copy & modify from claude-code)
  ├── maestro:status.md          (copy & modify from claude-code)
  ├── maestro:revert.md          (copy & modify from claude-code)
  ├── maestro:configure.md       (copy & modify from claude-code)
  ├── maestro:tui.md             (copy & modify from claude-code)
  └── maestro:memory.md          (copy & modify from claude-code)

/opencode/skill/maestro/templates/
  ├── workflow.md                (create OpenCode-specific)
  └── code_styleguides/
      ├── general.md             (copy from claude-code)
      ├── python.md              (copy from claude-code)
      ├── javascript.md          (copy from claude-code)
      ├── typescript.md          (copy from claude-code)
      ├── go.md                  (copy from claude-code)
      ├── html-css.md            (copy from claude-code)
      ├── react.md               (copy from claude-code)
      ├── vue.md                 (copy from claude-code)
      ├── rust.md                (copy from claude-code)
      ├── shell.md               (copy from claude-code)
      ├── nextjs.md              (copy from claude-code)
      ├── nodejs.md              (copy from claude-code)
      ├── docker.md              (copy from claude-code)
      └── graphql.md             (copy from claude-code)

/opencode/
  ├── opencode.jsonc.example     (create)
  └── install-opencode.sh        (create)

/docs/
  └── MIGRATION_CLAUDE_TO_OPENCODE.md  (create)
```

### Files to Modify
```
/opencode/skill/maestro/SKILL.md           (update agent references)
/opencode/skill/maestro/README.md          (update installation)
/opencode/skill/maestro/scripts/fix_templates.sh   (update paths)
/opencode/skill/maestro/scripts/load_templates.sh  (update paths)
/docs/OPENCODE.md                          (comprehensive update)
/docs/AGENTS.md                            (verify OpenCode agents)
/README.md                                 (add OpenCode variant section)
```

### Files to Remove
```
/opencode/skill/maestro/commands/conductor:*.md  (remove old symlinks)
```

## Order of Operations

### Critical Path (Must Follow This Order)
1. **Phase 1.1**: Create command files (foundation)
2. **Phase 1.2**: Create independent workflow (agent mappings)
3. **Phase 1.3**: Create code styleguides (templates)
4. **Phase 2.1**: Create OpenCode configuration (integration)
5. **Phase 2.2**: Create installer script (deployment)
6. **Phase 3.1**: Modify maestro:setup.md (first command)
7. **Phase 3.2**: Modify maestro:newTrack.md
8. **Phase 3.3**: Modify maestro:implement.md (critical agent logic)
9. **Phase 3.4**: Modify remaining commands
10. **Phase 4**: Update scripts and templates
11. **Phase 5**: Update all documentation
12. **Phase 6**: Complete testing
13. **Phase 7**: Deploy release

### Parallel Opportunities
- **Phase 1.3** can run parallel to **Phase 2.1**
- **Phase 3.2, 3.3, 3.4** can run in parallel after 3.1
- **Phase 5.1, 5.2, 5.3** can run in parallel after Phase 4
- **Phase 6.1, 6.2, 6.3** can run in parallel

## Dependencies

### External Dependencies
- OpenCode system installed and configured
- OpenCode agents available (codex-reviewer, gemini-analyzer, etc.)
- jq (for JSON manipulation in installer)
- Standard Unix utilities (bash, sed, grep, find)

### Internal Dependencies
- Claude Code variant command files (for copying)
- Claude Code templates (for copying or symlinking)
- Maestro core functionality (shared between variants)

## Risk Mitigation

### High-Risk Areas
1. **Agent Configuration** (Phase 2.1)
   - Risk: Incorrect agent mappings break functionality
   - Mitigation: Extensive testing, clear documentation, fallback mechanisms

2. **Command Modifications** (Phase 3)
   - Risk: Breaking changes to command logic
   - Mitigation: Incremental changes, thorough testing at each step

3. **Installer Script** (Phase 2.2)
   - Risk: Installation failures, user data loss
   - Mitigation: Backup functionality, dry-run mode, rollback option

### Rollback Plan
If critical issues found:
1. Revert to previous version using git
2. Document issues in GitHub issues
3. Create hotfix branch
4. Test thoroughly
5. Release patch version

## Acceptance Criteria

### Phase Completion Criteria
- **Phase 1 Complete**: All commands and templates exist and pass syntax checks
- **Phase 2 Complete**: Configuration valid, installer script runs successfully
- **Phase 3 Complete**: All commands reference OpenCode agents, no Claude Code remnants
- **Phase 4 Complete**: Scripts work correctly, templates load successfully
- **Phase 5 Complete**: All documentation accurate and verified
- **Phase 6 Complete**: All tests passing, coverage >90%
- **Phase 7 Complete**: Release tagged and published

### Overall Success Criteria
- [ ] All 8 commands functional with `/maestro` prefix
- [ ] OpenCode agent selection working automatically
- [ ] Templates load correctly from maestro-templates
- [ ] Installation works via one-line installer
- [ ] Documentation complete and accurate
- [ ] All tests passing
- [ ] Migration guide available
- [ ] Release notes prepared

## Success Metrics

### Quantitative Metrics
- **Commands**: 8/8 functional (100%)
- **Tests**: Pass rate >95%
- **Coverage**: >90% code coverage
- **Documentation**: 100% of commands documented
- **Installation**: <5 minutes, <10 commands

### Qualitative Metrics
- User can install and run `/maestro setup` successfully
- Agent selection feels natural and automatic
- Documentation is clear and comprehensive
- Migration from Claude Code variant is straightforward

## Open Questions

### Decision Required

1. **Code Styleguides: Copy or Symlink?**
   - **Option A**: Copy files (allows customization, duplicates content)
   - **Option B**: Symlink to shared (maintains single source, limits customization)
   - **Recommendation**: Copy for independence, can consolidate later

2. **Agent Selection: Automatic or User-Configurable?**
   - **Option A**: Fully automatic (simpler, less flexible)
   - **Option B**: User-configurable (more flexible, more complex)
   - **Recommendation**: Automatic with override option

3. **Workflow.md: Shared or Independent?**
   - **Option A**: Shared (easier maintenance, compromises agent specificity)
   - **Option B**: Independent (full customization, maintenance overhead)
   - **Recommendation**: Independent for OpenCode (agent differences significant)

4. **TUI Support: Include or Defer?**
   - **Option A**: Include now (feature complete, more testing)
   - **Option B**: Defer to v1.1 (faster release, known limitation)
   - **Recommendation**: Defer if not critical, prioritize core commands

5. **Memory Dashboard: Include or Defer?**
   - **Option A**: Include now (feature complete, more complexity)
   - **Option B**: Defer to v1.1 (faster release, can add later)
   - **Recommendation**: Defer, focus on core workflow first

## Next Steps

### Immediate Actions (Required)
1. **Review and approve this plan** - Get stakeholder sign-off
2. **Answer open questions** - Make decisions on unresolved items
3. **Assign tasks** - Determine who will execute each phase
4. **Set timeline** - Establish target dates for each phase
5. **Begin Phase 1** - Start with command file creation

### Coordination with Other Agents
- **opencode-scaffolder agent**: Research OpenCode-specific requirements
- **droid-factory agent**: Design and structure feedback
- **rovo-dev (this agent)**: Synthesize findings and create implementation plan

### Communication Plan
- Daily progress updates during implementation
- Blocker alerts immediately
- Test results summary after Phase 6
- Release announcement after Phase 7

## Appendix

### Agent Mapping Reference

```
Task Type                    | Claude Code Agent      | OpenCode Agent
-----------------------------|------------------------|------------------
Architecture & Strategy      | oracle                 | codex-reviewer
Large Codebase Analysis      | librarian              | gemini-analyzer
Rapid Prototyping            | explore                | opencode-scaffolder
Testing & Documentation      | document-writer        | qwen-coder
ETL/Data Pipelines           | N/A                   | amp-code
Security Review              | oracle                 | codex-reviewer
UI/UX Implementation         | frontend-ui-ux-engineer| opencode-scaffolder
```

### File Path Reference

```
Repository Root: /home/stan/Prod/maestro/

Claude Code Variant:
  Commands: /claude-code/commands/
  Templates: /claude-code/templates/

OpenCode Variant:
  Skill: /opencode/skill/maestro/
  Commands: /opencode/skill/maestro/commands/
  Templates: /opencode/skill/maestro/templates/
  Config: /opencode/opencode.jsonc.example
  Installer: /install-opencode.sh

Shared (installed locations):
  Commands: ~/.claude/commands/
  Templates: ~/.claude/maestro-templates/
  Skill: ~/.opencode/skill/maestro/
  Config: ~/.config/opencode/opencode.jsonc
```

### Testing Commands Reference

```bash
# Syntax check all bash scripts
find . -name "*.sh" -exec bash -n {} \;

# Verify all markdown files are valid
find . -name "*.md" -exec echo {} \;

# Check for broken symlinks
find . -type l ! -exec test -e {} \;

# Verify JSON files
jq empty opencode/opencode.jsonc.example

# Count lines of code
find opencode/skill/maestro/commands -name "*.md" | xargs wc -l

# Test installer (dry-run)
bash -x install-opencode.sh --dry-run 2>&1 | tee install.log
```

## Conclusion

This implementation plan provides a comprehensive roadmap for making the OpenCode variant of Maestro fully functional and independent. By following these phases in order, and addressing the open questions, we can deliver a robust, well-tested OpenCode integration that maintains feature parity with the Claude Code variant while leveraging OpenCode's unique agent ecosystem.

**Estimated Total Effort**: 25-30 hours
**Estimated Timeline**: 3-4 days (single developer) or 1 week (with review/testing)
**Risk Level**: MEDIUM (mitigated by thorough testing and rollback plan)

---

**Document Status**: READY FOR REVIEW
**Next Review**: After stakeholder approval
**Owner**: rovo-dev agent
**Last Updated**: 2026-01-05
