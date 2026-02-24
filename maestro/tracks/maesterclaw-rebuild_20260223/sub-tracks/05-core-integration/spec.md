# Subtrack 05: Core Integration

## Objective
Integrate maestro-claw with existing maestro-core traits (SecurityPolicy, Memory, Channel) without breaking changes.

## Requirements

### R1: SecurityPolicy Integration
- Tool execution respects SecurityPolicy from maestro-core
- AutonomyLevel enforced (HumanApproval, Supervised, Autonomous)
- SandboxManager integration for command execution

### R2: Memory Integration
- MemoryTool uses maestro-core Memory trait
- MemoryHook injects relevant memories into context
- Session persistence via memory system

### R3: Channel Integration
- Agent can receive messages from Channel implementations
- Agent can send responses via Channel trait
- ChannelRegistry integration

### R4: No Breaking Changes
- All existing maestro-core tests pass
- All existing cockpit tests pass
- Public APIs unchanged

## Acceptance Criteria
- [ ] SecurityPolicy enforced for tool execution
- [ ] Memory trait integrated for memory operations
- [ ] Channel trait integrated for message routing
- [ ] End-to-end agent execution with maestro-core
- [ ] All existing tests pass
- [ ] >98% test coverage for integration layer
