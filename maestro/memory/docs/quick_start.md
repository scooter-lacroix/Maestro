# Maestro Memory Quick Start

Use this to validate the integrated Nexus memory flow in under 5 minutes.

## Prerequisites

- Maestro CLI is installed and `maestro` is on `PATH`.
- You can run local commands in a writable user environment.
- Default database path is writable: `~/.maestro/maestro.db`.
- Port `18765` is available for the dashboard server.

## First Run (Current CLI Surface)

```bash
# 1) Confirm available memory commands
maestro memory --help

# 2) Check current memory system status
maestro memory status

# 3) Scan one or more project paths
maestro memory scan . --depth 3

# 4) Store a test memory
maestro memory store \
  --content "quick-start validation memory" \
  --category observation \
  --importance normal

# 5) Start dashboard server (keep this terminal open)
maestro memory serve
```

Open `http://127.0.0.1:18765` after `serve` starts.

## 5-Minute Validation Flow

1. Run `maestro memory status` and confirm it prints database + totals.
2. Run `maestro memory scan . --depth 2` and confirm at least one project/track is discovered (or zero with no scan errors).
3. Run `maestro memory store ...` and confirm command returns success.
4. Run `maestro memory status` again and confirm memory count increased by 1.
5. Run `maestro memory serve` and load the dashboard URL successfully.

## Common Failures and Immediate Fixes

- **`Database not found` from `status`**
  - Run `maestro memory status` (or `maestro memory store ...`) once to initialize the local DB/schema path.
  - Verify you are running under the intended user account (`~/.maestro/maestro.db`).

- **`Address already in use` on `serve`**
  - Free port `18765` or start with another port: `maestro memory serve --port 18766`.

- **`scan` returns nothing useful**
  - Pass explicit paths and a larger depth: `maestro memory scan /path/to/repos --depth 5`.

- **`store` fails due to invalid arguments**
  - Use supported category/importance values (for example: `observation`, `decision`, `normal`, `high`).

## Next References

- Integration details: `./memory_integration.md`
- Memori deprecation/migration: `./memori_deprecation_notice.md`
- Memori ↔ Nexus parity: `./memori_nexus_feature_parity.md`
- Memori → Nexus mapping: `./memori_to_nexus_mapping.md`
