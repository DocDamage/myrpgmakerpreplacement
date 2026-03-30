# Justfile for DocDamage Engine development tasks
# Install just: https://github.com/casey/just

# Default recipe - list all available commands
default:
    @just --list

# Format all code
fmt:
    cargo fmt

# Check formatting without making changes
fmt-check:
    cargo fmt -- --check

# Run clippy lints
clippy:
    cargo clippy --workspace --all-features -- -D warnings

# Build the entire workspace
build:
    cargo build --workspace --all-features

# Build in release mode
build-release:
    cargo build --workspace --all-features --release

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests with output
test-verbose:
    cargo test --workspace --all-features -- --nocapture

# Run only unit tests (excluding integration tests)
test-unit:
    cargo test --workspace --all-features --lib

# Run tests for a specific crate (e.g., just test-crate dde-core)
test-crate crate:
    cargo test --package {{crate}} --all-features

# Check code without building
check:
    cargo check --workspace --all-features

# Clean build artifacts
clean:
    cargo clean

# Generate documentation
doc:
    cargo doc --workspace --all-features --no-deps

# Generate and open documentation
doc-open:
    cargo doc --workspace --all-features --no-deps --open

# Run the main application
run:
    cargo run

# Run with release optimizations
run-release:
    cargo run --release

# Full CI check - format, clippy, build, and test
ci: fmt-check clippy build test
    @echo "✅ All CI checks passed!"

# Fix automatic issues (formatting, some clippy fixes)
fix:
    cargo fmt
    cargo clippy --workspace --all-features --fix --allow-dirty --allow-staged

# Update dependencies
update:
    cargo update

# Check for outdated dependencies (requires cargo-outdated)
outdated:
    cargo outdated

# Run security audit (requires cargo-audit)
audit:
    cargo audit

# Generate test coverage report (requires cargo-tarpaulin)
coverage:
    cargo tarpaulin --workspace --all-features --out html

# Watch for changes and run tests automatically (requires cargo-watch)
watch:
    cargo watch -x "test --workspace --all-features"

# Benchmark (requires benchmarks to be set up)
bench:
    cargo bench --workspace

# Pre-commit hook - quick checks before committing
pre-commit: fmt-check clippy test-unit
    @echo "✅ Pre-commit checks passed!"

# Pre-push hook - full test suite before pushing
pre-push: fmt-check clippy test
    @echo "✅ Pre-push checks passed!"
