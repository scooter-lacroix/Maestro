# Subtrack 03: Agent Engine

## Objective
Implement the core agent loop and hook system for the Claw Agent framework.

## Requirements

### R1: Agent Loop
- agent_loop() function with turn-by-turn execution
- Receives session, thread, provider, registry, hooks, config
- Returns final response string
- Loops on tool calls until text response
- Enforces max_turns limit
- Enforces timeout per iteration

### R2: Hook Trait
- name() returns hook identifier
- pre_execute(context: &mut HookContext) - before provider call
- post_execute(context: &HookContext) - after provider response

### R3: HookSystem
- register(hook: Arc<dyn Hook>)
- execute_pre(context: &mut HookContext) - runs all pre-hooks in order
- execute_post(context: &HookContext) - runs all post-hooks in order
- Error handling: continue or abort based on hook config

### R4: Built-in Hooks
- LoggingHook: Log requests and responses
- MemoryHook: Inject relevant memories into context

## Acceptance Criteria
- [ ] agent_loop() executes turn-by-turn
- [ ] Tool call detection and execution works
- [ ] Loop continues on tool calls, terminates on text
- [ ] max_turns limit enforced
- [ ] Hook trait and HookSystem implemented
- [ ] LoggingHook implemented
- [ ] MemoryHook integrated with maestro-core
- [ ] >98% test coverage
