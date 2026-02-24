# Subtrack 01: Session/Thread/Turn Model

## Objective
Implement the foundational conversation model for the Claw Agent framework: Session → Thread → Turn hierarchy with full serialization support.

## Requirements

### R1: Session Model
- Session is the top-level conversation container
- Contains multiple Threads with metadata
- Supports creation timestamp and unique ID
- Can add, get, and list threads

### R2: Thread Model
- Thread groups related Turns within a Session
- Contains ordered list of Turns
- Optional summary field for context compression
- Can build next turn from history
- Can convert to provider message format

### R3: Turn Model
- Turn represents a single request/response cycle
- Has role: User, Assistant, System, or Tool
- Contains content (text)
- Contains tool_calls (optional, from assistant)
- Contains tool_results (optional, from tool execution)
- Timestamp for ordering

### R4: Serialization
- All models derive serde Serialize/Deserialize
- JSON format for persistence and API transport
- Human-readable when possible

## Acceptance Criteria
- [ ] Session struct with id, threads, metadata, created_at
- [ ] Thread struct with id, session_id, turns, summary
- [ ] Turn struct with id, role, content, tool_calls, tool_results, timestamp
- [ ] TurnRole enum (User, Assistant, System, Tool)
- [ ] All models serialize/deserialize correctly
- [ ] Thread::to_messages() produces provider-compatible format
- [ ] >98% test coverage
