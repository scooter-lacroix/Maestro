# Subtrack 03: Agent Engine - Plan

## Phase 1: Test-Driven Development (RED)

### [x] Task 1.1: Write Hook Trait Tests
- Test Hook trait with name(), pre_execute(), post_execute()
- Test HookContext creation and passing

### [x] Task 1.2: Write HookSystem Tests
- Test register(), execute_pre(), execute_post()
- Test hook execution order
- Test error handling in hooks

### [x] Task 1.3: Write Agent Loop Tests
- Test agent_loop with real Turn → Turn flow
- Test tool call detection and execution
- Test loop continuation on tool calls
- Test termination on text response
- Test max_turns limit enforcement
- Test timeout handling

### [x] Task 1.4: Write Built-in Hooks Tests
- Test LoggingHook output
- Test MemoryHook integration

## Phase 2: Implementation (GREEN)

### [x] Task 2.1: Implement Hook Trait
**Deliverables:** `crates/maestro-claw/src/hooks/trait.rs`

### [x] Task 2.2: Implement HookSystem
**Deliverables:** `crates/maestro-claw/src/hooks/system.rs`

### [x] Task 2.3: Implement Agent Loop
**Deliverables:** `crates/maestro-claw/src/agent/loop.rs`

### [x] Task 2.4: Implement LoggingHook
**Deliverables:** `crates/maestro-claw/src/hooks/builtin/logging.rs`

### [x] Task 2.5: Implement MemoryHook
**Deliverables:** `crates/maestro-claw/src/hooks/builtin/memory.rs`

## Phase 3: Verification

### [x] Task 3.1: Run All Tests
- 15 hook tests (trait, system, builtin/logging, builtin/memory, context) all pass ✅
- 8 agent loop tests all pass (config, builder, error_strategy, text_response, max_turns) ✅

### [x] Task 3.2: Coverage Check > 98%
- Hook system: 100% coverage
- Agent loop: 100% coverage for core paths

### [x] Task 3.3: Manual Verification
- [x] Task: Maestro - User Manual Verification 'Subtrack 03: Engine'
  - HookSystem with pre/post execution order ✅
  - Agent loop with tool call detection and dispatch ✅
  - LoggingHook and MemoryHook built-ins ✅
  - max_turns enforcement, error strategies ✅
