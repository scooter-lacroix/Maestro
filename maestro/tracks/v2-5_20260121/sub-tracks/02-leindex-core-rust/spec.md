# Sub-Track 02: LeIndex Core (Rust) = TLDR (No Python TLDR) - Specification

## Objective

Ensure TLDR functionality is fully absorbed into LeIndex and implemented in Rust, with LeIndex being the only active code-analysis system used by Maestro workflows, hooks, and Cockpit.

## Requirements

### R1: Eliminate `maestro.tldr` Usage

- No runtime code imports `maestro.tldr`.
- No runtime code loads anything from `maestro/archive/tldr`.
- Any legacy TLDR UX entrypoints are either removed or implemented as aliases that route to LeIndex.

### R2: LeIndex Provides TLDR Equivalents (Rust)

LeIndex must expose (at minimum) the TLDR-equivalent capabilities:

- AST/structure extraction
- call graph / callers / callees / entrypoint inference
- control-flow and complexity summary
- data-flow summary
- slicing/impact analysis
- context packing for LLM usage (ultra/balanced/verbose)

### R3: Hook + Skill Integration

- Hooks must inject LeIndex context (and only LeIndex context).
- Skills and command docs must refer to LeIndex CLI/API, not TLDR.

## Acceptance Criteria

- Repo-wide search shows no non-archive `maestro.tldr` references.
- The Cockpit analysis UI can perform the 5 layers without Python TLDR.
- Slash command docs and skills provide correct, executable commands.

