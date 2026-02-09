# Analysis Tab UI Enhancement Design

## Overview

This document describes the enhanced Analysis tab UI that provides guided actions for the LeIndex 5-phase analysis system.

## Current State

The Analysis tab currently provides:
- Command input field (freeform text input)
- History view showing previous commands
- Status bar
- Basic examples in the help text

## Target State

The enhanced Analysis tab will provide:

1. **Quick Action Buttons** - Pre-configured workflow buttons
2. **Phase Buttons** - Individual phase execution buttons
3. **Enhanced History** - Persisted analysis history with bounded storage
4. **Context Bundle Export** - Export functionality for conductor loops

## UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│  🚀 Analysis Command Hub                    Mode: BALANCED   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  QUICK WORKFLOWS:                                           │
│  [F] Fast Orientation    [I] Implementation-Ready           │
│                                                             │
│  INDIVIDUAL PHASES:                                         │
│  [1] Structural Scan   [2] Dependencies   [3] Logic Flow  │
│  [4] Critical Path    [5] Optimization   [B] Bundle        │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Analysis History                                     │   │
│  │                                                      │   │
│  │ [2026-01-23 14:32] Phase 1: Structural Scan         │   │
│  │   → Found 142 functions, 8 classes                  │   │
│  │                                                      │   │
│  │ [2026-01-23 14:30] Fast Orientation (ultra)         │   │
│  │   → Scanned 20 files, 2.1K tokens                   │   │
│  │                                                      │   │
│  │ Type '/phase1 <path>' to begin                       │   │
│  │ (Press 'a' for command input, 'q' to quit)          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  STATUS: Idle (Press 1-5, F, I, B for quick actions)      │
│  Path: [./]                                                  │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Add Analysis Mode State

Add to `App` struct in `app.rs`:
```rust
// Analysis mode state
analysis_mode: AnalysisMode,
analysis_path: String,
analysis_selected_action: usize, // For quick action menu
```

Add enum:
```rust
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum AnalysisMode {
    #[default]
    Ultra,      // Fast orientation
    Balanced,   // Implementation-ready
}
```

### Phase 2: Update Analysis Tab Rendering

Update `render_analysis` in `tabs/analysis.rs` to show:
1. Mode indicator (Ultra/Balanced) in header
2. Quick action buttons (F, I)
3. Phase buttons (1-5, B for bundle)
4. Enhanced history display

### Phase 3: Add Key Bindings

Add to key handler in `app.rs`:
```rust
// Analysis tab key bindings
KeyCode::Char('1') => execute_phase1(),
KeyCode::Char('2') => execute_phase2(),
KeyCode::Char('3') => execute_phase3(),
KeyCode::Char('4') => execute_phase4(),
KeyCode::Char('5') => execute_phase5(),
KeyCode::Char('F') => run_fast_orientation(),
KeyCode::Char('I') => run_implementation_ready(),
KeyCode::Char('B') => generate_context_bundle(),
KeyCode::Char('M') => toggle_analysis_mode(),
```

### Phase 4: Implement Analysis Execution

Add methods to `App`:
```rust
async fn execute_analysis_phase(&mut self, phase: usize) -> Result<()> {
    let path = PathBuf::from(&self.analysis_path);
    let opts = PhaseOptions::new(path);

    let result = match phase {
        1 => phase1_structural_scan(&opts)?,
        2 => phase2_dependency_map(&opts)?,
        3 => phase3_logic_flow(&opts)?,
        4 => phase4_critical_path(&opts)?,
        5 => phase5_optimization_report(&opts)?,
        _ => return Err(anyhow!("Invalid phase")),
    };

    // Add to history
    self.analysis_history.push(result);

    // Enforce bounded storage (max 20 entries)
    if self.analysis_history.len() > 20 {
        self.analysis_history.remove(0);
    }

    Ok(())
}
```

## Quick Action Commands

### Fast Orientation (F)
```bash
/phase1 . --mode ultra --files 20
```

### Implementation-Ready (I)
```bash
/phase1 . --mode balanced --files 50
/phase2 . --mode balanced
/phase3 . --mode balanced --focus-files 5
/phase4 . --mode balanced --top 20
/phase5 . --mode balanced
```

### Context Bundle (B)
Runs all 5 phases and formats as JSON bundle for conductor.

## Bounded Storage

Analysis history will be limited to:
- Maximum 20 entries
- Oldest entries removed when limit exceeded
- Stored in memory (not persisted across sessions)

## Export Format

Context bundle for conductor:
```json
{
  "timestamp": "2026-01-23T14:32:00Z",
  "path": "./",
  "mode": "balanced",
  "phases": {
    "phase1": "...",
    "phase2": "...",
    "phase3": "...",
    "phase4": "...",
    "phase5": "..."
  },
  "token_count": 27000
}
```
