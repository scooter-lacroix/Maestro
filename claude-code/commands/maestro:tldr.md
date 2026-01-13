---
description: Access Maestro's 5-layer code analysis system (TLDR) for intelligent code understanding, context extraction, and semantic search.
---

# Maestro TLDR - 5-Layer Code Analysis

Access Maestro's powerful **TLDR (Too Long; Didn't Read)** code analysis system - a sophisticated 5-layer analysis framework that provides intelligent code understanding with up to 95% token reduction.

## Overview

TLDR analyzes your codebase at multiple layers of abstraction, providing concise, LLM-ready context instead of raw code dumps.

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 5: Program Dependence  → "What affects line 42?"      │
│ Layer 4: Data Flow           → "Where does this value go?"  │
│ Layer 3: Control Flow        → "How complex is this?"       │
│ Layer 2: Call Graph          → "Who calls this function?"   │
│ Layer 1: AST                 → "What functions exist?"      │
└─────────────────────────────────────────────────────────────┘
```

## Usage

```bash
/maestro:tldr <command> [options]
```

## Commands

### Project Analysis

#### `tree [path]`
Display project structure with key files.

**Example:**
```bash
/maestro:tldr tree src/
```

#### `structure [file]`
Show code structure (functions, classes, imports).

**Example:**
```bash
/maestro:tldr structure src/auth.py
```

**Output:**
- Function signatures
- Class definitions
- Import statements
- Decorators and metadata

### Layer 1: AST Analysis

#### `ast <file>`
Extract abstract syntax tree - functions, classes, imports.

**Use when:** You need to understand what's in a file without reading it.

**Example:**
```bash
/maestro:tldr ast src/models.py
```

**Typical savings:** 500 tokens vs 10,000+ raw file

### Layer 2: Call Graph

#### `callgraph <file>` or `callers <function>` or `callees <function>`
Analyze function call relationships.

**Use when:** You need to understand who calls what.

**Examples:**
```bash
# All call relationships in a file
/maestro:tldr callgraph src/services/payment.py

# Who calls a specific function
/maestro:tldr callers process_payment

# What a function calls
/maestro:tldr callees process_payment
```

**Typical savings:** 440 tokens vs thousands of lines

### Layer 3: Control Flow

#### `cfg <file>` or `complexity <file>`
Analyze control flow and complexity.

**Use when:** You need to understand code complexity and decision points.

**Example:**
```bash
/maestro:tldr cfg src/utils/validation.py
```

**Output:**
- Cyclomatic complexity
- Decision points
- Loop structures
- Nesting depth

**Typical savings:** 110 tokens vs complex code

### Layer 4: Data Flow

#### `dfg <file>` or `dataflow <file>`
Track variable definitions and uses.

**Use when:** You need to understand where data comes from and where it goes.

**Example:**
```bash
/maestro:tldr dfg src/api/endpoints.py
```

**Typical savings:** 130 tokens vs tracing manually

### Layer 5: Program Slicing

#### `slice <file> <line>` or `impact <function>`
Analyze program dependencies and impact.

**Use when:** You need to understand what affects a line or who calls a function.

**Examples:**
```bash
# What affects line 42?
/maestro:tldr slice src/auth.py 42

# Who calls this function? (backward slice)
/maestro:tldr impact validate_token

# What does this function affect? (forward slice)
/maestro:tldr slice-forward validate_token
```

**Typical savings:** 150 tokens vs manual analysis

### Search & Context

#### `search <query>`
Search code by content with semantic understanding.

**Example:**
```bash
/maestro:tldr search "database connection pooling"
```

#### `context <target> [project_path]`
Generate LLM-ready context for a file or function.

**Use when:** You want to provide Claude with optimal context about code.

**Examples:**
```bash
# Context for a main file
/maestro:tldr context main.py

# Context for a specific function
/maestro:tldr context authenticate_user src/auth.py

# Context for entire project
/maestro:tldr context . --project
```

**Output format:** Optimized for LLM consumption with:
- Function signatures
- Call relationships
- Data flow
- Complexity metrics

### Warm/Index

#### `warm [path]`
Index project for fast analysis.

**Example:**
```bash
/maestro:tldr warm .
```

## Automatic Hook Integration

TLDR features **run automatically** via Maestro's hooks:

1. **tldr-read hook**: When you read a file, TLDR context is automatically available
2. **tldr-context hook**: Before editing code, relevant context is injected
3. **smart-search hook**: Code searches use semantic understanding

You don't need to manually invoke TLDR for most operations - it works behind the scenes to provide Claude with optimal context.

## Examples from llm-tldr

The original llm-tldr commands map to Maestro TLDR as follows:

| llm-tldr Command | Maestro TLDR Equivalent |
|------------------|-------------------------|
| `tldr warm .` | `/maestro:tldr warm .` |
| `tldr context main --project .` | `/maestro:tldr context main.py` |
| `tldr context authenticate --project .` | `/maestro:tldr context authenticate src/auth.py` |
| `tldr impact helper_func` | `/maestro:tldr impact helper_func` |
| `tldr semantic "database connection"` | `/maestro:tldr search "database connection"` |

## Python API

You can also use TLDR directly in Python:

```python
from maestro.tldr import (
    TLRDAnalyzer,
    get_relevant_context,
    ASTAnalyzer,
    CallGraphAnalyzer,
)

# Analyze a file
analyzer = TLRDAnalyzer("src/auth.py")
result = analyzer.analyze(layers=["ast", "callgraph", "cfg"])

# Get context for LLM
context = get_relevant_context("authenticate_user", "src/auth.py")
```

## When to Use Each Layer

| Your Question | Use This Command |
|---------------|------------------|
| "What functions exist in this file?" | `/maestro:tldr ast <file>` |
| "Who calls this function?" | `/maestro:tldr callers <func>` or `/maestro:tldr impact <func>` |
| "What does this function call?" | `/maestro:tldr callees <func>` |
| "How complex is this code?" | `/maestro:tldr cfg <file>` or `/maestro:tldr complexity <file>` |
| "Where does this value go?" | `/maestro:tldr dfg <file>` |
| "What affects this line?" | `/maestro:tldr slice <file> <line>` |
| "Search for behavior" | `/maestro:tldr search "<query>"` |
| "Give Claude optimal context" | `/maestro:tldr context <target>` |

## Related Commands

- `/maestro:leindex` - Full-text and semantic code search via LeIndex
- `/maestro:configure` - Configure Maestro (including TLDR hooks)

## See Also

- [TLDR Overview](https://github.com/parcadei/llm-tldr) - Original llm-tldr project
- [LeIndex Documentation](/maestro:leindex) - Enhanced indexing and search
