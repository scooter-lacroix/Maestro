# OpenCode Maestro - Quick Start Implementation Guide

**Purpose**: Fast-track implementation checklist for OpenCode Maestro variant
**Companion to**: IMPLEMENTATION_PLAN.md (detailed plan)

## Pre-Implementation Checklist

### Prerequisites
- [ ] OpenCode system installed and working
- [ ] Git access to maestro repository
- [ ] Write access to `~/.opencode/skill/` and `~/.claude/`
- [ ] Required tools: `jq`, `bash`, `sed`, `grep`, `find`

### Decisions Needed (Answer Before Starting)
- [ ] Code styleguides: Copy (A) or Symlink (B)? → **RECOMMENDED: Copy**
- [ ] Agent selection: Automatic (A) or Configurable (B)? → **RECOMMENDED: Automatic**
- [ ] workflow.md: Shared (A) or Independent (B)? → **RECOMMENDED: Independent**
- [ ] TUI support: Include now (A) or Defer (B)? → **RECOMMENDED: Defer**
- [ ] Memory dashboard: Include now (A) or Defer (B)? → **RECOMMENDED: Defer**

## 7-Phase Implementation (Quick Reference)

### Phase 1: Foundation (2-3 hours)
**Goal**: Create independent file structure

```bash
# 1.1 Create command files
cd /home/stan/Prod/maestro
cp claude-code/commands/maestro:*.md opencode/skill/maestro/commands/
# Result: 8 command files created

# 1.2 Create independent workflow
cp claude-code/templates/workflow.md opencode/skill/maestro/templates/workflow.md
# Edit: Replace agent mappings (see Agent Mapping section below)
# Result: OpenCode-specific workflow

# 1.3 Create code styleguides
mkdir -p opencode/skill/maestro/templates/code_styleguides
cp claude-code/templates/code_styleguides/*.md opencode/skill/maestro/templates/code_styleguides/
# Result: 27 styleguide files
```

**Verification**:
```bash
ls -1 opencode/skill/maestro/commands/*.md | wc -l  # Should be 8
test -f opencode/skill/maestro/templates/workflow.md
ls -1 opencode/skill/maestro/templates/code_styleguides/*.md | wc -l  # Should be 27
```

### Phase 2: Configuration (2-3 hours)
**Goal**: Integrate with OpenCode system

```bash
# 2.1 Create OpenCode configuration
cat > opencode/opencode.jsonc.example <<'EOF'
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
EOF

# 2.2 Create installer script (use template from IMPLEMENTATION_PLAN.md)
cat > install-opencode.sh <<'EOF'
#!/bin/bash
# Full script in IMPLEMENTATION_PLAN.md Section 2.2
# Copy from there and customize
EOF
chmod +x install-opencode.sh
```

**Verification**:
```bash
jq empty opencode/opencode.jsonc.example  # Should succeed
bash -n install-opencode.sh  # Should have no syntax errors
```

### Phase 3: Command Modifications (4-5 hours)
**Goal**: Adapt commands for OpenCode

**Agent Mapping Reference** (use for all find/replace):
```
Claude Code → OpenCode
oracle → codex-reviewer
librarian → gemini-analyzer
explore → opencode-scaffolder
macgyver → qwen-coder
dexter → amp-code|opencode-scaffolder
```

**Command Prefix Reference**:
```
Claude Code: /maestro:
OpenCode: /maestro
```

**Template Path Reference**:
```
Claude Code: ~/.claude/conductor-templates/
OpenCode: ~/.claude/maestro-templates/
```

```bash
# 3.1-3.4 Modify all commands
cd opencode/skill/maestro/commands

# Global replacements (apply to all .md files)
for file in *.md; do
  # Fix command prefix
  sed -i 's|/maestro:|/maestro |g' "$file"

  # Fix template paths
  sed -i 's|conductor-templates|maestro-templates|g' "$file"

  # Fix agent references (manual review needed for context)
  # See detailed mapping in IMPLEMENTATION_PLAN.md
done

# Specific manual edits required:
# - maestro:implement.md: Update agent selection logic (Section 3.3)
# - maestro:setup.md: Update agent configuration options (Section 3.1)
# - maestro:newTrack.md: Update agent delegation examples (Section 3.2)
```

**Verification**:
```bash
# Check no old references remain
grep -r "conductor" .
grep -r "/maestro:" .
# Both should return empty

# Check new references present
grep -r "maestro-templates" . | head -3
grep -r "codex-reviewer\|gemini-analyzer\|opencode-scaffolder" . | head -3
```

### Phase 4: Scripts & Templates (1-2 hours)
**Goal**: Update supporting files

```bash
# 4.1 Update template scripts
cd opencode/skill/maestro/scripts

# Edit fix_templates.sh
sed -i 's|conductor-templates|maestro-templates|g' fix_templates.sh

# Edit load_templates.sh
sed -i 's|conductor-templates|maestro-templates|g' load_templates.sh

# Verification
bash -n fix_templates.sh
bash -n load_templates.sh
```

### Phase 5: Documentation (2-3 hours)
**Goal**: Update all docs

```bash
# 5.1 Update SKILL.md
cd opencode/skill/maestro
# Edit SKILL.md: Verify command accuracy, update agent references

# 5.2 Update README.md
# Edit README.md: Update installation, agent mappings, examples

# 5.3 Update docs/OPENCODE.md
cd ../../docs
# Edit OPENCODE.md: Comprehensive update of all sections

# 5.4 Create migration guide
cat > MIGRATION_CLAUDE_TO_OPENCODE.md <<'EOF'
# Migrating from Claude Code to OpenCode
[Content from IMPLEMENTATION_PLAN.md Section 5.4]
EOF
```

**Verification**:
```bash
# Check command examples in docs
grep -E "^/maestro " opencode/skill/maestro/SKILL.md | wc -l  # Should be >5
grep -E "^/maestro " docs/OPENCODE.md | wc -l  # Should be >10
```

### Phase 6: Testing (3-4 hours)
**Goal**: Verify everything works

```bash
# 6.1 Unit tests
cd /home/stan/Prod/maestro

# Create test script
cat > opencode/tests/test_commands.sh <<'EOF'
#!/bin/bash
# Test script from IMPLEMENTATION_PLAN.md Section 6.1
EOF
chmod +x opencode/tests/test_commands.sh

# Run tests
./opencode/tests/test_commands.sh

# 6.2 Integration tests
cat > opencode/tests/integration_test.sh <<'EOF'
#!/bin/bash
# Integration test from IMPLEMENTATION_PLAN.md Section 6.2
EOF
chmod +x opencode/tests/integration_test.sh

# Run integration test
./opencode/tests/integration_test.sh
```

**Success Criteria**:
- [ ] All unit tests pass (>95% pass rate)
- [ ] All integration tests pass
- [ ] Code coverage >90%
- [ ] No broken symlinks
- [ ] All JSON files valid

### Phase 7: Release (1 hour)
**Goal**: Deploy

```bash
# 7.1 Final checklist
# Review IMPLEMENTATION_PLAN.md Section 7.1

# 7.2 Release notes
cat > RELEASE_NOTES_OPENCODE.md <<'EOF'
# Maestro for OpenCode v1.0.0
[Content from IMPLEMENTATION_PLAN.md Section 7.2]
EOF

# 7.3 Tag and push
echo "1.0.0" > VERSION
git add VERSION
git commit -m "chore: bump version to 1.0.0 (opencode)"
git tag -a opencode-v1.0.0 -m "OpenCode Maestro v1.0.0"
git push origin master --tags
```

## Critical Find/Replace Patterns

### Agent Mappings (Apply to all command files)

```bash
# Find and replace patterns for agent references
# NOTE: Review context before applying!

# Architecture & Strategy
s/\boracle\b/codex-reviewer/g

# Analysis
s/\blibrarian\b/gemini-analyzer/g

# Implementation
s/\bexplore\b/opencode-scaffolder/g

# Testing
s/\bmacgyver\b/qwen-coder/g

# Complex tasks
s/\bdexter\b/(amp-code|opencode-scaffolder)/g
```

### Command Prefix

```bash
# Claude Code uses: /maestro:setup
# OpenCode uses: /maestro setup

s|/maestro:|/maestro |g
```

### Template Paths

```bash
# Old: ~/.claude/conductor-templates/
# New: ~/.claude/maestro-templates/

s|conductor-templates|maestro-templates|g
```

## Testing Commands

### Quick Verification (Run After Each Phase)

```bash
# Phase 1 verification
test -f opencode/skill/maestro/commands/maestro:setup.md
test -f opencode/skill/maestro/templates/workflow.md
ls opencode/skill/maestro/templates/code_styleguides/*.md | wc -l

# Phase 2 verification
jq empty opencode/opencode.jsonc.example
bash -n install-opencode.sh

# Phase 3 verification
grep -r "conductor" opencode/skill/maestro/commands/
# Expected: no results
grep -r "maestro-templates" opencode/skill/maestro/commands/
# Expected: multiple results

# Phase 4 verification
bash -n opencode/skill/maestro/scripts/*.sh

# Phase 5 verification
grep -E "^/maestro " opencode/skill/maestro/SKILL.md | wc -l
grep -E "^/maestro " docs/OPENCODE.md | wc -l

# Phase 6 verification
./opencode/tests/test_commands.sh
./opencode/tests/integration_test.sh
```

### Full Test Suite

```bash
# Syntax check all scripts
find . -name "*.sh" -exec bash -n {} \;

# Check for broken symlinks
find opencode/skill/maestro -type l ! -exec test -e {} \;

# Verify JSON files
for json in $(find . -name "*.json" -o -name "*.jsonc"); do
  jq empty "$json" 2>/dev/null || echo "Invalid: $json"
done

# Count command files
find opencode/skill/maestro/commands -name "*.md" | wc -l
# Expected: 8

# Count template files
find opencode/skill/maestro/templates -name "*.md" | wc -l
# Expected: 15+ (workflow + styleguides)
```

## Common Issues & Solutions

### Issue: Command not found
```
Symptom: /maestro setup returns "command not found"
Solution:
  1. Verify commands copied: ls ~/.opencode/skill/maestro/commands/
  2. Verify config updated: check ~/.config/opencode/opencode.jsonc
  3. Restart OpenCode
```

### Issue: Wrong agent selected
```
Symptom: Agent selection uses Claude Code agents
Solution:
  1. Check agent mappings in opencode.jsonc
  2. Verify command files updated (no "oracle", "librarian" references)
  3. Check workflow.md has correct agent mappings
```

### Issue: Templates not loading
```
Symptom: "workflow.md not found"
Solution:
  1. Verify templates installed: ls ~/.claude/maestro-templates/
  2. Check symlinks: find ~/.opencode/skill/maestro/templates -type l
  3. Run fix_templates.sh script
```

### Issue: Installation fails
```
Symptom: install-opencode.sh errors
Solution:
  1. Check prerequisites: jq, bash
  2. Verify file permissions: chmod +x install-opencode.sh
  3. Run with --debug flag
  4. Check logs in install.log
```

## Time Estimates (Per Phase)

| Phase | Tasks | Estimated Time | Dependencies |
|-------|-------|----------------|--------------|
| 1 | Foundation | 2-3 hours | None |
| 2 | Configuration | 2-3 hours | Phase 1 |
| 3 | Command Mods | 4-5 hours | Phase 1, 2 |
| 4 | Scripts | 1-2 hours | Phase 1, 2 |
| 5 | Documentation | 2-3 hours | Phase 3, 4 |
| 6 | Testing | 3-4 hours | All above |
| 7 | Release | 1 hour | All above |
| **Total** | | **15-21 hours** | |

## Parallel Work Opportunities

**Can run in parallel**:
- Phase 1.3 (styleguides) + Phase 2.1 (config)
- Phase 3.2, 3.3, 3.4 (after 3.1 complete)
- Phase 5.1, 5.2, 5.3 (after Phase 4 complete)
- Phase 6.1, 6.2, 6.3 (after Phase 5 complete)

**Must run sequentially**:
- Phase 1.1 → Phase 1.2 → Phase 3.1
- Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7

## Success Checklist

### Files Created (23 total)
- [ ] 8 command files in `/opencode/skill/maestro/commands/`
- [ ] 1 workflow.md in `/opencode/skill/maestro/templates/`
- [ ] 27 code styleguides in `/opencode/skill/maestro/templates/code_styleguides/`
- [ ] 1 opencode.jsonc.example in `/opencode/`
- [ ] 1 install-opencode.sh in repository root
- [ ] 1 MIGRATION_CLAUDE_TO_OPENCODE.md in `/docs/`

### Files Modified (6 total)
- [ ] SKILL.md (agent references)
- [ ] README.md (installation)
- [ ] fix_templates.sh (paths)
- [ ] load_templates.sh (paths)
- [ ] docs/OPENCODE.md (comprehensive)
- [ ] docs/AGENTS.md (verify)

### Tests Pass
- [ ] All unit tests pass (>95%)
- [ ] All integration tests pass
- [ ] Code coverage >90%
- [ ] Manual testing successful

### Documentation Complete
- [ ] All commands documented with examples
- [ ] Installation instructions verified
- [ ] Agent mappings documented
- [ ] Troubleshooting guide complete
- [ ] Migration guide available

### Ready for Release
- [ ] VERSION file updated
- [ ] Release notes prepared
- [ ] Git tag created
- [ ] Changelog updated

## Quick Commands Reference

### Setup (One-time)
```bash
cd /home/stan/Prod/maestro
# Ensure prerequisites
which jq bash sed grep find

# Start Phase 1
bash opencode/scripts/phase1_foundation.sh
```

### Development (Iterative)
```bash
# After editing commands
cd opencode/skill/maestro/commands
bash ../../tests/verify_commands.sh

# After editing scripts
cd scripts
bash -n *.sh  # Syntax check

# After editing config
jq empty opencode.jsonc.example
```

### Testing (Before Commit)
```bash
cd /home/stan/Prod/maestro

# Run all tests
./opencode/tests/run_all_tests.sh

# Check coverage
./opencode/tests/check_coverage.sh

# Verify installation
./install-opencode.sh --dry-run
```

### Release (Final)
```bash
cd /home/stan/Prod/maestro

# Final checks
./opencode/tests/pre_release_check.sh

# Tag release
git tag -a opencode-v1.0.0 -m "OpenCode Maestro v1.0.0"

# Push
git push origin master --tags
```

## Get Help

### Documentation
- **Detailed Plan**: `opencode/IMPLEMENTATION_PLAN.md`
- **Agent Mappings**: `docs/AGENTS.md`
- **OpenCode Guide**: `docs/OPENCODE.md`
- **Migration**: `docs/MIGRATION_CLAUDE_TO_OPENCODE.md` (create)

### Troubleshooting
1. Check `opencode/IMPLEMENTATION_PLAN.md` Section: "Common Issues & Solutions"
2. Review test logs in `opencode/tests/`
3. Check installation logs: `install.log`

### Status Check
```bash
# Show implementation progress
cd /home/stan/Prod/maestro
cat opencode/IMPLEMENTATION_PLAN.md | grep "^\- \[ \]" | wc -l  # Remaining tasks
cat opencode/IMPLEMENTATION_PLAN.md | grep "^\- \[x\]" | wc -l  # Completed
```

---

**Quick Start Status**: READY TO BEGIN
**Estimated Time to Complete**: 15-21 hours
**Recommended Approach**: Follow phases sequentially, leverage parallel opportunities
**Next Step**: Answer pre-implementation checklist questions, then begin Phase 1
