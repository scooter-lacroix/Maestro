# OpenCode CLI Agent Integration Investigation Report

**Date:** 2026-01-05
**Investigator:** Claude Code (goose-coder)
**Purpose:** Investigate how OpenCode handles CLI tool integration for agents
**Status:** COMPLETE

---

## Executive Summary

**Can OpenCode invoke external CLI tools as agents?**

**YES** - OpenCode has a sophisticated agent system that can wrap external CLI tools (like `gemini-cli`, `qwen-cli`, `codex`) as subagents. However, the approach is fundamentally different from Claude Code's plugin system:

- **Claude Code**: Uses native plugins with direct tool integration
- **OpenCode**: Uses markdown-based agent definitions that instruct agents to invoke external CLI tools via shell commands

**Key Finding:** OpenCode agents don't "wrap" CLI tools directly. Instead, agent definitions contain instructions that tell the agent to execute CLI commands using bash/shell tools.

---

## 1. Configuration Format

### 1.1 Agent Definition Locations

OpenCode agents are stored in TWO locations:

1. **Built-in agents:** `/home/stan/.config/opencode/agent/`
   - Contains `.md` files (markdown-based agent definitions)
   - Contains `.json` files (JSON-based agent configs)

2. **Custom agents:** Can be created via CLI
   - Location: Configurable via `opencode agent create --path`

### 1.2 Agent Definition Structure

#### Format A: Markdown-Based (Primary Format)

**Location:** `/home/stan/.config/opencode/agent/<agent-name>.md`

**Structure:**
```markdown
---
description: |
  Multi-line description of when to use this agent
  Includes usage examples and decision criteria
mode: subagent
model: inherit
---

# Agent Identity and Instructions

You are <agent-name>, a specialized agent with specific capabilities.

## Core Responsibilities
- What this agent does
- When to use it
- How it should behave

## CLI Tool Integration
Use the <tool-name> CLI in non-interactive mode with the format:
```bash
<tool-command> -p "prompt" @file/path
```

## Command Examples
<tool-command> -p "specific task" @src/file.js
```

**Example from `/home/stan/.config/opencode/agent/qwen-coder.md`:**
```markdown
---
description: |
  Use this agent when you need comprehensive code modification, refactoring...
mode: subagent
model: inherit
---
You are qwen-coder, a specialized coding assistant powered by Qwen Code CLI...

**Command Usage:**
Use the qwen CLI in non-interactive mode with the format: `qwen -p "detailed prompt" @file/path`

**Command Examples:**
```bash
qwen -p "Refactor this module to follow SOLID principles" @src/services/user-service.js
```
```

#### Format B: JSON-Based (Simpler Format)

**Location:** `/home/stan/.config/opencode/agent/<agent-name>.json`

**Structure:**
```json
{
  "name": "agent-name",
  "description": "Agent description",
  "model": "provider/model-name",
  "systemPrompt": "System prompt text",
  "tools": {
    "edit": true,
    "bash": true,
    "read": true,
    "write": true
  }
}
```

**Example from `/home/stan/.config/opencode/agent/orchestrator.json`:**
```json
{
  "name": "orchestrator",
  "description": "Orchestration-first agent that delegates to other tools",
  "model": "xai/grok-code-fast-1",
  "systemPrompt": "You are an orchestrator. Before doing any work, consider if you should delegate...",
  "tools": {
    "edit": true,
    "bash": true,
    "read": true,
    "write": true
  }
}
```

---

## 2. CLI Tool Integration Pattern

### 2.1 How External CLI Tools Are Integrated

OpenCode does NOT directly wrap CLI tools. Instead:

1. **Agent Definition**: Contains instructions on how to use the CLI tool
2. **Invocation**: Agent uses bash tool to execute CLI commands
3. **Pattern**: Agent markdown files include explicit command examples

**Key Pattern from `gemini-analyzer.md`:**
```markdown
**Technical Implementation:**

- Always use Gemini CLI in non-interactive mode (-p) for analysis tasks
- When performing modifications, use the interactive (-i) flag
- Command format (Analysis): gemini -p "your prompt" @path/to/files --yolo
- Command format (Modification): gemini -i "your prompt" @path/to/files --yolo
```

### 2.2 Discovered CLI Tools in Use

| CLI Tool | Purpose | Invocation Pattern | Location |
|----------|---------|-------------------|----------|
| **gemini** | Google Gemini AI | `gemini -p "prompt" @files --yolo` | `/home/stan/.nvm/versions/node/v22.19.0/bin/gemini` |
| **qwen** | Qwen Code AI | `qwen -p "prompt" @files` | `/home/stan/.nvm/versions/node/v22.19.0/bin/qwen` |
| **codex** | OpenAI Codex | `codex exec --skip-git-repo-check "task"` | `/home/stan/.nvm/versions/node/v22.19.0/bin/codex` |
| **opencode** | OpenCode CLI | `opencode run --agent <name> "prompt"` | `/home/stan/.opencode/bin/opencode` |

### 2.3 Agent Types

OpenCode supports TWO agent types:

1. **Primary Agents**: Full-featured, can be invoked directly
   - Examples: `build`, `plan`, `summary`, `compaction`
   - Have broader tool permissions

2. **Subagents**: Invoked by primary agents or other subagents
   - Examples: `gemini-analyzer`, `qwen-coder`, `codex-reviewer`, `amp-code`
   - Have restricted tool permissions
   - Specialized for specific tasks

---

## 3. Agent Creation Methods

### 3.1 Method 1: CLI-Based Creation

**Command:**
```bash
opencode agent create \
  --description "Agent description" \
  --mode subagent \
  --tools "bash,read,write,edit" \
  --model "provider/model"
```

**Options:**
- `--path`: Directory to generate agent file (default: agent directory)
- `--description`: What the agent should do
- `--mode`: Agent mode (`all`, `primary`, `subagent`)
- `--tools`: Comma-separated tools (bash, read, write, edit, list, glob, grep, webfetch, task, todowrite, todoread)
- `--model`: Model in `provider/model` format

### 3.2 Method 2: Manual File Creation

**For Markdown-Based Agents:**
1. Create file: `/home/stan/.config/opencode/agent/<agent-name>.md`
2. Add YAML frontmatter with `mode: subagent` and `description`
3. Add system prompt with CLI tool usage instructions
4. Include command examples

**For JSON-Based Agents:**
1. Create file: `/home/stan/.config/opencode/agent/<agent-name>.json`
2. Add required fields: `name`, `description`, `model`, `systemPrompt`, `tools`

### 3.3 Listing Available Agents

**Command:**
```bash
opencode agent list
```

**Output shows:**
- Agent name
- Agent type (primary/subagent)
- Permission matrix

---

## 4. Invocation Syntax

### 4.1 Invoking Agents from OpenCode CLI

**Basic invocation:**
```bash
opencode run --agent <agent-name> "your prompt here"
```

**Examples:**
```bash
# Invoke qwen-coder agent
opencode run --agent qwen-coder "Refactor this authentication module"

# Invoke gemini-analyzer agent
opencode run --agent gemini-analyzer "Analyze the entire codebase for security issues"

# Start OpenCode TUI with specific agent
opencode --agent gemini-analyzer
```

### 4.2 Agent Delegation Pattern

Agents can delegate to other agents via their system prompts:

**Example from `qwen-coder.md`:**
```markdown
## SUB-AGENT DELEGATION SYSTEM

You have access to TWO powerful sub-agents:

### 1. OpenCode with Grok Code Fast 1 (FREE, Ultra-Fast)
**How to invoke:**
```bash
opencode run --agent build "Implement [FEATURE]"
```

### 2. Codex CLI with GPT-5-Codex (High Quality)
**How to invoke:**
```bash
codex exec --skip-git-repo-check "YOUR_IMPLEMENTATION_TASK"
```
```

### 4.3 Permission System

Each agent has a permission matrix that controls:
- Which tools can be used
- Which files can be accessed
- Approval requirements for dangerous operations

**Example permissions:**
```json
[
  {
    "permission": "read",
    "pattern": "*.env",
    "action": "deny"
  },
  {
    "permission": "bash",
    "pattern": "*",
    "action": "allow"
  }
]
```

---

## 5. Comparison with Claude Code

### 5.1 Architectural Differences

| Aspect | Claude Code | OpenCode |
|--------|-------------|----------|
| **Agent Definition** | Native plugins with JSON manifests | Markdown or JSON files |
| **Tool Integration** | Direct tool registration in plugin | Agent instructed to use CLI via bash |
| **Invocation** | Plugin system with skill/tool hooks | Agent system with delegation |
| **Configuration** | `plugin.json` + commands/skills/hooks | Agent `.md` or `.json` files |
| **Extensibility** | Full plugin SDK with hooks | Agent creation + CLI tool wrapping |

### 5.2 Agent Definition Comparison

**Claude Code Plugin Structure:**
```
~/.claude/plugins/my-plugin/
├── plugin.json          # Plugin manifest
├── commands/            # Slash commands
├── skills/              # Skill definitions
├── hooks/               # Event hooks
└── agents/              # Agent definitions
```

**OpenCode Agent Structure:**
```
~/.config/opencode/agent/
├── my-agent.md          # Markdown-based definition
├── my-agent.json        # OR JSON-based definition
└── (no other directories needed)
```

### 5.3 Tool Integration Comparison

**Claude Code Approach:**
- Tools are directly registered in `plugin.json`
- Plugin provides tool implementations
- Native integration with Claude Code's tool system

**OpenCode Approach:**
- Agent markdown contains instructions to use CLI tools
- Agent invokes tools via bash command execution
- No direct tool registration, just prompt engineering

**Example:**
```markdown
# Claude Code (plugin.json)
{
  "tools": [
    {
      "name": "my-tool",
      "description": "My custom tool",
      "function": "myToolFunction"
    }
  ]
}

# OpenCode (agent.md)
---
mode: subagent
---
Use my-tool CLI like this:
```bash
my-tool -p "prompt" @files
```
```

---

## 6. Migration Considerations

### 6.1 Converting Claude Code Agents to OpenCode

**Step 1: Create OpenCode Agent Definition**
```bash
opencode agent create \
  --description "My specialized agent" \
  --mode subagent \
  --tools "bash,read,write,edit" \
  --model "provider/model"
```

**Step 2: Add CLI Tool Instructions**
Edit the generated agent file to include:
- CLI tool usage patterns
- Command examples
- API key management (if needed)
- Error handling patterns

**Step 3: Test Invocation**
```bash
opencode run --agent my-agent "test prompt"
```

### 6.2 Key Differences to Handle

1. **No Direct Tool Registration**: Must use bash to invoke CLI tools
2. **Prompt-Based Integration**: Tool usage is via instructions, not code
3. **Permission System**: Must configure agent permissions appropriately
4. **Model Selection**: Must specify which model the agent uses

### 6.3 Limitations

**OpenCode Limitations:**
- Cannot directly register custom tools (must use bash)
- Agent definitions are less structured than Claude Code plugins
- No event hooks or lifecycle management
- No built-in tool validation or schema enforcement

**Advantages:**
- Simpler agent creation (just markdown files)
- Easy to prototype new agents
- Flexible delegation patterns
- No compilation or build step required

---

## 7. Recommendations for Maestro

### 7.1 Automated Agent Creation Implementation

**Objective**: Automatically create OpenCode agents for available CLI tools during `/maestro:configure`

**Implementation Strategy:**

1. **Detect Available CLI Tools**
   ```bash
   # Check for common AI CLI tools
   which gemini qwen codex opencode
   ```

2. **Validate Tool Accessibility**
   ```bash
   # Test each tool
   gemini --version
   qwen --version
   codex --version
   ```

3. **Generate Agent Definitions**
   - Create `.md` files in `/home/stan/.config/opencode/agent/`
   - Include tool-specific usage patterns
   - Add command examples from documentation

4. **Register Agents in OpenCode**
   - Agents are auto-discovered from `.md` files
   - No additional registration needed

### 7.2 Checks to Perform During `/maestro:configure`

**Primary Checks:**
1. ✅ **CLI Tool Existence**: Verify tool is in PATH
   ```bash
   which <tool-name>
   ```

2. ✅ **Tool Version**: Check minimum version requirements
   ```bash
   <tool-name> --version
   ```

3. ✅ **API Key Availability**: Verify required credentials
   ```bash
   # Check for environment variables
   echo $GOOGLE_API_KEY
   echo $OPENAI_API_KEY
   ```

4. ✅ **Tool Functionality**: Test basic operation
   ```bash
   <tool-name> -p "test" --help
   ```

5. ✅ **Permission Requirements**: Check what tools the agent needs
   - Read/write file access
   - Bash execution
   - Network access

6. ✅ **Agent Directory Write Access**: Can create agent files
   ```bash
   ls -la /home/stan/.config/opencode/agent/
   ```

**Secondary Checks:**
7. ✅ **Existing Agent Conflicts**: Check for duplicate agent names
   ```bash
   opencode agent list | grep <agent-name>
   ```

8. ✅ **Model Availability**: Verify specified model is accessible
   ```bash
   opencode models <provider>
   ```

9. ✅ **Rate Limit Awareness**: Document API quotas
   - Gemini: 100 req/day (Pro), 1000 req/day (Flash)
   - Qwen: 2000 req/day
   - Codex: Varies by plan

### 7.3 Agent Template Structure

**Template for CLI Tool Agents:**
```markdown
---
description: |
  Use this agent when you need <specific-task-description>.
  Examples: <example-usage-scenarios>
mode: subagent
model: inherit
---

You are <agent-name>, a specialized agent powered by <tool-name> CLI.

## Core Responsibilities
- What this agent does
- When to use it
- Key capabilities

## CLI Tool Integration
Use the <tool-name> CLI in non-interactive mode:
```bash
<tool-command> -p "prompt" @files
```

## Command Examples
- Task 1: `<tool-command> -p "task1" @files`
- Task 2: `<tool-command> -p "task2" @files`

## API Key Management
- Primary: $API_KEY
- Backup: $API_KEY_2
- Rotation: <instructions>

## Rate Limits
- Daily quota: <number> requests/day
- Rate limit handling: <instructions>

## Specialization
- Primary domain: <domain>
- Strengths: <strengths>
- Routing criteria: <when-to-use>
```

### 7.4 Recommended Agent Names (Consistent with Claude Code)

Based on discovered OpenCode agents, use these naming conventions:

| CLI Tool | Agent Name | Pattern |
|----------|------------|---------|
| `gemini` | `gemini-analyzer` | `<tool>-<specialization>` |
| `qwen` | `qwen-coder` | `<tool>-<specialization>` |
| `codex` | `codex-reviewer` | `<tool>-<specialization>` |
| `maestro` | `maestro-orchestrator` | `<tool>-<specialization>` |

### 7.5 Implementation Priority

**Phase 1: Core Agents (High Priority)**
1. `gemini-analyzer` - Large codebase analysis
2. `qwen-coder` - Code refactoring and implementation
3. `codex-reviewer` - Production code review

**Phase 2: Specialized Agents (Medium Priority)**
4. `maestro-orchestrator` - Maestro-specific tasks
5. Custom domain-specific agents

**Phase 3: Enhancement (Low Priority)**
6. Agent optimization and tuning
7. Custom tool wrappers
8. Integration testing

---

## 8. Critical Findings

### 8.1 Key Discovery: Prompt-Based Tool Integration

**Most Important Finding:**
OpenCode agents integrate CLI tools through **prompt engineering**, not code. The agent's markdown file contains instructions that tell the LLM to execute bash commands that invoke the CLI tool.

**Example:**
```markdown
# In qwen-coder.md
Use the qwen CLI in non-interactive mode:
```bash
qwen -p "detailed prompt" @file/path
```
```

When the agent is invoked, it reads these instructions and executes:
```bash
qwen -p "refactor this code" @src/main.py
```

### 8.2 Advantages of This Approach

1. **Flexibility**: Easy to modify tool usage without code changes
2. **Simplicity**: No plugin SDK or compilation required
3. **Transparency**: Tool usage is visible in agent definition
4. **Rapid Prototyping**: Can test new agents quickly

### 8.3 Disadvantages

1. **No Type Safety**: CLI command syntax is not validated
2. **Error Handling**: Relies on LLM to handle errors correctly
3. **Performance**: Indirect invocation through bash is slower
4. **Debugging**: Harder to debug tool integration issues
5. **No Tool Discovery**: Tools must be manually documented

### 8.4 Security Considerations

**Risks:**
- Agents can execute arbitrary bash commands
- API keys may be exposed in agent definitions
- No sandbox enforcement by default

**Mitigations:**
- Use permission system to restrict dangerous operations
- Store API keys in environment variables, not agent files
- Review agent definitions before deployment
- Use `--approval-mode` for safety

---

## 9. Technical Details

### 9.1 OpenCode Version
```
OpenCode CLI v1.1.1
```

### 9.2 Agent Directory Structure
```
/home/stan/.config/opencode/
├── opencode.json           # Main configuration
├── agent/                  # Agent definitions
│   ├── orchestrator.json  # JSON-based agent
│   ├── amp-code.md        # Markdown-based agent
│   ├── codex-reviewer.md
│   ├── gemini-analyzer.md
│   ├── qwen-coder.md
│   └── (other agents)
└── plugin/                # Plugins (different from agents)
    └── prompt-enhancer.js
```

### 9.3 Available Tools in OpenCode

From `opencode agent create --help`:
- `bash` - Execute shell commands
- `read` - Read files
- `write` - Write files
- `edit` - Edit files
- `list` - List directory contents
- `glob` - File pattern matching
- `grep` - Search file contents
- `webfetch` - Fetch web content
- `task` - Spawn subagents
- `todowrite` - Write to-do items
- `todoread` - Read to-do items

### 9.4 Agent Permissions Matrix

Each agent has a permission matrix controlling:
- **Permission**: Tool or operation name
- **Pattern**: File pattern or wildcard
- **Action**: `allow`, `deny`, or `ask`

**Example:**
```json
[
  {
    "permission": "read",
    "pattern": "*.env",
    "action": "deny"
  },
  {
    "permission": "bash",
    "pattern": "*",
    "action": "allow"
  }
]
```

---

## 10. Conclusion

### 10.1 Summary of Findings

1. **OpenCode CAN invoke external CLI tools as agents** ✅
   - Method: Prompt-based instructions in agent markdown files
   - Pattern: Agent executes CLI tools via bash commands
   - Flexibility: High (easy to modify and test)

2. **Configuration is straightforward** ✅
   - Markdown-based agent definitions
   - JSON-based agent definitions
   - CLI-based agent creation tool

3. **Agent discovery is automatic** ✅
   - Agents in `/home/stan/.config/opencode/agent/` are auto-loaded
   - No registration required beyond file creation

4. **Integration differs from Claude Code** ⚠️
   - Claude Code: Native plugin system with direct tool registration
   - OpenCode: Prompt-based tool invocation via bash
   - Migration requires adaptation strategy

### 10.2 Recommendations for Maestro

**Implement during `/maestro:configure`:**

1. ✅ **Auto-detect CLI tools** (gemini, qwen, codex, etc.)
2. ✅ **Validate tool accessibility** (version, API keys, functionality)
3. ✅ **Generate agent definitions** (create `.md` files with tool-specific instructions)
4. ✅ **Configure permissions** (set appropriate tool access)
5. ✅ **Test agent invocation** (verify agents work correctly)
6. ✅ **Document configuration** (list created agents and their purposes)

**Implementation order:**
1. Start with gemini-analyzer (most critical for large codebase analysis)
2. Add qwen-coder (for refactoring and implementation)
3. Add codex-reviewer (for production validation)
4. Create maestro-specific agents (for Maestro workflows)

### 10.3 Next Steps

1. **Design Agent Templates**: Create standardized templates for common CLI tools
2. **Implement Detection Logic**: Build tool discovery system
3. **Create Generation Scripts**: Automate agent file creation
4. **Test Integration**: Verify agents work with Maestro workflows
5. **Document Migration Path**: Guide users from Claude Code to OpenCode

---

## Appendix A: Example Agent Definitions

### A.1 Gemini Analyzer Agent

**Location:** `/home/stan/.config/opencode/agent/gemini-analyzer.md`

**Key Features:**
- Large codebase analysis (1M+ token context)
- Security audits across multiple files
- Architecture pattern detection
- Read-only by default, modification on request

**Usage:**
```bash
opencode run --agent gemini-analyzer "Analyze the entire codebase for security vulnerabilities"
```

### A.2 Qwen Coder Agent

**Location:** `/home/stan/.config/opencode/agent/qwen-coder.md`

**Key Features:**
- Code refactoring and optimization
- Test generation
- Technical documentation
- Performance analysis

**Usage:**
```bash
opencode run --agent qwen-coder "Refactor this module to follow SOLID principles"
```

### A.3 Codex Reviewer Agent

**Location:** `/home/stan/.config/opencode/agent/codex-reviewer.md`

**Key Features:**
- High-rigor production review
- Security validation
- Architecture analysis
- GPT-5 reasoning capabilities

**Usage:**
```bash
opencode run --agent codex-reviewer "Review this authentication system for security issues"
```

---

## Appendix B: Command Reference

### B.1 Agent Management Commands

```bash
# List all agents
opencode agent list

# Create a new agent
opencode agent create --description "My agent" --mode subagent

# Run with specific agent
opencode run --agent <agent-name> "prompt"

# Start TUI with agent
opencode --agent <agent-name>
```

### B.2 CLI Tool Commands

```bash
# Gemini CLI
gemini -p "prompt" @files --yolo                    # Analysis
gemini -i "prompt" @files --yolo                    # Interactive modification

# Qwen CLI
qwen -p "prompt" @files                             # Non-interactive
qwen -p "prompt" --all-files                        # Entire codebase

# Codex CLI
codex exec --skip-git-repo-check "task"             # Non-interactive
codex exec --approval-mode read-only "analysis"     # Read-only analysis
codex exec --approval-mode auto "implementation"    # Auto-approve edits

# OpenCode CLI
opencode run --agent <name> "prompt"                # Invoke agent
opencode --agent <name>                             # Start TUI with agent
```

---

## Appendix C: Environment Variables

### C.1 API Key Management

**Gemini:**
- `GOOGLE_API_KEY` - Primary key (Gemini 2.5 Pro, 100 req/day)
- `GOOGLE_API_KEY_2` - Secondary key (backup)
- `GOOGLE_API_KEY_3` - Tertiary key (backup)

**Codex:**
- `OPENAI_API_KEY` - OpenAI API key for Codex access

**Qwen:**
- `QWEN_API_KEY` - Qwen API key (if required)

### C.2 OpenCode Configuration

**Location:** `/home/stan/.config/opencode/opencode.json`

**Key sections:**
- `mcp` - MCP server configurations
- `command` - Slash command definitions
- `plugin` - Plugin list
- `provider` - Model provider configurations

---

**End of Report**

**Investigation Status:** ✅ COMPLETE
**Next Action:** Implement automated agent creation in `/maestro:configure`
**Priority:** HIGH (enables multi-agent workflows in OpenCode)
