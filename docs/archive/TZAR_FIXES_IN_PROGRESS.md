# Tzar Review Fixes - In Progress

**Date:** 2025-01-13
**Status:** Multiple agents deployed in parallel

---

## Agents Deployed

| Agent | Task ID | Focus |
|-------|---------|-------|
| vibe | `b4d4048` | Security fixes (CRITICAL) |
| kilocode | `bc2e38f` | DB schema, fallback crash, timeouts |
| qwen | `b8b381c` | Handoff persistence, API mismatch, dependencies |
| gemini | `b4f4601` | Cross-platform, logging, hook interpreter |

---

## Issues Being Fixed

### CRITICAL (Security)

1. **Path Traversal** - `maestro/leindex/mcp_server.py:413`, `:500`
   - Add project-root containment validation
   - Restrict `set_project_path()` to approved directories

2. **Pickle RCE** - `maestro/leindex/project_settings.py:323`
   - Replace `pickle.load()` with JSON
   - Add cache file integrity validation

3. **Zip-Slip** - `maestro/update_wizard.py:211`
   - Add path validation before zip extraction
   - Check all file paths

### HIGH

4. **DB Schema Mismatch** - `maestro/memory/service.py:450`
   - Add `agent_type` column to `agent_namespaces`
   - Update migrations.py and models.py

5. **Fallback System Crash** - `maestro/fallbacks/fallbacks.py:242`
   - Fix `UnboundLocalError`
   - Proper exception handling

6. **Network Timeouts** - `update_wizard.py:105`, `:187`
   - Add timeouts to `requests.get()` calls
   - Add timeouts to daemon_client API calls

7. **Installer Broken Fallback** - `install-claude-code.sh:287`
   - Fix wrapper pointing to ephemeral temp repo
   - Add backup before overwriting `.mcp.json`

### MEDIUM

8. **Handoff Persistence Broken** - `maestro/handoffs.py:105`, `:107`
   - Fix validation for optional fields
   - Fix `to_dict()` keys emission

9. **Analytics API Mismatch** - `managers.py:295` vs `backends.py:95`
   - Fix `query_analytics()` **kwargs forwarding

10. **Dependency Hygiene** - Multiple files
    - Make optional imports lazy (loguru, rich, requests)
    - Add clean fallbacks

### IMPROVEMENTS

11. **Cross-Platform Issues**
    - Fix `os.geteuid()` on Windows
    - Fix `_command_exists()` return codes

12. **LeIndex Logging**
    - Remove import-time `logging.basicConfig()`

13. **Hook Interpreter Priority** - `maestro/hooks/executor.py`
    - Prefer `sys.executable` over `python3/python`

14. **Atomic Writes**
    - Use atomic file write pattern

---

## Progress

- [ ] Security fixes (vibe)
- [ ] DB schema + fallback (kilocode)
- [ ] Persistence + dependencies (qwen)
- [ ] Cross-platform + logging (gemini)

---

*Waiting for agents to complete...*
