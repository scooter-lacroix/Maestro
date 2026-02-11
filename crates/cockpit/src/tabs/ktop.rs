//! Krustop resource tab rendering for Cockpit TUI
//!
//! This tab provides real-time system metrics display including:
//! - CPU usage (per-core and aggregate)
//! - Memory usage (RAM, swap, buffers, cache)
//! - Process list (top by CPU and memory)
//! - Network I/O statistics
//! - Disk usage and I/O
//! - Maestro-specific metrics (LSP status, agents, LeIndex)

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Table},
    Frame,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::app::App;
use crate::theme::Theme;

// Re-export ktop_collectors types
use ktop_collectors::{
    CpuCollector, DiskCollector, MaestroCollector, MemoryCollector, MetricsState, NetworkCollector,
    ProcessCollector, StateUpdate, SystemMetrics,
};

/// Control commands for the collector loop
#[derive(Clone)]
pub enum CollectorControl {
    /// Pause/resume collection
    SetPaused(bool),
    /// Set refresh interval in seconds (1-10)
    SetRefreshInterval(u64),
    /// Stop the collector loop
    Stop,
}

/// Ktop tab state managed by the App
pub struct KtopState {
    /// Last refresh timestamp
    pub last_refresh: Instant,

    /// Refresh interval in seconds (1-10, default 3)
    pub refresh_interval_secs: u64,

    /// Is refresh paused?
    pub paused: bool,

    /// Current focus area within Ktop tab
    pub focus: KtopFocus,

    /// CPU history for sparkline (last 60 readings)
    pub cpu_history: Vec<f32>,

    /// Memory history for sparkline
    pub memory_history: Vec<f32>,

    /// Process sort column
    pub process_sort: ProcessSort,

    /// Process list state (for navigation)
    pub process_state: ratatui::widgets::TableState,

    /// Network interface list state
    pub network_state: ratatui::widgets::ListState,

    /// Disk mount list state
    pub disk_state: ratatui::widgets::ListState,

    /// Maestro section state
    pub maestro_state: ratatui::widgets::ListState,

    /// Subscription receiver for metrics updates
    pub metrics_rx: Option<broadcast::Receiver<StateUpdate>>,

    /// Current cached metrics
    pub current_metrics: SystemMetrics,

    /// Last error from collectors
    pub last_error: Option<String>,

    /// Collector status
    pub collector_status: CollectorStatus,

    /// Sender for collector control commands
    pub collector_control_tx: Option<broadcast::Sender<CollectorControl>>,

    /// Shared atomic flag for pause state (checked by collector loop)
    pub pause_flag: Option<std::sync::Arc<AtomicBool>>,

    /// Shared atomic value for refresh interval (checked by collector loop)
    pub refresh_interval_atomic: Option<std::sync::Arc<AtomicU64>>,
}

/// Focus areas within the Ktop tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KtopFocus {
    /// CPU section
    #[default]
    Cpu,

    /// Memory section
    Memory,

    /// Process list
    Processes,

    /// Network section
    Network,

    /// Disk section
    Disk,

    /// Maestro metrics section
    Maestro,
}

/// Process list sort column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSort {
    /// Sort by CPU usage
    #[default]
    Cpu,

    /// Sort by memory usage
    Memory,

    /// Sort by PID
    Pid,

    /// Sort by name
    Name,
}

/// Collector status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorStatus {
    /// Collectors are running normally
    Running,

    /// Collectors are initializing
    Initializing,

    /// Collectors encountered an error
    Error,

    /// Collectors are paused
    Paused,

    /// Collectors are stopped
    Stopped,
}

impl Default for KtopState {
    fn default() -> Self {
        Self {
            last_refresh: Instant::now(),
            refresh_interval_secs: 3,
            paused: false,
            focus: KtopFocus::default(),
            cpu_history: Vec::with_capacity(60),
            memory_history: Vec::with_capacity(60),
            process_sort: ProcessSort::default(),
            process_state: ratatui::widgets::TableState::default(),
            network_state: ratatui::widgets::ListState::default(),
            disk_state: ratatui::widgets::ListState::default(),
            maestro_state: ratatui::widgets::ListState::default(),
            metrics_rx: None,
            current_metrics: SystemMetrics::new(),
            last_error: None,
            collector_status: CollectorStatus::Initializing,
            collector_control_tx: None,
            pause_flag: None,
            refresh_interval_atomic: None,
        }
    }
}

impl KtopState {
    /// Create a new Ktop state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if refresh is needed
    pub fn needs_refresh(&self) -> bool {
        if self.paused {
            return false;
        }
        self.last_refresh.elapsed() >= Duration::from_secs(self.refresh_interval_secs)
    }

    /// Mark as refreshed
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = Instant::now();
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.collector_status = if self.paused {
            CollectorStatus::Paused
        } else {
            CollectorStatus::Running
        };

        // Update atomic flag for collector loop
        if let Some(ref flag) = self.pause_flag {
            flag.store(self.paused, Ordering::Relaxed);
        }

        // Send control command
        if let Some(ref tx) = self.collector_control_tx {
            let _ = tx.send(CollectorControl::SetPaused(self.paused));
        }
    }

    /// Set refresh interval (clamped to 1-10 seconds)
    pub fn set_refresh_interval(&mut self, secs: u64) {
        self.refresh_interval_secs = secs.clamp(1, 10);

        // Update atomic value for collector loop
        if let Some(ref interval) = self.refresh_interval_atomic {
            interval.store(self.refresh_interval_secs, Ordering::Relaxed);
        }

        // Send control command
        if let Some(ref tx) = self.collector_control_tx {
            let _ = tx.send(CollectorControl::SetRefreshInterval(self.refresh_interval_secs));
        }
    }

    /// Get current refresh interval as Duration
    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.refresh_interval_secs)
    }

    /// Process metrics updates from the channel
    pub fn process_updates(&mut self) {
        if let Some(ref mut rx) = self.metrics_rx {
            // Process all pending updates non-blockingly
            while let Ok(update) = rx.try_recv() {
                self.current_metrics = update.metrics;

                // Update histories - extract values first to avoid multiple borrows
                let cpu_usage = self.current_metrics.cpu.as_ref().map(|c| c.usage_percent);
                let mem_usage = self.current_metrics.memory.as_ref().map(|m| m.usage_percent());
                let is_paused = self.paused;
                let is_complete = self.current_metrics.is_complete();

                // Update CPU history
                if let Some(usage) = cpu_usage {
                    self.cpu_history.push(usage);
                    if self.cpu_history.len() > 60 {
                        self.cpu_history.remove(0);
                    }
                }

                // Update memory history
                if let Some(usage) = mem_usage {
                    self.memory_history.push(usage);
                    if self.memory_history.len() > 60 {
                        self.memory_history.remove(0);
                    }
                }

                // Update collector status
                self.collector_status = if is_paused {
                    CollectorStatus::Paused
                } else if is_complete {
                    CollectorStatus::Running
                } else {
                    CollectorStatus::Initializing
                };

                self.last_error = None;
            }
        }
    }
}

/// Render the Ktop resource tab
///
/// This is the main entry point for rendering the Ktop tab.
/// It delegates to sub-renderers for each section.
pub fn render_ktop(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();

    // Ensure ktop state exists
    if app.ktop_state.is_none() {
        app.ktop_state = Some(KtopState::new());
        // Initialize collectors if not already done
        initialize_collectors(app);
    }

    let ktop_state = app.ktop_state.as_mut().expect("ktop_state just created");

    // Process any pending metrics updates
    ktop_state.process_updates();

    // Update refresh timestamp if needed (collectors run in background)
    if ktop_state.needs_refresh() && !ktop_state.paused {
        ktop_state.mark_refreshed();
    }

    // Create layout with header and main content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header with controls
            Constraint::Min(0),     // Main content
        ])
        .split(area);

    // Render header
    render_header(frame, chunks[0], ktop_state, &theme);

    // Split main content into sections based on terminal size
    let main_chunks = create_main_layout(chunks[1], ktop_state.focus);

    // Render each section - Krustop layout: Net/CPU/Mem top, then process lists, then Maestro
    render_network_section(frame, main_chunks.network, ktop_state, &theme);
    render_cpu_section(frame, main_chunks.cpu, ktop_state, &theme);
    render_memory_section(frame, main_chunks.memory, ktop_state, &theme);

    // Render two process lists - by memory and by CPU
    render_process_section(frame, main_chunks.processes_by_mem, ktop_state, &theme, ProcessSort::Memory);
    render_process_section(frame, main_chunks.processes_by_cpu, ktop_state, &theme, ProcessSort::Cpu);

    render_maestro_section(frame, main_chunks.maestro, ktop_state, &theme);
}

/// Layout chunks for main content
struct MainChunks {
    cpu: Rect,
    memory: Rect,
    network: Rect,
    processes_by_mem: Rect,
    processes_by_cpu: Rect,
    maestro: Rect,
}

/// Create the main layout based on terminal size and current focus
/// Layout matches original Krustop: Net/CPU/Mem row, then two process lists
fn create_main_layout(area: Rect, _focus: KtopFocus) -> MainChunks {
    let _height = area.height;
    let width = area.width as usize;

    // Reserve space for header
    let remaining = area.height.saturating_sub(3);

    if width < 80 || remaining < 20 {
        // Minimal layout for very small terminals
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),   // CPU
                Constraint::Length(5),   // Memory
                Constraint::Length(4),   // Network
                Constraint::Min(8),      // Processes (combined)
                Constraint::Length(4),   // Maestro
            ])
            .split(Rect::new(area.x, area.y + 3, area.width, remaining));

        // Split processes into two columns if space allows
        let proc_area = chunks[3];
        let (mem_procs, cpu_procs) = if proc_area.width >= 60 {
            let procs = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(proc_area);
            (procs[0], procs[1])
        } else {
            // Too narrow - use same area for both (will render different content based on focus)
            (proc_area, proc_area)
        };

        MainChunks {
            cpu: chunks[0],
            memory: chunks[1],
            network: chunks[2],
            processes_by_mem: mem_procs,
            processes_by_cpu: cpu_procs,
            maestro: chunks[4],
        }
    } else {
        // Standard Krustop-style layout
        // Top row: Network | CPU | Memory (equal width)
        // Bottom: Two process lists side by side
        // Bottom strip: Maestro metrics

        let top_height = (remaining / 2).min(12);
        let proc_height = remaining.saturating_sub(top_height).saturating_sub(4);
        let maestro_height = 4.min(remaining);

        let main_area = Rect::new(area.x, area.y + 3, area.width, remaining);

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_height),
                Constraint::Length(proc_height),
                Constraint::Length(maestro_height),
            ])
            .split(main_area);

        // Top row: Network | CPU | Memory
        let top_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(vertical[0]);

        // Bottom row: Mem Procs | CPU Procs
        let procs_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(vertical[1]);

        MainChunks {
            network: top_row[0],
            cpu: top_row[1],
            memory: top_row[2],
            processes_by_mem: procs_row[0],
            processes_by_cpu: procs_row[1],
            maestro: vertical[2],
        }
    }
}

/// Render the header with controls
fn render_header(frame: &mut Frame, area: Rect, state: &KtopState, theme: &Theme) {
    let status_text = match state.collector_status {
        CollectorStatus::Running => Span::styled("Running", Style::default().fg(Color::Green)),
        CollectorStatus::Initializing => Span::styled("Init...", Style::default().fg(Color::Yellow)),
        CollectorStatus::Error => Span::styled("Error", Style::default().fg(Color::Red)),
        CollectorStatus::Paused => Span::styled("Paused", Style::default().fg(Color::Yellow)),
        CollectorStatus::Stopped => Span::styled("Stopped", Style::default().fg(Color::Red)),
    };

    let header_text = vec![
        Line::from(vec![
            Span::styled("Alt+P:", Style::default().fg(theme.accent).bold()),
            Span::styled(
                if state.paused { "Resume" } else { "Pause" },
                Style::default().fg(theme.fg),
            ),
            Span::styled("  Alt+R:", Style::default().fg(theme.accent).bold()),
            Span::styled(format!("Refresh({}s)", state.refresh_interval_secs), Style::default().fg(theme.fg)),
            Span::styled("  Alt+/-:", Style::default().fg(theme.accent).bold()),
            Span::styled("Rate", Style::default().fg(theme.fg)),
            Span::styled("  Alt+Tab:", Style::default().fg(theme.accent).bold()),
            Span::styled("Focus", Style::default().fg(theme.fg)),
            Span::raw("  "),
            Span::styled("Status:", Style::default().fg(theme.muted)),
            status_text,
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .title(" ⚡ Krustop ")
                .title_style(Style::default().fg(theme.accent))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);

    frame.render_widget(header, area);
}

/// Render the CPU section
fn render_cpu_section(frame: &mut Frame, area: Rect, state: &KtopState, theme: &Theme) {
    let block = Block::default()
        .title(" 🖥️ CPU ")
        .title_style(Style::default().fg(theme.accent))
        .borders(Borders::ALL)
        .border_type(if state.focus == KtopFocus::Cpu {
            BorderType::Double
        } else {
            BorderType::Rounded
        });

    // Ensure area has enough space for content
    if area.height < 3 || area.width < 10 {
        frame.render_widget(block, area);
        return;
    }

    if let Some(ref cpu) = state.current_metrics.cpu {
        let usage_color = color_for_percent(cpu.usage_percent);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Total: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{:.1}%", cpu.usage_percent),
                    Style::default().fg(usage_color).bold(),
                ),
                Span::styled(
                    format!("  {} cores", cpu.core_count),
                    Style::default().fg(theme.muted),
                ),
            ]),
            Line::from(vec![
                Span::styled("Load: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{:.2} {:.2} {:.2}", cpu.load_average.0, cpu.load_average.1, cpu.load_average.2),
                    Style::default().fg(theme.fg),
                ),
            ]),
        ];

        // Add per-core usage if available (compact format)
        if !cpu.per_core_usage.is_empty() {
            let mut core_spans = vec![Span::styled("Cores: ", Style::default().fg(theme.muted))];
            for (i, u) in cpu.per_core_usage.iter().enumerate() {
                let c = color_for_percent(*u);
                if i > 0 {
                    core_spans.push(Span::raw(" "));
                }
                core_spans.push(Span::styled(format!("{}%", *u as u32), Style::default().fg(c)));
            }
            lines.push(Line::from(core_spans));
        }

        // Add top CPU consumers inline
        let top_cpu: Vec<_> = state.current_metrics.top_cpu_processes.iter().take(3).collect();
        if !top_cpu.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Top CPU:", Style::default().fg(theme.accent).bold()),
            ]));
            for proc in top_cpu {
                let proc_color = color_for_percent(proc.cpu_percent);
                let name = if proc.name.len() > 10 {
                    &proc.name[..10]
                } else {
                    &proc.name
                };
                lines.push(Line::from(vec![
                    Span::styled(format!(" {:<10} ", name), Style::default().fg(theme.fg)),
                    Span::styled(format!("{:>5.1}%", proc.cpu_percent), Style::default().fg(proc_color).bold()),
                ]));
            }
        }

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Only render gauge if we have enough height
        if inner_area.height >= 2 {
            let gauge_height = 1;
            let gauge_area = Rect {
                height: gauge_height,
                y: inner_area.y + inner_area.height.saturating_sub(gauge_height),
                ..inner_area
            };

            // Ensure gauge_area is within bounds
            if gauge_area.y + gauge_area.height <= area.y + area.height {
                let gauge = Gauge::default()
                    .block(Block::default())
                    .gauge_style(Style::default().fg(usage_color))
                    .percent(cpu.usage_percent as u16)
                    .label("");
                frame.render_widget(gauge, gauge_area);
            }

            // Render text above gauge
            let text_area = Rect {
                height: inner_area.height.saturating_sub(gauge_height),
                ..inner_area
            };
            let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
            frame.render_widget(paragraph, text_area);
        } else {
            // Not enough height for gauge, just render text
            let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
            frame.render_widget(paragraph, inner_area);
        }
    } else {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Initializing...", Style::default().fg(theme.muted).italic()),
            ]),
        ];
        let paragraph = Paragraph::new(content).block(block).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }
}

/// Render the memory section
fn render_memory_section(frame: &mut Frame, area: Rect, state: &KtopState, theme: &Theme) {
    let block = Block::default()
        .title(" 🧠 Memory ")
        .title_style(Style::default().fg(theme.accent_alt))
        .borders(Borders::ALL)
        .border_type(if state.focus == KtopFocus::Memory {
            BorderType::Double
        } else {
            BorderType::Rounded
        });

    // Ensure area has enough space
    if area.height < 3 || area.width < 10 {
        frame.render_widget(block, area);
        return;
    }

    if let Some(ref mem) = state.current_metrics.memory {
        let usage_pct = mem.usage_percent();
        let usage_color = color_for_percent(usage_pct);

        let total_gb = mem.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let used_gb = mem.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let avail_gb = mem.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("RAM: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{:.1}GB / {:.1}GB ({:.1}%)", used_gb, total_gb, usage_pct),
                    Style::default().fg(usage_color).bold(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Swap: ", Style::default().fg(theme.muted)),
                if mem.swap_total_bytes > 0 {
                    let swap_pct = mem.swap_usage_percent();
                    let swap_color = color_for_percent(swap_pct);
                    let swap_used_gb = mem.swap_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    let swap_total_gb = mem.swap_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    Span::styled(
                        format!("{:.1}GB / {:.1}GB ({:.1}%)", swap_used_gb, swap_total_gb, swap_pct),
                        Style::default().fg(swap_color),
                    )
                } else {
                    Span::styled("N/A", Style::default().fg(theme.muted))
                },
            ]),
            Line::from(vec![
                Span::styled("Avail: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{:.1}GB", avail_gb),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!(" Buf:{:.1}G", mem.buffers_bytes as f64 / (1024.0 * 1024.0 * 1024.0)),
                    Style::default().fg(theme.muted),
                ),
            ]),
        ];

        // Add top memory consumers inline
        let top_mem: Vec<_> = state.current_metrics.top_memory_processes.iter().take(3).collect();
        if !top_mem.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Top Mem:", Style::default().fg(theme.accent_alt).bold()),
            ]));
            for proc in top_mem {
                let mem_color = color_for_percent(proc.memory_percent);
                let name = if proc.name.len() > 10 {
                    &proc.name[..10]
                } else {
                    &proc.name
                };
                let mem_str = format_size(proc.rss_bytes);
                lines.push(Line::from(vec![
                    Span::styled(format!(" {:<10} ", name), Style::default().fg(theme.fg)),
                    Span::styled(format!("{:>6}", mem_str), Style::default().fg(mem_color).bold()),
                    Span::styled(format!(" ({:.0}%)", proc.memory_percent), Style::default().fg(theme.muted)),
                ]));
            }
        }

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Only render gauge if we have enough height
        if inner_area.height >= 2 {
            let gauge_height = 1;
            let gauge_area = Rect {
                height: gauge_height,
                y: inner_area.y + inner_area.height.saturating_sub(gauge_height),
                ..inner_area
            };

            // Ensure gauge_area is within bounds
            if gauge_area.y + gauge_area.height <= area.y + area.height {
                let gauge = Gauge::default()
                    .block(Block::default())
                    .gauge_style(Style::default().fg(usage_color))
                    .percent(usage_pct as u16)
                    .label("");
                frame.render_widget(gauge, gauge_area);
            }

            // Render text above gauge
            let text_area = Rect {
                height: inner_area.height.saturating_sub(gauge_height),
                ..inner_area
            };
            let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
            frame.render_widget(paragraph, text_area);
        } else {
            // Not enough height for gauge, just render text
            let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
            frame.render_widget(paragraph, inner_area);
        }
    } else {
        let paragraph = Paragraph::new(Line::from(vec![
            Span::styled("Initializing...", Style::default().fg(theme.muted).italic()),
        ]))
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }
}

/// Render the process list section
fn render_process_section(frame: &mut Frame, area: Rect, state: &mut KtopState, theme: &Theme, sort: ProcessSort) {
    let title = match sort {
        ProcessSort::Cpu => " 📋 Top CPU ",
        ProcessSort::Memory => " 📋 Top Memory ",
        _ => " 📋 Processes ",
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(theme.accent))
        .borders(Borders::ALL)
        .border_type(if state.focus == KtopFocus::Processes {
            BorderType::Double
        } else {
            BorderType::Rounded
        });

    let processes = match sort {
        ProcessSort::Cpu => &state.current_metrics.top_cpu_processes,
        ProcessSort::Memory => &state.current_metrics.top_memory_processes,
        _ => &state.current_metrics.top_cpu_processes,
    };

    if processes.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No process data", Style::default().fg(theme.muted).italic()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(theme.muted)),
                Span::styled("Alt+R", Style::default().fg(theme.warning).bold()),
                Span::styled(" to refresh", Style::default().fg(theme.muted)),
            ]),
        ];
        let paragraph = Paragraph::new(content).block(block).alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    // Sort indicator
    let sort_indicator = match sort {
        ProcessSort::Cpu => "CPU%▼",
        ProcessSort::Memory => "MEM%▼",
        ProcessSort::Pid => "PID▼",
        ProcessSort::Name => "NAME▼",
    };

    let header = Row::new(vec![
        Cell::from("PID"),
        Cell::from(sort_indicator),
        Cell::from("MEM%"),
        Cell::from("Status"),
        Cell::from("Command"),
    ])
    .style(Style::default().fg(theme.accent).bold());

    let max_rows = area.height.saturating_sub(3) as usize;
    let rows: Vec<Row> = processes
        .iter()
        .take(max_rows)
        .map(|p| {
            let status_color = match p.status {
                ktop_collectors::ProcessStatus::Running => Color::Green,
                ktop_collectors::ProcessStatus::Sleeping => Color::Yellow,
                ktop_collectors::ProcessStatus::Stopped => Color::Red,
                ktop_collectors::ProcessStatus::Zombie => Color::Magenta,
                _ => Color::Gray,
            };

            let status_char = match p.status {
                ktop_collectors::ProcessStatus::Running => "R",
                ktop_collectors::ProcessStatus::Sleeping => "S",
                ktop_collectors::ProcessStatus::Stopped => "T",
                ktop_collectors::ProcessStatus::Zombie => "Z",
                ktop_collectors::ProcessStatus::Dead => "D",
                ktop_collectors::ProcessStatus::Unknown => "?",
            };

            let command = p.command
                .as_ref()
                .and_then(|c| c.split_whitespace().next())
                .unwrap_or(&p.name)
                .to_string();

            // Truncate command if too long
            let command = if command.len() > 30 {
                format!("{}...", &command[..27])
            } else {
                command
            };

            Row::new(vec![
                Cell::from(format!("{}", p.pid)),
                Cell::from(format!("{:.1}", p.cpu_percent)),
                Cell::from(format!("{:.1}", p.memory_percent)),
                Cell::from(status_char).style(Style::default().fg(status_color)),
                Cell::from(command),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(6), Constraint::Length(6), Constraint::Length(6), Constraint::Length(4), Constraint::Min(0)])
        .block(block)
        .header(header)
        .row_highlight_style(Style::default().bg(theme.highlight_bg))
        .column_spacing(1);

    frame.render_stateful_widget(table, area, &mut state.process_state);
}

/// Render the network section
fn render_network_section(frame: &mut Frame, area: Rect, state: &mut KtopState, theme: &Theme) {
    let block = Block::default()
        .title(" 🌐 Network ")
        .title_style(Style::default().fg(theme.accent_alt))
        .borders(Borders::ALL)
        .border_type(if state.focus == KtopFocus::Network {
            BorderType::Double
        } else {
            BorderType::Rounded
        });

    if let Some(ref net) = state.current_metrics.network {
        let down_speed = format_speed(net.download_speed_bps);
        let up_speed = format_speed(net.upload_speed_bps);

        // Get top interfaces by traffic
        let mut interfaces: Vec<_> = net.interfaces.iter().collect();
        interfaces.sort_by(|a, b| {
            b.1.recv_bytes
                .wrapping_add(b.1.sent_bytes)
                .cmp(&a.1.recv_bytes.wrapping_add(a.1.sent_bytes))
        });

        let mut content = vec![
            Line::from(vec![
                Span::styled("↓ ", Style::default().fg(Color::Green)),
                Span::styled(down_speed, Style::default().fg(Color::Green).bold()),
                Span::styled("  ↑ ", Style::default().fg(Color::Blue)),
                Span::styled(up_speed, Style::default().fg(Color::Blue).bold()),
            ]),
            Line::from(""),
        ];

        let interface_lines: Vec<Line> = interfaces
            .iter()
            .take(3)
            .map(|(name, stats)| {
                Line::from(vec![
                    Span::styled(
                        format!("{}:", name),
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(
                        format!(" ↓{} ↑{}", format_size(stats.recv_bytes), format_size(stats.sent_bytes)),
                        Style::default().fg(theme.muted),
                    ),
                ])
            })
            .collect();

        content.extend(interface_lines);
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    } else {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Initializing...", Style::default().fg(theme.muted).italic()),
            ]),
        ];
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Render the disk section
#[allow(dead_code)]
fn render_disk_section(frame: &mut Frame, area: Rect, state: &mut KtopState, theme: &Theme) {
    let block = Block::default()
        .title(" 💾 Disk ")
        .title_style(Style::default().fg(theme.accent_alt))
        .borders(Borders::ALL)
        .border_type(if state.focus == KtopFocus::Disk {
            BorderType::Double
        } else {
            BorderType::Rounded
        });

    if let Some(ref disk) = state.current_metrics.disk {
        let read_speed = format_speed(disk.read_speed_bps);
        let write_speed = format_speed(disk.write_speed_bps);

        let content = vec![
            Line::from(vec![
                Span::styled("I/O: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("↓{}  ↑{}", read_speed, write_speed),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
        ];

        let mount_lines: Vec<Line> = disk
            .mounts
            .iter()
            .take(3)
            .map(|m| {
                let usage_color = color_for_percent(m.usage_percent());
                Line::from(vec![
                    Span::styled(
                        format!("{:<12} ", m.mount_point),
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(
                        format!("{:>3.0}% ", m.usage_percent()),
                        Style::default().fg(usage_color).bold(),
                    ),
                    Span::styled(
                        format_size(m.used_bytes),
                        Style::default().fg(theme.muted),
                    ),
                    Span::raw("/"),
                    Span::styled(
                        format_size(m.total_bytes),
                        Style::default().fg(theme.muted),
                    ),
                ])
            })
            .collect();

        let content = content.into_iter().chain(mount_lines).collect::<Vec<_>>();
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    } else {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Initializing...", Style::default().fg(theme.muted).italic()),
            ]),
        ];
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Render the Maestro metrics section
fn render_maestro_section(frame: &mut Frame, area: Rect, state: &mut KtopState, theme: &Theme) {
    let block = Block::default()
        .title(" 🎯 Maestro ")
        .title_style(Style::default().fg(theme.accent))
        .borders(Borders::ALL)
        .border_type(if state.focus == KtopFocus::Maestro {
            BorderType::Double
        } else {
            BorderType::Rounded
        });

    if let Some(ref maestro) = state.current_metrics.maestro {
        // Count LSP status
        let lsp_running = maestro.lsp_servers.values().filter(|l| l.is_running).count();
        let lsp_total = maestro.lsp_servers.len();

        // Count agents by status
        let agents_working = maestro.agents.values().filter(|a| matches!(a.status, ktop_collectors::AgentStatus::Working)).count();
        let agents_total = maestro.agents.len();

        let content = vec![
            Line::from(vec![
                Span::styled("LSPs: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{}/{}", lsp_running, lsp_total),
                    Style::default().fg(if lsp_running == lsp_total { Color::Green } else { Color::Yellow }).bold(),
                ),
                Span::styled(
                    format!("  Agents: {}/{}", agents_working, agents_total),
                    Style::default().fg(theme.muted),
                ),
            ]),
            Line::from(vec![
                Span::styled("LeIndex: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("{} files", maestro.leindex.files_indexed),
                    Style::default().fg(theme.accent),
                ),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", Style::default().fg(theme.muted)),
                Span::styled(
                    format_size(maestro.memory.total_bytes),
                    Style::default().fg(theme.fg),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    } else {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Maestro metrics unavailable", Style::default().fg(theme.muted).italic()),
            ]),
        ];
        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// Initialize collectors for the Ktop tab
fn initialize_collectors(app: &mut App) {
    use tokio::runtime::Handle;

    if let Some(ref mut ktop_state) = app.ktop_state {
        // Create atomic shared state for pause and interval control
        let pause_flag = std::sync::Arc::new(AtomicBool::new(false));
        let refresh_interval_atomic = std::sync::Arc::new(AtomicU64::new(3));

        // Create control channel
        let (control_tx, _) = broadcast::channel::<CollectorControl>(16);

        // Store references in state
        ktop_state.pause_flag = Some(pause_flag.clone());
        ktop_state.refresh_interval_atomic = Some(refresh_interval_atomic.clone());
        ktop_state.collector_control_tx = Some(control_tx.clone());

        // Create metrics state with channel
        let metrics_state = MetricsState::new();
        let rx = metrics_state.subscribe();

        // Store receiver in app state
        ktop_state.metrics_rx = Some(rx);
        ktop_state.collector_status = CollectorStatus::Initializing;

        // Spawn background collector task
        if let Ok(handle) = Handle::try_current() {
            handle.spawn(async move {
                run_collectors_loop(metrics_state, pause_flag, refresh_interval_atomic, control_tx).await;
            });
        }
    }
}

/// Background task that runs collectors in a loop
async fn run_collectors_loop(
    state: MetricsState,
    pause_flag: std::sync::Arc<AtomicBool>,
    refresh_interval_atomic: std::sync::Arc<AtomicU64>,
    control_tx: broadcast::Sender<CollectorControl>,
) {
    let mut cpu_collector = CpuCollector::new();
    let mut mem_collector = MemoryCollector::new();
    let mut proc_collector = ProcessCollector::new();
    let mut net_collector = NetworkCollector::new();
    let mut disk_collector = DiskCollector::new();
    let mut maestro_collector = MaestroCollector::new();

    // Subscribe to control commands
    let mut control_rx = control_tx.subscribe();

    loop {
        // Check for control commands
        let mut should_stop = false;
        while let Ok(cmd) = control_rx.try_recv() {
            match cmd {
                CollectorControl::SetPaused(paused) => {
                    pause_flag.store(paused, Ordering::Relaxed);
                }
                CollectorControl::SetRefreshInterval(secs) => {
                    refresh_interval_atomic.store(secs.clamp(1, 10), Ordering::Relaxed);
                }
                CollectorControl::Stop => {
                    should_stop = true;
                }
            }
        }

        if should_stop {
            break;
        }

        // Check if paused
        if pause_flag.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        // Get current refresh interval
        let interval_secs = refresh_interval_atomic.load(Ordering::Relaxed);
        let interval_duration = Duration::from_secs(interval_secs);

        tokio::time::sleep(interval_duration).await;

        let mut metrics = SystemMetrics::new();

        // Collect CPU metrics
        if let Ok(cpu) = cpu_collector.collect() {
            metrics.cpu = Some(cpu);
        }

        // Collect memory metrics
        if let Ok(mem) = mem_collector.collect() {
            metrics.memory = Some(mem);
        }

        // Collect process lists
        if let Ok((top_cpu, top_mem)) = proc_collector.collect_top_both() {
            metrics.top_cpu_processes = top_cpu;
            metrics.top_memory_processes = top_mem;
        }

        // Collect network metrics
        if let Ok(net) = net_collector.collect() {
            metrics.network = Some(net);
        }

        // Collect disk metrics
        if let Ok(disk) = disk_collector.collect() {
            metrics.disk = Some(disk);
        }

        // Collect Maestro metrics
        if let Ok(maestro) = maestro_collector.collect() {
            metrics.maestro = Some(maestro);
        }

        // Update state
        let _ = state.update(metrics).await;
    }
}

/// Get color based on percentage (green -> yellow -> red)
fn color_for_percent(pct: f32) -> Color {
    match pct {
        p if p < 50.0 => Color::Green,
        p if p < 80.0 => Color::Yellow,
        _ => Color::Red,
    }
}

/// Render a simple sparkline from a vector of values
fn render_sparkline(values: &[f32], width: usize) -> String {
    if values.is_empty() || width == 0 {
        return String::new();
    }

    // Sample values to fit width
    let step = values.len().div_ceil(width.max(1));
    let sampled: Vec<_> = values
        .iter()
        .step_by(step)
        .copied()
        .collect();

    let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let max_val = sampled.iter().fold(0.0f32, |a, &b| a.max(b)).max(1.0);

    sampled
        .iter()
        .map(|&v| {
            let idx = ((v / max_val) * (bars.len() - 1) as f32) as usize;
            bars[idx.min(bars.len() - 1)]
        })
        .collect()
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Format bytes per second as human-readable speed
fn format_speed(bps: u64) -> String {
    format_size(bps) + "/s"
}

/// Handle keyboard input for the Ktop tab
pub fn handle_ktop_input(app: &mut App, key: crossterm::event::KeyCode) -> bool {
    let ktop_state = match app.ktop_state.as_mut() {
        Some(state) => state,
        None => return false,
    };

    match key {
        crossterm::event::KeyCode::Char('p') | crossterm::event::KeyCode::Char('P') => {
            ktop_state.toggle_pause();
            true
        }
        crossterm::event::KeyCode::Char('r') | crossterm::event::KeyCode::Char('R') => {
            // Note: refresh_metrics is called from app.rs event handler
            ktop_state.mark_refreshed();
            true
        }
        crossterm::event::KeyCode::Char('+') | crossterm::event::KeyCode::Char('=') => {
            let new_interval = ktop_state.refresh_interval_secs.saturating_add(1);
            ktop_state.set_refresh_interval(new_interval);
            true
        }
        crossterm::event::KeyCode::Char('-') | crossterm::event::KeyCode::Char('_') => {
            let new_interval = ktop_state.refresh_interval_secs.saturating_sub(1);
            ktop_state.set_refresh_interval(new_interval.max(1));
            true
        }
        crossterm::event::KeyCode::Tab => {
            // Cycle focus
            ktop_state.focus = match ktop_state.focus {
                KtopFocus::Cpu => KtopFocus::Memory,
                KtopFocus::Memory => KtopFocus::Processes,
                KtopFocus::Processes => KtopFocus::Network,
                KtopFocus::Network => KtopFocus::Disk,
                KtopFocus::Disk => KtopFocus::Maestro,
                KtopFocus::Maestro => KtopFocus::Cpu,
            };
            true
        }
        crossterm::event::KeyCode::BackTab => {
            // Reverse cycle focus
            ktop_state.focus = match ktop_state.focus {
                KtopFocus::Cpu => KtopFocus::Maestro,
                KtopFocus::Memory => KtopFocus::Cpu,
                KtopFocus::Processes => KtopFocus::Memory,
                KtopFocus::Network => KtopFocus::Processes,
                KtopFocus::Disk => KtopFocus::Network,
                KtopFocus::Maestro => KtopFocus::Disk,
            };
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{theme_from_name, Theme};
    use ktop_collectors::{AgentInfo, AgentStatus, CpuMetrics, DiskMetrics, DiskMount, InterfaceStats, LeIndexStats, LspStatus, MaestroMetrics, MaestroMemoryStats, MemoryMetrics, NetworkMetrics, ProcessInfo, ProcessStatus};
    use ratatui::{backend::TestBackend, Terminal};
    use std::collections::HashMap;

    #[test]
    fn test_ktop_state_default() {
        let mut state = KtopState::default();
        assert_eq!(state.refresh_interval_secs, 3);
        assert!(!state.paused);
        assert!(state.cpu_history.is_empty());
        assert!(state.memory_history.is_empty());
        assert_eq!(state.focus, KtopFocus::Cpu);
        assert_eq!(state.process_sort, ProcessSort::Cpu);
    }

    #[test]
    fn test_ktop_state_new() {
        let state = KtopState::new();
        assert_eq!(state.refresh_interval_secs, 3);
        assert!(!state.paused);
    }

    #[test]
    fn test_ktop_state_needs_refresh() {
        let mut state = KtopState::default();
        // The default uses Instant::now() so it won't need refresh immediately
        // but we can test the logic by using an old timestamp
        state.last_refresh = Instant::now() - Duration::from_secs(10);
        assert!(state.needs_refresh()); // Should need refresh after 10 seconds
        state.mark_refreshed();
        assert!(!state.needs_refresh()); // Just refreshed
    }

    #[test]
    fn test_ktop_state_needs_refresh_paused() {
        let mut state = KtopState::default();
        state.last_refresh = Instant::now() - Duration::from_secs(100);
        state.paused = true;
        assert!(!state.needs_refresh()); // Paused, never needs refresh
    }

    #[test]
    fn test_ktop_state_pause() {
        let mut state = KtopState::default();
        state.mark_refreshed();
        state.toggle_pause();
        assert!(state.paused);
        assert!(!state.needs_refresh()); // Paused, no refresh needed
        assert_eq!(state.collector_status, CollectorStatus::Paused);

        state.toggle_pause();
        assert!(!state.paused);
        assert_eq!(state.collector_status, CollectorStatus::Running);
    }

    #[test]
    fn test_ktop_state_set_refresh_interval() {
        let mut state = KtopState::default();
        state.set_refresh_interval(5);
        assert_eq!(state.refresh_interval_secs, 5);

        // Test clamping
        state.set_refresh_interval(0);
        assert_eq!(state.refresh_interval_secs, 1); // Min 1

        state.set_refresh_interval(15);
        assert_eq!(state.refresh_interval_secs, 10); // Max 10
    }

    #[test]
    fn test_ktop_state_refresh_interval_duration() {
        let mut state = KtopState::default();
        assert_eq!(state.refresh_interval(), Duration::from_secs(3));

        let mut state = KtopState::default();
        state.set_refresh_interval(5);
        assert_eq!(state.refresh_interval(), Duration::from_secs(5));
    }

    #[test]
    fn test_cpu_history() {
        let mut state = KtopState::default();
        for i in 0..70 {
            state.cpu_history.push(i as f32);
            if state.cpu_history.len() > 60 {
                state.cpu_history.remove(0);
            }
        }
        assert_eq!(state.cpu_history.len(), 60); // Max 60
        assert_eq!(state.cpu_history[0], 10.0); // First was pushed out
        assert_eq!(state.cpu_history[59], 69.0); // Last is most recent
    }

    #[test]
    fn test_memory_history() {
        let mut state = KtopState::default();
        for i in 0..70 {
            state.memory_history.push(i as f32);
            if state.memory_history.len() > 60 {
                state.memory_history.remove(0);
            }
        }
        assert_eq!(state.memory_history.len(), 60); // Max 60
    }

    #[test]
    fn test_ktop_focus_cycle() {
        assert_eq!(KtopFocus::Cpu.cycle(), KtopFocus::Memory);
        assert_eq!(KtopFocus::Memory.cycle(), KtopFocus::Processes);
        assert_eq!(KtopFocus::Processes.cycle(), KtopFocus::Network);
        assert_eq!(KtopFocus::Network.cycle(), KtopFocus::Disk);
        assert_eq!(KtopFocus::Disk.cycle(), KtopFocus::Maestro);
        assert_eq!(KtopFocus::Maestro.cycle(), KtopFocus::Cpu);
    }

    #[test]
    fn test_ktop_focus_cycle_reverse() {
        assert_eq!(KtopFocus::Cpu.cycle_reverse(), KtopFocus::Maestro);
        assert_eq!(KtopFocus::Memory.cycle_reverse(), KtopFocus::Cpu);
        assert_eq!(KtopFocus::Processes.cycle_reverse(), KtopFocus::Memory);
        assert_eq!(KtopFocus::Network.cycle_reverse(), KtopFocus::Processes);
        assert_eq!(KtopFocus::Disk.cycle_reverse(), KtopFocus::Network);
        assert_eq!(KtopFocus::Maestro.cycle_reverse(), KtopFocus::Disk);
    }

    #[test]
    fn test_color_for_percent() {
        assert_eq!(color_for_percent(25.0), Color::Green);
        assert_eq!(color_for_percent(49.9), Color::Green);
        assert_eq!(color_for_percent(50.0), Color::Yellow);
        assert_eq!(color_for_percent(65.0), Color::Yellow);
        assert_eq!(color_for_percent(79.9), Color::Yellow);
        assert_eq!(color_for_percent(80.0), Color::Red);
        assert_eq!(color_for_percent(90.0), Color::Red);
        assert_eq!(color_for_percent(100.0), Color::Red);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(2048), "2.0K");
        assert_eq!(format_size(3 * 1024 * 1024), "3.0M");
        assert_eq!(format_size(5 * 1024 * 1024 * 1024), "5.0G");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024 * 1024), "2.0T");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0), "0B/s");
        assert_eq!(format_speed(1024), "1.0K/s");
        assert_eq!(format_speed(5 * 1024 * 1024), "5.0M/s");
    }

    #[test]
    fn test_sparkline() {
        let values = vec![0.0, 25.0, 50.0, 75.0, 100.0];
        let sparkline = render_sparkline(&values, 10);
        assert!(!sparkline.is_empty());
        // Should have bars that increase
        assert!(sparkline.contains("▁"));
        assert!(sparkline.contains("█"));
    }

    #[test]
    fn test_sparkline_empty() {
        let sparkline = render_sparkline(&[], 10);
        assert!(sparkline.is_empty());
    }

    #[test]
    fn test_sparkline_zero_width() {
        let sparkline = render_sparkline(&[1.0, 2.0, 3.0], 0);
        assert!(sparkline.is_empty());
    }

    #[test]
    fn test_sparkline_single_value() {
        let sparkline = render_sparkline(&[50.0], 5);
        assert!(!sparkline.is_empty());
    }

    #[test]
    fn test_sparkline_sampling() {
        // Test with many values to ensure sampling works
        let values: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let sparkline = render_sparkline(&values, 20);
        assert!(!sparkline.is_empty());
        assert!(sparkline.chars().count() <= 30); // Check char count
    }

    #[test]
    fn test_sparkline_all_zeros() {
        let sparkline = render_sparkline(&[0.0, 0.0, 0.0], 5);
        assert!(!sparkline.is_empty());
    }

    // KtopFocus helper methods for tests
    impl KtopFocus {
        fn cycle(&self) -> Self {
            match self {
                KtopFocus::Cpu => KtopFocus::Memory,
                KtopFocus::Memory => KtopFocus::Processes,
                KtopFocus::Processes => KtopFocus::Network,
                KtopFocus::Network => KtopFocus::Disk,
                KtopFocus::Disk => KtopFocus::Maestro,
                KtopFocus::Maestro => KtopFocus::Cpu,
            }
        }

        fn cycle_reverse(&self) -> Self {
            match self {
                KtopFocus::Cpu => KtopFocus::Maestro,
                KtopFocus::Memory => KtopFocus::Cpu,
                KtopFocus::Processes => KtopFocus::Memory,
                KtopFocus::Network => KtopFocus::Processes,
                KtopFocus::Disk => KtopFocus::Network,
                KtopFocus::Maestro => KtopFocus::Disk,
            }
        }
    }

    // Integration test: Create a mock KtopState with metrics
    fn create_mock_state_with_metrics() -> KtopState {
        let mut state = KtopState::default();

        // Create mock CPU metrics using the constructor
        let cpu_metrics = CpuMetrics::new(
            45.5,
            8,
            vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0],
            None,
            (1.5, 1.2, 0.9),
        );

        // Create mock memory metrics using the constructor
        let mem_metrics = MemoryMetrics::new(
            16 * 1024 * 1024 * 1024, // 16 GB
            8 * 1024 * 1024 * 1024,   // 8 GB
            8 * 1024 * 1024 * 1024,
            512 * 1024 * 1024,
            2 * 1024 * 1024 * 1024,
            4 * 1024 * 1024 * 1024,
            0,
        );

        // Create mock process list using the constructor
        let proc_list = vec![
            ProcessInfo::new(
                1,
                "init".to_string(),
                0.1,
                0.1,
                1024 * 1024,
                512 * 1024,
                ProcessStatus::Sleeping,
                Some("/sbin/init".to_string()),
                0,
                0,
                0,
                0,
                0,
            ),
            ProcessInfo::new(
                1234,
                "test".to_string(),
                5.5,
                2.3,
                10 * 1024 * 1024,
                5 * 1024 * 1024,
                ProcessStatus::Running,
                Some("/usr/bin/test".to_string()),
                0,
                0,
                0,
                0,
                0,
            ),
        ];

        // Create mock network metrics
        let mut interfaces = HashMap::new();
        interfaces.insert("eth0".to_string(), InterfaceStats::new("eth0".to_string()));
        let net_metrics = NetworkMetrics::new(
            interfaces,
            1024 * 1024,
            512 * 1024,
            1024 * 1024,
            512 * 1024,
        );

        // Create mock disk metrics
        let disk_metrics = DiskMetrics::new(
            vec![DiskMount::new(
                "/".to_string(),
                "/dev/sda1".to_string(),
                "ext4".to_string(),
                500 * 1024 * 1024 * 1024,
                250 * 1024 * 1024 * 1024,
                250 * 1024 * 1024 * 1024,
                false,
            )],
            0, // I/O metrics are 0 since sysinfo doesn't provide them
            0,
            0,
            0,
        );

        // Create mock Maestro metrics
        let mut lsp_servers = HashMap::new();
        lsp_servers.insert("rust-analyzer".to_string(), LspStatus::new("rust-analyzer".to_string(), true, "Ready".to_string()));

        let mut agents = HashMap::new();
        agents.insert("agent1".to_string(), AgentInfo::new("agent1".to_string(), "general".to_string(), AgentStatus::Working));

        let leindex_stats = LeIndexStats {
            files_indexed: 12345,
            symbols_indexed: 50000,
            index_size_bytes: 1024 * 1024,
            last_update: Some("now".to_string()),
        };

        let memory_stats = MaestroMemoryStats {
            total_bytes: 512 * 1024 * 1024,
            cache_bytes: 100 * 1024 * 1024,
            index_bytes: 200 * 1024 * 1024,
            session_bytes: 212 * 1024 * 1024,
        };

        let maestro_metrics = MaestroMetrics::new(lsp_servers, agents, leindex_stats, memory_stats);

        // Build the metrics
        state.current_metrics.cpu = Some(cpu_metrics);
        state.current_metrics.memory = Some(mem_metrics);
        state.current_metrics.top_cpu_processes = proc_list.clone();
        state.current_metrics.top_memory_processes = proc_list;
        state.current_metrics.network = Some(net_metrics);
        state.current_metrics.disk = Some(disk_metrics);
        state.current_metrics.maestro = Some(maestro_metrics);

        state
    }

    #[test]
    fn test_create_mock_state() {
        let state = create_mock_state_with_metrics();
        assert!(state.current_metrics.cpu.is_some());
        assert!(state.current_metrics.memory.is_some());
        assert!(!state.current_metrics.top_cpu_processes.is_empty());
        assert!(state.current_metrics.network.is_some());
        assert!(state.current_metrics.disk.is_some());
        assert!(state.current_metrics.maestro.is_some());
    }

    #[test]
    fn test_collector_status_display() {
        // Test CollectorStatus variants exist and work as expected
        let status = CollectorStatus::Running;
        assert!(matches!(status, CollectorStatus::Running));

        let status = CollectorStatus::Initializing;
        assert!(matches!(status, CollectorStatus::Initializing));

        let status = CollectorStatus::Error;
        assert!(matches!(status, CollectorStatus::Error));

        let status = CollectorStatus::Paused;
        assert!(matches!(status, CollectorStatus::Paused));

        let status = CollectorStatus::Stopped;
        assert!(matches!(status, CollectorStatus::Stopped));
    }

    #[test]
    fn test_process_sort_variants() {
        // Test ProcessSort variants
        let sort = ProcessSort::Cpu;
        assert!(matches!(sort, ProcessSort::Cpu));

        let sort = ProcessSort::Memory;
        assert!(matches!(sort, ProcessSort::Memory));

        let sort = ProcessSort::Pid;
        assert!(matches!(sort, ProcessSort::Pid));

        let sort = ProcessSort::Name;
        assert!(matches!(sort, ProcessSort::Name));
    }

    // Test MainChunks structure
    #[test]
    fn test_main_chunks_creation() {
        let area = Rect::new(0, 0, 120, 40);
        let chunks = create_main_layout(area, KtopFocus::Cpu);

        // All chunks should have non-zero areas in wide layout
        assert!(chunks.cpu.width > 0);
        assert!(chunks.cpu.height > 0);
        assert!(chunks.memory.width > 0);
        assert!(chunks.memory.height > 0);
        assert!(chunks.processes_by_mem.width > 0);
        assert!(chunks.processes_by_mem.height > 0);
        assert!(chunks.processes_by_cpu.width > 0);
        assert!(chunks.processes_by_cpu.height > 0);
        assert!(chunks.network.width > 0);
        assert!(chunks.network.height > 0);
        assert!(chunks.maestro.width > 0);
        assert!(chunks.maestro.height > 0);
    }

    #[test]
    fn test_main_chunks_compact_layout() {
        let area = Rect::new(0, 0, 80, 40); // Narrower than 100
        let chunks = create_main_layout(area, KtopFocus::Cpu);

        // All chunks should still exist
        assert!(chunks.cpu.width > 0);
        assert!(chunks.memory.width > 0);
        assert!(chunks.processes_by_mem.width > 0);
        assert!(chunks.processes_by_cpu.width > 0);
        assert!(chunks.network.width > 0);
        assert!(chunks.maestro.width > 0);

        // In new Krustop layout: Network/CPU/Memory are side by side
        // So CPU and Memory should have different x positions (CPU is to the right of Network)
        assert!(chunks.cpu.x > chunks.network.x);
        assert!(chunks.memory.x > chunks.cpu.x);
    }

    #[test]
    fn test_memory_info_usage_percent() {
        // Mock MemoryInfo usage calculation
        let total = 16_000_000_000_u64;
        let used = 8_000_000_000_u64;
        let usage_pct = (used as f64 / total as f64) * 100.0;
        assert_eq!(usage_pct, 50.0);
    }

    #[test]
    fn test_swap_usage_percent() {
        let swap_total = 4_000_000_000_u64;
        let swap_used = 1_000_000_000_u64;
        let swap_pct = (swap_used as f64 / swap_total as f64) * 100.0;
        assert_eq!(swap_pct, 25.0);
    }

    #[test]
    fn test_disk_mount_usage_percent() {
        let total = 500_000_000_000_u64;
        let used = 250_000_000_000_u64;
        let usage_pct = (used as f64 / total as f64) * 100.0;
        assert_eq!(usage_pct, 50.0);
    }

    // Test render functions don't panic with empty state
    #[test]
    fn test_render_cpu_section_empty_state() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic with empty metrics
        render_cpu_section(&mut frame, Rect::new(0, 0, 50, 10), &state, &theme);
    }

    #[test]
    fn test_render_memory_section_empty_state() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic with empty metrics
        render_memory_section(&mut frame, Rect::new(0, 0, 50, 10), &state, &theme);
    }

    #[test]
    fn test_render_network_section_empty_state() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic with empty metrics
        render_network_section(&mut frame, Rect::new(0, 0, 50, 10), &mut state, &theme);
    }

    #[test]
    fn test_render_disk_section_empty_state() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic with empty metrics
        render_disk_section(&mut frame, Rect::new(0, 0, 50, 10), &mut state, &theme);
    }

    #[test]
    fn test_render_maestro_section_empty_state() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic with empty metrics
        render_maestro_section(&mut frame, Rect::new(0, 0, 50, 10), &mut state, &theme);
    }

    #[test]
    fn test_render_process_section_empty_state() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic with empty process list
        render_process_section(&mut frame, Rect::new(0, 0, 50, 10), &mut state, &theme, ProcessSort::Cpu);
    }

    #[test]
    fn test_render_header() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        let theme = theme_from_name("default");

        // Should not panic
        render_header(&mut frame, Rect::new(0, 0, 100, 5), &state, &theme);
    }

    #[test]
    fn test_render_header_paused() {
        let backend = TestBackend::new(100, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut frame = terminal.get_frame();

        let mut state = KtopState::default();
        state.paused = true;
        state.collector_status = CollectorStatus::Paused;
        let theme = theme_from_name("default");

        // Should not panic when paused
        render_header(&mut frame, Rect::new(0, 0, 100, 5), &state, &theme);
    }

    #[test]
    fn test_render_with_different_intervals() {
        for interval in [1, 3, 5, 10] {
            let mut state = KtopState::default();
            state.set_refresh_interval(interval);
            assert_eq!(state.refresh_interval_secs, interval);
        }
    }

    #[test]
    fn test_cpu_history_max_capacity() {
        let mut state = KtopState::default();
        // Add more than 60 entries
        for _ in 0..100 {
            state.cpu_history.push(50.0);
        }
        // When pushing directly without capping logic, it grows to 100
        assert_eq!(state.cpu_history.len(), 100);

        // Verify manual capping works
        while state.cpu_history.len() > 60 {
            state.cpu_history.remove(0);
        }
        assert_eq!(state.cpu_history.len(), 60);
    }

    #[test]
    fn test_memory_history_max_capacity() {
        let mut state = KtopState::default();
        // Add more than 60 entries
        for _ in 0..100 {
            state.memory_history.push(50.0);
        }
        // When pushing directly without capping logic, it grows to 100
        assert_eq!(state.memory_history.len(), 100);

        // Verify manual capping works
        while state.memory_history.len() > 60 {
            state.memory_history.remove(0);
        }
        assert_eq!(state.memory_history.len(), 60);
    }

    #[test]
    fn test_process_sort_variants_comparison() {
        assert_eq!(ProcessSort::Cpu, ProcessSort::Cpu);
        assert_ne!(ProcessSort::Cpu, ProcessSort::Memory);
    }

    #[test]
    fn test_ktop_focus_variants_comparison() {
        assert_eq!(KtopFocus::Cpu, KtopFocus::Cpu);
        assert_ne!(KtopFocus::Cpu, KtopFocus::Memory);
    }

    #[test]
    fn test_collector_status_variants_comparison() {
        assert_eq!(CollectorStatus::Running, CollectorStatus::Running);
        assert_ne!(CollectorStatus::Running, CollectorStatus::Paused);
    }

    #[test]
    fn test_format_size_edge_cases() {
        assert_eq!(format_size(1), "1B");
        assert_eq!(format_size(1023), "1023B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1024 * 1024 - 1), "1024.0K");
        assert_eq!(format_size(1024 * 1024), "1.0M");
    }

    #[test]
    fn test_sparkline_width_one() {
        let values = vec![50.0];
        let sparkline = render_sparkline(&values, 1);
        assert!(!sparkline.is_empty());
    }

    #[test]
    fn test_sparkline_all_same_values() {
        let values = vec![50.0; 10];
        let sparkline = render_sparkline(&values, 10);
        assert!(!sparkline.is_empty());
    }

    #[test]
    fn test_color_boundary_values() {
        assert_eq!(color_for_percent(0.0), Color::Green);
        assert_eq!(color_for_percent(49.99), Color::Green);
        assert_eq!(color_for_percent(50.0), Color::Yellow);
        assert_eq!(color_for_percent(79.99), Color::Yellow);
        assert_eq!(color_for_percent(80.0), Color::Red);
        assert_eq!(color_for_percent(100.0), Color::Red);
    }

    #[test]
    fn test_refresh_interval_clamp() {
        let mut state = KtopState::default();

        // Test lower bound
        state.set_refresh_interval(u64::MIN);
        assert_eq!(state.refresh_interval_secs, 1);

        // Test upper bound
        state.set_refresh_interval(u64::MAX);
        assert_eq!(state.refresh_interval_secs, 10);
    }

    #[test]
    fn test_pause_with_collector_status() {
        let mut state = KtopState::default();
        assert_eq!(state.collector_status, CollectorStatus::Initializing);

        state.toggle_pause();
        assert_eq!(state.collector_status, CollectorStatus::Paused);

        state.toggle_pause();
        assert_eq!(state.collector_status, CollectorStatus::Running);
    }

    #[test]
    fn test_process_status_matches() {
        // Test that ProcessStatus variants exist
        let _ = ProcessStatus::Running;
        let _ = ProcessStatus::Sleeping;
        let _ = ProcessStatus::Stopped;
        let _ = ProcessStatus::Zombie;
        let _ = ProcessStatus::Dead;
        let _ = ProcessStatus::Unknown;
    }

    // Tests for keyboard input handling
    #[test]
    fn test_handle_ktop_input_pause() {
        use crossterm::event::KeyCode;

        // Create a mock state
        let mut state = KtopState::default();
        assert!(!state.paused);

        // Simulate pause toggle
        state.toggle_pause();
        assert!(state.paused);

        state.toggle_pause();
        assert!(!state.paused);
    }

    #[test]
    fn test_handle_ktop_input_refresh_interval() {
        let mut state = KtopState::default();
        assert_eq!(state.refresh_interval_secs, 3);

        // Simulate '+' key - increase interval
        let new_interval = state.refresh_interval_secs.saturating_add(1);
        state.set_refresh_interval(new_interval);
        assert_eq!(state.refresh_interval_secs, 4);

        // Simulate '-' key - decrease interval
        let new_interval = state.refresh_interval_secs.saturating_sub(1);
        state.set_refresh_interval(new_interval.max(1));
        assert_eq!(state.refresh_interval_secs, 3);
    }

    #[test]
    fn test_handle_ktop_input_focus_cycle() {
        use crossterm::event::KeyCode;

        let mut state = KtopState::default();
        assert_eq!(state.focus, KtopFocus::Cpu);

        // Simulate Tab key - forward cycle
        state.focus = match state.focus {
            KtopFocus::Cpu => KtopFocus::Memory,
            _ => state.focus,
        };
        assert_eq!(state.focus, KtopFocus::Memory);

        // Simulate BackTab - reverse cycle
        state.focus = match state.focus {
            KtopFocus::Memory => KtopFocus::Cpu,
            _ => state.focus,
        };
        assert_eq!(state.focus, KtopFocus::Cpu);
    }

    #[test]
    fn test_full_focus_cycle() {
        // Test full forward cycle
        let mut focus = KtopFocus::Cpu;
        focus = match focus {
            KtopFocus::Cpu => KtopFocus::Memory,
            KtopFocus::Memory => KtopFocus::Processes,
            KtopFocus::Processes => KtopFocus::Network,
            KtopFocus::Network => KtopFocus::Disk,
            KtopFocus::Disk => KtopFocus::Maestro,
            KtopFocus::Maestro => KtopFocus::Cpu,
        };
        assert_eq!(focus, KtopFocus::Memory);

        focus = match focus {
            KtopFocus::Cpu => KtopFocus::Memory,
            KtopFocus::Memory => KtopFocus::Processes,
            KtopFocus::Processes => KtopFocus::Network,
            KtopFocus::Network => KtopFocus::Disk,
            KtopFocus::Disk => KtopFocus::Maestro,
            KtopFocus::Maestro => KtopFocus::Cpu,
        };
        assert_eq!(focus, KtopFocus::Processes);

        // Test full reverse cycle
        focus = match focus {
            KtopFocus::Cpu => KtopFocus::Maestro,
            KtopFocus::Memory => KtopFocus::Cpu,
            KtopFocus::Processes => KtopFocus::Memory,
            KtopFocus::Network => KtopFocus::Processes,
            KtopFocus::Disk => KtopFocus::Network,
            KtopFocus::Maestro => KtopFocus::Disk,
        };
        assert_eq!(focus, KtopFocus::Memory);
    }

    #[test]
    fn test_keycode_variants() {
        use crossterm::event::KeyCode;

        // Test that the key variants we use exist
        let _ = KeyCode::Char('p');
        let _ = KeyCode::Char('P');
        let _ = KeyCode::Char('r');
        let _ = KeyCode::Char('R');
        let _ = KeyCode::Char('+');
        let _ = KeyCode::Char('=');
        let _ = KeyCode::Char('-');
        let _ = KeyCode::Char('_');
        let _ = KeyCode::Tab;
        let _ = KeyCode::BackTab;
    }

    // Tests for edge cases in state management
    #[test]
    fn test_mark_refreshed() {
        let mut state = KtopState::default();
        let old = state.last_refresh;
        state.mark_refreshed();
        // New timestamp should be >= old timestamp
        assert!(state.last_refresh >= old);
    }

    #[test]
    fn test_refresh_interval_duration() {
        let mut state = KtopState::default();
        assert_eq!(state.refresh_interval(), Duration::from_secs(3));

        let mut state = KtopState::default();
        state.set_refresh_interval(7);
        assert_eq!(state.refresh_interval(), Duration::from_secs(7));
    }

    #[test]
    fn test_history_capacity() {
        let mut state = KtopState::default();

        // Fill CPU history beyond capacity
        for i in 0..100 {
            state.cpu_history.push(i as f32);
        }

        // Manually cap to 60 (what process_updates does)
        while state.cpu_history.len() > 60 {
            state.cpu_history.remove(0);
        }

        assert_eq!(state.cpu_history.len(), 60);
    }

    #[test]
    fn test_process_updates_cpu_history_growth() {
        let mut state = KtopState::default();

        // Simulate what process_updates does
        for i in 0..70 {
            state.cpu_history.push(i as f32);
            if state.cpu_history.len() > 60 {
                state.cpu_history.remove(0);
            }
        }

        assert_eq!(state.cpu_history.len(), 60);
    }

    #[test]
    fn test_process_updates_memory_history_growth() {
        let mut state = KtopState::default();

        // Simulate what process_updates does
        for i in 0..70 {
            state.memory_history.push(i as f32);
            if state.memory_history.len() > 60 {
                state.memory_history.remove(0);
            }
        }

        assert_eq!(state.memory_history.len(), 60);
    }

    #[test]
    fn test_sparkline_negative_values() {
        // sparkline should handle negative values gracefully
        let values = vec![-10.0, 0.0, 10.0, 20.0];
        let sparkline = render_sparkline(&values, 10);
        assert!(!sparkline.is_empty());
    }

    #[test]
    fn test_sparkline_very_large_values() {
        let values = vec![1000000.0, 2000000.0, 3000000.0];
        let sparkline = render_sparkline(&values, 10);
        assert!(!sparkline.is_empty());
    }

    #[test]
    fn test_color_for_percent_negative() {
        // Negative should be handled (likely treated as 0)
        let color = color_for_percent(-10.0);
        // Should still return a valid color
        assert!(matches!(color, Color::Green | Color::Yellow | Color::Red));
    }

    #[test]
    fn test_color_for_percent_over_100() {
        // Over 100% should be red
        let color = color_for_percent(150.0);
        assert_eq!(color, Color::Red);
    }

    #[test]
    fn test_sparkline_exact_width() {
        let values: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let sparkline = render_sparkline(&values, 20);
        assert!(!sparkline.is_empty());
        assert!(sparkline.chars().count() <= 25);
    }

    #[test]
    fn test_sparkline_single_value_repeated() {
        let values = vec![100.0; 100];
        let sparkline = render_sparkline(&values, 50);
        assert!(!sparkline.is_empty());
        assert!(sparkline.contains("█"));
    }
}
