---
name: maestro
description: "Maestro spec-driven development for Claude Code. Load this skill when invoking Maestro commands so workflows stay aligned with LeIndex and the Rust TUI."
---

# Maestro in Claude Code

Use this skill whenever you run Maestro inside Claude Code. Keep it native to Claude Code and the Agent Skills standard.

- **Commands:** `/maestro`, `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`, `/maestro:orchestrate`, `/maestro:status`, `/maestro:revert`, `/maestro:leindex`, `/maestro:tui`.
- **Skill install path (user scope):** `~/.claude/skills/maestro/` (installed by the Maestro wizard).
- **MCP:** `claude.mcpServers.leindex` → `{ "command": "maestro", "args": ["mcp", "tool-search"], "type": "stdio" (if required) }` so Claude Code reaches the Maestro MCP pool through the dynamic broker. Do **not** point to any TLDR/legacy endpoints.
- **No cross-tool bleed:** Avoid references to `~/.config/gemini`, `~/.amp`, or OpenCode paths.

## Quick flow
1) `/maestro setup` — initialize/refresh product, tech stack, workflow, track registry.
2) `/maestro newTrack "<goal>"` — generate spec + plan with clarifying questions.
3) `/maestro implement <track>` — execute plan with LeIndex 5-phase analysis for scoped file access.
4) `/maestro orchestrate` — cockpit/orchestrator panel (Rust TUI pane).
5) `/maestro status` — report progress and blockers.
