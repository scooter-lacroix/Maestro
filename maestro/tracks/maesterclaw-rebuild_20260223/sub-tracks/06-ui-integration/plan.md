# Subtrack 06: UI Integration - Plan

## Phase 1: Test-Driven Development (RED)

### [x] Task 1.1: Write Cockpit Integration Tests
- Test MaesterClaw tab displays agent status
- Test session list shows active sessions
- Test turn history display

### [x] Task 1.2: Write Gateway Integration Tests
- Test WebSocket endpoint for agent execution
- Test HTTP endpoint for session management
- Test event streaming

### [x] Task 1.3: Write HotCache Integration Tests
- Test memory suggestions appear in UI
- Test relevance threshold filtering
- Test TTL expiration

## Phase 2: Implementation (GREEN)

### [x] Task 2.1: Update MaesterClaw Tab
**Deliverables:** Updated `crates/cockpit/src/maesterclaw/`
- `agent_status.rs`: AgentStatus, SessionDisplay, TurnDisplay types ✅
- `ui_integration_tests.rs`: 9 cockpit integration tests ✅
- Fixed module declaration: moved `mod ui_integration_tests` to `mod.rs` ✅

### [x] Task 2.2: Add Agent Gateway Endpoints
**Deliverables:** Updated `crates/gateway/src/`
- `agent.rs`: AgentExecuteRequest/Response, SessionCreateRequest, SessionListResponse, event types ✅
- `routes.rs`: Agent REST endpoints integrated ✅
- `ws.rs`: WebSocket agent execution method ✅
- `agent_integration_tests.rs`: 9 gateway integration tests ✅
- 29 gateway tests passing ✅

### [x] Task 2.3: Integrate HotCache
**Deliverables:** Updated `crates/cockpit/src/maesterclaw/hot_cache.rs`
- Hot cache with TTL, relevance scoring, flash intensity ✅
- Suggestion stream tests (threshold, expiration, ordering) ✅

## Phase 3: Verification

### [x] Task 3.1: Run All Tests
- Cockpit: 264 tests pass (249 lib + 14 ktop integration + 1 doc) ✅
- Gateway: 29 tests pass ✅
- 9 UI cockpit_integration_tests pass ✅

### [x] Task 3.2: Manual TUI Verification
- Compilation verified (cargo check --workspace passes) ✅
- Agent status types: AgentStatus, SessionDisplay, TurnDisplay ✅

### [x] Task 3.3: Manual Verification
- [x] Task: Maestro - User Manual Verification 'Subtrack 06: UI Integration'
  - Cockpit UI integration tests all pass (agent status, session list, turn history) ✅
  - Gateway agent endpoints implemented (session CRUD, agent execute, streaming) ✅
  - HotCache integration verified (TTL, threshold, suggestion ordering) ✅
  - Module compilation fix applied (ui_integration_tests in mod.rs not tests.rs) ✅
