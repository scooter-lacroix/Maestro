---
name: maestro
description: "Maestro spec-driven development for Claude Code. Use this skill when running /maestro* commands (setup, newTrack, implement, orchestrate, status, revert, leindex, tui) so workflows stay aligned with LeIndex and the Rust TUI."
---

# Maestro in Claude Code

You are running Maestro inside Claude Code. Keep the experience native and deterministic:

- **Command surface:** `/maestro`, `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`, `/maestro:orchestrate`, `/maestro:status`, `/maestro:revert`, `/maestro:leindex`, `/maestro:tui`.
- **Where commands live:** `~/.claude/commands/maestro:*.md` (installed by the Maestro wizard).
- **MCP:** `mcpServers.leindex` points to `maestro mcp tool-search` (stdio). Use this instead of any legacy TLDR routing.
- **Skills location:** `~/.claude/skills/maestro/` (this file). Use it whenever Maestro workflows are requested.

## Quick workflow
1) `/maestro setup` — initialize or refresh project (`product.md`, `tech-stack.md`, `workflow.md`, `tracks.md`).
2) `/maestro newTrack "<goal>"` — generate spec + plan with clarifying questions.
3) `/maestro implement <track>` — execute plan with appropriate agents; lean on LeIndex for analysis.
4) `/maestro orchestrate` — cockpit/orchestrator loop (Rust TUI panel) for multi-task control.
5) `/maestro status` — report phases/tasks, blockers, and next actions.
6) `/maestro revert [track|phase|task]` — controlled rollback.
7) `/maestro leindex` — LeIndex-first analysis; no TLDR imports.
8) `/maestro tui` — launch the Rust cockpit (primary TUI; Go/legacy TUIs are retired).

## Guardrails
- **Do not** import or call `maestro.tldr` or anything under `maestro/archive/tldr`; those files are reference-only. All analysis uses the Rust LeIndex core.
- Prefer **LeIndex 5-phase analysis** when exploring code. Keep token use tight by asking for scoped file sets.
- When suggesting actions, keep within Maestro commands above; avoid ad-hoc shell unless necessary.
- After new CLI tools are installed, remind the user to run `/maestro:configure` to refresh integrations.

## Integration checks
- Commands resolve from `~/.claude/commands`; no cross-tool paths (OpenCode, etc.).
- LeIndex MCP entry exists in `~/.claude/.mcp.json` under `mcpServers.leindex` with `command: maestro`, `args: ["mcp", "tool-search"], type: "stdio"`.
- Rust TUI is the primary UI; Go or Python TUIs are deprecated.

## When to activate this skill
- Anytime a Maestro command is requested in Claude Code.
- When orchestrating tracks, running the cockpit/orchestrator panel, or performing LeIndex-backed analysis.
- When users mention “Maestro framework” or “maestro workflow” inside Claude Code.
