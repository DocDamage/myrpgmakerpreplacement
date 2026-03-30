//! Benchmarks for database operations
//!
//! Run with: cargo bench -p dde-db

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dde_db::Database;
use std::path::PathBuf;

/// Create a temporary database path
fn temp_db_path() -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let random = rand::random::<u64>();
    std::env::temp_dir().join(format!("bench_db_{}_{}.db", timestamp, random))
}

/// Benchmark database creation
fn bench_database_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("database_creation");

    group.bench_function("create_new", |b| {
        b.iter(|| {
            let path = temp_db_path();
            let db = Database::create_new(&path, "Benchmark Project").unwrap();
            black_box(db.integrity_check().unwrap());
            // Cleanup
            let _ = std::fs::remove_file(&path);
        });
    });

    group.bench_function("open_existing", |b| {
        let path = temp_db_path();
        // Create once
        {
            let _db = Database::create_new(&path, "Benchmark Project").unwrap();
        }

        b.iter(|| {
            let db = Database::open(&path).unwrap();
            black_box(db.get_project_meta().unwrap());
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
    });

    group.finish();
}

/// Benchmark save slot operations
fn bench_save_slot_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("save_slot_operations");

    group.bench_function("save_to_slot", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();

        b.iter(|| {
            db.save_to_slot(1).unwrap();
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let slot_path = format!("{}.slot{:02}.dde", path.display(), 1);
        let _ = std::fs::remove_file(&slot_path);
    });

    group.bench_function("load_from_slot", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();
        db.save_to_slot(1).unwrap();

        b.iter(|| {
            let slot_path = db.load_from_slot(1).unwrap();
            black_box(slot_path);
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let slot_path = format!("{}.slot{:02}.dde", path.display(), 1);
        let _ = std::fs::remove_file(&slot_path);
    });

    group.bench_function("list_slots", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();
        // Create multiple slots
        for i in 1..=5 {
            db.save_to_slot(i).unwrap();
        }

        b.iter(|| {
            let slots = db.list_slots().unwrap();
            black_box(slots.len());
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
        for i in 1..=5 {
            let slot_path = format!("{}.slot{:02}.dde", path.display(), i);
            let _ = std::fs::remove_file(&slot_path);
        }
    });

    group.finish();
}

/// Benchmark metadata operations
fn bench_metadata_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_operations");

    group.bench_function("get_project_meta", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();

        b.iter(|| {
            let meta = db.get_project_meta().unwrap();
            black_box(meta.project_name);
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
    });

    group.bench_function("integrity_check", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();

        b.iter(|| {
            let valid = db.integrity_check().unwrap();
            black_box(valid);
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
    });

    group.bench_function("slot_exists", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();
        db.save_to_slot(1).unwrap();

        b.iter(|| {
            let exists = db.slot_exists(1);
            black_box(exists);
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let slot_path = format!("{}.slot{:02}.dde", path.display(), 1);
        let _ = std::fs::remove_file(&slot_path);
    });

    group.finish();
}

/// Benchmark screenshot operations
fn bench_screenshot_operations(c: &mut Criterion) {
    use dde_db::{ScreenshotData, ScreenshotFormat};

    let mut group = c.benchmark_group("screenshot_operations");

    group.bench_function("save_slot_with_screenshot_small", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();

        // Small screenshot (320x180 = ~50KB for raw RGBA)
        let screenshot = ScreenshotData {
            data: vec![0u8; 320 * 180 * 4],
            width: 320,
            height: 180,
            format: ScreenshotFormat::Png,
        };

        b.iter(|| {
            db.save_to_slot_with_screenshot(1, 3600000, Some(&screenshot))
                .unwrap();
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let slot_path = format!("{}.slot{:02}.dde", path.display(), 1);
        let _ = std::fs::remove_file(&slot_path);
    });

    group.bench_function("get_slot_info", |b| {
        let path = temp_db_path();
        let db = Database::create_new(&path, "Benchmark Project").unwrap();

        let screenshot = ScreenshotData {
            data: vec![0u8; 320 * 180 * 4],
            width: 320,
            height: 180,
            format: ScreenshotFormat::Png,
        };
        db.save_to_slot_with_screenshot(1, 3600000, Some(&screenshot))
            .unwrap();

        b.iter(|| {
            let info = db.get_slot_info(1).unwrap();
            black_box(info);
        });

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let slot_path = format!("{}.slot{:02}.dde", path.display(), 1);
        let _ = std::fs::remove_file(&slot_path);
    });

    group.finish();
}

/// Benchmark delete operations
fn bench_delete_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_operations");

    group.bench_function("delete_slot", |b| {
        b.iter_with_setup(
            || {
                let path = temp_db_path();
                let db = Database::create_new(&path, "Benchmark Project").unwrap();
                db.save_to_slot(1).unwrap();
                (path, db)
            },
            |(path, db)| {
                db.delete_slot(1).unwrap();
                // Recreate for next iteration
                db.save_to_slot(1).unwrap();
                let _ = std::fs::remove_file(&path);
                let slot_path = format!("{}.slot{:02}.dde", path.display(), 1);
                let _ = std::fs::remove_file(&slot_path);
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_database_creation,
    bench_save_slot_operations,
    bench_metadata_operations,
    bench_screenshot_operations,
    bench_delete_operations
);
criterion_main!(benches);
