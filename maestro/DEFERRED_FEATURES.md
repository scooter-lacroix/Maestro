# Deferred Features - Maestro v1.1 Roadmap

This document tracks features that were intentionally deferred from the initial implementation to future releases.

## Status: Deferred to v1.1

The following features have been deferred to allow focus on core functionality in the initial release:

### 1. TUI (Terminal User Interface) Integration

**Description:** Full integration with Maestro TUI for interactive project management, track visualization, and progress monitoring.

**Reason for Deferral:**
- TUI integration requires significant additional development
- Core workflow can function effectively via CLI commands
- Allows for thorough testing of core features before adding UI layer

**Planned for v1.1:**
- Interactive track selection and management
- Visual progress tracking
- In-TUI task execution and monitoring
- Real-time status updates

**Current Alternative:**
- Use `/maestro:status` for progress tracking
- Use `/maestro:newTrack`, `/maestro:implement`, `/maestro:revert` for management

---

### 2. Memory Dashboard Integration

**Description:** Web-based dashboard for visualizing project context, track history, and memory system state.

**Reason for Deferral:**
- Dashboard requires additional infrastructure (frontend, API)
- Memory system functions via CLI commands
- Allows focus on memory content quality before visualization

**Planned for v1.1:**
- Web-based dashboard for Memory System
- Visual track history and completion metrics
- Context browsing and search
- Real-time memory updates

**Current Alternative:**
- Use `/maestro:memory` to interact with memory system
- Use `/maestro:status` for project state
- Access memory directly via nexus-memory MCP

---

## Implementation Notes for Future Development

When implementing these features in v1.1, consider:

### TUI Integration
- Leverage existing `maestro-tui` package in `maestro/tui/`
- Ensure TUI works with both Claude Code and OpenCode
- Maintain CLI functionality as primary interface
- Add TUI-specific commands (e.g., `/maestro:tui`)

### Memory Dashboard
- Build on existing Memory Dashboard frontend
- Create REST API for memory access
- Implement real-time updates via WebSocket
- Ensure dashboard works with both nexus-memory and memori-memory-mcp

### Dependencies
These features depend on:
- ✅ Core workflow stability
- ✅ Memory system maturity
- ✅ Agent orchestration reliability
- ✅ Track completion tracking

---

## Tracking

- **Created:** 2025-01-05
- **Last Updated:** 2025-01-05
- **Target Release:** v1.1
- **Status:** Deferred

## Related Issues/Tracks

- TUI Integration Track (to be created in v1.1)
- Memory Dashboard Track (to be created in v1.1)
