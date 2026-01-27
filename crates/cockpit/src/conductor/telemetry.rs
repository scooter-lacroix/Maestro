//! Telemetry broadcast system for Cockpit Conductor
//!
//! Internal event bus to decouple engine polling from UI rendering.

use tokio::sync::broadcast;
use crate::conductor::model::ConductorEvent;

/// Global telemetry bus
pub struct TelemetryBus {
    tx: broadcast::Sender<ConductorEvent>,
}

impl TelemetryBus {
    /// Create a new telemetry bus
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    /// Get a subscriber to the bus
    pub fn subscribe(&self) -> broadcast::Receiver<ConductorEvent> {
        self.tx.subscribe()
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: ConductorEvent) {
        let _ = self.tx.send(event);
    }
}

lazy_static::lazy_static! {
    /// Global instance of the telemetry bus
    pub static ref BUS: TelemetryBus = TelemetryBus::new();
}
