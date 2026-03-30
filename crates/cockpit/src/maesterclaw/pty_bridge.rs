use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Line-oriented runtime events emitted by the bridge.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    OutputLine(String),
    Exited(Option<i32>),
    Error(String),
}

/// Launch configuration for a CLI-backed claw session.
#[derive(Debug, Clone, Default)]
pub struct PtyLaunchConfig {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub rows: u16,
    pub cols: u16,
}

pub struct PtyBridge {
    master: Box<dyn portable_pty::MasterPty + Send>,
    stdin: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    events: Receiver<PtyEvent>,
    exit_reported: bool,
}

impl PtyBridge {
    pub fn launch(config: PtyLaunchConfig) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows.max(24),
                cols: config.cols.max(80),
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("failed to allocate pseudo terminal")?;

        let mut command = CommandBuilder::new(&config.program);
        command.args(&config.args);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        if let Some(cwd) = &config.cwd {
            command.cwd(cwd);
        }

        for (key, value) in &config.env {
            command.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(command)
            .with_context(|| format!("failed to launch {}", config.program))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("failed to clone pty reader")?;
        let stdin = pair
            .master
            .take_writer()
            .context("failed to create pty writer")?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut pending = Vec::new();

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        if !pending.is_empty() {
                            let line = String::from_utf8_lossy(&pending).to_string();
                            let _ = tx.send(PtyEvent::OutputLine(line));
                        }
                        break;
                    }
                    Ok(n) => {
                        pending.extend_from_slice(&buf[..n]);
                        while let Some(pos) = pending.iter().position(|byte| *byte == b'\n') {
                            let mut line = pending.drain(..=pos).collect::<Vec<_>>();
                            while matches!(line.last(), Some(b'\n' | b'\r')) {
                                line.pop();
                            }
                            let text = String::from_utf8_lossy(&line).to_string();
                            let _ = tx.send(PtyEvent::OutputLine(text));
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(PtyEvent::Error(err.to_string()));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            child,
            stdin,
            events: rx,
            exit_reported: false,
        })
    }

    pub fn send_input(&mut self, input: &str) -> Result<()> {
        let stdin = &mut self.stdin;
        stdin.write_all(input.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    pub fn poll_events(&mut self) -> Vec<PtyEvent> {
        let mut drained = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            drained.push(event);
        }

        if !self.exit_reported {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.exit_reported = true;
                drained.push(PtyEvent::Exited(Some(status.exit_code() as i32)));
            }
        }

        drained
    }

    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows: rows.max(24),
                cols: cols.max(80),
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("failed to resize pty")
    }

    pub fn terminate(&mut self) -> Result<()> {
        if self.is_running() {
            self.child
                .kill()
                .context("failed to terminate claw process")?;
        }
        Ok(())
    }
}
