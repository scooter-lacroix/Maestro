# Ktop (Krustop) Codebase Analysis

**Generated:** 2025-02-10
**Source:** https://github.com/scooter-lacroix/krustop.git
**Analysis Tool:** LeIndex (5-phase analysis)
**File:** `ktop.py` (1064 lines)

---

## Executive Summary

Ktop is a terminal-based system resource monitor written in Python, designed specifically for tracking resource usage during hybrid LLM workloads. It provides real-time monitoring of CPU, GPU, memory, network, and process statistics with an interactive TUI built on the Rich library.

**Key Characteristics:**
- Single-file architecture (1064 lines)
- Linux-specific (reads `/proc` directly for performance)
- Modular data collection with 5-second caching for expensive operations
- Rich terminal UI with 50 color themes and gradient visualizations
- Optional NVIDIA GPU monitoring via pynvml

---

## Project Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         main()                                   │
│                    (CLI Entry Point)                             │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                         KTop.__init__()                          │
│  • Initialize theme system                                       │
│  • Setup history deques (cpu, net, gpu)                         │
│  • Initialize GPU (optional pynvml)                              │
│  • Cache system constants (page_size, clock_ticks, num_cpus)    │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                ┌────────────────┼────────────────┐
                │                │                │
                ▼                ▼                ▼
        ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
        │ Data Collectors│  │ UI Builders  │  │ Event Handler│
        │              │  │              │  │              │
        │ _sample_cpu  │  │ _cpu_panel   │  │ _read_key    │
        │ _sample_net  │  │ _mem_panel   │  │ _handle_key  │
        │ _gpu_info    │  │ _net_panel   │  │              │
        │ _sample_temps│  │ _gpu_panels  │  │              │
        │ _scan_procs  │  │ _proc_table  │  │              │
        │ _check_oom   │  │ _temp_strip  │  │              │
        │              │  │ _status_bar  │  │              │
        │              │  │ _theme_picker│  │              │
        └──────────────┘  └──────────────┘  └──────────────┘
                │                │                │
                └────────────────┼────────────────┘
                                 ▼
                    ┌──────────────────────┐
                    │     KTop._build()     │
                    │  (Layout Composition) │
                    └──────────┬───────────┘
                               ▼
                    ┌──────────────────────┐
                    │   KTop.run()         │
                    │  • 50ms key polling  │
                    │  • Live.render()     │
                    │  • Signal handling   │
                    └──────────────────────┘
```

---

## Module Structure and Responsibilities

### 1. Data Collection Module

Located at lines 17169-23791 in `ktop.py`

| Method | Responsibility | Cache Interval |
|--------|----------------|----------------|
| `_sample_cpu()` | CPU usage percent via psutil | None (every call) |
| `_sample_net()` | Network I/O bytes/sec calculation | None (delta-based) |
| `_gpu_info()` | GPU utilization and memory via pynvml | None (every call) |
| `_sample_temps()` | CPU/Mem/GPU temps with thresholds | None (every call) |
| `_scan_procs()` | Top 10 processes by CPU/memory | 5 seconds |
| `_check_oom()` | OOM kill detection via journalctl | 5 seconds |

**Key Implementation Details:**

```python
# Process scanning directly reads /proc for performance
def _scan_procs(self) -> None:
    """Scan process list from /proc directly, cached for 5 seconds."""
    now = time.monotonic()
    if now - self._last_proc_scan < 5.0 and self._procs_by_mem:
        return
    # ... direct /proc/{pid}/stat and /proc/{pid}/statm parsing
```

**Network Auto-Scaling:**
- Maintains `net_max_speed` tracking maximum observed speed
- Provides percentage-based visualization regardless of bandwidth

### 2. UI Rendering Module

Located at lines 25761-35805 in `ktop.py`

| Method | Responsibility | Components |
|--------|----------------|------------|
| `_cpu_panel()` | CPU usage panel with sparkline history | Overall %, cores, freq, history |
| `_mem_panel()` | RAM/Swap usage panel | Used/total percentages |
| `_net_panel()` | Network panel with dual sparklines | Upload (up), download (down) |
| `_gpu_panels()` | Per-GPU panels (horizontal layout) | Util%, memory%, history |
| `_proc_table()` | Top 10 processes table | PID, name, used/shared/CPU% |
| `_temp_strip()` | Temperature bar charts | CPU, Mem, per-GPU temps |
| `_status_bar()` | Bottom status bar | Key hints, OOM kill status |
| `_theme_picker()` | Full-screen theme overlay | 3-column grid with preview |

**Dynamic Width Calculations:**
- All panels calculate widths based on `console.width // 3 - 6`
- Ensures responsive layout across terminal sizes
- Minimum width protection: `max(20, ...)`

### 3. Theme System Module

Located at lines 2012-2513 in `ktop.py`

**Theme Structure:**
```python
THEMES: dict[str, dict] = {}
# Each theme dict contains:
{
    "gpu": str,        # GPU panel color
    "cpu": str,        # CPU panel color
    "mem": str,        # Memory panel color
    "proc_mem": str,   # Process memory table color
    "proc_cpu": str,   # Process CPU table color
    "bar_low": str,    # Low threshold color
    "bar_mid": str,    # Mid threshold color
    "bar_high": str,   # High threshold color
    "net": str,        # Network panel color (default: cpu)
    "net_up": str,     # Upload color (default: gpu)
    "net_down": str    # Download color
}
```

**50 Built-in Themes:**
- Classic & Editor: Default, Monokai, Dracula, Nord, Solarized, Gruvbox, One Dark, Tokyo Night
- Monochrome: Monochrome, Green Screen, Amber, Phosphor
- Color: Ocean, Sunset, Forest, Lava, Arctic, Sakura, Mint, Lavender, Coral
- Cyberpunk: Cyberpunk, Neon, Synthwave, Vaporwave, Matrix
- Pastel: Pastel, Soft, Cotton Candy, Ice Cream
- Bold: Electric, Inferno, Glacier, Twilight, Autumn, Spring, Summer, Winter
- High Contrast: High Contrast, Blueprint, Redshift, Emerald, Royal, Bubblegum, Horizon

**Color Utilities:**
- `_color_to_rgb()`: Parse Rich color names/hex to RGB (cached)
- `_lerp_rgb()`: Linear interpolation for gradient bars
- `_color_for()`: Select color based on percentage thresholds
- `_bar()`: Render gradient progress bars

### 4. Event Handling Module

Located at lines 13221-14020 and 43658-45078 in `ktop.py`

**Non-blocking Keyboard Input:**
```python
def _read_key() -> str | None:
    """Non-blocking read of a single keypress. Returns key name or None."""
    fd = sys.stdin.fileno()
    if not select.select([fd], [], [], 0)[0]:
        return None
    # ... parse escape sequences for arrow keys
```

**Key Bindings:**
| Key | Action | Context |
|-----|--------|---------|
| `q`, `Q`, `ESC` | Quit | Normal |
| `t`, `T` | Open theme picker | Normal |
| Arrow keys | Navigate themes | Theme picker |
| `ENTER` | Select theme | Theme picker |
| `ESC` | Cancel | Theme picker |

**Main Loop Timing:**
- 50ms input polling for responsive navigation
- Configurable data refresh interval (default: 1.0s)
- Rich Live display at 4 Hz

---

## Data Flow Diagrams

### CPU Monitoring Flow

```
┌──────────────────┐
│  _cpu_panel()    │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  _sample_cpu()   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐      ┌──────────────────┐
│ psutil.cpu_      │ ───► │ cpu_hist deque   │
│ percent()        │      │ (maxlen=300)     │
└──────────────────┘      └──────────────────┘
                                  │
                                  ▼
                         ┌──────────────────┐
                         │ _sparkline()     │
                         │ (history chart)  │
                         └──────────────────┘
```

### Network Monitoring Flow

```
┌──────────────────┐
│  _net_panel()    │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐      ┌──────────────────────┐
│  _sample_net()   │ ───► │ net_up_hist deque    │
│                  │      │ net_down_hist deque  │
│                  │      │ net_max_speed (auto) │
└────────┬─────────┘      └──────────────────────┘
         │
         ▼
┌──────────────────┐
│ psutil.net_io_   │
│ counters()       │
└──────────────────┘
         │
         ▼ (delta calculation)
┌──────────────────────────────┐
│ bytes = current - last       │
│ rate = bytes / time_delta    │
└──────────────────────────────┘
```

### Process Scanning Flow

```
┌─────────────────────────────────────┐
│  _scan_procs() (cached 5s)          │
└─────────────────┬───────────────────┘
                  │
                  ▼
         ┌────────────────┐
         │ os.listdir(    │
         │ "/proc")       │
         └────────┬───────┘
                  │
                  ▼ (for each pid)
         ┌────────────────┐
         │ read /proc/{id}/│
         │ stat, statm    │
         └────────┬───────┘
                  │
                  ▼
    ┌─────────────────────────────┐
    │ Parse:                      │
    │ - name (between parens)     │
    │ - utime, stime (CPU)        │
    │ - rss_pages (memory)        │
    │ - shared_pages              │
    │ - CPU% = delta / time / ncpus│
    └────────┬────────────────────┘
             │
             ▼
    ┌─────────────────────────────┐
    │ Sort by memory_percent or   │
    │ cpu_percent                 │
    │ Return top 10               │
    └─────────────────────────────┘
```

### GPU Monitoring Flow (Optional)

```
┌─────────────────────────────────────┐
│  _gpu_info() (pynvml required)      │
└─────────────────┬───────────────────┘
                  │
                  ▼ (for each GPU)
    ┌───────────────────────────────┐
    │ nvmlDeviceGetHandleByIndex(i) │
    └────────┬──────────────────────┘
             │
             ▼
    ┌───────────────────────────────┐
    │ nvmlDeviceGetUtilizationRates │
    │ nvmlDeviceGetMemoryInfo       │
    │ nvmlDeviceGetName             │
    └────────┬──────────────────────┘
             │
             ▼
    ┌───────────────────────────────┐
    │ Append to:                    │
    │ - gpu_util_hist[i] deque      │
    │ - gpu_mem_hist[i] deque       │
    └───────────────────────────────┘
```

---

## Key Dependencies and Their Purposes

| Dependency | Version | Purpose | Usage |
|------------|---------|---------|-------|
| `psutil` | - | System and process utilities | CPU%, memory, network, temps, process info |
| `rich` | - | Terminal formatting and TUI | Panels, tables, layouts, live display, colors |
| `pynvml` | Optional | NVIDIA GPU monitoring | GPU utilization, memory, temperature |
| `argparse` | stdlib | CLI argument parsing | refresh rate, theme, simulation mode |
| `signal` | stdlib | Signal handling | SIGINT, SIGTERM cleanup |
| `termios/tty` | stdlib | Terminal control | Non-blocking input, cbreak mode |
| `select` | stdlib | I/O multiplexing | Non-blocking key polling |
| `subprocess` | stdlib | External commands | journalctl for OOM kills |
| `json/pathlib` | stdlib | Config persistence | Theme save/load |

**External Command Dependencies:**
- `journalctl` (Linux): Required for OOM kill tracking
- `/proc` filesystem: Direct process reading (Linux-specific)

---

## API Surface and Interfaces

### Main Class: `KTop`

**Constructor:**
```python
KTop(refresh: float = 1.0, sim: bool = False)
```

**Public Methods:**
```python
def run(self) -> None:
    """Main event loop - non-blocking key polling + Live display."""
```

### Module-Level Functions

```python
def main():
    """CLI entry point with argparse."""
```

### Helper Functions (Module-level)

```python
# Theme definition helper
def _t(name, gpu, cpu, mem, pm, pc, lo, mid, hi, net=None, net_up=None, net_down=None)

# Configuration persistence
def _load_config() -> dict
def _save_config(cfg: dict) -> None

# Color utilities
def _color_to_rgb(name: str) -> tuple[int, int, int]
def _lerp_rgb(c1, c2, t: float) -> str
def _color_for(pct: float, theme: dict | None) -> str

# UI builders
def _bar(pct: float, width: int = 25, theme: dict | None = None) -> str
def _sparkline(values, width: int | None = None) -> str
def _sparkline_down(values, width: int | None = None) -> str

# Formatting
def _fmt_bytes(b: float) -> str
def _fmt_speed(b: float) -> str

# Input
def _read_key() -> str | None
```

---

## State Management Patterns

### 1. Rolling History State

```python
# Fixed-size deques prevent memory growth
self.cpu_hist: deque[float] = deque(maxlen=HISTORY_LEN)  # 300
self.net_up_hist: deque[float] = deque(maxlen=HISTORY_LEN)
self.net_down_hist: deque[float] = deque(maxlen=HISTORY_LEN)
self.gpu_util_hist: dict[int, deque] = {}  # Per-GPU
self.gpu_mem_hist: dict[int, deque] = {}
```

**Pattern:** Bounded circular buffer with automatic overflow

### 2. Process Cache State

```python
# Expensive operation - cached with timestamp
self._procs_by_mem: list[dict] = []
self._procs_by_cpu: list[dict] = []
self._last_proc_scan: float = 0.0
self._proc_cpu_prev: dict[int, int] = {}  # CPU delta calculation
```

**Pattern:** Time-based cache invalidation (5 seconds)

### 3. Network Delta State

```python
# Previous values for delta calculation
self._last_net_sent: int
self._last_net_recv: int
self._last_net_time: float
self.net_max_speed: float = 1.0  # Auto-scaling ceiling
```

**Pattern:** Stateful delta tracking with adaptive scaling

### 4. Theme Picker State

```python
# Modal state
self.picking_theme: bool = False
self.theme_cursor: int = THEME_NAMES.index(self.theme_name)
self.theme_scroll: int = 0
```

**Pattern:** View-model separation with cursor tracking

### 5. Configuration Persistence

```python
CONFIG_DIR = Path.home() / ".config" / "ktop"
CONFIG_FILE = CONFIG_DIR / "config.json"
# Stores: {"theme": "Tokyo Night"}
```

**Pattern:** File-based user preferences

### 6. OOM Kill Tracking

```python
self._last_oom_check: float = 0.0
self._last_oom_str: str | None = None
```

**Pattern:** Cached external command result

---

## Hotspots and Complexity Analysis

From LeIndex Phase 4 analysis:

| Symbol | Complexity | Impact | Score |
|--------|------------|--------|-------|
| `KTop._prof_time` | 4 | 34 | 0.561 |
| `_lerp_rgb` | 4 | 34 | 0.561 |
| `KTop._handle_key` | 3 | 34 | 0.526 |
| `KTop._proc_table` | 3 | 34 | 0.526 |
| `KTop._temp_cell` | 3 | 34 | 0.526 |
| `KTop._top_procs` | 3 | 34 | 0.526 |
| `KTop.__init__` | 2 | 34 | 0.492 |
| `KTop._build` | 2 | 34 | 0.492 |
| `KTop._check_oom` | 2 | 34 | 0.492 |

**Note:** Profiling code (`_prof_time`, `_lerp_rgb`) shows higher complexity due to conditional logic.

---

## Entry Points (Phase 3 Analysis)

1. `main()` - CLI entry with argparse
2. `KTop.__init__` - Initialization
3. `_t()` - Theme definition helper
4. `_lerp_rgb()` - Color gradient utility
5. `KTop._prof_time()` - Profiling wrapper
6. `KTop._handle_key()` - Input handling
7. `KTop._proc_table()` - Process table UI
8. `KTop._temp_cell()` - Temperature cell UI
9. `KTop._top_procs()` - Process ranking
10. `KTop._check_oom()` - OOM detection

**Impacted Nodes:** 35 nodes connected from these entry points

---

## Testing Approach

**Current State:** No explicit test files in repository

**Testing Features:**
1. **Simulation Mode (`--sim`)**: Fake OOM kills for testing UI
2. **Profiling Mode**: Writes to `/tmp/ktop_profile.log` with timing data
   - Section timing (avg ms, max ms, calls)
   - Frame-by-frame breakdown
   - Automatic flushing every 5 seconds

**Recommended Test Areas:**
1. Data collector accuracy (psutil wrappers)
2. Process scanning edge cases (permission errors, zombie processes)
3. Theme application and persistence
4. Keyboard handling edge cases
5. GPU availability scenarios
6. Configuration file parsing

---

## Code Statistics

| Metric | Value |
|--------|-------|
| Total Lines | 1,064 |
| Parsed Signatures | 37 |
| PDG Nodes | 76 |
| PDG Edges | 464 |
| External Imports | 38 |
| Internal Imports | 0 |
| Themes | 50 |
| History Length | 300 samples |

---

## Portability Considerations

**Linux-Specific:**
- `/proc` filesystem reading (process scanning)
- `journalctl` for OOM kills
- `termios`/`tty` for terminal control

**Optional Components:**
- pynvml (NVIDIA GPU) - graceful degradation
- GPU monitoring - works without GPU

**Cross-Platform Adaptation Required:**
1. macOS: Different `/proc` structure, no journalctl
2. Windows: No `/proc`, different terminal handling

---

## Integration Points for Maestro

### 1. Data Collection Layer

Ktop collectors can be adapted to Maestro's async architecture:

```rust
// Proposed Rust equivalent structure
pub struct CpuCollector {
    history: Arc<Mutex<VecDeque<f64>>>,
}

pub struct NetworkCollector {
    history_up: Arc<Mutex<VecDeque<f64>>>,
    history_down: Arc<Mutex<VecDeque<f64>>>,
    max_speed: Arc<Mutex<f64>>,
}
```

### 2. Theme System

The color theme dictionary structure maps well to Rust enums:

```rust
pub struct Theme {
    pub gpu: String,
    pub cpu: String,
    pub mem: String,
    pub bar_low: String,
    pub bar_mid: String,
    pub bar_high: String,
    // ...
}
```

### 3. UI Components

Rich panels/tables → Ratatui widgets:
- `Panel` → `Block`
- `Table` → `Table` (ratatui)
- `Layout` → `Layout` (ratatui)

### 4. Event Loop

Ktop's 50ms polling → Maestro's tick-based event system

---

## Recommendations from Phase 5 Analysis

1. **Resolve External Dependencies:** 38 external import edges could be better documented
2. **Focus Testing:** Prioritize `ktop.py` as the sole source file
3. **Review Hotspots:** `KTop._prof_time` has complexity=4, impact=34 (profiling code only)

---

## Appendix: File Structure

```
krustop/
├── ktop.py          (1,064 lines - main application)
├── requirements.txt (psutil, rich, pynvml)
├── setup.sh         (venv setup + install script)
├── README.md        (user documentation)
├── CHANGELOG.md     (version history)
├── CLAUDE.md        (AI assistant instructions)
└── screenshot.png   (demo screenshot)
```

---

**End of Analysis**
