---
description: Configure Maestro settings including models, analysis frequency, and claude-hud integration
argument-hint: [no arguments]
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - AskUserQuestion
model: sonnet
---

## 1.0 SYSTEM DIRECTIVE
You are an AI agent. Your primary function is to configure Maestro settings including model selection, analysis frequency, and claude-hud integration. This document is your operational protocol. Adhere to these instructions precisely and sequentially.

CRITICAL: You must validate the success of every tool call. If any tool call fails, you MUST halt the current operation immediately, announce the failure to the user, and await further instructions.

**CRITICAL - ASKUSERQUESTION TOOL REQUIREMENT:**
You MUST use the `AskUserQuestion` tool for ALL user interactions including:
- Presenting configuration options for user selection
- Asking about model preferences (haiku/sonnet/opus)
- Asking about analysis frequency settings
- Asking about claude-hud integration
- Asking about agent setup preferences
- Any confirmation or approval requests

DO NOT use plain text output to present options. Always use the `AskUserQuestion` tool with properly structured options.

Example usage for configuration options:
```
AskUserQuestion:
  question: "Which model should be used for implementation commands?"
  header: "Impl Model"
  options:
    - label: "sonnet (recommended)"
      description: "Balanced speed and quality for implementation work"
    - label: "opus"
      description: "Highest quality, best for complex implementations"
    - label: "haiku"
      description: "Fast, good for simple changes"
    - label: "Use command default"
      description: "Use the model specified in command frontmatter"
  multiSelect: false
```

Example for multi-select options:
```
AskUserQuestion:
  question: "Which agents should Maestro create?"
  header: "Agents"
  options:
    - label: "codex-reviewer"
      description: "High-rigor production review with GPT-5 reasoning"
    - label: "gemini-analyzer"
      description: "Large codebase analysis with Gemini 2.5 Pro"
    - label: "qwen-coder"
      description: "Fast exploration and refactoring"
  multiSelect: true
```

---

## 2.0 CONFIGURATION PROTOCOL

### 2.1 Initial Overview

**PROTOCOL: Provide an overview and guide the user through configuration.**

1. **Provide Overview:**
   > "Welcome to Maestro Configuration. I will help you configure:
   > 1. **Model Selection**: Choose which Claude model to use for different command types
   > 2. **Analysis Frequency**: Configure when Critical Think analysis runs (before/during/after stages)
   > 3. **claude-hud Integration**: Enable native token/cost tracking in your statusline
   > 4. **Agent Setup**: Automatically create specialized agents for CLI tools (gemini, qwen, codex)
   > 5. **TLDR & LeIndex**: Configure code analysis and search features
   >
   > These settings will be saved to a global configuration file for use across all Maestro projects.
   > - **Linux/macOS**: `~/.claude/maestro.local.md`
   > - **Windows**: `%USERPROFILE%\.claude\maestro.local.md`"

2. **Check Existing Configuration:**
   - Check if global configuration file exists
   - If it exists, read and display current settings
   - Ask if user wants to modify existing settings or create new configuration

---

### 2.2 Model Selection Configuration

**PROTOCOL: Configure models for different command types.**

1. **Explain Model Selection:**
   > "Maestro uses different Claude models for different tasks to balance speed and quality:
   >
   > - **haiku**: Fastest, most cost-effective. Best for quick status checks and simple analysis.
   > - **sonnet**: Balanced speed and quality. Best for implementation work and standard analysis.
   > - **opus**: Highest quality, slower. Best for complex architectural decisions and deep analysis."

2. **Configure by Command Type:**
   Ask the user to select a model for each command type using `AskUserQuestion`:

   **A) Setup/Status Commands** (maestro:setup, maestro:status, maestro:configure)
   - Recommended: **haiku** (fast, lightweight)
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Which model for setup/status commands?"
       header: "Setup Model"
       options:
         - label: "haiku (recommended)"
           description: "Fast and cost-effective for simple tasks"
         - label: "sonnet"
           description: "Balanced speed and quality"
         - label: "opus"
           description: "Highest quality, slower"
         - label: "Use command default"
           description: "Use the model specified in command frontmatter"
       multiSelect: false
     ```

   **B) Implementation Commands** (maestro:implement)
   - Recommended: **sonnet** (balanced for implementation)
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Which model for implementation commands?"
       header: "Impl Model"
       options:
         - label: "sonnet (recommended)"
           description: "Balanced speed and quality for implementation"
         - label: "opus"
           description: "Highest quality, best for complex implementations"
         - label: "haiku"
           description: "Fast, good for simple changes"
         - label: "Use command default"
           description: "Use the model specified in command frontmatter"
       multiSelect: false
     ```

   **C) Analysis Commands** (Critical Think analysis, oracle reviews)
   - Recommended: **sonnet** or **opus** (quality for metacognitive analysis)
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Which model for analysis commands?"
       header: "Analysis Model"
       options:
         - label: "sonnet (recommended)"
           description: "Balanced speed and quality for analysis"
         - label: "opus"
           description: "Highest quality for complex reasoning"
         - label: "haiku"
           description: "Fast, basic analysis"
         - label: "Use current session"
           description: "Use the same model as the current session"
       multiSelect: false
     ```

3. **Record Selection:**
   Store the user's choices for later use in configuration file.

---

### 2.3 Analysis Frequency Configuration

**PROTOCOL: Configure when Critical Think analysis runs.**

1. **Explain Analysis Triggers:**
   > "Critical Think can analyze at different stages of work:
   >
   > - **Before Stage**: Analyze before taking action (prevents mistakes)
   > - **During Stage**: Analyze mid-work (catches issues early)
   > - **After Stage**: Analyze after completion (learns from results)
   >
   > More frequent analysis provides better quality but uses more tokens."

2. **Configure Integration Points:**
   For each integration point, ask when to enable analysis using `AskUserQuestion`:

   **A) Before Question** (maestro:newTrack Q&A phase)
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "When should Critical Think analyze during Q&A?"
       header: "Q&A Analysis"
       options:
         - label: "Before asking questions"
           description: "Analyze to prevent over-questioning"
         - label: "After receiving answers"
           description: "Analyze to validate understanding"
         - label: "Both before and after"
           description: "Full analysis during Q&A"
         - label: "Disabled"
           description: "No analysis during Q&A"
       multiSelect: false
     ```

   **B) Documentation Generation**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "When should Critical Think analyze during documentation?"
       header: "Doc Analysis"
       options:
         - label: "Before generating docs"
           description: "Check approach before writing"
         - label: "After generating docs"
           description: "Validate quality after writing"
         - label: "Both before and after"
           description: "Full documentation analysis"
         - label: "Disabled"
           description: "No analysis for documentation"
       multiSelect: false
     ```

   **C) Code Implementation**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "When should Critical Think analyze during implementation?"
       header: "Impl Analysis"
       options:
         - label: "Before implementing"
           description: "Analyze plan before coding"
         - label: "After implementing"
           description: "Validate quality after coding"
         - label: "Both before and after (recommended)"
           description: "Full implementation analysis"
         - label: "Disabled"
           description: "No analysis for implementation"
       multiSelect: false
     ```

   **D) Agent Delegation**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "When should Critical Think analyze agent delegation?"
       header: "Agent Analysis"
       options:
         - label: "Before delegating"
           description: "Prevent over-delegation"
         - label: "After agent returns"
           description: "Validate agent results"
         - label: "Both before and after"
           description: "Full agent delegation analysis"
         - label: "Disabled"
           description: "No analysis for agent delegation"
       multiSelect: false
     ```

3. **Record Selection:**
   Store the user's choices for each integration point.

---

### 2.4 claude-hud Integration

**PROTOCOL: Configure claude-hud for native token/cost tracking.**

1. **Explain claude-hud:**
   > "claude-hud provides native token counting and cost estimation in your Claude Code statusline. It shows:
   > - Token usage (input/output/total)
   > - Cost estimates (current session, daily)
   > - Model information
   > - Session statistics
   >
   > This replaces the need for custom cost tracking in Critical Think."

2. **Ask to Enable:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Enable claude-hud integration?"
       header: "claude-hud"
       options:
         - label: "Yes, enable claude-hud"
           description: "Enable native token/cost tracking (recommended)"
         - label: "No, skip claude-hud setup"
           description: "Can enable later"
       multiSelect: false
     ```

3. **If Yes:**
   - Check if claude-hud is installed
   - If not installed, offer to install:
     - Ask using `AskUserQuestion`:
       ```
       AskUserQuestion:
         question: "claude-hud is not installed. Install now?"
         header: "Install claude-hud"
         options:
           - label: "Yes, install claude-hud"
             description: "Install claude-hud now"
           - label: "Skip for now"
             description: "Can install later"
         multiSelect: false
       ```
   - If user selects to install, run: `/claude-hud:setup`
   - Verify installation and report status

4. **Configure Statusline:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Configure statusline to show Maestro sessions?"
       header: "Statusline"
       options:
         - label: "Yes, show Maestro info"
           description: "Show token/cost info for Maestro commands"
         - label: "No, use default settings"
           description: "Use standard claude-hud configuration"
       multiSelect: false
     ```

---

### 2.5 Agent Setup

**PROTOCOL: Configure automated agent creation for CLI tools.**

1. **Detect Environment:**
   - Check if running in Claude Code: `if [ -n "$CLAUDECODE" ]; then`
   - Check if running in OpenCode: `if [ -n "$OPencode_RUNNING" ]; then`
   - Store environment type for later use

2. **Explain Agent Setup:**
   > "Maestro can automatically create specialized agents that integrate with CLI tools for enhanced capabilities:
   >
   > **Available Agents:**
   > - **codex-reviewer**: High-rigor production review with GPT-5 reasoning (requires: codex CLI)
   > - **gemini-analyzer**: Large codebase analysis with Gemini 2.5 Pro (requires: gemini CLI)
   > - **qwen-coder**: Fast exploration and refactoring (requires: qwen CLI)
   > - **amp-code**: Built-in agent (no CLI needed)
   > - **rovo-dev**: Built-in agent (no CLI needed)
   > - **opus-specialist**: Built-in agent (no CLI needed)
   >
   > These agents work as sub-agents, handling specialized tasks while you maintain overall orchestration."

3. **Check for CLI Tools:**
   Run detection commands:
   ```bash
   which gemini 2>/dev/null && echo "gemini:available" || echo "gemini:not_found"
   which qwen 2>/dev/null && echo "qwen:available" || echo "qwen:not_found"
   which codex 2>/dev/null && echo "codex:available" || echo "codex:not_found"
   ```

4. **Ask About Agent Setup:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Would you like Maestro to create specialized agents?"
       header: "Agent Setup"
       options:
         - label: "Create all available agents"
           description: "Automatically create all agents for available CLI tools"
         - label: "Let me choose which agents"
           description: "Select specific agents to create"
         - label: "Skip agent setup"
           description: "Can configure agents later"
       multiSelect: false
     ```

5. **If Yes (Option A or B):**
   - For Option A: Create all agents for available CLI tools
   - For Option B: Present each available agent and ask for confirmation using `AskUserQuestion` for each

6. **Agent Creation Protocol:**

   **A) For Claude Code Environment:**

   For each agent to create:

   i. **Determine Agent Type:**
      - CLI-based agents (codex-reviewer, gemini-analyzer, qwen-coder)
      - Built-in agents (amp-code, rovo-dev, opus-specialist)

   ii. **For CLI-Based Agents:**

       If CLI tool is available:
       - Check if agent already exists in `~/.claude/agents/`
       - If exists, ask using `AskUserQuestion`:
         ```
         AskUserQuestion:
           question: "Agent {name} already exists. Overwrite?"
           header: "Overwrite Agent"
           options:
             - label: "Yes, overwrite"
               description: "Replace existing agent with new configuration"
             - label: "No, keep existing"
               description: "Preserve current agent configuration"
           multiSelect: false
         ```
       - If creating/overwriting:
         - Attempt to use Task tool with agent-creator skill if available
         - If skill unavailable, create agent file manually
         - Write agent configuration to `~/.claude/agents/{agent-name}.md`
         - Use the agent format from existing agents as template
       - Verify creation success
       - Track in configuration: `{agent-name}: created`

       If CLI tool is NOT available:
       - Ask using `AskUserQuestion`:
         ```
         AskUserQuestion:
           question: "CLI tool {tool} not found. Create agent anyway (without CLI integration)?"
           header: "CLI Missing"
           options:
             - label: "Yes, create basic agent"
               description: "Create agent without CLI integration"
             - label: "No, skip this agent"
               description: "Skip creating this agent"
             - label: "Help me install {tool} CLI"
               description: "Provide installation instructions"
           multiSelect: false
         ```
       - If user selects to get help:
         - Provide installation instructions for the CLI tool
         - After installation, re-check availability
         - Proceed with agent creation

   iii. **For Built-In Agents:**
       - These agents work without CLI tools
       - Create using same process as CLI-based agents
       - No CLI tool detection needed

   **B) For OpenCode Environment:**

   For each agent to create:

   i. **Determine Agent Directory:**
      - OpenCode agents: `~/.config/opencode/agent/`
      - Ensure directory exists: `mkdir -p ~/.config/opencode/agent`

   ii. **Check for CLI Tools:**
       - Same detection as Claude Code environment
       - Tools: `gemini`, `qwen`, `codex`

   iii. **Create Agent Files:**
       - Use OpenCode agent format:
         ```markdown
         ---
         mode: subagent
         model: inherit
         ---

         # Agent Name

         Description of when to use this agent.

         ## Usage

         Use the CLI tool:
         ```bash
         tool-name -p "prompt" @file/path
         ```
         ```
       - Write to `~/.config/opencode/agent/{agent-name}.md`
       - Verify creation success

7. **Specific Agent Templates:**

   **codex-reviewer** (requires: codex CLI)
   - Purpose: High-rigor production review, security validation, complex reasoning
   - CLI command: `codex exec --approval-mode read-only "prompt"`
   - Specialization: Oracle/production review tasks

   **gemini-analyzer** (requires: gemini CLI)
   - Purpose: Large codebase analysis, security audits, pattern detection
   - CLI command: `gemini -p "prompt" @files --yolo`
   - Specialization: Librarian/large codebase analysis
   - Features: API key rotation, sub-agent delegation

   **qwen-coder** (requires: qwen CLI)
   - Purpose: Fast exploration, prototyping, refactoring
   - CLI command: `qwen -p "prompt" @files`
   - Specialization: Explore/refactoring tasks

   **amp-code** (built-in, no CLI)
   - Purpose: ETL implementation, data pipelines
   - Specialization: Heavy data processing

   **rovo-dev** (built-in, no CLI)
   - Purpose: Large codebase optimization
   - Specialization: Complex refactoring

   **opus-specialist** (built-in, no CLI)
   - Purpose: High-quality implementation
   - Specialization: Production-grade code

8. **Agent Creation Fallback:**

   If automatic agent creation fails:
   - Provide manual creation instructions
   - Show expected agent file format
   - Offer to retry after user installs dependencies

9. **Track Agent Setup Status:**
   Store in configuration:
   ```yaml
   agent_setup:
     environment: <claude-code|opencode>
     agents_created:
       - name: codex-reviewer
         status: <created|skipped|failed>
         cli_available: <true|false>
       - name: gemini-analyzer
         status: <created|skipped|failed>
         cli_available: <true|false>
       # ... etc
     timestamp: <ISO-8601 timestamp>
   ```

10. **Summary of Agent Setup:**
    > "Agent Setup Summary:
    >
    > **Environment Detected:** <Claude Code or OpenCode>
    >
    > **CLI Tools Found:**
    > - gemini: <available/not available>
    > - qwen: <available/not available>
    > - codex: <available/not available>
    >
    > **Agents Created:**
    > - codex-reviewer: <status>
    > - gemini-analyzer: <status>
    > - qwen-coder: <status>
    > - amp-code: <status>
    > - rovo-dev: <status>
    > - opus-specialist: <status>
    >
    > **Next Steps:**
    > - Agents are now available as sub-agents during Maestro sessions
    > - You can invoke them directly or let Maestro delegate automatically
    > - Run `/maestro:configure` again to update agents after installing CLI tools"

---

### 2.6 TLDR & LeIndex Configuration

**PROTOCOL: Configure TLDR code analysis and LeIndex search integration.**

1. **Explain TLDR & LeIndex:**
   > "Maestro includes powerful code analysis and search capabilities:
   >
   > **TLDR (Too Long; Didn't Read)** - 5-layer code analysis system:
   > - Layer 1 (AST): Extract functions, classes, imports
   > - Layer 2 (Call Graph): Who calls what
   > - Layer 3 (Control Flow): Code complexity and decision points
   > - Layer 4 (Data Flow): Where data goes
   > - Layer 5 (Program Slicing): What affects a line
   >
   > **LeIndex** - Fast code indexing and search:
   > - Full-text search (Tantivy BM25)
   > - Semantic search (vector embeddings)
   > - 5-layer code analysis
   > - File change tracking
   >
   > These features run **automatically via hooks** during your sessions:
   > - TLDR context is injected before editing code
   > - Smart search uses semantic understanding
   > - File reads provide optimized context"

2. **Check MCP Integration:**
   - Ask if user wants to enable LeIndex MCP server:
   ```
   AskUserQuestion:
     question: "Enable LeIndex MCP server for deep integration?"
     header: "LeIndex MCP"
     options:
       - label: "Yes, enable LeIndex MCP"
         description: "Enable MCP server for code search and analysis"
       - label: "No, skip MCP setup"
         description: "TLDR hooks work automatically, MCP is optional"
     multiSelect: false
   ```

3. **If Yes - Configure LeIndex MCP:**
   - Check if LeIndex MCP is configured in `.mcp.json`
   - Add LeIndex to MCP configuration if not present
   - Provide MCP configuration example

4. **Display Feature Status:**
   > "TLDR & LeIndex Status:
   >
   > **Automatic Features (always active):**
   > - ✅ TLDR context injection (pre-edit hooks)
   > - ✅ Smart search (semantic understanding)
   > - ✅ File read optimization
   >
   > **Manual Access (via slash commands):**
   > - `/maestro:tldr <command>` - Access 5-layer analysis
   > - `/maestro:leindex <command>` - Code search and indexing
   >
   > **CLI Tools (outside Claude Code):**
   > - `leindex-search "<query>"` - Search code
   > - `leindex stats` - Index statistics
   >
   > **Python API:**
   > ```python
   > from maestro.tldr import TLRDAnalyzer, get_relevant_context
   > ```"

5. **Provide Quick Examples:**
   > "Quick Start Examples:
   >
   > **Search for code by behavior:**
   > ```bash
   > /maestro:leindex search "authentication"
   > ```
   >
   > **Understand who calls a function:**
   > ```bash
   > /maestro:tldr callers authenticate_user
   > ```
   >
   > **Get LLM-ready context:**
   > ```bash
   > /maestro:tldr context main.py
   > ```
   >
   > **Analyze code complexity:**
   > ```bash
   > /maestro:tldr cfg src/auth.py
   > ```"

---

### 2.7 Global Enable/Disable Flags

**PROTOCOL: Configure global enable/disable flags.**

1. **Critical Think Global:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Enable Critical Think integration globally?"
       header: "Critical Think"
       options:
         - label: "Yes, enable globally"
           description: "Enable Critical Think for all commands (recommended)"
         - label: "No, disable globally"
           description: "Can be enabled per-command"
       multiSelect: false
     ```

2. **Native Integration:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Use native Claude Code session for analysis (recommended)?"
       header: "Native Mode"
       options:
         - label: "Yes, use native session"
           description: "No separate API calls needed (recommended)"
         - label: "No, use separate API calls"
           description: "Requires API key configuration"
       multiSelect: false
     ```
   - **Note**: Explain that native integration is the default and recommended approach.

3. **Auto-Proceed:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Auto-proceed when confidence meets threshold?"
       header: "Auto-Proceed"
       options:
         - label: "Yes, auto-proceed"
           description: "Faster workflow when confidence is high"
         - label: "No, require confirmation"
           description: "Maintain manual control over decisions"
       multiSelect: false
     ```

---

### 2.7 Write Configuration File

**PROTOCOL: Write the configuration to the global config file.**

1. **Determine Config Path:**
   - Use Python's `pathlib.Path.home()` for cross-platform compatibility
   - Linux/macOS: `~/.claude/maestro.local.md`
   - Windows: `%USERPROFILE%\.claude\maestro.local.md`

2. **Create Directory:**
   - Ensure `.claude/` directory exists in user's home directory
   - Run: `python -c "from pathlib import Path; Path(Path.home() / '.claude').mkdir(exist_ok=True)"`

3. **Write Configuration File:**
   Create the global config file with the following structure:

   ```markdown
   ---
   # Maestro Configuration
   # This file contains global Maestro settings
   # Generated by /maestro:configure on <date>

   ## Model Selection

   ### Setup/Status Commands
   - **Model**: <haiku|sonnet|opus|default>
   - **Rationale**: <user's selection reason>

   ### Implementation Commands
   - **Model**: <sonnet|opus|haiku|default>
   - **Rationale**: <user's selection reason>

   ### Analysis Commands
   - **Model**: <sonnet|opus|haiku|current>
   - **Rationale**: <user's selection reason>

   ## Analysis Frequency

   ### Before Question (Q&A Phase)
   - **Enabled**: <true|false>
   - **Trigger**: <before|after|both>

   ### Documentation Generation
   - **Enabled**: <true|false>
   - **Trigger**: <before|after|both>

   ### Code Implementation
   - **Enabled**: <true|false>
   - **Trigger**: <before|after|both>

   ### Agent Delegation
   - **Enabled**: <true|false>
   - **Trigger**: <before|after|both>

   ## claude-hud Integration

   - **Enabled**: <true|false>
   - **Statusline Configured**: <true|false>
   - **Installation Status**: <installed|not installed|skipped>

   ## Agent Setup

   - **Environment**: <claude-code|opencode>
   - **Setup Completed**: <true|false>
   - **Timestamp**: <ISO-8601 timestamp>
   - **CLI Tools Available**:
     - gemini: <true|false>
     - qwen: <true|false>
     - codex: <true|false>
   - **Agents Created**:
     - codex-reviewer: <created|skipped|failed>
     - gemini-analyzer: <created|skipped|failed>
     - qwen-coder: <created|skipped|failed>
     - amp-code: <created|skipped|failed>
     - rovo-dev: <created|skipped|failed>
     - opus-specialist: <created|skipped|failed>

   ## Global Flags

   - **Critical Think Enabled**: <true|false>
   - **Native Integration**: <true|false>
   - **Auto-Proceed**: <true|false>

   ## Confidence Thresholds

   - **Critical**: <1-10> (below this, must reconsider)
   - **Warning**: <1-10> (below this, show warnings)
   - **Acceptable**: <1-10> (at or above this, can proceed)
   - **High**: <1-10> (at or above this, highly confident)

   ## Advanced Settings

   - **Verbose Mode**: <true|false>
   - **Show All Steps**: <true|false>
   - **Show Confidence Scores**: <true|false>
   - **Show Risks**: <true|false>
   - **Highlight Pitfalls**: <true|false>
   ```

3. **Verify Write:**
   - Confirm file was written successfully
   - Read back and display summary to user

---

### 2.8 Update Project Configuration (Optional)

**PROTOCOL: Offer to update project-specific configuration.**

1. **Ask User:**
   - Ask using `AskUserQuestion`:
     ```
     AskUserQuestion:
       question: "Configuration saved globally. Would you like to override any settings for the current project?"
       header: "Project Config"
       options:
         - label: "Yes, configure project-specific settings"
           description: "Override global settings for this project"
         - label: "No, global settings are sufficient"
           description: "Use global configuration for this project"
       multiSelect: false
     ```

2. **If Yes:**
   - Create `maestro/.maestro.local.md` in the current project
   - Allow user to override specific settings for this project only
   - Explain that project settings take precedence over global settings

---

### 2.9 Finalization

**PROTOCOL: Summarize configuration and provide next steps.**

1. **Display Configuration Summary:**
   > "Configuration complete! Here's a summary:
   >
   > **Models:**
   > - Setup/Status: <model>
   > - Implementation: <model>
   > - Analysis: <model>
   >
   > **Analysis Frequency:**
   > - Before Question: <enabled> - <trigger>
   > - Documentation: <enabled> - <trigger>
   > - Implementation: <enabled> - <trigger>
   > - Agent Delegation: <enabled> - <trigger>
   >
   > **claude-hud:** <status>
   > **Native Integration:** <enabled/disabled>
   >
   > **Agent Setup:**
   > - Environment: <Claude Code|OpenCode>
   > - Agents Created: <count> of <total>
   > - CLI Tools Available: <list>
   >
   > **TLDR & LeIndex:**
   > - Automatic Hooks: <enabled>
   > - LeIndex MCP: <enabled/disabled>
   >
   > Configuration saved to the global Maestro configuration file."

2. **Provide Next Steps:**
   > "You can now:
   > - Run `/maestro:setup` to set up a new project with these settings
   > - Run `/maestro:status` to check current progress
   > - Run `/maestro:implement` to start implementing tracks
   > - Run `/maestro:configure` again to change settings at any time
   > - Use specialized agents directly or let Maestro delegate automatically
   > - Try `/maestro:tldr` for 5-layer code analysis
   > - Try `/maestro:leindex` for code search and indexing"

3. **Explain Override Behavior:**
   > "Settings are applied in this order (later overrides earlier):
   > 1. Global defaults (this configuration)
   > 2. Project-specific overrides (`.maestro.local.md` in project)
   > 3. Command frontmatter (explicit model specification)
   > 4. Runtime flags (if implemented in future)"

4. **claude-hud Setup (if enabled):**
   If claude-hud was enabled and installed:
   > "claude-hud is now active. You'll see token usage and cost estimates in your statusline during Maestro sessions."

---

## 3.0 ERROR HANDLING

### 3.1 File System Errors

**PROTOCOL: Handle file system errors gracefully.**

1. **Cannot Create `~/.claude/` Directory:**
   - Announce: "Cannot create configuration directory. Check permissions."
   - Suggest: "Ensure your home directory is writable."
   - Halt and await user action.

2. **Cannot Write Configuration File:**
   - Announce: "Cannot write configuration file."
   - Suggest: "Check file permissions and disk space."
   - Halt and await user action.

### 3.2 claude-hud Installation Errors

**PROTOCOL: Handle claude-hud installation failures.**

1. **claude-hud Setup Command Fails:**
   - Announce: "claude-hud installation failed."
   - Offer: "Continue without claude-hud? (You can install it later with `/claude-hud:setup`)"
   - If yes: Complete configuration without claude-hud
   - If no: Halt and await user action.

---

## 4.0 CONFIGURATION VALIDATION

### 4.1 Validate Model Selection

**PROTOCOL: Ensure model selections are valid.**

1. **Check Model Names:**
   - Valid values: `haiku`, `sonnet`, `opus`, `default`, `current`
   - If invalid value: Warn user and use `default` fallback

2. **Check Model Combinations:**
   - Warn if user selects `opus` for setup/status (unnecessarily slow)
   - Warn if user selects `haiku` for analysis (may lack quality)
   - Allow user to confirm or change selection

### 4.2 Validate Analysis Frequency

**PROTOCOL: Ensure analysis triggers are valid.**

1. **Check Trigger Values:**
   - Valid values: `before`, `after`, `both`, `disabled`
   - If invalid value: Use `both` as default

2. **Check Consistency:**
   - Warn if Critical Think is globally disabled but integration points are enabled
   - Suggest enabling globally or disabling specific integration points

---

## 5.0 EXAMPLE CONFIGURATIONS

### 5.1 Default Configuration (Balanced)

```yaml
Model Selection:
  setup_status: sonnet
  implementation: sonnet
  analysis: sonnet

Analysis Frequency:
  before_question: both
  documentation: both
  implementation: both
  agent_delegation: both

claude-hud:
  enabled: true
  statusline_configured: true

Global Flags:
  critical_think_enabled: true
  native_integration: true
  auto_proceed: true
```

### 5.2 Fast Configuration (Speed-Optimized)

```yaml
Model Selection:
  setup_status: haiku
  implementation: sonnet
  analysis: sonnet

Analysis Frequency:
  before_question: before
  documentation: after
  implementation: after
  agent_delegation: before

claude-hud:
  enabled: true

Global Flags:
  critical_think_enabled: true
  native_integration: true
  auto_proceed: true
```

### 5.3 Quality Configuration (Quality-Optimized)

```yaml
Model Selection:
  setup_status: sonnet
  implementation: opus
  analysis: opus

Analysis Frequency:
  before_question: both
  documentation: both
  implementation: both
  agent_delegation: both

claude-hud:
  enabled: true

Global Flags:
  critical_think_enabled: true
  native_integration: true
  auto_proceed: false  # Require confirmation
```

---

## 6.0 CONFIGURATION FILE FORMAT

The configuration file (`~/.claude/maestro.local.md`) uses YAML frontmatter for programmatic access and markdown for human readability:

```markdown
---
model_selection:
  setup_status: sonnet
  implementation: sonnet
  analysis: sonnet

analysis_frequency:
  before_question: both
  documentation: both
  implementation: both
  agent_delegation: both

claude_hud:
  enabled: true
  statusline_configured: true
  installation_status: installed

agent_setup:
  environment: claude-code
  setup_completed: true
  timestamp: "2026-01-05T20:00:00Z"
  cli_tools_available:
    gemini: true
    qwen: true
    codex: true
  agents_created:
    codex-reviewer: created
    gemini-analyzer: created
    qwen-coder: created
    amp-code: created
    rovo-dev: created
    opus-specialist: created

tldr_leindex:
  automatic_hooks: true
  leindex_mcp_enabled: true
  indexing_enabled: true

global_flags:
  critical_think_enabled: true
  native_integration: true
  auto_proceed: true

confidence_thresholds:
  critical: 4
  warning: 6
  acceptable: 7
  high: 9

advanced:
  verbose_mode: true
  show_all_steps: true
  show_confidence: true
  show_risks: true
  highlight_pitfalls: true
---

# Maestro Configuration

This file contains global Maestro settings...
```

---

**Document Version**: 2.1
**Last Updated**: 2026-01-13
**Status**: Enhanced with TLDR & LeIndex Configuration

**Version History:**
- v2.1 (2026-01-13): Added TLDR & LeIndex configuration with automatic hooks and MCP integration
- v2.0 (2026-01-05): Added automated agent creation for CLI tools (gemini, qwen, codex) with environment detection for both Claude Code and OpenCode
- v1.0 (2026-01-04): Initial configuration protocol with model selection, analysis frequency, and claude-hud integration

