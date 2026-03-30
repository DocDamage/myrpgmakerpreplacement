//! Benchmarks for camera and rendering math
//!
//! Run with: cargo bench -p dde-render

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dde_render::camera::Camera;
use glam::{Mat4, Vec2, Vec3};

/// Benchmark camera matrix calculations
fn bench_camera_matrices(c: &mut Criterion) {
    let mut group = c.benchmark_group("camera_matrices");

    group.bench_function("view_matrix", |b| {
        let camera = Camera::new(Vec2::new(100.0, 100.0));
        b.iter(|| {
            let view = camera.view_matrix();
            black_box(view);
        });
    });

    group.bench_function("projection_matrix", |b| {
        let camera = Camera::new(Vec2::new(100.0, 100.0));
        b.iter(|| {
            let proj = camera.projection_matrix(1920.0, 1080.0);
            black_box(proj);
        });
    });

    group.bench_function("combined_matrix", |b| {
        let camera = Camera::new(Vec2::new(100.0, 100.0));
        b.iter(|| {
            let view = camera.view_matrix();
            let proj = camera.projection_matrix(1920.0, 1080.0);
            let combined = proj * view;
            black_box(combined);
        });
    });

    group.finish();
}

/// Benchmark camera movement
fn bench_camera_movement(c: &mut Criterion) {
    let mut group = c.benchmark_group("camera_movement");

    group.bench_function("set_position", |b| {
        let mut camera = Camera::new(Vec2::new(0.0, 0.0));
        let mut i = 0.0;
        b.iter(|| {
            camera.set_position(Vec2::new(i, i * 0.5));
            i += 1.0;
            black_box(&camera);
        });
    });

    group.bench_function("move_by", |b| {
        let mut camera = Camera::new(Vec2::new(0.0, 0.0));
        b.iter(|| {
            camera.move_by(Vec2::new(1.0, 0.5));
            black_box(&camera);
        });
    });

    group.bench_function("zoom", |b| {
        let mut camera = Camera::new(Vec2::new(100.0, 100.0));
        let mut zoom = 1.0f32;
        b.iter(|| {
            camera.set_zoom(zoom);
            zoom = (zoom + 0.1) % 3.0;
            if zoom < 0.5 {
                zoom = 0.5;
            }
            black_box(&camera);
        });
    });

    group.finish();
}

/// Benchmark coordinate transformations
fn bench_coordinate_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_transforms");

    group.bench_function("world_to_screen", |b| {
        let camera = Camera::new(Vec2::new(500.0, 500.0));
        let mut x = 0.0f32;
        b.iter(|| {
            let world_pos = Vec2::new(x, x * 0.5);
            let screen = camera.world_to_screen(world_pos, 1920.0, 1080.0);
            x = (x + 10.0) % 1000.0;
            black_box(screen);
        });
    });

    group.bench_function("screen_to_world", |b| {
        let camera = Camera::new(Vec2::new(500.0, 500.0));
        let mut x = 0.0f32;
        b.iter(|| {
            let screen_pos = Vec2::new(x, x * 0.5);
            let world = camera.screen_to_world(screen_pos, 1920.0, 1080.0);
            x = (x + 10.0) % 1920.0;
            black_box(world);
        });
    });

    group.finish();
}

/// Benchmark sprite batch calculations
fn bench_sprite_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprite_batching");

    for sprite_count in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("calculate_instances", sprite_count),
            sprite_count,
            |b, &count| {
                b.iter(|| {
                    let mut instances = Vec::with_capacity(count);
                    for i in 0..count {
                        let x = (i % 100) as f32 * 32.0;
                        let y = (i / 100) as f32 * 32.0;
                        let transform = Mat4::from_translation(Vec3::new(x, y, 0.0));
                        instances.push(transform);
                    }
                    black_box(instances.len());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark tilemap coordinate calculations
fn bench_tilemap_math(c: &mut Criterion) {
    let mut group = c.benchmark_group("tilemap_math");

    group.bench_function("world_to_tile", |b| {
        let tile_size = 32.0f32;
        let mut x = 0.0f32;
        b.iter(|| {
            let world_x = x;
            let world_y = x * 0.5;
            let tile_x = (world_x / tile_size).floor() as i32;
            let tile_y = (world_y / tile_size).floor() as i32;
            x = (x + 5.0) % 1000.0;
            black_box((tile_x, tile_y));
        });
    });

    group.bench_function("tile_to_world", |b| {
        let tile_size = 32.0f32;
        let mut i = 0i32;
        b.iter(|| {
            let tile_x = i % 100;
            let tile_y = i / 100;
            let world_x = tile_x as f32 * tile_size;
            let world_y = tile_y as f32 * tile_size;
            i = (i + 1) % 10000;
            black_box((world_x, world_y));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_camera_matrices,
    bench_camera_movement,
    bench_coordinate_transforms,
    bench_sprite_batching,
    bench_tilemap_math
);
criterion_main!(benches);
