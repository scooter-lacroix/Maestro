---
name: completion-check
description: "Verify that newly built infrastructure is actually wired into the system before marking it complete. Use when finishing infrastructure work, checking if hooks are registered, validating database connections, tracing execution paths, or searching for dead code and orphaned implementations."
user-invocable: false
---

# Completion Check

Verify that infrastructure is connected to the system and actively used before marking work as complete. Infrastructure is not done when the code is written — it is done when it is wired in and exercised end-to-end.

## Workflow

1. **Trace the execution path** from user intent to the new infrastructure code
2. **Verify registrations** (hooks, settings, config) point to the new code
3. **Confirm backends** match the intended architecture
4. **Run end-to-end validation** to prove the infrastructure is invoked
5. **Search for orphaned code** or parallel implementations

## Verification Steps

### Trace Execution Path

```bash
grep -r "claude -p" src/
grep -r "Task(" src/
```

### Check Hook Registration

```bash
ls -la .maestro/hooks/my-hook.sh
grep "my-hook" .maestro/settings.json
```

### Verify Database Backend

```bash
grep -r "sqlite:///" src/
grep -r "duckdb" src/
```

### End-to-End Test

```bash
uv run python -m my_feature
cat /tmp/debug.log
```

### Find Orphaned Implementations

```bash
ast-grep --pattern 'async function $NAME() { $$$ }' | \
  xargs -I {} grep -r "{}" src/
```

## Completion Checklist

Before declaring infrastructure complete:

- [ ] Traced execution path from entry point to infrastructure
- [ ] Verified hooks are registered in .maestro/settings.json
- [ ] Confirmed correct database/backend in use
- [ ] Ran end-to-end test showing infrastructure invoked
- [ ] Searched for dead code or parallel implementations
- [ ] Checked configuration files match implementation

## Anti-Patterns

- Marking infrastructure "complete" without testing the execution path
- Assuming code is wired just because it compiles
- Building parallel systems (e.g., Task tool vs claude -p spawn)
- Using the wrong backend (SQLite when PostgreSQL is architected)
