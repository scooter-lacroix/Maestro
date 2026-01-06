# OpenCode CLI Agent Integration - Quick Reference

## TL;DR

**YES**, OpenCode can invoke external CLI tools as agents, but through a **prompt-based approach** (not direct tool registration like Claude Code).

## Key Pattern

OpenCode agents use **markdown files** that contain instructions telling the agent to execute CLI tools via bash:

```markdown
---
mode: subagent
model: inherit
---
You are qwen-coder. Use the qwen CLI like this:
```bash
qwen -p "prompt" @files
```
```

## Agent Locations

- **Built-in agents:** `/home/stan/.config/opencode/agent/*.md`
- **Custom agents:** Same directory (auto-discovered)
- **CLI creation:** `opencode agent create --description "..." --mode subagent`

## Discovered CLI Tools

| Tool | Agent Name | Pattern |
|------|-----------|---------|
| `gemini` | `gemini-analyzer` | `gemini -p "prompt" @files --yolo` |
| `qwen` | `qwen-coder` | `qwen -p "prompt" @files` |
| `codex` | `codex-reviewer` | `codex exec --skip-git-repo-check "task"` |
| `opencode` | (built-in) | `opencode run --agent <name> "prompt"` |

## Commands

```bash
# List agents
opencode agent list

# Create agent
opencode agent create --description "My agent" --mode subagent

# Run agent
opencode run --agent gemini-analyzer "Analyze this codebase"

# Start TUI with agent
opencode --agent qwen-coder
```

## Agent Template

```markdown
---
description: |
  Use this agent for [specific task]
mode: subagent
model: inherit
---
You are [agent-name], powered by [CLI-tool].

## Usage
```bash
[cli-tool] -p "prompt" @files
```

## Examples
- [cli-tool] -p "task1" @src/file.js
- [cli-tool] -p "task2" @directory/
```

## Comparison: Claude Code vs OpenCode

| Aspect | Claude Code | OpenCode |
|--------|-------------|----------|
| Agent Format | `plugin.json` + skills/commands | `.md` or `.json` files |
| Tool Integration | Direct tool registration | Prompt-based bash invocation |
| Location | `~/.claude/plugins/` | `~/.config/opencode/agent/` |
| Complexity | Higher (SDK, hooks, etc.) | Lower (just markdown) |

## For Maestro Implementation

**Checks during `/maestro:configure`:**

1. ✅ Detect CLI tools: `which gemini qwen codex`
2. ✅ Validate tools: `gemini --version`, `qwen --version`
3. ✅ Check API keys: `echo $GOOGLE_API_KEY`
4. ✅ Test functionality: `gemini -p "test" --help`
5. ✅ Create agent definitions in `/home/stan/.config/opencode/agent/`
6. ✅ Verify with: `opencode agent list`

**Priority agents to create:**
1. `gemini-analyzer` - Large codebase analysis
2. `qwen-coder` - Refactoring and implementation
3. `codex-reviewer` - Production validation
4. `maestro-orchestrator` - Maestro-specific tasks

## Critical Finding

**OpenCode agents integrate CLI tools through PROMPT ENGINEERING, not code.**

The agent's markdown file contains instructions that tell the LLM to execute bash commands that invoke the CLI tool. This is fundamentally different from Claude Code's native plugin system.

## Full Report

See: `/home/stan/Prod/maestro/opencode/OPENCODE-CLI-AGENT-INVESTIGATION.md`
