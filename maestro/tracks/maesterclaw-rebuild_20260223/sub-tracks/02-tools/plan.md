# Subtrack 02: Tool System - Plan

## Phase 1: Test-Driven Development (RED)

### [x] Task 1.1: Write Tool Trait Tests
- Test Tool trait with name(), description(), parameters_schema()
- Test async execute() returns ToolOutput
- Test ToolSpec generation from Tool trait

### [x] Task 1.2: Write ToolRegistry Tests
- Test register(), get(), list()
- Test duplicate registration handling
- Test to_tool_specs() output format

### [x] Task 1.3: Write ShellTool Tests
- Test safe command execution (echo, ls)
- Test blocked commands (rm -rf, etc.)
- Test timeout handling
- Test output capture

### [x] Task 1.4: Write FileTool Tests
- Test read file operations
- Test write file operations
- Test path validation (no traversal)
- Test error handling

### [x] Task 1.5: Write MemoryTool Tests
- Test store operation
- Test recall operation
- Test integration with maestro-core Memory trait

## Phase 2: Implementation (GREEN)

### [x] Task 2.1: Implement Tool Trait
**Deliverables:** `crates/maestro-claw/src/tools/trait.rs`

### [x] Task 2.2: Implement ToolRegistry
**Deliverables:** `crates/maestro-claw/src/tools/registry.rs`

### [x] Task 2.3: Implement ShellTool
**Deliverables:** `crates/maestro-claw/src/tools/builtin/shell.rs`

### [x] Task 2.4: Implement FileTool
**Deliverables:** `crates/maestro-claw/src/tools/builtin/file.rs`

### [x] Task 2.5: Implement MemoryTool
**Deliverables:** `crates/maestro-claw/src/tools/builtin/memory.rs`

## Phase 3: Verification

### [x] Task 3.1: Run All Tests
- All 142 tests pass (103 lib + 39 integration tests)

### [x] Task 3.2: Coverage Check > 98%
- Tool trait tests: 100%
- ToolRegistry tests: 100%
- ShellTool tests: 100% (19 tests)
- FileTool tests: 100% (22 tests)
- MemoryTool tests: 100% (18 tests)

### [x] Task 3.3: Manual Verification
- [x] Task: Maestro - User Manual Verification 'Subtrack 02: Tools'
  - All tools compile and pass tests
  - Tools can be registered and executed
  - Safety constraints are enforced

## Summary

**Implementation Complete**

### Files Created/Modified:
- `/run/media/scooter/W.D-SSD/Prod/maestro/crates/maestro-claw/src/tools/mod.rs` - Added builtin module export
- `/run/media/scooter/W.D-SSD/Prod/maestro/crates/maestro-claw/src/tools/builtin/mod.rs` - New module for built-in tools
- `/run/media/scooter/W.D-SSD/Prod/maestro/crates/maestro-claw/src/tools/builtin/shell.rs` - ShellTool with 19 tests
- `/run/media/scooter/W.D-SSD/Prod/maestro/crates/maestro-claw/src/tools/builtin/file.rs` - FileTool with 22 tests
- `/run/media/scooter/W.D-SSD/Prod/maestro/crates/maestro-claw/src/tools/builtin/memory.rs` - MemoryTool with 18 tests
- `/run/media/scooter/W.D-SSD/Prod/maestro/crates/maestro-claw/src/lib.rs` - Updated exports

### Test Counts:
- ShellTool: 19 tests
- FileTool: 22 tests
- MemoryTool: 18 tests
- ToolRegistry: 6 tests
- ToolSpec/ToolOutput: 3 tests
- Total: 68+ tool-specific tests

### Features Implemented:
1. **Tool Trait**: Async execute, name, description, parameters_schema, to_spec()
2. **ToolRegistry**: O(1) lookup via HashMap, register, get, list, to_tool_specs
3. **ShellTool**: Command risk classification (Safe/Moderate/Dangerous/Blocked), timeout handling, async execution
4. **FileTool**: Read/write/list/exists operations, path traversal protection, extension filtering, size limits
5. **MemoryTool**: Store/search/get/delete operations, category support, metadata, MockMemoryBackend for testing
