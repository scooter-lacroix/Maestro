# Maestro v1 to v2 Migration Guide

This guide helps you migrate from Maestro v1.x to Maestro v2.0.

## Overview of Changes

Maestro v2 represents a major architectural evolution that unifies three separate systems:

- **Nexus Memory System** - Now fully integrated with 95-100% capture reliability
- **109 Skills** - Rebranded from CCv3 to Maestro namespace
- **28 Agents** - Rebranded from CCv3 to Maestro namespace
- **30 Hooks** - Rebranded from CCv3 to Maestro namespace
- **TLDR Code Analysis** - Integrated 5-layer code analysis
- **UV Package Management** - New Python package management system

## Breaking Changes

### 1. Package Structure

**v1:**
```
maestro/
├── commands/
├── templates/
└── tracks/
```

**v2:**
```
maestro/
├── maestro/              # UV-managed Python package
│   ├── __init__.py
│   ├── cli.py
│   ├── config/
│   ├── core/
│   │   ├── agents/
│   │   └── tracks/
│   ├── memory/
│   │   ├── database/
│   │   ├── coordination/
│   │   ├── hooks/
│   │   └── embeddings/
│   ├── skills/
│   ├── agents/
│   ├── hooks/
│   ├── tldr/
│   ├── critical_think/
│   └── tui/
├── claude-code/
│   ├── commands/
│   └── templates/
├── tracks/
└── scripts/
```

### 2. Command Syntax

**v1:**
```bash
/maestro setup
/maestro newTrack "Add feature"
/maestro implement track-name
```

**v2 (Claude Code):**
```bash
/maestro:setup
/maestro:newTrack Add feature
/maestro:implement track-name
```

**v2 (OpenCode):**
```bash
/maestro setup
/maestro newTrack "Add feature"
/maestro implement track-name
```

### 3. Memory System

**v1:**
- Separate Nexus Memory MCP server
- External memory database

**v2:**
- Unified memory system built into Maestro core
- Single SQLite database at `~/.maestro/memory.db`
- No external MCP required

### 4. Skill and Agent Names

All skills and agents have been rebranded to the Maestro namespace:

**v1:** `/cc:workflow`, `/cc:tdd`, `/cc:refactor`
**v2:** `/maestro:workflow`, `/maestro:tdd`, `/maestro:refactor`

## Migration Steps

### Step 1: Backup Your Data

Before migrating, back up your existing Maestro data:

```bash
# Backup memory database (if using Nexus)
cp ~/.nexus/memory.db ~/.nexus/memory.db.backup

# Backup project tracks
cp -r ~/my-project/maestro ~/my-project/maestro.backup
```

### Step 2: Install Maestro v2

**Option A: Marketplace Installation (Claude Code)**
```bash
# Add the marketplace repository
/plugin marketplace add scooter-lacroix/maestro

# Install Maestro v2
/plugin install maestro

# Run setup
/maestro:setup
```

**Option B: Full Installation (CLI Tools)**
```bash
# One-line installer
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-claude-code.sh | bash

# Or for OpenCode
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-opencode.sh | bash
```

### Step 3: Migrate Memory Data

Maestro v2 includes an automatic migration script for basic data:

```bash
# Run the migration script
python -m maestro.memory.migrations.migrate_memory --source ~/.nexus/memory.db

# Or using the CLI
maestro memory migrate --from ~/.nexus/memory.db
```

**What Gets Migrated Automatically:**
- Project contexts
- Track specifications
- User preferences
- Basic facts

**What Requires Manual Migration:**
- Coordination state (file claims, handoffs, ledgers)
- Agent-specific memory namespaces
- Complex task specifications

### Step 4: Update Command References

Update any scripts or documentation that reference v1 commands:

**Find and Replace:**
```bash
# In your project files
# v1 -> v2 (Claude Code)
/maestro setup      -> /maestro:setup
/maestro newTrack   -> /maestro:newTrack
/maestro implement  -> /maestro:implement
/maestro status     -> /maestro:status
/maestro revert     -> /maestro:revert

# v1 -> v2 (OpenCode - no change needed)
# Commands remain the same for OpenCode
```

### Step 5: Update Skill and Agent References

If you have custom skills or agents:

```bash
# Update skill frontmatter
# Old: command: /cc:workflow
# New: command: /maestro:workflow

# Update agent references
# Old: agent: cc-oracle
# New: agent: maestro
```

### Step 6: Verify Installation

```bash
# Check version
maestro --version
# Should output: Maestro v2.0.0

# Run diagnostics
/maestro:configure

# Verify memory system
maestro memory status
```

## Data Migration Details

### Memory Database Migration

The migration script handles:

1. **Memories Table**
   - Preserves all memory entries
   - Maps old agent types to new Maestro namespaces
   - Adds project_id and track_id fields

2. **Agent Namespaces**
   - Merges CC namespaces into Maestro namespaces
   - Preserves namespace-specific settings

3. **Sessions**
   - Migrates session tracking data
   - Updates to new schema format

**Manual Migration Required For:**

1. **File Claims**
   ```sql
   -- Export from old database
   SELECT * FROM file_claims;

   -- Import to new database with updated schema
   INSERT INTO file_claims (...) VALUES (...);
   ```

2. **Handoffs**
   ```yaml
   # Convert handoff format
   # Old: CC-specific format
   # New: Maestro unified format
   ```

3. **Continuity Ledgers**
   - Export ledger entries
   - Transform to new format
   - Import to v2 database

## Configuration Migration

### v1 Configuration (.maestro.json)

```json
{
  "version": "1.0",
  "memory_enabled": true,
  "tdd_enforced": true,
  "coverage_target": 80
}
```

### v2 Configuration (defaults.yaml)

```yaml
version: "2.0"

core:
  tdd_enforced: true
  coverage_target: 98

memory:
  enabled: true
  embeddings_enabled: true
  coordination:
    file_claims_enabled: true
    handoffs_enabled: true
    ledgers_enabled: true

skills:
  workflow:
    enabled: true
  tdd:
    enforcement: "require"
```

Run `/maestro:configure` to generate the new configuration.

## Troubleshooting

### Issue: Memory Not Migrating

**Solution:**
```bash
# Check database integrity
sqlite3 ~/.nexus/memory.db "PRAGMA integrity_check;"

# Run migration with verbose output
maestro memory migrate --from ~/.nexus/memory.db --verbose

# If failing, export and import manually
python -c "
from maestro.memory.database.managers import MemoryManager
import json
# Export old data
with open('old_memories.json', 'w') as f:
    json.dump(old_data, f)
# Import to new database
"
```

### Issue: Commands Not Recognized

**Solution:**
```bash
# Verify installation
ls ~/.claude/commands/ | grep maestro

# Reinstall commands
./install-claude-code.sh

# For OpenCode
cat ~/.config/opencode/opencode.json | grep maestro
```

### Issue: Agent Not Found

**Solution:**
```bash
# Run configuration to verify agents
/maestro:configure

# Check agent registry
cat maestro/agents/registry.yaml

# Verify agent files exist
ls maestro/agents/
```

### Issue: Skill Not Activating

**Solution:**
```bash
# Check skill registry
cat maestro/skills/skill-rules.json

# Verify skill files exist
ls maestro/skills/

# Test skill activation
python -c "
from maestro.skills.activation import activate_skills_for_prompt
result = activate_skills_for_prompt('I need to write tests')
print(result.to_dict())
"
```

## Rollback Procedure

If you need to rollback to v1:

```bash
# Restore backup
cp ~/.nexus/memory.db.backup ~/.nexus/memory.db
cp -r ~/my-project/maestro.backup ~/my-project/maestro

# Uninstall v2
/plugin uninstall maestro

# Reinstall v1 (from v1 branch)
git checkout v1.0
./install-claude-code.sh
```

## Example Migration Scenarios

### Scenario 1: Simple Project with No Custom Skills

```bash
# 1. Backup
cp -r myproject/maestro myproject/maestro.backup

# 2. Install v2
curl -sSL https://raw.githubusercontent.com/scooter-lacroix/Maestro/master/install-claude-code.sh | bash

# 3. Migrate memory
maestro memory migrate --from ~/.nexus/memory.db

# 4. Verify
/maestro:status
```

### Scenario 2: Project with Custom Skills

```bash
# 1. Backup custom skills
cp -r ~/.claude/skills/custom ~/custom-skills.backup

# 2. Update skill files
# Edit each skill's frontmatter
# Old: command: /cc:custom-skill
# New: command: /maestro:custom-skill

# 3. Copy to new location
cp -r ~/custom-skills.backup ~/.claude/skills/

# 4. Verify skill activation
/maestro:custom-skill test
```

### Scenario 3: Project with Active Tracks

```bash
# 1. Backup tracks
cp -r myproject/maestro/tracks myproject/maestro/tracks.backup

# 2. Migrate each track
for track in myproject/maestro/tracks/*/; do
    # Update metadata format
    python -m maestro.cli migrate-track "$track"
done

# 3. Verify migration
/maestro:status
```

## Support

For migration issues:

1. Check this guide first
2. Review [GitHub Issues](https://github.com/scooter-lacroix/Maestro/issues)
3. Create a new issue with:
   - Maestro v1 version
   - Migration steps taken
   - Error messages
   - Environment details

## Summary

| Aspect | v1 | v2 |
|--------|----|----|
| Package | Plugin-based | UV-managed Python package |
| Memory | External Nexus MCP | Built-in unified memory |
| Skills | CC namespace | Maestro namespace |
| Agents | CC namespace | Maestro namespace |
| Commands | `/maestro command` | `/maestro:command` (Claude Code) |
| Hooks | CCv3 hooks | Maestro hooks |
| Configuration | JSON | YAML |
| Python | 3.10+ | 3.11+ |
| Package Manager | None required | UV |

**Migration Time Estimate:**
- Simple projects: 15-30 minutes
- Projects with custom skills: 1-2 hours
- Complex projects with active tracks: 2-4 hours

**Success Indicators:**
- All commands work with new syntax
- Memory system shows correct statistics
- Tracks load and display correctly
- Skills activate as expected
- Tests pass with >98% coverage
