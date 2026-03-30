//! Benchmarks for Lua scripting engine
//!
//! Run with: cargo bench -p dde-lua

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dde_lua::{LuaEngine, SandboxConfig};
use mlua::Value;

/// Create a test engine with default sandbox config
fn create_test_engine() -> LuaEngine {
    let config = SandboxConfig {
        memory_limit: Some(1024 * 1024 * 10), // 10MB
        timeout_ms: 5000,
        allow_filesystem: false,
        allow_network: false,
        allow_os: false,
        max_recursion: 1000,
    };
    LuaEngine::new(config).expect("Failed to create Lua engine")
}

/// Benchmark Lua engine creation
fn bench_engine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine_creation");

    group.bench_function("new_engine", |b| {
        let config = SandboxConfig {
            memory_limit: Some(1024 * 1024),
            timeout_ms: 1000,
            allow_filesystem: false,
            allow_network: false,
            allow_os: false,
            max_recursion: 100,
        };
        b.iter(|| {
            let engine = LuaEngine::new(config.clone()).unwrap();
            black_box(&engine);
        });
    });

    group.finish();
}

/// Benchmark script execution
fn bench_script_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("script_execution");

    group.bench_function("simple_arithmetic", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute("return 1 + 2 + 3 + 4 + 5").unwrap();
            black_box(result);
        });
    });

    group.bench_function("string_concat", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine
                .execute(r#"return "Hello" .. " " .. "World""#)
                .unwrap();
            black_box(result);
        });
    });

    group.bench_function("table_operations", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine
                .execute(
                    r#"
                    local t = {}
                    for i = 1, 100 do
                        t[i] = i * 2
                    end
                    local sum = 0
                    for _, v in ipairs(t) do
                        sum = sum + v
                    end
                    return sum
                "#,
                )
                .unwrap();
            black_box(result);
        });
    });

    group.bench_function("fibonacci_20", |b| {
        let mut engine = create_test_engine();
        let script = r#"
            local function fib(n)
                if n <= 1 then return n end
                return fib(n - 1) + fib(n - 2)
            end
            return fib(20)
        "#;
        b.iter(|| {
            let result: Value = engine.execute(script).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark DDE API calls
fn bench_api_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_calls");

    group.bench_function("get_tile", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute("return dde.get_tile(10, 20)").unwrap();
            black_box(result);
        });
    });

    group.bench_function("get_entity", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine
                .execute("return dde.get_entity('npc_001')")
                .unwrap();
            black_box(result);
        });
    });

    group.bench_function("get_stat", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute("return dde.get_stat('health')").unwrap();
            black_box(result);
        });
    });

    group.bench_function("random", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute("return dde.random()").unwrap();
            black_box(result);
        });
    });

    group.bench_function("random_range", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute("return dde.random_range(1, 100)").unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark script loading
fn bench_script_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("script_loading");

    let small_script = r#"
        local function greet(name)
            return "Hello, " .. name
        end
        return greet("World")
    "#;

    let medium_script = r#"
        local M = {}
        
        function M.calculate_damage(base, multiplier, crit)
            local dmg = base * multiplier
            if crit then
                dmg = dmg * 2
            end
            return math.floor(dmg)
        end
        
        function M.apply_buff(stat, amount, duration)
            return {
                stat = stat,
                amount = amount,
                duration = duration,
                applied_at = 0
            }
        end
        
        return M
    "#;

    group.bench_function("load_small_script", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute(small_script).unwrap();
            black_box(result);
        });
    });

    group.bench_function("load_medium_script", |b| {
        let mut engine = create_test_engine();
        b.iter(|| {
            let result: Value = engine.execute(medium_script).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark sandbox overhead with different memory limits
fn bench_sandbox_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("sandbox_overhead");

    // Benchmark with different memory limits
    for memory_mb in [1, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("memory_limit_mb", memory_mb),
            memory_mb,
            |b, &mem| {
                let config = SandboxConfig {
                    memory_limit: Some(mem * 1024 * 1024),
                    timeout_ms: 5000,
                    allow_filesystem: false,
                    allow_network: false,
                    allow_os: false,
                    max_recursion: 1000,
                };
                let mut engine = LuaEngine::new(config).unwrap();
                b.iter(|| {
                    let result: Value = engine
                        .execute(
                            r#"
                            local sum = 0
                            for i = 1, 1000 do
                                sum = sum + i
                            end
                            return sum
                        "#,
                        )
                        .unwrap();
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark function calls
fn bench_function_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("function_calls");

    // First define a function in Lua
    let mut engine = create_test_engine();
    engine.execute(
        r#"
        function add(a, b)
            return a + b
        end
        
        function multiply(x, y, z)
            return x * y * z
        end
        
        function sum_many(a, b, c, d)
            return a + b + c + d
        end
    "#,
    ).unwrap();

    group.bench_function("call_add_0_args", |b| {
        b.iter(|| {
            let result = engine.call_function("add", vec![]).unwrap();
            black_box(result);
        });
    });

    group.bench_function("call_add_2_args", |b| {
        use mlua::Value;
        b.iter(|| {
            let args = vec![Value::Integer(10), Value::Integer(20)];
            let result = engine.call_function("add", args).unwrap();
            black_box(result);
        });
    });

    group.bench_function("call_multiply_3_args", |b| {
        use mlua::Value;
        b.iter(|| {
            let args = vec![Value::Integer(2), Value::Integer(3), Value::Integer(4)];
            let result = engine.call_function("multiply", args).unwrap();
            black_box(result);
        });
    });

    group.bench_function("call_sum_many_4_args", |b| {
        use mlua::Value;
        b.iter(|| {
            let args = vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
            ];
            let result = engine.call_function("sum_many", args).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_engine_creation,
    bench_script_execution,
    bench_api_calls,
    bench_script_loading,
    bench_sandbox_overhead,
    bench_function_calls
);
criterion_main!(benches);
