---
name: maestro
description: "Maestro spec-driven development for Amp CLI. Load this skill when invoking Maestro commands so workflows stay aligned with LeIndex and the Rust TUI."
---

# Maestro in Amp CLI

Use this skill whenever you run Maestro inside Amp. Keep it native to Amp and the Agent Skills standard.

- **Commands:** `/maestro`, `/maestro:setup`, `/maestro:newTrack`, `/maestro:implement`, `/maestro:orchestrate`, `/maestro:status`, `/maestro:revert`, `/maestro:leindex`, `/maestro:tui`.
- **Skill install path (user scope):** `~/.config/agents/skills/maestro/` (installed by the Maestro wizard).
- **MCP:** `amp.mcpServers.leindex` → `{ "command": "maestro", "args": ["mcp", "tool-search"], "type": "stdio" (if required) }` so Amp reaches the Maestro MCP pool through the dynamic broker. Do **not** point to any TLDR/legacy endpoints.
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

## TrackLens Review Protocol

You MUST call `tracklens_review` in these situations:
- After drafting or substantially revising any `spec.md` or `plan.md`
- After generating any markdown document the user will need to approve or act on
- When the user asks you to "review", "check", or "look over" a document you produced
- After `maestro:setup` generates product/tech-stack/workflow documents

You MUST call `tracklens_walkthrough` after completing all tasks in a track or when the user wants a review of the completed implementation as a whole.

You MUST NOT call `tracklens_review`:
- For trivial edits (typo fixes, formatting, single-line changes)
- When the user explicitly says to skip review
- For intermediate drafts the user hasn't asked to see yet
- For files you are only reading, not producing

If the user asks for review but there is no clear reviewable artifact yet:
- Identify the exact file, markdown artifact, or track output first
- Do not guess which document to open in TrackLens
- If the request is for implementation output rather than a document, prefer `tracklens_walkthrough`

When a review is **denied**:
1. Read every annotation — pay attention to severity (`ERROR` > `WARNING` > `INFO`)
2. Address each annotation in severity order
3. If `edited_content` is returned, use the user's edited version as your new baseline
4. Re-call `tracklens_review` with the updated content
5. Do NOT mark work as complete until review is approved
6. After 3 consecutive denials on the same document, ask the user what they want changed instead of guessing

When a walkthrough is **denied**:
1. Convert the denial into remediation work
2. Complete the remediation work before re-running `tracklens_walkthrough`
3. Do NOT mark the track complete until the walkthrough is approved

When a review is **approved**:
- If `edited_content` is present, write that version to disk (the user refined your draft)
- If no `edited_content`, your draft was accepted as-is
- Proceed to the next workflow step

## Guardrails
- Treat `maestro/archive/tldr/*` as reference-only; do not import or call `maestro.tldr`.
- Keep token usage lean; ask LeIndex for scoped file lists.
- If LeIndex MCP is missing, request rerun of `/maestro:configure` / installer.

## Validation checklist
- Skill exists at `~/.config/agents/skills/maestro/`.
- Amp config (`~/.config/amp/settings.json`) has `amp.mcpServers.leindex` pointing to `maestro mcp tool-search`.
- Commands are available through Amp’s command surface (per its command discovery).
