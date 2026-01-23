# Sub-Track 04: CLI First-Class Integrations - Plan

## Phase 1: Canonical Integration Contract

### [x] Task 1.1: Define "first-class" expectations per tool
- [x] Enumerate what "first-class" means for each client:
  - Command surface mechanism (prompts vs custom commands vs templates)
  - MCP configuration location + schema
  - Optional: hooks/skills/agents support
- [x] Lock the canonical Maestro command list (minimum set) and naming conventions per tool
- [x] Define a compatibility policy (aliases allowed, but no TLDR runtime)

### [x] Task 1.2: Define install targets + filesystem layout in repo
- [x] Create a dedicated repository layout for integration artifacts (per tool)
- [x] Decide what is canonical source-of-truth for command bodies:
  - Single canonical command docs → generated per tool, or
  - Per-tool authored commands with shared includes/templates
- [x] Define update policy for installed artifacts (overwrite vs merge vs versioned)

## Phase 2: Package the Integrations (Artifacts)

### [x] Task 2.1: OpenCode integration package (self-contained)
- [x] Define OpenCode-owned install paths:
  - `~/.config/opencode/commands/` for command markdown
  - `~/.config/opencode/skill/maestro/` for the Maestro skill
- [x] Update OpenCode templates in `opencode.json` to reference OpenCode paths (no `~/.claude`)
- [x] Ensure command naming does not collide (prefer `/maestro …` or `/maestro:*` policy)

### [x] Task 2.2: Codex CLI integration package (custom prompts)
- [x] Generate prompt markdown files for `$CODEX_HOME/prompts`:
  - Include `description:` and `argument-hint:` frontmatter
  - Ensure prompt bodies are compatible with Codex placeholder substitution rules
- [x] Decide naming strategy:
  - One prompt per command (e.g., `maestro:setup.md` invoked as `/prompts:maestro:setup`)
  - Or a single router prompt (e.g., `/prompts:maestro COMMAND=setup`)
- [x] Provide a deterministic upgrade path (version tags in frontmatter comments or file header)

### [x] Task 2.3: Gemini CLI extension package
- [x] Create `gemini-extension.json` and `commands/` TOML custom commands for:
  - `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`, `/maestro:orchestrate`, etc.
- [x] Include `mcpServers` in the extension (optional if user settings already handle it)
- [x] Include `GEMINI.md` context file describing Maestro usage within Gemini CLI

### [x] Task 2.4: Qwen Code extension package
- [x] Create `qwen-extension.json` and `commands/` TOML custom commands matching Gemini set
- [x] Include `mcpServers` in the extension (optional) and a `QWEN.md` context file
- [x] Confirm command TOML prompt patterns (`{{args}}`, `@{}`, `!{}`) are used correctly

### [x] Task 2.5: Amp + Droid MCP-only integration packages
- [x] Amp: define JSON patch for `~/.config/amp/settings.json` under `amp.mcpServers`
- [x] Droid: define JSON patch for Factory MCP config with required transport/type fields

## Phase 3: Installer / Integrator Implementation

### [x] Task 3.1: Implement a single "integrator" entrypoint
- [x] Choose integration engine location:
  - Rust (`maestro integrate …`) preferred for v2.5
  - Setup wizard calls integrator, not ad-hoc shell snippets
- [x] Provide subcommands:
  - `install <tool>` / `install --all`
  - `uninstall <tool>`
  - `doctor <tool>` (validate)
  - `print <tool>` (emit config patches)

### [x] Task 3.2: Idempotent config editing + backups
- [x] JSON config updates (OpenCode, Gemini, Qwen, Amp, Droid):
  - Preserve unknown keys
  - Merge/update `leindex` entry (replace only that entry)
  - Create timestamped backups
- [x] TOML config updates (Codex):
  - Use a TOML editor that preserves formatting where possible
  - Replace/update `[mcp_servers.leindex]` deterministically

### [x] Task 3.3: Artifact installation (copy/symlink + version checks)
- [x] Install per-tool artifacts into correct directories
- [x] Detect and handle conflicting prior installs (old locations, legacy names)
- [x] Provide "dry run" mode to show changes without applying

### [x] Task 3.4: Automated tests
- [x] Unit tests for config patching across all supported schemas
- [x] Regression tests that assert OpenCode integration never references `~/.claude`
- [x] Tests for idempotence (apply twice, no diff)

## Phase 4: Documentation

### [x] Task 4.1: Add per-client docs pages
- [x] `docs/CODEX.md`
- [x] `docs/GEMINI.md`
- [x] `docs/QWEN.md`
- [x] `docs/AMP.md`
- [x] `docs/DROID.md`
- [x] Update `docs/OPENCODE.md` to reflect first-class, self-contained install

### [x] Task 4.2: Update README install matrix
- [x] Make tool selection explicit (what gets installed where)
- [x] Provide "quick verify" commands per tool

## Phase 5: Verification

### [ ] Task 5.1: Manual verification (local)
- [ ] OpenCode: `/maestro setup` works and loads from OpenCode-owned paths
- [ ] Codex: `/prompts:maestro:setup` (or chosen naming) executes correct protocol
- [ ] Gemini/Qwen: `/maestro:setup` commands resolve and run
- [ ] Amp/Droid: `/mcp` lists `leindex` and tools are callable

### [ ] Task 5.2: Maestro - User Manual Verification 'Sub-Track 04' (Protocol in workflow.md)

