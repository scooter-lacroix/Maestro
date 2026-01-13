# Sub-Track 02: Cross-Platform Compatibility Fixes - Specification

## Overview

Port critical cross-platform fixes from Continuous-Claude-v3 commits to ensure Maestro works correctly on Linux, macOS, and Windows/WSL.

**Priority:** 2 (Bug Fixes)
**Parent Track:** v2-refinements_20260112

## Functional Requirements

### FR-1: Temp Directory Fixes (714171d)
- Replace hardcoded `/tmp` paths with platform-agnostic alternatives
- Use `tempfile` module in Python, appropriate env vars in shell
- Verify on all target platforms

### FR-2: Hook Execution Fixes (f78fd0d, b10a490)
- Hooks execute from project root, not hooks directory
- Global script paths resolve correctly on all platforms
- Hook launcher uses correct CWD

### FR-3: Python Executable Fixes (63f85f7, ad01f2c)
- Detect `python` vs `python3` availability
- Use cross-platform Python runner for hooks
- Fallback logic for Windows compatibility

### FR-4: Memory Daemon Fixes (185c2c0, 6a53e57, d360129)
- JSONL lookup handles truncated session IDs
- existsSync check before local path usage
- PID check prevents duplicate daemon spawns

### FR-5: Rich Markup Fix (4fee297)
- Escape Rich markup characters in error messages
- Prevent markup injection in user-facing output

## Commits to Port

| Commit | Description |
|--------|-------------|
| `714171d` | Use cross-platform temp directory paths |
| `f78fd0d` | Cross-platform hooks + global script paths |
| `63f85f7` | Use `python` instead of `python3` for Windows |
| `eadf80b` | Cross-platform skill-activation-prompt hook |
| `ad01f2c` | Cross-platform Python runner for hooks |
| `b10a490` | Hooks execute from project root |
| `185c2c0` | Memory daemon JSONL lookup fix |
| `6a53e57` | daemon-client existsSync check |
| `d360129` | PID check for duplicate spawns |
| `4fee297` | Escape Rich markup in errors |

## Acceptance Criteria

1. [ ] All listed commits ported and tested
2. [ ] Tests pass on Linux
3. [ ] Tests pass on macOS
4. [ ] Tests pass on Windows/WSL
5. [ ] All tests passing with >98% coverage
6. [ ] Tzar of Excellence review approved

## Out of Scope

- New cross-platform features beyond the listed commits
- Windows native (non-WSL) support
