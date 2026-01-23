# Maestro for Codex CLI

This directory contains the **Codex CLI** integration artifacts for Maestro.

## How Codex CLI integrates

Codex CLI supports **custom prompts** stored under `$CODEX_HOME/prompts` (typically `~/.codex/prompts/`). Each Markdown file becomes a slash-command-like entry invoked as:

- `/prompts:<file-stem>`

Codex also supports MCP servers in `~/.codex/config.toml` under `[mcp_servers.<name>]`.

## What Maestro installs for Codex

- **Custom prompts**: `${CODEX_HOME:-~/.codex}/prompts/*.md`
  - Example: `/prompts:maestro_setup`
- **Canonical Maestro command protocols**: `~/.maestro/integrations/commands/*.md` (or the install path chosen in the Conductor Wizard)
  - Codex prompts instruct the model to read these files at runtime.
- **LeIndex MCP server config**: `${CODEX_HOME:-~/.codex}/config.toml` (`[mcp_servers.leindex]`)

## Files

- `codex-cli/prompts/` – the Codex custom prompt files (small “router” prompts that load the installed Maestro command protocols)
