# Ktop Resource Tab - User Guide

## Introduction

The Ktop resource tab is a system monitoring interface integrated into Maestro TUI. It provides real-time visibility into:

- **System Resources:** CPU, memory, disk, network
- **Processes:** Top processes by CPU and memory usage
- **Maestro Telemetry:** LSP status, agent activity, LeIndex stats

## Getting Started

### Accessing the Ktop Tab

1. Launch Maestro TUI: `maestro tui`
2. Navigate using `Tab` or `Shift+Tab` until you reach the Ktop tab
3. The tab is labeled with a 📊 icon

### First View

On first opening, you'll see:
- **Header:** Tab title and control hints
- **CPU Section:** Usage graphs and load averages
- **Memory Section:** RAM and swap usage bars
- **Process List:** Top processes sorted by CPU
- **Network Section:** Interface statistics
- **Disk Section:** Mount points and I/O rates
- **Maestro Section:** LSP, agent, and index status

## Navigation

### Moving Between Sections

| Key | Action |
|-----|--------|
| `Tab` | Move to next section/widget |
| `Shift+Tab` | Move to previous section/widget |
| `Arrow Keys` | Navigate within lists/tables |
| `Enter` | Select item or expand details |

### Global Shortcuts

| Key | Action |
|-----|--------|
| `q` | Return to main navigation |
| `F1` | Show help overlay |
| `Ctrl+C` | Exit Maestro TUI |
| `Space` | Pause/resume metric updates |

## Understanding the Display

### CPU Section

**What you see:**
- Overall CPU usage as a percentage
- Per-core usage bars (if space allows)
- Load averages (1min, 5min, 15min)
- Historical usage sparkline

**What it means:**
- Usage 0-50%: Light load, headroom available
- Usage 50-80%: Moderate load, normal activity
- Usage 80-100%: Heavy load, may impact performance

**Colors:**
- Green: 0-50% usage
- Yellow: 50-80% usage
- Red: 80-100% usage

### Memory Section

**What you see:**
- Total RAM and current usage
- Usage percentage and bar graph
- Swap usage (if enabled)
- Cached and buffered memory

**What it means:**
- Linux uses free memory for cache
- "Available" memory is what matters, not "free"
- High cache usage is good, not bad

**Understanding the bars:**
```
RAM: ████░░░░░░░░░░░░░░ 12.5GB / 32GB (39%)
     ^Used              ^Total
```

### Process List

**What you see:**
- Process name and PID
- CPU usage percentage
- Memory usage percentage
- Process status (Running, Sleeping, etc.)

**Sorting:**
- Press `p` to sort by CPU (default)
- Press `m` to sort by memory
- Press `n` to sort by name

**Status indicators:**
- `●` Running
- `○` Sleeping
- `■` Stopped
- `⚠` Zombie

### Network Section

**What you see:**
- Network interfaces (eth0, wlan0, etc.)
- Download/upload speeds
- Total bytes transferred
- Packet counts and errors

**Interpreting speeds:**
- Shows bytes/second (B/s)
- Auto-scales to KB/s, MB/s, GB/s
- Real-time bandwidth measurement

### Disk Section

**What you see:**
- Mount points (/home, /var, etc.)
- Used/total space
- I/O rates (read/write speeds)
- File system type

**Usage indicators:**
- Green: < 70% used
- Yellow: 70-90% used
- Red: > 90% used

### Maestro Telemetry

#### LSP Status
- Server name (rust-analyzer, ruff, etc.)
- Running state (● Running, ■ Stopped)
- Files tracked and diagnostics count

#### Agent Activity
- Active agents by name
- Agent type (general-purpose, feature-dev, etc.)
- Current status (Working, Idle, Paused)

#### LeIndex Stats
- Files indexed count
- Symbols indexed count
- Index size in MB
- Last update time

## Common Workflows

### Monitoring During Build

When running `maestro orchestrate`:

1. Open Ktop tab
2. Watch CPU usage during agent activity
3. Monitor memory for leaks
4. Check agent status in Maestro section

### Investigating Slow Performance

1. Check CPU usage - is anything at 100%?
2. Check memory - is swap being used?
3. Check process list - identify high-usage processes
4. Check I/O - is disk waiting?

### Tracking LSP Health

1. Navigate to Ktop tab
2. Look at Maestro → LSP Status section
3. Verify servers show ● Running
4. Check diagnostics count for errors

## Customization

### Adjusting Refresh Rate

**Faster updates (1-2 seconds):**
- Good for: Real-time debugging
- Trade-off: Higher CPU usage

**Slower updates (5-10 seconds):**
- Good for: Background monitoring
- Trade-off: Less responsive

**Paused (Space):**
- Good for: Examining current state
- Trade-off: No updates until resumed

### Changing Display Mode

The Ktop tab automatically adapts to your terminal size:

- **Large terminals (80x24+):** Full display with graphs
- **Medium terminals (80x20):** Standard display
- **Small terminals (<80x20):** Compact mode

## Tips and Tricks

### 1. Spot Memory Leaks

Monitor a process over time:
1. Note its memory percentage
2. Press Space to pause
3. Wait 30 seconds
4. Press Space to resume
5. Check if memory increased

### 2. Identify CPU Spikes

1. Set refresh rate to 1 second (press `-` until shows `1s`)
2. Watch per-core graphs
3. Note which core spikes
4. Check process list for high-CPU processes

### 3. Check Network Bandwidth

1. Start a download/upload
2. Watch Network section
3. Verify speed matches expected rate
4. Check for errors on interface

### 4. Monitor Agent Resources

During orchestrate:
1. Open Ktop tab
2. Find agent in Maestro → Agent Activity
3. Check CPU and memory for each agent
4. Identify resource-intensive agents

## Troubleshooting

### Problem: Metrics show zero

**Cause:** Permissions or initialization issue

**Solution:**
1. Ensure Maestro has read access to /proc
2. Try refreshing (press `r`)
3. Restart Maestro TUI

### Problem: Historical graphs missing

**Cause:** Terminal too small or disabled

**Solution:**
1. Increase terminal size
2. Check if compact mode is active
3. Verify history isn't disabled in config

### Problem: LSP status shows "Missing"

**Cause:** LSP not installed or not in PATH

**Solution:**
1. Check LSP installation guide in LSPs tab
2. Install missing LSP
3. Restart session

## Keyboard Reference Card

```
┌─────────────────────────────────────────┐
│ KTOP TAB KEYBOARD REFERENCE            │
├─────────────────────────────────────────┤
│ Navigation:                             │
│   Tab          Next section             │
│   Shift+Tab    Previous section         │
│   Arrows       Navigate in lists        │
│                                          │
│ Controls:                                │
│   +            Slower refresh           │
│   -            Faster refresh           │
│   Space        Pause/Resume             │
│   r            Manual refresh           │
│                                          │
│ Process List:                            │
│   p            Sort by CPU              │
│   m            Sort by memory           │
│   n            Sort by name             │
│                                          │
│ Display:                                │
│   c            Toggle compact mode      │
│   g            Toggle graphs            │
│   h            Toggle history           │
│                                          │
│ Help:                                   │
│   F1 / ?       Show this help           │
│   q            Back to navigation       │
└─────────────────────────────────────────┘
```

## Getting Help

- Press `F1` or `?` in Ktop tab for context-sensitive help
- Check LSPs tab for LSP installation help
- See `maestro/docs/ktop-configuration.md` for config options
