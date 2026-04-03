---
name: remember
description: "Store a learning, pattern, or decision in the persistent memory system with auto-type detection and BGE embeddings. Use when the user wants to save a solution, record a decision, store a pattern, remember what worked or failed, or persist knowledge for future sessions."
user-invocable: true
arguments: "[--type TYPE] <content>"
---

# Remember

Store a learning in the PostgreSQL-backed memory system with automatic type detection and semantic embeddings for future recall.

## Workflow

1. Parse the user's input for an optional `--type` flag and the learning content
2. If no type specified, auto-detect from content keywords (see Learning Types)
3. Store the learning with embeddings, tags, and session metadata
4. Return confirmation with the stored learning ID

## Execution

Run the following command, replacing `<TYPE>` and `<CONTENT>` with the appropriate values:

```bash
cd $CLAUDE_PROJECT_DIR/opc && PYTHONPATH=. uv run python scripts/store_learning.py \
  --session-id "manual-$(date +%Y%m%d-%H%M)" \
  --type <TYPE> \
  --content "<CONTENT>" \
  --context "manual entry via /remember" \
  --confidence medium
```

## Learning Types

| Type | Use For | Auto-detect Keywords |
|------|---------|---------------------|
| `WORKING_SOLUTION` | Fixes, solutions that worked (default) | — |
| `ARCHITECTURAL_DECISION` | Design choices, system structure | "decided", "chose", "architecture" |
| `CODEBASE_PATTERN` | Patterns discovered in code | "pattern", "always", "convention" |
| `FAILED_APPROACH` | What didn't work | "failed", "didn't work", "don't" |
| `ERROR_FIX` | Specific error resolutions | "error", "fix", "bug" |

## Examples

```
/remember TypeScript hooks require npm install before they work
/remember --type ARCHITECTURAL_DECISION Session affinity uses terminal PID
/remember --type FAILED_APPROACH Don't use subshell for store_learning command
```
