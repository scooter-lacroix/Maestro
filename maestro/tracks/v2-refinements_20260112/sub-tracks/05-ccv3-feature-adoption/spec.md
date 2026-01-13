# Sub-Track 05: Continuous-Claude-v3 Feature Adoption - Specification

## Overview

Adopt high-value features from Continuous-Claude-v3 including visual dashboard, update wizard, memory daemon, standardized handoffs, and $0 built-in fallbacks for external services.

**Priority:** 5 (Enhancements)
**Parent Track:** v2-refinements_20260112

## Functional Requirements

### FR-1: Visual Dashboard (commit 60690e0)
- Token savings visualization with progress bars
- Sparklines for usage trends
- Don Norman UX principles (visibility, feedback, mapping)
- Integration with `maestro:status` command
- Integration with `maestro:implement` output

### FR-2: Update Wizard (commit bfe021b)
- Self-update capability for Maestro installation
- Component sync across variants
- Version checking against remote
- Migration support for breaking changes
- Rollback capability

### FR-3: Memory Daemon
- Centralized Python daemon for cross-terminal coordination
- JSON-RPC or REST API for state queries
- Lifecycle management (start/stop/status)
- Real-time state sharing between terminals

### FR-4: Standardized Handoff Schema (commit 3a1e9f5)
- YAML format with structured blocks:
  - `goal`: Current objective
  - `now`: In-progress work
  - `next`: Planned next steps
- Migration from free-form markdown ledgers
- claude-hud statusline integration
- Programmatic parsing support

### FR-5: $0 Built-in Fallbacks
External services are optional; built-in defaults provide equivalent functionality:

| External Service | Built-in Fallback |
|-----------------|-------------------|
| Perplexity | WebSearch tool integration |
| Braintrust | Local metrics collection and evaluation |
| Nia | Built-in assistant functionality |

- Configuration option to upgrade to external services
- Seamless switching between built-in and external
- No functionality loss with defaults

## Acceptance Criteria

1. [ ] Dashboard displays token savings with sparklines
2. [ ] Update wizard syncs components correctly
3. [ ] Memory daemon enables cross-terminal coordination
4. [ ] Handoff schema migrates existing ledgers
5. [ ] All fallbacks provide feature parity with external services
6. [ ] All tests passing with >98% coverage
7. [ ] Tzar of Excellence review approved

## Out of Scope

- Third-party API integration code (optional enhancement)
- New command syntax changes
- OpenCode variant parity
