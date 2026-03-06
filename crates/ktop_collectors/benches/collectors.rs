//! Performance benchmarks for ktop_collectors
//!
//! These benchmarks measure the performance of collector operations to ensure
//! they meet the performance targets specified in the requirements:
//! - Refresh cycle overhead: < 5% CPU at default refresh rate
//! - Memory overhead: < 50MB baseline
//! - Rendering: < 16ms per frame (60 FPS target)
//!
//! Run benchmarks with: cargo bench --bench collectors

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ktop_collectors::cpu::CpuCollector;
use ktop_collectors::types::{CpuMetrics, MemoryMetrics, ProcessInfo, SystemMetrics};
use std::time::Duration;

/// Benchmark CPU collector creation
fn bench_cpu_collector_new(c: &mut Criterion) {
    c.bench_function("cpu_collector_new", |b| {
        b.iter(|| {
            let _collector = CpuCollector::new();
        });
    });
}

/// Benchmark CPU collector collection
fn bench_cpu_collector_collect(c: &mut Criterion) {
    let mut collector = CpuCollector::new();
    // Warm up
    let _ = collector.collect();

    c.bench_function("cpu_collector_collect", |b| {
        b.iter(|| {
            black_box(collector.collect().unwrap());
        });
    });
}

/// Benchmark CPU collector lightweight usage collection
fn bench_cpu_collector_collect_usage(c: &mut Criterion) {
    let mut collector = CpuCollector::new();
    // Warm up
    let _ = collector.collect();

    c.bench_function("cpu_collector_collect_usage", |b| {
        b.iter(|| {
            black_box(collector.collect_usage().unwrap());
        });
    });
}

/// Benchmark CPU collector with different refresh intervals
fn bench_cpu_collector_refresh_intervals(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_collector_refresh_interval");

    for interval_ms in [100, 250, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(interval_ms),
            interval_ms,
            |b, &ms| {
                let mut collector = CpuCollector::with_refresh_interval(Duration::from_millis(ms));
                let _ = collector.collect(); // Warm up

                b.iter(|| {
                    black_box(collector.collect().unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark CPU metrics creation
fn bench_cpu_metrics_new(c: &mut Criterion) {
    c.bench_function("cpu_metrics_new", |b| {
        b.iter(|| {
            black_box(CpuMetrics::new(
                50.0,
                8,
                vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0],
                Some(3200.0),
                (1.0, 0.8, 0.5),
            ));
        });
    });
}

/// Benchmark Memory metrics creation
fn bench_memory_metrics_new(c: &mut Criterion) {
    c.bench_function("memory_metrics_new", |b| {
        b.iter(|| {
            black_box(MemoryMetrics::new(
                16_000_000_000,
                8_000_000_000,
                8_000_000_000,
                1_000_000_000,
                2_000_000_000,
                4_000_000_000,
                1_000_000_000,
            ));
        });
    });
}

/// Benchmark ProcessInfo creation
fn bench_process_info_new(c: &mut Criterion) {
    c.bench_function("process_info_new", |b| {
        b.iter(|| {
            black_box(ProcessInfo::new(
                1234,
                "test_process".to_string(),
                5.5,
                10.0,
                1_000_000,
                500_000,
                ktop_collectors::types::ProcessStatus::Running,
                Some("/usr/bin/test_process --arg".to_string()),
            ));
        });
    });
}

/// Benchmark SystemMetrics creation
fn bench_system_metrics_new(c: &mut Criterion) {
    c.bench_function("system_metrics_new", |b| {
        b.iter(|| {
            black_box(SystemMetrics::new());
        });
    });
}

/// Benchmark SystemMetrics completeness check
fn bench_system_metrics_is_complete(c: &mut Criterion) {
    let mut metrics = SystemMetrics::new();

    c.bench_function("system_metrics_is_complete_empty", |b| {
        b.iter(|| {
            black_box(metrics.is_complete());
        });
    });

    // Now with populated metrics
    metrics.cpu = Some(CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0)));
    metrics.memory = Some(MemoryMetrics::new(
        16_000_000_000,
        8_000_000_000,
        8_000_000_000,
        0,
        0,
        0,
        0,
    ));
    metrics.network = Some(ktop_collectors::types::NetworkMetrics::new(
        std::collections::HashMap::new(),
        0,
        0,
        0,
        0,
    ));
    metrics.disk = Some(ktop_collectors::types::DiskMetrics::new(vec![], 0, 0, 0, 0));
    metrics.maestro = Some(ktop_collectors::types::MaestroMetrics::empty());

    c.bench_function("system_metrics_is_complete_full", |b| {
        b.iter(|| {
            black_box(metrics.is_complete());
        });
    });
}

/// Benchmark memory usage percentage calculation
fn bench_memory_usage_percent(c: &mut Criterion) {
    let mem = MemoryMetrics::new(16_000_000_000, 8_000_000_000, 8_000_000_000, 0, 0, 0, 0);

    c.bench_function("memory_usage_percent", |b| {
        b.iter(|| {
            black_box(mem.usage_percent());
        });
    });
}

/// Benchmark metrics age calculation
fn bench_metrics_age(c: &mut Criterion) {
    let cpu = CpuMetrics::new(50.0, 8, vec![], None, (0.0, 0.0, 0.0));

    c.bench_function("metrics_age", |b| {
        b.iter(|| {
            black_box(cpu.age());
        });
    });
}

criterion_group!(
    benches,
    bench_cpu_collector_new,
    bench_cpu_collector_collect,
    bench_cpu_collector_collect_usage,
    bench_cpu_collector_refresh_intervals,
    bench_cpu_metrics_new,
    bench_memory_metrics_new,
    bench_process_info_new,
    bench_system_metrics_new,
    bench_system_metrics_is_complete,
    bench_memory_usage_percent,
    bench_metrics_age
);

criterion_main!(benches);
