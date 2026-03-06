# Ktop Resource Tab - Configuration Guide

## Overview

The Ktop resource tab provides real-time system monitoring within the Maestro TUI. It displays CPU usage, memory consumption, process information, network I/O, disk usage, and Maestro-specific telemetry.

## Configuration Options

### Refresh Rate

The refresh rate controls how often the metrics are updated.

**Default:** 3-4 seconds
**Range:** 1-10 seconds

#### Setting Refresh Rate

You can configure the refresh rate in several ways:

1. **Via Keyboard Shortcut (when tab is active):**
   - Press `+` to increase refresh rate (slower updates, less CPU)
   - Press `-` to decrease refresh rate (faster updates, more CPU)
   - Press `Space` to pause/resume updates

2. **Via Configuration File:**
   Add to your Maestro config file:
   ```toml
   [ktop]
   refresh_rate_seconds = 3
   ```

3. **Via Environment Variable:**
   ```bash
   export KTOP_REFRESH_RATE=3
   ```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Navigate between tabs |
| `+` | Increase refresh interval (slower) |
| `-` | Decrease refresh interval (faster) |
| `Space` | Pause/resume updates |
| `p` | Sort by CPU (Process list) |
| `m` | Sort by memory (Process list) |
| `h` | Show help overlay |
| `q` | Quit (when in tab-specific mode) |
| `F1` | Show keyboard shortcuts |

### Display Modes

#### Compact Mode

For smaller terminals, Ktop automatically switches to compact mode:
- Hides less critical information
- Condenses displays
- Focuses on essential metrics

#### Full Mode

When terminal size allows:
- All metrics visible
- Historical graphs
- Detailed process information

## Maestro-Specific Metrics

### LSP Server Status

Shows connected LSP servers with:
- Server name and status (Running, Stopped, Error)
- Files being tracked
- Diagnostics count
- Response latency

### Agent Telemetry

Displays active/running agents:
- Agent name and type
- Current status (Idle, Working, Paused, Error)
- CPU and memory usage

### LeIndex Statistics

Shows index health:
- Number of indexed files
- Number of indexed symbols
- Index size
- Last update timestamp

### Maestro Memory System

Memory allocation breakdown:
- Total memory allocated
- Cache memory
- Index memory
- Session memory

## Performance Considerations

### CPU Overhead

The Ktop tab is designed to use < 5% CPU at default refresh rate. If you notice higher CPU usage:

1. Increase refresh interval (press `+`)
2. Disable historical graphing
3. Enable compact mode

### Memory Overhead

Baseline memory usage is < 50MB. Memory is primarily used for:
- Metric history buffers
- Process list cache
- Network statistics tracking

## Troubleshooting

### Metrics Not Updating

1. Check if updates are paused (press Space to toggle)
2. Verify refresh rate isn't set too high
3. Check system permissions for /proc and /sys

### High CPU Usage

1. Increase refresh interval
2. Reduce the number of processes displayed
3. Disable per-core CPU graphs

### Missing Maestro Metrics

Maestro-specific metrics require:
- LSP bridge to be running
- Active sessions with code files
- LeIndex to be initialized

## Advanced Configuration

### Custom Thresholds

Set warning/critical thresholds for metrics:

```toml
[ktop.thresholds]
cpu_warning = 70.0
cpu_critical = 90.0
memory_warning = 80.0
memory_critical = 95.0
```

### Process Filtering

Control which processes are shown:

```toml
[ktop.processes]
exclude = ["kworker*", "systemd-*"]
max_display = 20
```

### Network Interfaces

Filter which network interfaces to monitor:

```toml
[ktop.network]
include = ["eth*", "wlan*"]
exclude = ["docker0", "virbr*"]
```

## Integration with Other Tools

### Export Metrics

Metrics can be exported for external monitoring:

```bash
maestro tui --export-metrics /tmp/ktop-metrics.json
```

### JSON Format

Exported metrics follow this structure:

```json
{
  "cpu": {
    "usage_percent": 45.2,
    "core_count": 8,
    "load_average": [2.1, 1.8, 1.5]
  },
  "memory": {
    "total_bytes": 17179869184,
    "used_bytes": 8589934592,
    "usage_percent": 50.0
  },
  "timestamp": "2025-02-10T12:00:00Z"
}
```
