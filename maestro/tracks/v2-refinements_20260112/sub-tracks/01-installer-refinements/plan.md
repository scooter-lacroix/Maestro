# Sub-Track 01: Installer Refinements - Plan

## Overview

Implementation plan for installer enhancements.

---

## Phase 1: Config Backup

### [x] Task 1.1: Implement Config Backup (ee881c2)
- [x] Write tests for backup functionality (timestamped archive creation)
- [x] Write tests for skip behavior when ~/.claude doesn't exist
- [x] Implement backup logic in `install-claude-code.sh`
- [x] Add restore capability for rollback scenarios
- [x] Test backup/restore cycle end-to-end

---

## Phase 2: TypeScript Hook Building

### [x] Task 1.2: Add TypeScript Hook Building (9b76722)
- [x] Write tests for package.json detection
- [x] Write tests for npm availability check
- [x] Add `package.json` detection in hooks directory
- [x] Implement `npm install && npm run build` step
- [x] Handle build failures gracefully with user notification
- [x] Add warning when npm not available

---

## Phase 3: Documentation

### [x] Task 1.3: Create Gap Documentation
- [x] Read and analyze wizard.py from OPC/CC-v3
- [x] Read and analyze install-claude-code.sh
- [x] Create `docs/INSTALLER_GAPS.md` with comparison table
- [x] Document which gaps are intentional vs future work
- [x] Cross-reference with PRE_MERGE_ANALYSIS_REPORT.md

---

## Phase 4: MCP Setup

### [x] Task 1.4: Add MCP Server Setup (82707df)
- [x] Write tests for mcp.json configuration creation
- [x] Write tests for path validation
- [x] Implement mcp.json installation step
- [x] Validate MCP server paths exist
- [x] Add optional connectivity check

---

## Phase 5: Verification

### [ ] Task 1.5: Maestro - User Manual Verification 'Sub-Track 01' (Protocol in workflow.md)
