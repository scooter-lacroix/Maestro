//! Tab Close Service
//!
//! Handles RequestTabClose messages by forwarding them to the websocket as CloseTab requests.

use crate::{message::tabs::RequestTabClose, prelude::*};
use tab_api::client::Request;

pub struct CloseTabService {
    _run: Lifeline,
}

impl Service for CloseTabService {
    type Bus = TabBus;
    type Lifeline = anyhow::Result<Self>;

    fn spawn(bus: &Self::Bus) -> Self::Lifeline {
        let mut rx = bus.rx::<RequestTabClose>()?;
        let mut tx_request = bus.tx::<Request>()?;

        let _run = Self::try_task("run", async move {
            while let Some(RequestTabClose(tab_id)) = rx.recv().await {
                debug!("Closing tab: {:?}", tab_id);
                tx_request.send(Request::CloseTab(tab_id)).await?;
            }

            Ok(())
        });

        Ok(Self { _run })
    }
}
