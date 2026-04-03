---
name: recall
description: "Semantic memory retrieval that queries stored learnings from past sessions using PostgreSQL and BGE embeddings. Use when the user wants to search previous learnings, recall past solutions, find what worked before, or retrieve context from earlier sessions."
user-invocable: true
arguments: "<query> [--k N] [--vector-only | --text-only]"
---

# Recall

Query the memory system for relevant learnings from past sessions using semantic search.

## Workflow

1. Parse the user's query and any flags (`--k`, `--vector-only`, `--text-only`)
2. Execute the recall script against the PostgreSQL-backed memory store
3. Present the top results with learning type, confidence, and session context

## Execution

Run the following command, replacing `<QUERY>` with the user's search terms:

```bash
cd $CLAUDE_PROJECT_DIR/opc && PYTHONPATH=. uv run python scripts/recall_learnings.py --query "<QUERY>" --k 5
```

### Options

| Flag | Effect |
|------|--------|
| `--k N` | Return N results instead of the default 5 |
| `--vector-only` | Use pure vector search for higher precision |
| `--text-only` | Use text search only for faster results |

## Output Format

Present results as a numbered list:

```
## Memory Recall: "<query>"

### 1. [TYPE] (confidence: high, id: abc123)
<full content>

### 2. [TYPE] (confidence: medium, id: def456)
<full content>
```

## Examples

```
/recall hook development patterns
/recall TypeScript errors --k 10 --vector-only
```
