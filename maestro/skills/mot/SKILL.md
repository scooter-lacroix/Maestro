---
name: mot
description: System health check (MOT) for skills, agents, hooks, and memory
model: sonnet
allowed-tools: [Read, Bash, Glob, Grep]
---

# MOT - System Health Check

Run comprehensive health checks on all Maestro components.

## Usage

```
/mot              # Full audit (all categories)
/mot skills       # Just skills
/mot agents       # Just agents
/mot hooks        # Just hooks
/mot memory       # Just memory system
/mot --fix        # Auto-fix simple issues
/mot --quick      # P0 checks only (fast)
```

## Audit Process

### Phase 1: Skills Audit
```bash
# Count skills
echo "=== SKILLS ==="
SKILL_COUNT=$(find .maestro/skills -name "SKILL.md" | wc -l | xargs)
echo "Found $SKILL_COUNT skill files"

# Check frontmatter parsing
FAIL=0
for skill in $(find .maestro/skills -name "SKILL.md"); do
  if ! head -1 "$skill" | grep -q "^---$"; then
    echo "FAIL: No frontmatter: $skill"
    FAIL=$((FAIL+1))
  fi
done
echo "Frontmatter: $((SKILL_COUNT - FAIL)) pass, $FAIL fail"

# Check name matches directory
FAIL=0
for skill in $(find .maestro/skills -name "SKILL.md"); do
  dir=$(basename $(dirname "$skill"))
  name=$(grep "^name:" "$skill" 2>/dev/null | head -1 | cut -d: -f2 | xargs)
  if [ -n "$name" ] && [ "$dir" != "$name" ]; then
    echo "FAIL: Name mismatch $dir vs $name"
    FAIL=$((FAIL+1))
  fi
done
echo "Name consistency: $((SKILL_COUNT - FAIL)) pass, $FAIL fail"
```

### Phase 2: Agents Audit
```bash
echo "=== AGENTS ==="
AGENT_COUNT=$(ls .maestro/agents/*.md 2>/dev/null | wc -l | xargs)
echo "Found $AGENT_COUNT agent files"

# Check required fields
FAIL=0
for agent in .maestro/agents/*.md; do
  [ -f "$agent" ] || continue

  # Check name field exists
  if ! grep -q "^name:" "$agent"; then
    echo "FAIL: Missing name: $agent"
    FAIL=$((FAIL+1))
    continue
  fi

  # Check model is valid
  model=$(grep "^model:" "$agent" | head -1 | cut -d: -f2 | xargs)
  case "$model" in
    opus|sonnet|haiku) ;;
    *) echo "FAIL: Invalid model '$model': $agent"; FAIL=$((FAIL+1)) ;;
  esac
done
echo "Agent validation: $((AGENT_COUNT - FAIL)) pass, $FAIL fail"

# Check for dangling references (agents that reference non-existent agents)
echo "Checking agent cross-references..."
for agent in .maestro/agents/*.md; do
  [ -f "$agent" ] || continue
  # Find subagent_type references
  refs=$(grep -oE 'subagent_type[=:]["'\'']*([a-z-]+)' "$agent" 2>/dev/null | sed 's/.*["'\'']//' | sed 's/["'\'']$//')
  for ref in $refs; do
    if [ ! -f ".maestro/agents/$ref.md" ]; then
      echo "WARN: $agent references non-existent agent: $ref"
    fi
  done
done
```

### Phase 3: Hooks Audit
```bash
echo "=== HOOKS ==="

# Check TypeScript source count
TS_COUNT=$(ls .maestro/hooks/src/*.ts 2>/dev/null | wc -l | xargs)
echo "Found $TS_COUNT TypeScript source files"

# Check bundles exist
BUNDLE_COUNT=$(ls .maestro/hooks/dist/*.mjs 2>/dev/null | wc -l | xargs)
echo "Found $BUNDLE_COUNT built bundles"

# Check shell wrappers are executable
FAIL=0
for sh in .maestro/hooks/*.sh; do
  [ -f "$sh" ] || continue
  if [ ! -x "$sh" ]; then
    echo "FAIL: Not executable: $sh"
    FAIL=$((FAIL+1))
  fi
done
SH_COUNT=$(ls .maestro/hooks/*.sh 2>/dev/null | wc -l | xargs)
echo "Shell wrappers: $((SH_COUNT - FAIL)) executable, $FAIL need chmod +x"

# Check hooks registered in settings.json exist
echo "Checking registered hooks..."
FAIL=0
# Extract hook commands from settings.json and verify files exist
grep -oE '"command":\s*"[^"]*\.sh"' .maestro/settings.json 2>/dev/null | \
  sed 's/.*"\([^"]*\.sh\)".*/\1/' | \
  sed 's|\$CLAUDE_PROJECT_DIR|.maestro|g' | \
  sed "s|\$HOME|$HOME|g" | \
  sort -u | while read hook; do
    # Resolve to actual path
    resolved=$(echo "$hook" | sed 's|^\./||')
    if [ ! -f "$resolved" ] && [ ! -f "./$resolved" ]; then
      echo "WARN: Registered hook not found: $hook"
    fi
  done
```

### Phase 4: Memory Audit
```bash
echo "=== MEMORY SYSTEM ==="

# Check SQLite Database
DB_PATH="$HOME/.maestro/memory.db"
if [ ! -f "$DB_PATH" ]; then
  echo "FAIL: SQLite database not found at $DB_PATH"
else
  echo "PASS: SQLite database found"

  # Test connection and WAL mode
  if sqlite3 "$DB_PATH" "PRAGMA journal_mode;" | grep -q "wal"; then
    echo "PASS: SQLite reachable and WAL mode enabled"
  else
    echo "FAIL: SQLite WAL mode not enabled"
  fi

  # Check sqlite-vec
  # This requires a python check usually
fi

# Check DuckDB Analytics
DUCKDB_PATH="$HOME/.maestro/analytics.duckdb"
if [ ! -f "$DUCKDB_PATH" ]; then
  echo "WARN: DuckDB analytics not found at $DUCKDB_PATH"
else
  echo "PASS: DuckDB analytics found"
fi

# Check Python dependencies
echo "Checking Python dependencies..."
(cd opc && uv run python -c "import sqlite3; import duckdb; import numpy" 2>/dev/null) && \
  echo "PASS: Python dependencies (sqlite3, duckdb, numpy) available" || \
  echo "WARN: Some Python dependencies missing"
```

### Phase 5: Cross-Reference Audit
```bash
echo "=== CROSS-REFERENCES ==="

# Check skills reference valid agents
echo "Checking skill → agent references..."
FAIL=0
for skill in $(find .maestro/skills -name "SKILL.md"); do
  refs=$(grep -oE 'subagent_type[=:]["'\'']*([a-z-]+)' "$skill" 2>/dev/null | sed 's/.*["'\'']//' | sed 's/["'\'']$//')
  for ref in $refs; do
    if [ -n "$ref" ] && [ ! -f ".maestro/agents/$ref.md" ]; then
      echo "FAIL: $skill references missing agent: $ref"
      FAIL=$((FAIL+1))
    fi
  done
done
echo "Skill→Agent refs: $FAIL broken"
```

## Auto-Fix (--fix flag)

If `--fix` is specified, automatically fix:

1. **Make shell wrappers executable**
   ```bash
   chmod +x .maestro/hooks/*.sh
   ```

2. **Rebuild hooks if TypeScript newer than bundles**
   ```bash
   cd .maestro/hooks && npm run build
   ```

3. **Create missing cache directories**
   ```bash
   mkdir -p .maestro/cache/agents/{explorer,implementer,planner,spark}
   mkdir -p .maestro/cache/mot
   ```

## Output Format

Write full report to `.maestro/cache/mot/report-{timestamp}.md`:

```markdown
# MOT Health Report
Generated: {timestamp}

## Summary
| Category | Pass | Fail | Warn |
|----------|------|------|------|
| Skills   | 204  | 2    | 0    |
| Agents   | 47   | 1    | 3    |
| Hooks    | 58   | 2    | 1    |
| Memory   | 4    | 0    | 1    |
| X-Refs   | 0    | 0    | 2    |

## Issues Found

### P0 - Critical
- [FAIL] Hook build failed: tldr-context-inject.ts

### P1 - High
- [FAIL] Agent references missing: scot → explorer (typo)

### P2 - Medium
- [WARN] 3 hooks need rebuild (dist older than src)

### P3 - Low
- [INFO] VOYAGE_API_KEY not set (using local BGE)
```

## Exit Codes

- `0` - All P0/P1 checks pass
- `1` - Any P0/P1 failure
- `2` - Only P2/P3 warnings

## Quick Mode (--quick)

Only run P0 checks:
1. Frontmatter parses
2. Hooks build
3. Shell wrappers executable
4. PostgreSQL reachable
