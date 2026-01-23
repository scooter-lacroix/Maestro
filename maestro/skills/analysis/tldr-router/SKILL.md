# LeIndex Smart Router

Maps questions to the optimal LeIndex command. Use this to pick the right layer.

## Question → Command Mapping

### "What files/functions exist?"
```bash
leindex tree . --ext .py          # File overview
leindex structure src/ --lang python  # Function/class overview
```
**Use:** Starting exploration, orientation

### "What does X call / who calls X?"
```bash
leindex context <function> --project . --depth 2
leindex calls src/
```
**Use:** Understanding architecture, finding entry points

### "How complex is X?"
```bash
leindex cfg <file> <function>
```
**Use:** Identifying refactoring candidates, understanding difficulty

### "Where does variable Y come from?"
```bash
leindex dfg <file> <function>
```
**Use:** Debugging, understanding data flow

### "What affects line Z?"
```bash
leindex slice <file> <function> <line>
```
**Use:** Impact analysis, safe refactoring

### "Search for pattern P"
```bash
leindex search "pattern" src/
```
**Use:** Finding code, structural search

### "Natural language search"
```bash
leindex semantic "authentication flow"
```
**Use:** Semantic code search using embeddings

## Decision Tree

```
START
  │
  ├─► "What exists?" ──► tree / structure
  │
  ├─► "How does X connect?" ──► context / calls
  │
  ├─► "Why is X complex?" ──► cfg
  │
  ├─► "Where does Y flow?" ──► dfg
  │
  ├─► "What depends on Z?" ──► slice
  │
  ├─► "Find something" ──► search
  │
  └─► "Describe X in plain English" ──► semantic
```

## Intent Detection Keywords

| Intent | Keywords | Layer |
|--------|----------|-------|
| Navigation | "what", "where", "find", "exists" | tree, structure, search |
| Architecture | "calls", "uses", "connects", "depends" | context, calls |
| Complexity | "complex", "refactor", "branches", "paths" | cfg |
| Data Flow | "variable", "value", "assigned", "comes from" | dfg |
| Impact | "affects", "changes", "slice", "dependencies" | slice/pdg |
| Debug | "bug", "error", "investigate", "broken" | cfg + dfg + context |
| Semantic | "describe", "what does", "how works" | semantic search |

## Python API

```python
from maestro.leindex import (
    get_relevant_context,
    semantic_search,
    ContextExtractor,
)

# Get context for an entry point
context = get_relevant_context("/path/to/project", "main")

# Semantic search
results = semantic_search("how does authentication work?", "/path/to/project")

# Full analysis with token savings
extractor = ContextExtractor()
result = extractor.extract_for_file("src/api.py")
print(f"Token savings: {result.savings_percent:.1f}%")
```

## Automatic Hook Integration

The `leindex-context` and `leindex-read` hooks automatically:
1. Detect intent from your messages
2. Route to appropriate layers
3. Inject context into tool calls

You don't need to manually run these commands - the hooks do it for you.

## Manual Override

If you need a specific layer the hooks didn't provide:

```python
from maestro.leindex import (
    CFGAnalyzer,
    DFGAnalyzer,
    SlicingAnalyzer,
)

cfg_analyzer = CFGAnalyzer()
dfg_analyzer = DFGAnalyzer()
slicing_analyzer = SlicingAnalyzer()

# Analyze specific function
cfg = cfg_analyzer.analyze(source, "function_name", "file.py")
dfg = dfg_analyzer.analyze_function(source, "function_name", "file.py")
slice_result = slicing_analyzer.slice_backward(source, "function_name", 42, "file.py")
```
