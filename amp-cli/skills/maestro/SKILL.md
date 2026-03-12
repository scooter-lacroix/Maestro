---
name: maestro
description: "Maestro spec-driven development for Amp CLI. Load this skill when invoking Maestro commands so workflows stay aligned with LeIndex and the Rust TUI."
---

# Maestro in Amp CLI

Use this skill whenever you run Maestro inside Amp. Keep it native to Amp and the Agent Skills standard.

- **Commands:** `/maestro`, `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`, `/maestro:orchestrate`, `/maestro:status`, `/maestro:revert`, `/maestro:leindex`, `/maestro:tui`.
- **Skill install path (user scope):** `~/.config/agents/skills/maestro/` (installed by the Maestro wizard).
- **MCP:** `amp.mcpServers.leindex` → `{ "command": "maestro", "args": ["mcp", "proxy", "leindex"], "type": "stdio" (if required) }` so Amp reaches the Maestro MCP pool. Do **not** point to any TLDR/legacy endpoints.
- **No cross-tool bleed:** Avoid references to `~/.claude`, `~/.gemini`, or OpenCode paths.

## Quick flow
1) `/maestro setup` — initialize/refresh product, tech stack, workflow, track registry.
2) `/maestro newTrack "<goal>"` — generate spec + plan with clarifying questions.
3) `/maestro implement <track>` — execute plan with LeIndex 5-phase analysis for scoped file access.
4) `/maestro orchestrate` — cockpit/orchestrator panel (Rust TUI pane).
5) `/maestro status` — report progress and blockers.
6) `/maestro revert [track|phase|task]` — controlled rollback.
7) `/maestro leindex` — analysis via LeIndex; no TLDR imports.
8) `/maestro tui` — launch the Rust cockpit (primary TUI; Go/Python TUIs are deprecated).

## Guardrails
- Treat `maestro/archive/tldr/*` as reference-only; do not import or call `maestro.tldr`.
- Keep token usage lean; ask LeIndex for scoped file lists.
- If LeIndex MCP is missing, request rerun of `/maestro:configure` / installer.

## Validation checklist
- Skill exists at `~/.config/agents/skills/maestro/`.
- Amp config (`~/.config/amp/settings.json`) has `amp.mcpServers.leindex` pointing to `maestro mcp proxy leindex`.
- Commands are available through Amp’s command surface (per its command discovery).
