# Maestro for Droid CLI (Factory)

Maestro integrates with **Droid CLI (Factory)** via MCP configuration so Droid can call Maestro/LeIndex tooling.

This repository’s installer (`install.sh`) is the single entrypoint and wires Droid by updating:

- `~/.factory/mcp.json` (key: `mcpServers`, requires `type: "stdio"` for stdio servers)

