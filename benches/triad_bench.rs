//! Benchmarks for TriadCounter using Criterion v0.8.0

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use triad_counter_rs::TriadCounterPlugin;

/// Generate a random-ish signed adjacency matrix
fn generate_matrix(n: usize) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                // Deterministic "random" pattern based on indices
                let val = ((i * 7 + j * 13) % 5) as f64 - 2.0;
                matrix[i][j] = val;
            }
        }
    }
    matrix
}

fn bench_triad_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("triad_counting");

    for size in [10, 25, 50, 100, 200, 300] {
        let matrix = generate_matrix(size);
        let plugin = TriadCounterPlugin::from_matrix(matrix);

        group.bench_with_input(BenchmarkId::new("sequential", size), &plugin, |b, p| {
            b.iter(|| {
                let counts = p.count_triads_sequential();
                black_box(counts.total())
            })
        });

        group.bench_with_input(BenchmarkId::new("optimized", size), &plugin, |b, p| {
            b.iter(|| {
                let counts = p.count_triads_optimized();
                black_box(counts.total())
            })
        });
    }

    group.finish();
}

fn bench_large_networks(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_networks");
    group.sample_size(20); // Fewer samples for large networks

    for size in [500, 750, 1000] {
        let matrix = generate_matrix(size);
        let plugin = TriadCounterPlugin::from_matrix(matrix);

        group.bench_with_input(BenchmarkId::new("sequential", size), &plugin, |b, p| {
            b.iter(|| {
                let counts = p.count_triads_sequential();
                black_box(counts.total())
            })
        });

        group.bench_with_input(BenchmarkId::new("parallel", size), &plugin, |b, p| {
            b.iter(|| {
                let counts = p.count_triads_parallel_chunked();
                black_box(counts.total())
            })
        });
    }

    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for size in [50, 100, 200] {
        let matrix = generate_matrix(size);

        group.bench_with_input(
            BenchmarkId::new("from_matrix_and_run", size),
            &matrix,
            |b, m| {
                b.iter(|| {
                    let mut plugin = TriadCounterPlugin::from_matrix(m.clone());
                    plugin.run();
                    black_box(plugin.counts().total())
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_triad_counting,
    bench_large_networks,
    bench_full_pipeline
);
criterion_main!(benches);
