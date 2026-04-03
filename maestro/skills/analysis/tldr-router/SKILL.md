---
name: tldr-router
description: "Route user questions to the optimal LeIndex analysis command based on intent detection. Use when the user asks about code structure, function calls, complexity, data flow, dependencies, or needs to search a codebase using LeIndex."
user-invocable: false
---

# TL;DR Router

Map user questions to the correct LeIndex command by detecting intent from keywords and question patterns.

## Workflow

1. Parse the user's question for intent keywords
2. Match intent to the appropriate LeIndex layer using the routing table
3. Execute the matched command or delegate to automatic hooks

## Intent Routing Table

| Intent | Keywords | Command |
|--------|----------|---------|
| Navigation | "what", "where", "find", "exists" | `leindex tree` / `leindex structure` / `leindex search` |
| Architecture | "calls", "uses", "connects", "depends" | `leindex context` / `leindex calls` |
| Complexity | "complex", "refactor", "branches", "paths" | `leindex cfg` |
| Data Flow | "variable", "value", "assigned", "comes from" | `leindex dfg` |
| Impact | "affects", "changes", "slice", "dependencies" | `leindex slice` |
| Debug | "bug", "error", "investigate", "broken" | `leindex cfg` + `dfg` + `context` |
| Semantic | "describe", "what does", "how works" | `leindex semantic` |

## Command Reference

### File and structure overview
```bash
leindex tree . --ext .py
leindex structure src/ --lang python
```

### Call graph and architecture
```bash
leindex context <function> --project . --depth 2
leindex calls src/
```

### Complexity analysis
```bash
leindex cfg <file> <function>
```

### Data flow analysis
```bash
leindex dfg <file> <function>
```

### Impact and slicing
```bash
leindex slice <file> <function> <line>
```

### Code search
```bash
leindex search "pattern" src/
leindex semantic "authentication flow"
```

## Automatic Hook Integration

The `leindex-context` and `leindex-read` hooks automatically detect intent, route to appropriate layers, and inject context into tool calls. Manual commands are only needed when hooks do not provide the specific layer required.

## Python API

```python
from maestro.leindex import get_relevant_context, semantic_search

context = get_relevant_context("/path/to/project", "main")
results = semantic_search("how does authentication work?", "/path/to/project")
```
