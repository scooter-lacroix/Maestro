# Maestro for AMP CLI

Maestro integrates with **AMP CLI (Sourcegraph)** via MCP configuration so AMP can call Maestro/LeIndex tooling.

This repository’s installer (`install.sh`) is the single entrypoint and wires AMP by updating:

- `~/.config/amp/settings.json` (key: `amp.mcpServers`)

If AMP adds first-class custom command packs in a future release, this directory will host the command assets.

