//! I/O Bridge Service for direct stdin/stdout forwarding
//!
//! This service bridges SendStdout and SendStdin messages to the websocket,
//! enabling direct I/O operations with specific tabs.

use crate::{
    message::main::{SendStdin, SendStdout},
    prelude::*,
};
use tab_api::client::Request;

pub struct MainIoBridgeService {
    _run: Lifeline,
}

impl Service for MainIoBridgeService {
    type Bus = MainBus;
    type Lifeline = anyhow::Result<Self>;

    fn spawn(bus: &Self::Bus) -> Self::Lifeline {
        let mut rx_stdout = bus.rx::<SendStdout>()?;
        let mut rx_stdin = bus.rx::<SendStdin>()?;
        let mut tx_request = bus.tx::<Request>()?;

        let _run = Self::try_task("run", async move {
            loop {
                tokio::select! {
                    Some(send_stdout) = rx_stdout.recv() => {
                        let SendStdout(tab_id, output_chunk) = send_stdout;
                        debug!("Forwarding stdout for tab {}: {} bytes", tab_id.0, output_chunk.data.len());
                        // stdout is received from the daemon, not sent
                        // This message type is for future use in output handling
                    }
                    Some(send_stdin) = rx_stdin.recv() => {
                        let SendStdin(tab_id, input_chunk) = send_stdin;
                        debug!("Forwarding stdin for tab {}: {} bytes", tab_id.0, input_chunk.data.len());
                        tx_request.send(Request::Input(tab_id, input_chunk)).await?;
                    }
                    else => break,
                }
            }

            Ok(())
        });

        Ok(Self { _run })
    }
}
