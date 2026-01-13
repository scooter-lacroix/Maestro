# Sub-Track 02: Cross-Platform Compatibility Fixes - Plan

## Overview

Implementation plan for porting cross-platform fixes from CC-v3.

---

## Phase 1: Temp Directory Fixes

### [x] Task 2.1: Port Temp Directory Fixes (714171d)
- [x] Write cross-platform temp path tests
- [x] Identify all hardcoded `/tmp` references
- [x] Replace with platform-agnostic paths (tempfile module / os.TempDir)
- [x] Verify on Linux, macOS, Windows/WSL

---

## Phase 2: Hook Execution Fixes

### [x] Task 2.2: Port Hook Execution Fixes (f78fd0d, b10a490)
- [x] Write tests for hook execution from project root
- [x] Write tests for global script path resolution
- [x] Update hook launcher to use project root as CWD
- [x] Fix global script path resolution
- [x] Test on all platforms

---

## Phase 3: Python Executable Fixes

### [x] Task 2.3: Port Python Executable Fixes (63f85f7, ad01f2c)
- [x] Write tests for python/python3 detection
- [x] Write tests for cross-platform Python runner
- [x] Implement detection logic (python vs python3)
- [x] Implement cross-platform Python runner
- [x] Add fallback logic for Windows compatibility

---

## Phase 4: Memory Daemon Fixes

### [x] Task 2.4: Port Memory Daemon Fixes (185c2c0, 6a53e57, d360129)
- [x] Write tests for truncated session ID lookup
- [x] Write tests for existsSync check
- [x] Write tests for PID duplicate check
- [x] Fix JSONL lookup for truncated session IDs
- [x] Add existsSync check before local path usage
- [x] Implement PID check to prevent duplicate spawns

---

## Phase 5: Rich Markup Fix

### [x] Task 2.5: Port Rich Markup Fix (4fee297)
- [x] Write test for special character escaping
- [x] Identify Rich markup injection points
- [x] Escape Rich markup in error messages
- [x] Verify no markup injection in user-facing output

---

## Phase 6: Verification

### [x] Task 2.6: Maestro - User Manual Verification 'Sub-Track 02' (Protocol in workflow.md)
