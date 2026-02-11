//! State management for metrics
//!
//! This module provides reactive state management with delta updates.

use crate::types::SystemMetrics;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::sync::broadcast;

/// Default channel capacity for state updates
const DEFAULT_CHANNEL_CAPACITY: usize = 100;

/// Metrics state with reactive updates
#[derive(Clone)]
pub struct MetricsState {
    /// Inner state wrapped in Arc<RwLock> for thread-safe access
    inner: Arc<RwLock<MetricsStateInner>>,

    /// Sender for state updates
    tx: broadcast::Sender<StateUpdate>,
}

/// Inner state data
struct MetricsStateInner {
    /// Current metrics
    pub metrics: SystemMetrics,

    /// Last update timestamp
    pub last_update: Instant,

    /// Update counter
    pub update_count: u64,
}

/// State update notification
#[derive(Debug, Clone)]
pub struct StateUpdate {
    /// What changed in this update
    pub changes: UpdateFlags,

    /// New metrics (partial or full)
    pub metrics: SystemMetrics,

    /// Update index (for ordering)
    pub index: u64,

    /// Time of this update
    pub timestamp: Instant,
}

/// Flags indicating what changed in an update
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpdateFlags {
    /// CPU metrics changed
    pub cpu: bool,

    /// Memory metrics changed
    pub memory: bool,

    /// Process lists changed
    pub processes: bool,

    /// Network metrics changed
    pub network: bool,

    /// Disk metrics changed
    pub disk: bool,

    /// Maestro metrics changed
    pub maestro: bool,
}

impl UpdateFlags {
    /// Create empty flags (nothing changed)
    pub fn none() -> Self {
        Self::default()
    }

    /// Create flags indicating everything changed
    pub fn all() -> Self {
        Self {
            cpu: true,
            memory: true,
            processes: true,
            network: true,
            disk: true,
            maestro: true,
        }
    }

    /// Check if any flag is set
    pub fn is_empty(&self) -> bool {
        !(self.cpu || self.memory || self.processes || self.network || self.disk || self.maestro)
    }

    /// Create flags from comparing two metrics states
    pub fn from_metrics(old: &SystemMetrics, new: &SystemMetrics) -> Self {
        Self {
            cpu: Self::cpu_changed(old, new),
            memory: Self::memory_changed(old, new),
            processes: Self::processes_changed(old, new),
            network: Self::network_changed(old, new),
            disk: Self::disk_changed(old, new),
            maestro: Self::maestro_changed(old, new),
        }
    }

    /// Check if CPU metrics meaningfully changed
    fn cpu_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
        match (&old.cpu, &new.cpu) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(o), Some(n)) => {
                (o.usage_percent - n.usage_percent).abs() > 1.0
                    || o.core_count != n.core_count
            }
        }
    }

    /// Check if memory metrics meaningfully changed
    fn memory_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
        match (&old.memory, &new.memory) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(o), Some(n)) => {
                // Check for >1% change
                let old_pct = o.usage_percent();
                let new_pct = n.usage_percent();
                (old_pct - new_pct).abs() > 1.0
            }
        }
    }

    /// Check if process lists changed
    fn processes_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
        old.top_cpu_processes != new.top_cpu_processes
            || old.top_memory_processes != new.top_memory_processes
    }

    /// Check if network metrics meaningfully changed
    fn network_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
        match (&old.network, &new.network) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(_), Some(_)) => true, // Always consider network "changed" due to counters
        }
    }

    /// Check if disk metrics meaningfully changed
    fn disk_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
        match (&old.disk, &new.disk) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(_), Some(_)) => true, // Always consider disk "changed" due to I/O counters
        }
    }

    /// Check if Maestro metrics changed
    fn maestro_changed(old: &SystemMetrics, new: &SystemMetrics) -> bool {
        match (&old.maestro, &new.maestro) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(o), Some(n)) => {
                o.lsp_servers != n.lsp_servers
                    || o.agents != n.agents
                    || o.leindex.files_indexed != n.leindex.files_indexed
            }
        }
    }
}

impl MetricsState {
    /// Create a new metrics state
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);

        Self {
            inner: Arc::new(RwLock::new(MetricsStateInner {
                metrics: SystemMetrics::new(),
                last_update: Instant::now(),
                update_count: 0,
            })),
            tx,
        }
    }

    /// Create a new metrics state with a custom channel capacity
    pub fn with_channel_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);

        Self {
            inner: Arc::new(RwLock::new(MetricsStateInner {
                metrics: SystemMetrics::new(),
                last_update: Instant::now(),
                update_count: 0,
            })),
            tx,
        }
    }

    /// Update metrics with delta optimization
    pub async fn update(&self, new_metrics: SystemMetrics) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        let old_metrics = inner.metrics.clone();

        // Calculate what changed
        let changes = UpdateFlags::from_metrics(&old_metrics, &new_metrics);

        // Update state
        inner.metrics = new_metrics;
        inner.last_update = Instant::now();
        inner.update_count += 1;

        // Send notification
        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        // Ignore send errors (no receivers)
        let _ = self.tx.send(update);

        Ok(changes)
    }

    /// Update only CPU metrics
    pub async fn update_cpu(&self, cpu: crate::types::CpuMetrics) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        inner.metrics.cpu = Some(cpu);
        inner.last_update = Instant::now();
        inner.update_count += 1;

        let changes = UpdateFlags { cpu: true, ..Default::default() };

        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        let _ = self.tx.send(update);
        Ok(changes)
    }

    /// Update only memory metrics
    pub async fn update_memory(&self, memory: crate::types::MemoryMetrics) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        inner.metrics.memory = Some(memory);
        inner.last_update = Instant::now();
        inner.update_count += 1;

        let changes = UpdateFlags { memory: true, ..Default::default() };

        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        let _ = self.tx.send(update);
        Ok(changes)
    }

    /// Update only process lists
    pub async fn update_processes(&self, top_cpu: Vec<crate::types::ProcessInfo>, top_memory: Vec<crate::types::ProcessInfo>) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        inner.metrics.top_cpu_processes = top_cpu;
        inner.metrics.top_memory_processes = top_memory;
        inner.last_update = Instant::now();
        inner.update_count += 1;

        let changes = UpdateFlags { processes: true, ..Default::default() };

        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        let _ = self.tx.send(update);
        Ok(changes)
    }

    /// Update only network metrics
    pub async fn update_network(&self, network: crate::types::NetworkMetrics) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        inner.metrics.network = Some(network);
        inner.last_update = Instant::now();
        inner.update_count += 1;

        let changes = UpdateFlags { network: true, ..Default::default() };

        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        let _ = self.tx.send(update);
        Ok(changes)
    }

    /// Update only disk metrics
    pub async fn update_disk(&self, disk: crate::types::DiskMetrics) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        inner.metrics.disk = Some(disk);
        inner.last_update = Instant::now();
        inner.update_count += 1;

        let changes = UpdateFlags { disk: true, ..Default::default() };

        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        let _ = self.tx.send(update);
        Ok(changes)
    }

    /// Update only Maestro metrics
    pub async fn update_maestro(&self, maestro: crate::types::MaestroMetrics) -> Result<UpdateFlags, broadcast::error::SendError<()>> {
        let mut inner = self.inner.write().await;
        inner.metrics.maestro = Some(maestro);
        inner.last_update = Instant::now();
        inner.update_count += 1;

        let changes = UpdateFlags { maestro: true, ..Default::default() };

        let update = StateUpdate {
            changes,
            metrics: inner.metrics.clone(),
            index: inner.update_count,
            timestamp: inner.last_update,
        };

        let _ = self.tx.send(update);
        Ok(changes)
    }

    /// Subscribe to state updates
    pub fn subscribe(&self) -> broadcast::Receiver<StateUpdate> {
        self.tx.subscribe()
    }

    /// Get a snapshot of current metrics
    pub async fn snapshot(&self) -> SystemMetrics {
        self.inner.read().await.metrics.clone()
    }

    /// Check if metrics are stale (older than given duration)
    pub async fn is_stale(&self, max_age: Duration) -> bool {
        let inner = self.inner.read().await;
        inner.last_update.elapsed() > max_age
    }

    /// Get the age of current metrics
    pub async fn age(&self) -> Duration {
        self.inner.read().await.last_update.elapsed()
    }

    /// Get the update count
    pub async fn update_count(&self) -> u64 {
        self.inner.read().await.update_count
    }
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[tokio::test]
    async fn test_metrics_state_new() {
        let state = MetricsState::new();
        let snapshot = state.snapshot().await;

        // Should have empty metrics initially
        assert!(snapshot.cpu.is_none());
        assert!(snapshot.memory.is_none());
    }

    #[tokio::test]
    async fn test_metrics_state_update() {
        let state = MetricsState::new();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        let changes = state.update_cpu(cpu).await.expect("Failed to update");

        assert!(changes.cpu);
        assert!(!changes.memory);
    }

    #[tokio::test]
    async fn test_metrics_state_update_full() {
        let state = MetricsState::new();

        let mut metrics = SystemMetrics::new();
        metrics.cpu = Some(CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0)));
        metrics.memory = Some(MemoryMetrics::new(16_000_000_000, 8_000_000_000, 8_000_000_000, 0, 0, 0, 0));

        let changes = state.update(metrics).await.expect("Failed to update");

        assert!(changes.cpu);
        assert!(changes.memory);
    }

    #[tokio::test]
    async fn test_metrics_state_subscribe() {
        let state = MetricsState::new();
        let mut rx = state.subscribe();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu).await.expect("Failed to update");

        // Should receive update
        let update = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(update.is_ok());

        let update = update.unwrap().unwrap();
        assert!(update.changes.cpu);
    }

    #[tokio::test]
    async fn test_metrics_state_snapshot() {
        let state = MetricsState::new();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu).await.expect("Failed to update");

        let snapshot = state.snapshot().await;
        assert!(snapshot.cpu.is_some());
        assert_eq!(snapshot.cpu.unwrap().usage_percent, 50.0);
    }

    #[tokio::test]
    async fn test_metrics_state_is_stale() {
        let state = MetricsState::new();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu).await.expect("Failed to update");

        // Should not be stale immediately
        assert!(!state.is_stale(Duration::from_secs(1)).await);

        // Wait and check again
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(state.is_stale(Duration::from_millis(50)).await);
    }

    #[tokio::test]
    async fn test_metrics_state_age() {
        let state = MetricsState::new();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu).await.expect("Failed to update");

        tokio::time::sleep(Duration::from_millis(50)).await;
        let age = state.age().await;
        assert!(age >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_metrics_state_update_count() {
        let state = MetricsState::new();
        assert_eq!(state.update_count().await, 0);

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu).await.expect("Failed to update");

        assert_eq!(state.update_count().await, 1);
    }

    #[test]
    fn test_update_flags_none() {
        let flags = UpdateFlags::none();
        assert!(flags.is_empty());
    }

    #[test]
    fn test_update_flags_all() {
        let flags = UpdateFlags::all();
        assert!(!flags.is_empty());
        assert!(flags.cpu);
        assert!(flags.memory);
    }

    #[test]
    fn test_update_flags_from_metrics() {
        let mut old = SystemMetrics::new();
        let mut new = SystemMetrics::new();

        old.cpu = Some(CpuMetrics::new(10.0, 8, vec![], None, (0.0, 0.0, 0.0)));
        new.cpu = Some(CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0)));

        let flags = UpdateFlags::from_metrics(&old, &new);
        assert!(flags.cpu);
        assert!(!flags.memory);
    }

    #[test]
    fn test_update_flags_cpu_no_change() {
        let old = SystemMetrics::new();
        let new = SystemMetrics::new();

        let flags = UpdateFlags::from_metrics(&old, &new);
        assert!(!flags.cpu);
    }

    #[test]
    fn test_update_flags_cpu_small_change() {
        let mut old = SystemMetrics::new();
        let mut new = SystemMetrics::new();

        old.cpu = Some(CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0)));
        new.cpu = Some(CpuMetrics::new(50.5, 8, vec![], None, (0.0, 0.0, 0.0))); // < 1% change

        let flags = UpdateFlags::from_metrics(&old, &new);
        assert!(!flags.cpu); // Should not flag small changes
    }

    #[test]
    fn test_update_flags_memory_changed() {
        let mut old = SystemMetrics::new();
        let mut new = SystemMetrics::new();

        old.memory = Some(MemoryMetrics::new(16_000_000_000, 8_000_000_000, 8_000_000_000, 0, 0, 0, 0));
        new.memory = Some(MemoryMetrics::new(16_000_000_000, 10_000_000_000, 6_000_000_000, 0, 0, 0, 0));

        let flags = UpdateFlags::from_metrics(&old, &new);
        assert!(flags.memory);
    }

    #[test]
    fn test_update_flags_processes_changed() {
        let mut old = SystemMetrics::new();
        let mut new = SystemMetrics::new();

        old.top_cpu_processes = vec![ProcessInfo::new(
            1,
            "test".to_string(),
            5.0,
            10.0,
            1000,
            0,
            ProcessStatus::Running,
            None,
            0,
            0,
            0,
            0,
            0,
        )];

        new.top_cpu_processes = vec![ProcessInfo::new(
            2,
            "test2".to_string(),
            5.0,
            10.0,
            1000,
            0,
            ProcessStatus::Running,
            None,
            0,
            0,
            0,
            0,
            0,
        )];

        let flags = UpdateFlags::from_metrics(&old, &new);
        assert!(flags.processes);
    }

    #[tokio::test]
    async fn test_metrics_state_with_channel_capacity() {
        let state = MetricsState::with_channel_capacity(10);
        let _rx = state.subscribe(); // Add a receiver

        for i in 0..15 {
            let cpu = CpuMetrics::new(i as f32, 8, vec![], None, (0.0, 0.0, 0.0));
            state.update_cpu(cpu).await.expect("Failed to update");
        }

        // Should still work despite exceeding capacity
        let snapshot = state.snapshot().await;
        assert!(snapshot.cpu.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_updates() {
        let state = Arc::new(MetricsState::new());
        let mut handles = vec![];

        // Spawn multiple tasks updating different metrics concurrently
        for i in 0..5 {
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                let cpu = CpuMetrics::new(i as f32, 8, vec![], None, (0.0, 0.0, 0.0));
                state_clone.update_cpu(cpu).await.expect("Failed to update");
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.expect("Task failed");
        }

        // Final count should be at least 5
        assert!(state.update_count().await >= 5);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let state = MetricsState::new();

        let mut rx1 = state.subscribe();
        let mut rx2 = state.subscribe();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu).await.expect("Failed to update");

        // Both subscribers should receive the update
        let update1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv()).await;
        let update2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv()).await;

        assert!(update1.is_ok());
        assert!(update2.is_ok());
    }

    #[tokio::test]
    async fn test_state_update_index_increments() {
        let state = MetricsState::new();
        let mut rx = state.subscribe();

        let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));
        state.update_cpu(cpu.clone()).await.expect("Failed to update");

        let update1 = rx.recv().await.unwrap();
        assert_eq!(update1.index, 1);

        state.update_cpu(cpu).await.expect("Failed to update");

        let update2 = rx.recv().await.unwrap();
        assert_eq!(update2.index, 2);
    }
}
