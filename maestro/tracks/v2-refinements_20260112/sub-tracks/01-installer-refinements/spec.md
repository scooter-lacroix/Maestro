# Sub-Track 01: Installer Refinements - Specification

## Overview

Enhance `install-claude-code.sh` with production-ready features including config backup, TypeScript hook building, gap documentation, and MCP server setup.

**Priority:** 1 (Foundation)
**Parent Track:** v2-refinements_20260112

## Functional Requirements

### FR-1: Config Backup
- Before any installation, backup existing `~/.claude` directory
- Use timestamped archive format: `.claude.backup.YYYYMMDD_HHMMSS`
- Provide restore capability for rollback scenarios
- Skip backup if `~/.claude` doesn't exist

### FR-2: TypeScript Hook Building
- Detect `package.json` in hooks directory
- Run `npm install && npm run build` when detected
- Handle build failures gracefully with clear user notification
- Skip if npm is not available (with warning)

### FR-3: Gap Documentation
- Create `docs/INSTALLER_GAPS.md`
- Compare wizard.py (OPC) vs install-claude-code.sh features
- Clearly mark intentional gaps vs future work items
- Include table format for easy scanning

### FR-4: MCP Server Setup
- Install `mcp.json` configuration file
- Validate MCP server paths exist
- Test MCP server connectivity post-install (optional check)

## Acceptance Criteria

1. [x] Config backup creates timestamped archive before installation (completed in ee881c2)
2. [x] TypeScript hooks build successfully when package.json present (completed in 9b76722)
3. [x] INSTALLER_GAPS.md documents all differences from wizard.py (completed in previous commit)
4. [x] MCP server configuration installed and validated (completed in 82707df)
5. [ ] All tests passing with >98% coverage
6. [ ] Tzar of Excellence review approved

## Out of Scope

- Database configuration (handled in Sub-Track 03)
- Cross-platform fixes (handled in Sub-Track 02)
- New command development
