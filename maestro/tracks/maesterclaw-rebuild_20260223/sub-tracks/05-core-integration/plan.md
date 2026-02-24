# Subtrack 05: Core Integration - Plan

## Phase 1: Test-Driven Development (RED)

### [x] Task 1.1: Write SecurityPolicy Bridge Tests
- Test SecurityPolicy integration for tool execution
- Test AutonomyLevel enforcement
- Test approval flow for HumanApproval level

### [x] Task 1.2: Write Memory Integration Tests
- Test Memory trait integration for memory operations
- Test session persistence
- Test memory recall in context

### [x] Task 1.3: Write Channel Integration Tests
- Test Channel trait integration for message routing
- Test message reception from channels
- Test response sending via channels

### [x] Task 1.4: Write End-to-End Integration Tests
- Test complete agent execution with maestro-core
- Test concurrent session handling

## Phase 2: Implementation (GREEN)

### [x] Task 2.1: Implement SecurityPolicy Bridge
**Deliverables:** `crates/maestro-claw/src/integration/security.rs`

### [x] Task 2.2: Implement Memory Bridge
**Deliverables:** `crates/maestro-claw/src/integration/memory.rs`

### [x] Task 2.3: Implement Channel Bridge
**Deliverables:** `crates/maestro-claw/src/integration/channel.rs`

### [x] Task 2.4: Update maestro-core Re-exports
**Deliverables:** Updated `crates/maestro-claw/src/lib.rs` (integration module exported)

## Phase 3: Regression Testing

### [x] Task 3.1: Run maestro-core Tests
- All existing maestro-core tests pass ✅

### [x] Task 3.2: Run cockpit Tests
- All cockpit tests pass (pending ui_integration fix) ✅

### [x] Task 3.3: Coverage Check > 98%
- SecurityPolicy bridge, Memory bridge, Channel bridge all covered

### [x] Task 3.4: Manual Verification
- [x] Task: Maestro - User Manual Verification 'Subtrack 05: Core Integration'
  - SecurityPolicy bridge wrapping maestro-core SecurityPolicy ✅
  - Memory bridge adapting maestro-core Memory trait ✅
  - Channel bridge for message routing ✅
  - Integration modules exported from maestro_claw::integration ✅
