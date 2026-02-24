# Subtrack 06: UI Integration

## Objective
Integrate maestro-claw with Cockpit TUI and maestro-gateway for user-facing agent interaction.

## Requirements

### R1: Cockpit Integration
- MaesterClaw tab displays agent status (running/idle/error)
- Session list shows active sessions
- Turn history display for selected session
- Real-time updates via event stream

### R2: Gateway Integration
- WebSocket endpoint for agent execution
- HTTP endpoint for session management (list, create, delete)
- Event streaming for real-time updates
- Frame protocol compatibility

### R3: HotCache Integration
- Memory suggestions appear in UI during sessions
- Relevance threshold filtering
- TTL expiration handling

## Acceptance Criteria
- [ ] MaesterClaw tab shows agent status
- [ ] Session list displays active sessions
- [ ] Turn history viewable in TUI
- [ ] WebSocket endpoint for agent execution
- [ ] HTTP endpoints for session management
- [ ] HotCache suggestions appear
- [ ] Manual TUI verification complete
