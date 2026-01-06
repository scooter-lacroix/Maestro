---
description: Migrate Agent Deck configuration to Maestro
argument-hint: [--dry-run]
allowed-tools:
  - Bash
model: haiku
---

## Maestro Agent Deck Migration Command

Migrates Agent Deck configuration and profiles to Maestro format.

## When to Use

User runs: `maestro migrate:agent-deck` or `/maestro:migrate:agent-deck`

## Protocol

1. Check if Agent Deck config exists:
   ```bash
   ls ~/.agent-deck/config.json
   ```

2. If not found:
   ```
   No Agent Deck configuration found.
   Expected location: ~/.agent-deck/

   If you have Agent Deck installed elsewhere, please manually copy your config.
   ```

3. If found, run migration:
   ```bash
   python maestro/memory/migrations/agent_deck_migration.py
   ```

4. For dry run (no changes):
   ```bash
   python maestro/memory/migrations/agent_deck_migration.py --dry-run
   ```

## Migration Steps

The migration will:

1. **Create backup**: `~/.agent-deck.backup.YYYYMMDD_HHMMSS/`
2. **Migrate config**: `~/.agent-deck/config.json` → `~/.maestro/config.json`
3. **Migrate profiles**: `~/.agent-deck/profiles/` → `~/.maestro/profiles/`
4. **Verify**: Check migrated config is valid

## What Gets Migrated

- Session configurations
- Profile settings
- MCP server definitions
- Group definitions
- Tool configurations

## After Migration

1. Test Maestro TUI:
   ```bash
   maestro tui
   ```

2. Verify sessions appear correctly

3. If satisfied, uninstall Agent Deck:
   ```bash
   rm -rf ~/.agent-deck
   ```

4. Keep backup for a few days before deleting
