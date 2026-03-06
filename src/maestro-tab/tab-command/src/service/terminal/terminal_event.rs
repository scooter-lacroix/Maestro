use crate::prelude::*;
use crate::{
    env::terminal_size, message::terminal::TerminalInput, state::terminal::TerminalSizeState,
};
use anyhow::Context;
use crossterm::event::Event;
use std::time::Duration;

/// Broadcasts the TerminalSizeState, and sends TerminalSend::Resize events.
pub struct TerminalEventService {
    _update: Lifeline,
}

impl Service for TerminalEventService {
    type Bus = TerminalBus;
    type Lifeline = anyhow::Result<Self>;

    fn spawn(bus: &TerminalBus) -> Self::Lifeline {
        let mut tx = bus.tx::<TerminalSizeState>()?;
        let mut tx_send = bus.tx::<TerminalInput>()?;

        #[allow(unreachable_code)]
        let _update = Self::try_task("run", async move {
            let mut last_state: Option<TerminalSizeState> = None;
            loop {
                let size = terminal_size().expect("get terminal size");
                let state = TerminalSizeState(size);

                // Use the dimension methods to check for changes
                let dims_changed = last_state.as_ref().map_or(true, |last| {
                    last.dimensions() != state.dimensions()
                });

                if dims_changed {
                    debug!(
                        "Terminal size changed: {} cols x {} rows",
                        state.cols(),
                        state.rows()
                    );

                    last_state = Some(state.clone());

                    tx.send(state.clone())
                        .await
                        .context("send TerminalStateSize")?;

                    tx_send
                        .send(TerminalInput::Resize(state.dimensions()))
                        .await
                        .context("send TerminalInput::Resize")?;
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            Ok(())
        });

        Ok(Self { _update })
    }
}

fn _block_for_event() -> Option<Event> {
    if crossterm::event::poll(Duration::from_millis(500)).unwrap_or(false) {
        crossterm::event::read().ok()
    } else {
        None
    }
}
