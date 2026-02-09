# LeIndex Analysis Workflows

## Overview

This document defines the two primary analysis workflows for the LeIndex 5-phase analysis system integrated into the Maestro Cockpit TUI.

---

## Workflow 1: Fast Orientation (Exploration Mode)

**Purpose:** Quick structural scan for code exploration and understanding.

**Use Cases:**
- Understanding a new codebase quickly
- Finding entry points and main files
- Identifying project structure
- Getting oriented before deep analysis

**Characteristics:**
- Ultra mode for maximum token savings (98% compression)
- Focus on high-level structure only
- Fast execution (< 5 seconds for most projects)
- Output is "exploration only" - not suitable for implementation

### Phase 1: Structural Scan (Ultra)

**Command:** `/phase1 <path> --mode ultra --files 20`

**Output:**
- Top 20 files by priority
- Basic AST info (functions, classes, imports)
- File hierarchy
- Entry point identification

**Example Output:**
```
## Phase 1: Structural Scan
=== Top 20 Files ===
1. src/main.rs (100) - Entry point
2. src/lib.rs (80) - Library root
...
=== Structure ===
- Functions: 142
- Classes: 8
- Entry Points: main()
```

**Token Budget:** ~2K tokens

---

## Workflow 2: Implementation-Ready (Balanced Mode)

**Purpose:** Full 5-phase analysis with balanced mode for generating implementation-ready context bundles.

**Use Cases:**
- Conductor loop context generation
- Pre-implementation analysis
- Code modification planning
- Refactoring preparation

**Characteristics:**
- Balanced mode for implementation-ready output (82% savings, LLM actionable)
- All 5 phases executed
- Includes function signatures, types, dependencies
- Output is suitable for code generation

### Phase 1: Structural Scan (Balanced)

**Command:** `/phase1 <path> --mode balanced --files 50`

**Output:**
- Top 50 files (expanded coverage)
- Complete function signatures with types
- Import statements
- Module structure

**Token Budget:** ~8K tokens

### Phase 2: Dependency Map

**Command:** `/phase2 <path> --mode balanced`

**Output:**
- Call graph analysis
- Function callers/callees
- Module dependencies
- Import/require relationships

**Token Budget:** ~6K tokens

### Phase 3: Logic Flow (CFG)

**Command:** `/phase3 <path> --mode balanced --focus-files 5`

**Output:**
- Control flow complexity metrics
- Cyclomatic complexity per function
- Branch analysis
- Loop structures

**Token Budget:** ~4K tokens

### Phase 4: Critical Path (DFG)

**Command:** `/phase4 <path> --mode balanced --top 20`

**Output:**
- Data flow analysis
- Critical data dependencies
- Variable usage tracking
- Side effect identification

**Token Budget:** ~5K tokens

### Phase 5: Optimization Report (Slicing)

**Command:** `/phase5 <path> --mode balanced`

**Output:**
- Program slicing results
- Minimal relevant code subsets
- Impact analysis
- Optimization opportunities

**Token Budget:** ~4K tokens

### Context Bundle Summary

**Total Token Budget:** ~27K tokens for all 5 phases

**Components:**
1. Function signatures with types
2. Key dependencies (callers/callees)
3. Control complexity indicators
4. Data flow hints
5. Optimization recommendations

**Usage in Conductor Loop:**
```python
context = {
    "phase1": phase1_structural_scan(opts),
    "phase2": phase2_dependency_map(opts),
    "phase3": phase3_logic_flow(opts),
    "phase4": phase4_critical_path(opts),
    "phase5": phase5_optimization_report(opts),
}
```

---

## Quick Reference Commands

### Fast Orientation
```bash
/phase1 . --mode ultra --files 20
```

### Implementation-Ready (All Phases)
```bash
/phase1 . --mode balanced --files 50
/phase2 . --mode balanced
/phase3 . --mode balanced --focus-files 5
/phase4 . --mode balanced --top 20
/phase5 . --mode balanced
```

### Context Bundle (Conductor Loop)
```bash
/bundle . --mode balanced
```
*(Note: This shortcut runs all 5 phases and formats as a context bundle)*

---

## Mode Comparison

| Feature | Ultra Mode | Balanced Mode |
|---------|-----------|---------------|
| Purpose | Exploration | Implementation |
| Token Savings | 98% | 82% |
| Function Signatures | Names only | Full with types |
| Dependencies | High-level | Complete with types |
| Control Flow | None | Full CFG |
| Data Flow | None | Full DFG |
| Suitability | Exploration only | Code generation |

---

## Integration with Cockpit TUI

The Analysis tab in Cockpit provides:

1. **Quick Actions** - Pre-configured buttons for common workflows
2. **Phase Buttons** - Individual phase execution buttons
3. **History** - Analysis result history with bounded storage
4. **Export** - Context bundle export for conductor loops

### UI Layout

```
┌─────────────────────────────────────────────┐
│  🚀 Analysis Command Hub                    │
│                                             │
│  [Quick Orientation] [Implementation Ready] │
│  [Phase 1] [Phase 2] [Phase 3]             │
│  [Phase 4] [Phase 5] [Context Bundle]       │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │ History View                        │   │
│  │ ...                                 │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  STATUS: Idle                               │
│  Command > [_]                               │
└─────────────────────────────────────────────┘
```
