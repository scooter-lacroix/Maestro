# Subtrack 01: Session/Thread/Turn Model - Plan

## Phase 1: Test-Driven Development (RED)

### [x] Task 1.1: Write Session Model Tests
- Test Session creation with id, threads, metadata
- Test Session serialization/deserialization
- Test Session::add_thread(), Session::get_thread()
- Test Session::new() with auto-generated ID

### [x] Task 1.2: Write Thread Model Tests
- Test Thread creation with session_id, turns, summary
- Test Thread::add_turn()
- Test Thread::build_next_turn() for provider request
- Test Thread::to_messages() for provider format

### [x] Task 1.3: Write Turn Model Tests
- Test Turn creation with role, content, tool_calls, tool_results
- Test TurnRole enum variants
- Test Turn serialization
- Test timestamp handling

## Phase 2: Implementation (GREEN)

### [x] Task 2.1: Implement Session Struct
**Deliverables:** `crates/maestro-claw/src/session/session.rs`
- Session struct with id, threads, metadata, created_at
- Session::new(), Session::add_thread(), Session::get_thread()
- Serde derives

### [x] Task 2.2: Implement Thread Struct
**Deliverables:** `crates/maestro-claw/src/session/thread.rs`
- Thread struct with id, session_id, turns, summary
- Thread::new(), Thread::add_turn(), Thread::build_next_turn()
- Thread::to_messages() returning Vec<ProviderMessage>

### [x] Task 2.3: Implement Turn Struct
**Deliverables:** `crates/maestro-claw/src/session/turn.rs`
- Turn struct with id, role, content, tool_calls, tool_results, timestamp
- TurnRole enum
- Turn::new() for each role variant

### [x] Task 2.4: Create Module Exports
**Deliverables:** `crates/maestro-claw/src/session/mod.rs`
- Re-export Session, Thread, Turn, TurnRole

## Phase 3: Verification

### [x] Task 3.1: Run All Tests
- 20 session_model + 10 thread_model + 18 turn_model = 48 integration tests pass
- 15 lib tests pass

### [x] Task 3.2: Coverage Check
- Coverage > 98% for session module

### [x] Task 3.3: Manual Verification
- [x] Task: Maestro - User Manual Verification 'Subtrack 01: Session'
  - Session struct with UUID, threads HashMap, metadata, created_at ✅
  - Thread with turns Vec, add_turn(), to_messages() ✅
  - Turn with TurnRole, content, tool_calls, tool_results ✅
  - All 48 integration tests passing ✅
