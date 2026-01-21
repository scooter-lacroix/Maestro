# Sub-Track 04: CLI First-Class Integrations - Plan

## Phase 1: Canonical Integration Contract

### [ ] Task 1.1: Define “first-class” expectations per tool
- [ ] Enumerate what “first-class” means for each client:
  - Command surface mechanism (prompts vs custom commands vs templates)
  - MCP configuration location + schema
  - Optional: hooks/skills/agents support
- [ ] Lock the canonical Maestro command list (minimum set) and naming conventions per tool
- [ ] Define a compatibility policy (aliases allowed, but no TLDR runtime)

### [ ] Task 1.2: Define install targets + filesystem layout in repo
- [ ] Create a dedicated repository layout for integration artifacts (per tool)
- [ ] Decide what is canonical source-of-truth for command bodies:
  - Single canonical command docs → generated per tool, or
  - Per-tool authored commands with shared includes/templates
- [ ] Define update policy for installed artifacts (overwrite vs merge vs versioned)

## Phase 2: Package the Integrations (Artifacts)

### [ ] Task 2.1: OpenCode integration package (self-contained)
- [ ] Define OpenCode-owned install paths:
  - `~/.config/opencode/commands/` for command markdown
  - `~/.config/opencode/skill/maestro/` for the Maestro skill
- [ ] Update OpenCode templates in `opencode.json` to reference OpenCode paths (no `~/.claude`)
- [ ] Ensure command naming does not collide (prefer `/maestro …` or `/maestro:*` policy)

### [ ] Task 2.2: Codex CLI integration package (custom prompts)
- [ ] Generate prompt markdown files for `$CODEX_HOME/prompts`:
  - Include `description:` and `argument-hint:` frontmatter
  - Ensure prompt bodies are compatible with Codex placeholder substitution rules
- [ ] Decide naming strategy:
  - One prompt per command (e.g., `maestro:setup.md` invoked as `/prompts:maestro:setup`)
  - Or a single router prompt (e.g., `/prompts:maestro COMMAND=setup`)
- [ ] Provide a deterministic upgrade path (version tags in frontmatter comments or file header)

### [ ] Task 2.3: Gemini CLI extension package
- [ ] Create `gemini-extension.json` and `commands/` TOML custom commands for:
  - `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`, `/maestro:orchestrate`, etc.
- [ ] Include `mcpServers` in the extension (optional if user settings already handle it)
- [ ] Include `GEMINI.md` context file describing Maestro usage within Gemini CLI

### [ ] Task 2.4: Qwen Code extension package
- [ ] Create `qwen-extension.json` and `commands/` TOML custom commands matching Gemini set
- [ ] Include `mcpServers` in the extension (optional) and a `QWEN.md` context file
- [ ] Confirm command TOML prompt patterns (`{{args}}`, `@{}`, `!{}`) are used correctly

### [ ] Task 2.5: Amp + Droid MCP-only integration packages
- [ ] Amp: define JSON patch for `~/.config/amp/settings.json` under `amp.mcpServers`
- [ ] Droid: define JSON patch for Factory MCP config with required transport/type fields

## Phase 3: Installer / Integrator Implementation

### [ ] Task 3.1: Implement a single “integrator” entrypoint
- [ ] Choose integration engine location:
  - Rust (`maestro integrate …`) preferred for v2.5
  - Setup wizard calls integrator, not ad-hoc shell snippets
- [ ] Provide subcommands:
  - `install <tool>` / `install --all`
  - `uninstall <tool>`
  - `doctor <tool>` (validate)
  - `print <tool>` (emit config patches)

### [ ] Task 3.2: Idempotent config editing + backups
- [ ] JSON config updates (OpenCode, Gemini, Qwen, Amp, Droid):
  - Preserve unknown keys
  - Merge/update `leindex` entry (replace only that entry)
  - Create timestamped backups
- [ ] TOML config updates (Codex):
  - Use a TOML editor that preserves formatting where possible
  - Replace/update `[mcp_servers.leindex]` deterministically

### [ ] Task 3.3: Artifact installation (copy/symlink + version checks)
- [ ] Install per-tool artifacts into correct directories
- [ ] Detect and handle conflicting prior installs (old locations, legacy names)
- [ ] Provide “dry run” mode to show changes without applying

### [ ] Task 3.4: Automated tests
- [ ] Unit tests for config patching across all supported schemas
- [ ] Regression tests that assert OpenCode integration never references `~/.claude`
- [ ] Tests for idempotence (apply twice, no diff)

## Phase 4: Documentation

### [ ] Task 4.1: Add per-client docs pages
- [ ] `docs/CODEX.md`
- [ ] `docs/GEMINI.md`
- [ ] `docs/QWEN.md`
- [ ] `docs/AMP.md`
- [ ] `docs/DROID.md`
- [ ] Update `docs/OPENCODE.md` to reflect first-class, self-contained install

### [ ] Task 4.2: Update README install matrix
- [ ] Make tool selection explicit (what gets installed where)
- [ ] Provide “quick verify” commands per tool

## Phase 5: Verification

### [ ] Task 5.1: Manual verification (local)
- [ ] OpenCode: `/maestro setup` works and loads from OpenCode-owned paths
- [ ] Codex: `/prompts:maestro:setup` (or chosen naming) executes correct protocol
- [ ] Gemini/Qwen: `/maestro:setup` commands resolve and run
- [ ] Amp/Droid: `/mcp` lists `leindex` and tools are callable

### [ ] Task 5.2: Maestro - User Manual Verification 'Sub-Track 04' (Protocol in workflow.md)

