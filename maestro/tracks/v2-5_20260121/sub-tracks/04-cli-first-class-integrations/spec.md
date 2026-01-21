# Sub-Track 04: CLI First-Class Integrations - Specification

## Objective

Make Maestro a **first-class citizen** across the target CLI tool ecosystem by providing:

1. A native-feeling command/extension surface per tool (as each tool supports it).
2. A consistent, installable integration story (via `maestro-setup` and/or a dedicated `maestro integrate` command).
3. Correct MCP configuration for LeIndex (and other Maestro MCP servers) per tool.

Target tools (v2.5 scope):

- Claude Code (Anthropic)
- OpenCode
- Codex CLI (OpenAI)
- Gemini CLI (Google)
- Qwen Code (QwenLM)
- AMP CLI (Sourcegraph)
- Droid CLI (Factory)

## Requirements

### R1: No Cross-Tool Dependencies

- OpenCode integration must **not** depend on Claude Code directories (no `~/.claude/...` references).
- Each tool’s integration artifacts must install into that tool’s **native** config/extension locations.

### R2: Native Command Surface per Tool

Each tool must expose Maestro commands using its native extension mechanism:

- **Claude Code**: plugin marketplace command pack (`claude-code/commands/*`) + hooks/skills/agents as needed.
- **OpenCode**: `opencode.json` `"command"` templates + installed Maestro command markdown files under an OpenCode-owned path.
- **Codex CLI**: custom prompts under `$CODEX_HOME/prompts` (invoked as `/prompts:<name>`), with YAML-like frontmatter (`description`, `argument-hint`) and placeholder expansion.
- **Gemini CLI**: extension under `~/.gemini/extensions/maestro` (`gemini-extension.json` + `commands/` TOML custom commands).
- **Qwen Code**: extension under `~/.qwen/extensions/maestro` (`qwen-extension.json` + `commands/` TOML custom commands).
- **AMP CLI**: MCP configuration under `~/.config/amp/settings.json` (`amp.mcpServers`).
- **Droid CLI**: MCP configuration under the documented Factory config location, including any required transport/type fields.

The Maestro command set exposed must minimally include:

- `maestro:setup`
- `maestro:newTrack`
- `maestro:implement`
- `maestro:orchestrate`
- `maestro:status`
- `maestro:revert`
- `maestro:configure`
- `maestro:leindex`
- `maestro:tui`

Compatibility aliases may exist (e.g., `/maestro:tldr` → LeIndex-backed), but must not revive TLDR runtime dependencies.

### R3: Correct MCP Integration

For each tool, install or update MCP server config so the tool can reach LeIndex:

- **LeIndex** server name: `leindex`
- Default transport: stdio
- Default command: `leindex`
- Default args: `["mcp"]`

Where tools require different schemas/keys, installers must conform exactly:

- Codex: `[mcp_servers.leindex]` in `config.toml`
- Gemini/Qwen: `"mcpServers": { "leindex": { "command": ..., "args": ... } }` in `settings.json` (or extension `mcpServers`)
- Amp: `"amp.mcpServers": { "leindex": { ... } }`
- OpenCode: `"mcp": { "leindex": { "type": "local", "command": [...] } }`
- Droid: ensure required `type` field is present when mandated (e.g., `type: "stdio"`).

### R4: Idempotent Installer + Validation

- Installation must be **repeatable** and safe:
  - Create backups of user config files before edits.
  - Merge/update existing config, preserving unrelated entries.
  - Avoid duplication on re-run.
- Provide validation/doctor checks for:
  - presence of Maestro command definitions in the tool
  - presence of LeIndex MCP server entry
  - absence of forbidden cross-tool references (e.g., OpenCode → `~/.claude`)

## Acceptance Criteria

- A user can select any subset of the target tools and run the installer once to achieve:
  - LeIndex MCP availability inside the tool
  - Maestro commands usable in that tool’s native UX
- OpenCode integration contains **zero** references to Claude Code directories or command files.
- Codex CLI integration uses `$CODEX_HOME/prompts` and works with Codex’s prompt frontmatter + placeholder rules.
- Gemini/Qwen integrations are shipped as valid extensions with commands in supported TOML format.
- Amp and Droid MCP configuration formats match their documented schemas.

