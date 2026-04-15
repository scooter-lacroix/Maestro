use anyhow::Result;
use chrono::Utc;

use super::pty_bridge::{PtyBridge, PtyEvent, PtyLaunchConfig};
use super::{AgentOutputLine, ClawSession, ClawSessionStatus, OutputLineType};

/// High-level orchestration loop for a MaestroClaw CLI-backed session.
pub struct ClawLoop {
    pub session: ClawSession,
    bridge: PtyBridge,
}

impl ClawLoop {
    pub fn launch(session: ClawSession, launch: PtyLaunchConfig) -> Result<Self> {
        let bridge = PtyBridge::launch(launch)?;
        Ok(Self { session, bridge })
    }

    pub fn submit(&mut self, input: &str) -> Result<()> {
        self.bridge.send_input(input)
    }

    pub fn poll(&mut self) -> Vec<AgentOutputLine> {
        let mut lines = Vec::new();
        let mut exited = false;
        let mut errored = false;

        for event in self.bridge.poll_events() {
            match event {
                PtyEvent::OutputLine(content) => {
                    lines.push(AgentOutputLine {
                        timestamp: Utc::now(),
                        content,
                        line_type: OutputLineType::AgentText,
                    });
                }
                PtyEvent::Exited(code) => {
                    exited = true;
                    self.session.status = ClawSessionStatus::Stopped;
                    lines.push(AgentOutputLine {
                        timestamp: Utc::now(),
                        content: format!("process exited with code {:?}", code),
                        line_type: OutputLineType::SystemMessage,
                    });
                }
                PtyEvent::Error(content) => {
                    errored = true;
                    self.session.status = ClawSessionStatus::Error;
                    lines.push(AgentOutputLine {
                        timestamp: Utc::now(),
                        content,
                        line_type: OutputLineType::Error,
                    });
                }
            }
        }

        if !exited && !errored && self.bridge.is_running() {
            self.session.status = ClawSessionStatus::Running;
        }

        lines
    }

    pub fn stop(&mut self) -> Result<()> {
        self.session.status = ClawSessionStatus::Stopped;
        self.bridge.terminate()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.bridge.resize(rows, cols)
    }
}
