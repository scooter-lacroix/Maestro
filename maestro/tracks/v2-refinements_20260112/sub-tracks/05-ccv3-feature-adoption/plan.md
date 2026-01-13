# Sub-Track 05: Continuous-Claude-v3 Feature Adoption - Plan

## Overview

Implementation plan for adopting CC-v3 features.

---

## Phase 1: Visual Dashboard

### [x] Task 5.1: Implement Visual Dashboard (60690e0)
- [x] Write tests for token savings calculation
- [x] Write tests for progress bar rendering
- [x] Write tests for sparkline generation
- [x] Implement token savings visualization
- [x] Implement progress bars
- [x] Implement sparklines for trends
- [x] Apply Don Norman UX principles
- [x] Integrate with `maestro:status`
- [x] Integrate with `maestro:implement` output

---

## Phase 2: Update Wizard

### [x] Task 5.2: Implement Update Wizard (bfe021b)
- [x] Write tests for version checking
- [x] Write tests for component sync
- [x] Write tests for migration handling
- [x] Write tests for rollback capability
- [x] Create update command infrastructure
- [x] Implement remote version check
- [x] Implement pull and sync logic
- [x] Add migration support for breaking changes
- [x] Add rollback capability

---

## Phase 3: Memory Daemon

### [x] Task 5.3: Implement Memory Daemon
- [x] Write tests for daemon server startup
- [x] Write tests for API endpoints
- [x] Write tests for lifecycle management
- [x] Create centralized Python daemon
- [x] Implement JSON-RPC or REST API
- [x] Add start/stop/status commands
- [x] Enable cross-terminal state sharing
- [x] Add graceful shutdown handling

---

## Phase 4: Handoff Schema

### [x] Task 5.4: Implement Standardized Handoff Schema (3a1e9f5)
- [x] Write tests for YAML handoff parsing
- [x] Write tests for schema validation
- [x] Write tests for migration from markdown
- [x] Define schema with `goal`, `now`, `next` blocks
- [x] Implement YAML serialization/deserialization
- [x] Implement migration from markdown ledgers
- [x] Add claude-hud statusline integration
- [x] Document schema for users

---

## Phase 5: Built-in Fallbacks

### [x] Task 5.5: Implement $0 Built-in Fallbacks
- [x] Write tests for Perplexity fallback (WebSearch tool)
- [x] Implement WebSearch-based search fallback
- [x] Write tests for Braintrust fallback (local metrics)
- [x] Implement local metrics collection and evaluation
- [x] Write tests for Nia fallback (if applicable)
- [x] Implement built-in assistant functionality
- [x] Add configuration for optional external service upgrade
- [x] Document fallback capabilities

---

## Phase 6: Verification

### [x] Task 5.6: Maestro - User Manual Verification 'Sub-Track 05' (Protocol in workflow.md)
