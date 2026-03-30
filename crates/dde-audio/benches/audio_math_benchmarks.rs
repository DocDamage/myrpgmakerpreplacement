//! Benchmarks for audio math and utilities
//!
//! Run with: cargo bench -p dde-audio
//!
//! Note: These benchmarks test audio calculations without requiring
//! actual audio hardware or files.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Volume conversion utilities
fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.max(0.0001).log10()
}

/// Benchmark decibel conversions
fn bench_db_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("db_conversions");

    group.bench_function("db_to_linear", |b| {
        let mut db = -60.0f32;
        b.iter(|| {
            let linear = db_to_linear(db);
            db = (db + 1.0) % 60.0 - 60.0;
            black_box(linear);
        });
    });

    group.bench_function("linear_to_db", |b| {
        let mut linear = 0.0f32;
        b.iter(|| {
            let db = linear_to_db(linear);
            linear = (linear + 0.01) % 1.0;
            black_box(db);
        });
    });

    group.bench_function("roundtrip", |b| {
        let mut value = -60.0f32;
        b.iter(|| {
            let linear = db_to_linear(value);
            let back_to_db = linear_to_db(linear);
            value = (value + 1.0) % 60.0 - 60.0;
            black_box(back_to_db);
        });
    });

    group.finish();
}

/// Benchmark volume mixing calculations
fn bench_volume_mixing(c: &mut Criterion) {
    let mut group = c.benchmark_group("volume_mixing");

    group.bench_function("mix_2_channels", |b| {
        let volume = 0.8f32;
        b.iter(|| {
            let sample1 = 0.5f32;
            let sample2 = -0.3f32;
            let mixed = (sample1 + sample2) * volume;
            black_box(mixed);
        });
    });

    group.bench_function("mix_4_channels", |b| {
        let master_volume = 0.8f32;
        b.iter(|| {
            let samples = [0.5f32, -0.3f32, 0.2f32, -0.4f32];
            let mixed: f32 = samples.iter().sum::<f32>() * master_volume;
            black_box(mixed);
        });
    });

    group.bench_function("apply_fade", |b| {
        let start_vol = 0.0f32;
        let end_vol = 1.0f32;
        let steps = 100usize;
        b.iter(|| {
            let mut volumes = Vec::with_capacity(steps);
            for i in 0..steps {
                let t = i as f32 / steps as f32;
                let vol = start_vol + (end_vol - start_vol) * t;
                volumes.push(vol);
            }
            black_box(volumes.len());
        });
    });

    group.finish();
}

/// Benchmark sample rate conversions
fn bench_sample_rate_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("sample_rate_conversion");

    group.bench_function("linear_interpolation", |b| {
        let samples = vec![0.0f32, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5];
        let ratio = 44100.0 / 48000.0;
        let mut pos = 0.0f32;
        b.iter(|| {
            let idx = pos as usize % samples.len();
            let frac = pos.fract();
            let s1 = samples[idx];
            let s2 = samples[(idx + 1) % samples.len()];
            let interpolated = s1 + (s2 - s1) * frac;
            pos += ratio;
            black_box(interpolated);
        });
    });

    group.bench_function("nearest_neighbor", |b| {
        let samples = vec![0.0f32, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5];
        let ratio = 44100.0 / 48000.0;
        let mut pos = 0.0f32;
        b.iter(|| {
            let idx = (pos as usize) % samples.len();
            let sample = samples[idx];
            pos += ratio;
            black_box(sample);
        });
    });

    group.finish();
}

/// Benchmark audio buffer operations
fn bench_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_operations");

    for buffer_size in [256, 512, 1024, 2048].iter() {
        group.bench_with_input(
            BenchmarkId::new("clear_buffer", buffer_size),
            buffer_size,
            |b, &size| {
                let mut buffer = vec![0.0f32; size];
                b.iter(|| {
                    for sample in &mut buffer {
                        *sample = 0.0;
                    }
                    black_box(&buffer);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("scale_buffer", buffer_size),
            buffer_size,
            |b, &size| {
                let mut buffer: Vec<f32> = (0..size).map(|i| i as f32 * 0.001).collect();
                let scale = 0.8f32;
                b.iter(|| {
                    for sample in &mut buffer {
                        *sample *= scale;
                    }
                    black_box(&buffer);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("mix_buffers", buffer_size),
            buffer_size,
            |b, &size| {
                let buffer_a: Vec<f32> = (0..size).map(|i| (i as f32 * 0.01).sin()).collect();
                let buffer_b: Vec<f32> = (0..size).map(|i| (i as f32 * 0.02).cos()).collect();
                let mut result = vec![0.0f32; size];
                b.iter(|| {
                    for i in 0..size {
                        result[i] = buffer_a[i] + buffer_b[i];
                    }
                    black_box(&result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark pan calculations
fn bench_pan_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pan_calculations");

    group.bench_function("constant_power_pan", |b| {
        let mut pan = -1.0f32;
        b.iter(|| {
            let angle = (pan + 1.0) * std::f32::consts::PI / 4.0;
            let left = angle.cos();
            let right = angle.sin();
            pan = (pan + 0.1) % 2.0 - 1.0;
            black_box((left, right));
        });
    });

    group.bench_function("linear_pan", |b| {
        let mut pan = -1.0f32;
        b.iter(|| {
            let left = (1.0 - pan) * 0.5;
            let right = (1.0 + pan) * 0.5;
            pan = (pan + 0.1) % 2.0 - 1.0;
            black_box((left, right));
        });
    });

    group.finish();
}

/// Benchmark pitch calculations
fn bench_pitch_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pitch_calculations");

    group.bench_function("semitone_to_ratio", |b| {
        let mut semitones = -12.0f32;
        b.iter(|| {
            let ratio = 2f32.powf(semitones / 12.0);
            semitones = (semitones + 0.5) % 24.0 - 12.0;
            black_box(ratio);
        });
    });

    group.bench_function("cents_to_ratio", |b| {
        let mut cents = -100.0f32;
        b.iter(|| {
            let ratio = 2f32.powf(cents / 1200.0);
            cents = (cents + 10.0) % 200.0 - 100.0;
            black_box(ratio);
        });
    });

    group.bench_function("frequency_to_midi", |b| {
        let mut freq = 20.0f32;
        b.iter(|| {
            let midi_note = 69.0 + 12.0 * (freq / 440.0).log2();
            freq = (freq + 10.0) % 20000.0;
            black_box(midi_note);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_db_conversions,
    bench_volume_mixing,
    bench_sample_rate_conversion,
    bench_buffer_operations,
    bench_pan_calculations,
    bench_pitch_calculations
);
criterion_main!(benches);
