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
   Ask the user to select a model for each command type:

   **A) Setup/Status Commands** (maestro:setup, maestro:status, maestro:configure)
   - Recommended: **haiku** (fast, lightweight)
   - Ask: "Which model for setup/status commands?"
     - A) haiku (recommended - fast)
     - B) sonnet (balanced)
     - C) opus (high quality)
     - D) Use command frontmatter default

   **B) Implementation Commands** (maestro:implement)
   - Recommended: **sonnet** (balanced for implementation)
   - Ask: "Which model for implementation commands?"
     - A) sonnet (recommended - balanced)
     - B) opus (high quality)
     - C) haiku (fast)
     - D) Use command frontmatter default

   **C) Analysis Commands** (Critical Think analysis, oracle reviews)
   - Recommended: **sonnet** or **opus** (quality for metacognitive analysis)
   - Ask: "Which model for analysis commands?"
     - A) sonnet (recommended - balanced)
     - B) opus (high quality)
     - C) haiku (fast)
     - D) Use current session model

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
   For each integration point, ask when to enable analysis:

   **A) Before Question** (maestro:newTrack Q&A phase)
   - Ask: "When should Critical Think analyze during Q&A?"
     - A) Before asking questions (prevent over-questioning)
     - B) After receiving answers (validate understanding)
     - C) Both before and after
     - D) Disabled (no analysis during Q&A)

   **B) Documentation Generation**
   - Ask: "When should Critical Think analyze during documentation?"
     - A) Before generating docs (check approach)
     - B) After generating docs (validate quality)
     - C) Both before and after
     - D) Disabled

   **C) Code Implementation**
   - Ask: "When should Critical Think analyze during implementation?"
     - A) Before implementing (plan analysis)
     - B) After implementing (quality validation)
     - C) Both before and after (recommended)
     - D) Disabled

   **D) Agent Delegation**
   - Ask: "When should Critical Think analyze agent delegation?"
     - A) Before delegating (prevent over-delegation)
     - B) After agent returns (validate results)
     - C) Both before and after
     - D) Disabled

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
   - Ask: "Enable claude-hud integration?"
     - A) Yes, enable claude-hud (recommended)
     - B) No, skip claude-hud setup

3. **If Yes:**
   - Check if claude-hud is installed
   - If not installed, offer to install:
     - Ask: "claude-hud is not installed. Install now?"
       - A) Yes, install claude-hud
       - B) Skip for now
   - If user selects A, run: `/claude-hud:setup`
   - Verify installation and report status

4. **Configure Statusline:**
   - Ask: "Configure statusline to show Maestro sessions?"
     - A) Yes, show token/cost info for Maestro commands
     - B) No, use default claude-hud settings

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
   - Ask: "Would you like Maestro to create specialized agents?"
     - A) Yes, create all available agents
     - B) Yes, but let me choose which agents
     - C) No, skip agent setup

5. **If Yes (Option A or B):**
   - For Option A: Create all agents for available CLI tools
   - For Option B: Present each available agent and ask for confirmation

6. **Agent Creation Protocol:**

   **A) For Claude Code Environment:**

   For each agent to create:

   i. **Determine Agent Type:**
      - CLI-based agents (codex-reviewer, gemini-analyzer, qwen-coder)
      - Built-in agents (amp-code, rovo-dev, opus-specialist)

   ii. **For CLI-Based Agents:**

       If CLI tool is available:
       - Check if agent already exists in `~/.claude/agents/`
       - If exists, ask: "Agent {name} already exists. Overwrite?"
         - A) Yes, overwrite
         - B) No, keep existing
       - If creating/overwriting:
         - Attempt to use Task tool with agent-creator skill if available
         - If skill unavailable, create agent file manually
         - Write agent configuration to `~/.claude/agents/{agent-name}.md`
         - Use the agent format from existing agents as template
       - Verify creation success
       - Track in configuration: `{agent-name}: created`

       If CLI tool is NOT available:
       - Ask: "CLI tool {tool} not found. Create agent anyway (without CLI integration)?"
         - A) Yes, create basic agent
         - B) No, skip this agent
         - C) Help me install {tool} CLI
       - If C:
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

### 2.6 Global Enable/Disable Flags

**PROTOCOL: Configure global enable/disable flags.**

1. **Critical Think Global:**
   - Ask: "Enable Critical Think integration globally?"
     - A) Yes, enabled (recommended)
     - B) No, disabled (can be enabled per-command)

2. **Native Integration:**
   - Ask: "Use native Claude Code session for analysis (recommended)?"
     - A) Yes, use native session (no separate API calls)
     - B) No, use separate API calls (requires API key)

   **Note**: Explain that native integration is the default and recommended approach.

3. **Auto-Proceed:**
   - Ask: "Auto-proceed when confidence meets threshold?"
     - A) Yes, auto-proceed (faster workflow)
     - B) No, require confirmation (more control)

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
   - Ask: "Configuration saved globally. Would you like to override any settings for the current project?"
     - A) Yes, configure project-specific settings
     - B) No, global settings are sufficient

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
   > Configuration saved to the global Maestro configuration file."

2. **Provide Next Steps:**
   > "You can now:
   > - Run `/maestro:setup` to set up a new project with these settings
   > - Run `/maestro:status` to check current progress
   > - Run `/maestro:implement` to start implementing tracks
   > - Run `/maestro:configure` again to change settings at any time
   > - Use specialized agents directly or let Maestro delegate automatically"

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

**Document Version**: 2.0
**Last Updated**: 2026-01-05
**Status**: Enhanced with Agent Setup

**Version History:**
- v2.0 (2026-01-05): Added automated agent creation for CLI tools (gemini, qwen, codex) with environment detection for both Claude Code and OpenCode
- v1.0 (2026-01-04): Initial configuration protocol with model selection, analysis frequency, and claude-hud integration

