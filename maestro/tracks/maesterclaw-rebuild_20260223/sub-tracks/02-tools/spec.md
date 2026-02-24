# Subtrack 02: Tool System

## Objective
Implement the Tool trait, ToolRegistry, and built-in tools for the Claw Agent framework.

## Requirements

### R1: Tool Trait
- name() returns tool identifier
- description() returns human-readable description
- parameters_schema() returns JSON Schema for parameters
- execute(args: Value) -> Result<ToolOutput> (async)

### R2: ToolRegistry
- register(tool: Arc<dyn Tool>)
- get(name: &str) -> Option<&Arc<dyn Tool>>
- list() -> Vec<&dyn Tool>
- to_tool_specs() -> Vec<ToolSpec> for provider calls
- O(1) lookup performance

### R3: Built-in Tools
- ShellTool: Execute shell commands with safety constraints
- FileTool: Read/write file operations
- MemoryTool: Store/recall operations via maestro-core Memory trait

### R4: Tool Output
- ToolOutput struct with content and is_error flag
- Sanitized output (no sensitive data leakage)

## Acceptance Criteria
- [ ] Tool trait defined with all required methods
- [ ] ToolRegistry with O(1) lookup
- [ ] ShellTool implemented and tested (real execution)
- [ ] FileTool implemented and tested (real file ops)
- [ ] MemoryTool implemented and tested (real memory)
- [ ] >98% test coverage
