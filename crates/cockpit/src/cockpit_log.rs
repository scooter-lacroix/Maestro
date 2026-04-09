//! Per-session file logging for the Maestro Cockpit TUI.
//!
//! Creates a compact log file for each cockpit process lifetime, stored under
//! `~/.maestro/logs/cockpit/`. A JSONL manifest (`sessions.jsonl`) provides
//! fast discovery of past sessions.

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Maximum log age before auto-pruning (30 days).
const MAX_LOG_AGE_DAYS: u64 = 30;

/// Directory for cockpit log files, relative to ~/.maestro/
const COCKPIT_LOG_SUBDIR: &str = "logs/cockpit";

/// Name of the session manifest file.
const MANIFEST_FILE: &str = "sessions.jsonl";

/// Shared inner state between the tracing writer and the guard.
struct SharedWriter {
    writer: BufWriter<File>,
    line_count: u64,
}

/// Handle returned by [`init`]. Drop to flush and finalize the manifest entry.
pub struct CockpitLogGuard {
    log_path: PathBuf,
    manifest_path: PathBuf,
    started: DateTime<Utc>,
    shared: Arc<Mutex<SharedWriter>>,
    /// Thread-local subscriber guard, used when global subscriber is already set.
    _thread_local_guard: Option<tracing::subscriber::DefaultGuard>,
}

/// One entry in the sessions manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    /// Process ID of the cockpit process.
    pub pid: u32,
    /// ISO 8601 timestamp when the cockpit session started.
    pub started: DateTime<Utc>,
    /// ISO 8601 timestamp when the cockpit session ended (None if still running).
    pub ended: Option<DateTime<Utc>>,
    /// Filename of the log file (relative to the cockpit log directory).
    pub log_file: String,
}

/// Returns the cockpit log directory: `~/.maestro/logs/cockpit/`
fn log_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".maestro").join(COCKPIT_LOG_SUBDIR))
}

/// Build a log filename from timestamp and PID.
fn log_filename(now: &DateTime<Local>, pid: u32) -> String {
    format!(
        "cockpit-{}-{}.log",
        now.format("%Y%m%d_%H%M%S"),
        pid
    )
}

/// Prune log files and manifest entries older than MAX_LOG_AGE_DAYS.
fn prune_old_logs(log_dir: &Path) {
    let cutoff = Local::now() - chrono::Duration::days(MAX_LOG_AGE_DAYS as i64);
    let cutoff_str = cutoff.format("%Y%m%d").to_string();

    // Load active session filenames (ended == None) to protect from deletion.
    // Only treat as active if started within the retention window — sessions
    // with ended == None that are older than MAX_LOG_AGE_DAYS are stale orphans
    // (crashed without Drop) and should be pruned.
    let cutoff_utc = cutoff.with_timezone(&Utc);
    let manifest_path = log_dir.join(MANIFEST_FILE);
    let active_log_files: HashSet<String> = if manifest_path.exists() {
        fs::read_to_string(&manifest_path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                serde_json::from_str::<SessionEntry>(line)
                    .ok()
                    .filter(|e| e.ended.is_none() && e.started > cutoff_utc)
                    .map(|e| e.log_file)
            })
            .collect()
    } else {
        HashSet::new()
    };

    // Prune log files (skip files belonging to still-active sessions)
    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Only prune cockpit log files, never the manifest
            if name_str.starts_with("cockpit-") && name_str.ends_with(".log") {
                // Skip if this file belongs to an active session
                if active_log_files.contains(name_str.as_ref()) {
                    continue;
                }
                // Extract date portion: cockpit-YYYYMMDD_...
                if let Some(date_part) = name_str.strip_prefix("cockpit-") {
                    if let Some(date) = date_part.get(..8) {
                        if date < cutoff_str.as_str() {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    // Prune manifest entries
    if !manifest_path.exists() {
        return;
    }
    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let kept: Vec<String> = content
        .lines()
        .filter(|line| {
            serde_json::from_str::<SessionEntry>(line)
                .map(|e| e.started > cutoff_utc || e.ended.map_or(true, |t| t > cutoff_utc))
                .unwrap_or(true)
        })
        .map(String::from)
        .collect();
    // Use atomic write via temp file + rename to avoid clobbering concurrent updates
    let tmp_path = manifest_path.with_extension("jsonl.tmp");
    if fs::write(&tmp_path, kept.join("\n") + "\n").is_ok() {
        let _ = fs::rename(&tmp_path, &manifest_path);
    }
}

/// Atomically append a session entry to the manifest as a single write.
fn append_manifest_entry(path: &Path, entry: &SessionEntry) -> Result<()> {
    let line = serde_json::to_string(entry)? + "\n";
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())?;
    Ok(())
}

/// Initialize per-session file logging. Returns a guard that flushes on drop.
///
/// Creates the log file, writes the manifest entry, and installs a
/// `tracing-subscriber` that writes compact log lines to the file.
pub fn init() -> Result<CockpitLogGuard> {
    let dir = log_dir()?;
    fs::create_dir_all(&dir)?;

    // Prune old logs on startup
    prune_old_logs(&dir);

    let now = Local::now();
    let pid = std::process::id();
    let filename = log_filename(&now, pid);
    let log_path = dir.join(&filename);
    let manifest_path = dir.join(MANIFEST_FILE);

    // Open log file
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&log_path)
        .context("Failed to create cockpit log file")?;
    let mut writer = BufWriter::new(file);

    // Write header
    writeln!(
        writer,
        "# Maestro Cockpit Session Log - {} (PID {})",
        now.format("%Y-%m-%d %H:%M:%S %Z"),
        pid
    )?;
    writeln!(
        writer,
        "# Log file: {}",
        log_path.display()
    )?;
    writeln!(writer, "---")?;
    writer.flush()?;

    let shared = Arc::new(Mutex::new(SharedWriter {
        writer,
        line_count: 0,
    }));

    // Write manifest entry
    let entry = SessionEntry {
        pid,
        started: now.with_timezone(&Utc),
        ended: None,
        log_file: filename,
    };
    append_manifest_entry(&manifest_path, &entry)?;

    // Install tracing subscriber writing to the shared writer.
    let make_writer = CockpitMakeWriter {
        shared: shared.clone(),
    };
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(
                        "maestro=info,maestro_cockpit=info,leindex=info",
                    )
                }),
        )
        .with_writer(make_writer)
        .with_ansi(false)
        .compact()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    let subscriber = subscriber.finish();

    // Try to set as global default. If a subscriber is already installed
    // (e.g. the `maestro` CLI init'd one before calling cockpit::run()),
    // fall back to a thread-local default so the main event loop still
    // writes to our log file.
    let global_ok = tracing::subscriber::set_global_default(subscriber).is_ok();

    tracing::info!("Cockpit logging initialized — PID {} (global={})", pid, global_ok);

    // When the global subscriber is already set (e.g. the maestro CLI installed one),
    // we cannot replace it. The thread-local fallback below covers the main TUI event
    // loop thread but NOT spawned tasks, which inherit the pre-existing global subscriber.
    // This is acceptable because the critical logging path is the main TUI thread;
    // background tasks (McpPool, etc.) will use whatever global subscriber was already set.

    let _thread_local_guard = if !global_ok {
        // Install as thread-local default for the main event loop thread.
        // This requires a fresh subscriber since the first one was consumed by
        // set_global_default(). Spawned tokio tasks will NOT be captured by this —
        // they use the pre-existing global subscriber.
        let make_writer2 = CockpitMakeWriter {
            shared: shared.clone(),
        };
        let sub2 = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| {
                        tracing_subscriber::EnvFilter::new(
                            "maestro=info,maestro_cockpit=info,leindex=info",
                        )
                    }),
            )
            .with_writer(make_writer2)
            .with_ansi(false)
            .compact()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .finish();
        Some(tracing::subscriber::set_default(sub2))
    } else {
        None
    };

    Ok(CockpitLogGuard {
        log_path,
        manifest_path,
        started: now.with_timezone(&Utc),
        shared: shared.clone(),
        _thread_local_guard,
    })
}

impl Drop for CockpitLogGuard {
    fn drop(&mut self) {
        // Flush the shared writer (recover from poisoned mutex)
        let mut guard = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        let _ = guard.writer.flush();

        // Write completion entry to manifest
        let ended = Utc::now();
        let filename = self
            .log_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let entry = SessionEntry {
            pid: std::process::id(),
            started: self.started,
            ended: Some(ended),
            log_file: filename,
        };

        let _ = append_manifest_entry(&self.manifest_path, &entry);
    }
}

// ---------- Internal: tracing-compatible writer ----------

/// A `MakeWriter` impl that produces write guards for the shared log file.
struct CockpitMakeWriter {
    shared: Arc<Mutex<SharedWriter>>,
}

/// A write guard that dereferences to the shared BufWriter.
struct CockpitWriteGuard<'a> {
    lock: std::sync::MutexGuard<'a, SharedWriter>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CockpitMakeWriter {
    type Writer = CockpitWriteGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        CockpitWriteGuard {
            lock: self.shared.lock().unwrap_or_else(|e| e.into_inner()),
        }
    }
}

impl Write for CockpitWriteGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.lock.writer.write(buf)?;
        // Count newlines for approximate line tracking
        self.lock.line_count += buf.iter().filter(|&&b| b == b'\n').count() as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.lock.writer.flush()
    }
}

// ---------- Log discovery ----------

/// List all past cockpit sessions from the manifest, most recent first.
pub fn list_sessions() -> Result<Vec<SessionEntry>> {
    let dir = log_dir()?;
    let manifest_path = dir.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&manifest_path)?;
    let mut map: HashMap<(u32, DateTime<Utc>), SessionEntry> = HashMap::new();
    for entry in content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str::<SessionEntry>(l).ok())
    {
        let key = (entry.pid, entry.started);
        map.entry(key)
            .and_modify(|existing| {
                // Prefer the entry with ended = Some(_) (completed session)
                if existing.ended.is_none() && entry.ended.is_some() {
                    *existing = entry.clone();
                }
            })
            .or_insert(entry);
    }
    let mut entries: Vec<SessionEntry> = map.into_values().collect();
    entries.sort_by(|a, b| b.started.cmp(&a.started));
    Ok(entries)
}

/// Find the log file path for a given PID and start time.
pub fn find_log(pid: u32, started: &DateTime<Utc>) -> Option<PathBuf> {
    let dir = log_dir().ok()?;
    let sessions = list_sessions().ok()?;
    sessions
        .into_iter()
        .find(|s| s.pid == pid && s.started == *started)
        .map(|s| dir.join(s.log_file))
}

/// Read the last N lines from a session's log file.
pub fn tail_log(log_path: &Path, n: usize) -> Result<Vec<String>> {
    let content = fs::read_to_string(log_path)?;
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.starts_with('#') && *l != "---")
        .rev()
        .take(n)
        .map(String::from)
        .collect();
    Ok(lines.into_iter().rev().collect())
}
