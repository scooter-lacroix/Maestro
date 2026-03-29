# Provider Boundary Verification

This document is the verification matrix for the Maestro provider-boundary migration.

## Purpose

The goal is not just to confirm that binaries exist. The goal is to prove that:

- Maestro installs the full runtime surface
- Standalone LeIndex is the authoritative analysis provider for Maestro-managed sessions
- Standalone Nexus is the authoritative memory and cognition provider for Maestro-managed sessions
- The Maestro MCP pool remains intact as shared infrastructure for non-overlapping servers
- Compatibility fallbacks are loud, explicit, and not on the normal path

## Automated Verification Matrix

| Surface | Command | Expected result | Covers |
|---------|---------|-----------------|--------|
| Maestro runtime | `maestro --help` | CLI starts and shows top-level commands | T11.1, T11.4 |
| Maestro MCP pool | `maestro mcp --help` | Exposes `serve`, `proxy`, and `tool-search` | T11.1, T11.3, T12.4 |
| Standalone LeIndex | `leindex --version` | Binary resolves and reports a version | T11.1, T11.3 |
| LeIndex command surface | `leindex --help` | Includes `index`, `search`, `analyze`, `phase`, and `mcp` | T11.1, T11.4, T12.1 |
| LeIndex MCP surface | `leindex mcp --help` | Exposes `tool-search` for direct provider wiring | T11.1, T11.4 |
| Standalone Nexus | `nexus --version` | Binary resolves and reports a version | T11.1, T11.3 |
| Nexus init | `nexus init --help` | Help text is available and init can be invoked | T11.2, T11.4 |
| Nexus session | `nexus session --help` | Session/runtime surface is available | T11.2, T11.4, T12.1 |
| Installer verification | `bash install.sh` from a local checkout | Completes install-time validation without omitting providers | T11.1, T11.4 |
| Build verification | `cargo check -p leindex-core -p maestro-cockpit -p maestro-claw` | Workspace builds cleanly for the touched surfaces | T12.1, T12.4 |
| Hook syntax verification | `python -m py_compile maestro/hooks/*.py maestro/hooks/*/*.py` | Hook scripts are syntactically valid | T10.2, T12.1 |

## Managed-Session Verification

Use the following checks to confirm the provider boundary is being honored:

| Surface | Expected behavior |
|---------|-------------------|
| MaestroClaw | Active session shows provider metadata, prompt input, runtime output, and no hidden LeIndex/Nexus fallback as the default path |
| Sessions tab | New managed sessions inherit the canonical provider-aware launch profile |
| Conductor | Task execution shows the same provider profile and pool boundary metadata |
| Memory tab | Memory content, storage agent, and access history come from Nexus-backed truth |
| Legacy compatibility hooks | If they activate, they emit `hook_warning` and `legacy_compatibility_path` rather than silently masquerading as the normal path |

## Manual / TUI Checklist

- Start `maestro tui`.
- Open `MaestroClaw` and confirm the active session renders the live agent surface, not placeholder text.
- Open `Sessions` and launch a managed CLI session.
- Open `Conductor` and confirm track expansion, runtime logs, and iteration history work.
- Open `Memory` and expand a memory item to full content.
- Confirm the relation graph can be used to navigate to a memory entry.
- Run `maestro mcp --help` and verify the pool is present.
- Run `leindex --help` and verify standalone analysis commands are present directly.
- Run `nexus init --help` and `nexus session --help` to confirm standalone Nexus is available.
- Open a shell outside Maestro and confirm supported CLIs behave normally without Maestro-managed session injection.

## Outside-Maestro Behavior

Supported CLIs must remain normal when they are launched outside Maestro.

That means:

- No Maestro-managed provider profile should be injected
- No Maestro-only suppression policy should be present
- No `hook_warning` legacy fallback path should appear unless a compatibility module is intentionally used

This is the main guardrail against hidden coupling.
