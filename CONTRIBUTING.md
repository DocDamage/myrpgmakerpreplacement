# Contributing to DocDamage Engine

Thank you for your interest in contributing to the DocDamage Engine! This document provides guidelines and workflows for contributing.

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Git](https://git-scm.com/)
- (Optional) [just](https://github.com/casey/just) for running development tasks

### System Dependencies

#### Linux
```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libudev-dev
```

#### macOS
No additional dependencies required.

#### Windows
No additional dependencies required.

### Building

```bash
# Clone the repository
git clone https://github.com/DocDamage/myrpgmakerpreplacement.git
cd myrpgmakerpreplacement

# Build the project
cargo build --workspace

# Or use just (if installed)
just build
```

## Development Workflow

### Code Quality Standards

We maintain high code quality standards. All code must pass:

1. **Formatting**: Code must be formatted with `rustfmt`
2. **Linting**: Code must pass `clippy` with no warnings
3. **Tests**: All tests must pass

### Quick Commands

```bash
# Format code
cargo fmt

# Run lints
cargo clippy --workspace --all-features -- -D warnings

# Run tests
cargo test --workspace --all-features

# Run all CI checks locally (if you have 'just' installed)
just ci
```

### Pre-commit Hooks

We recommend setting up pre-commit hooks to catch issues early:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# (Optional) Install push hooks for full test suite
pre-commit install --hook-type pre-push
```

## Making Changes

### Branch Naming

- Feature branches: `feature/description`
- Bug fixes: `fix/description`
- Documentation: `docs/description`

### Commit Messages

Follow conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style (formatting, missing semi colons, etc)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

Examples:
```
feat(dde-ai): add quest generation templates

fix(dde-editor): resolve timeline playback issue

docs: update API documentation for save system
```

### Pull Request Process

1. **Create a branch** for your changes
2. **Make your changes** with clear, focused commits
3. **Run CI checks locally**:
   ```bash
   just ci
   ```
4. **Push your branch** and create a Pull Request
5. **Ensure CI passes** on your PR
6. **Request review** from maintainers
7. **Address feedback** and merge when approved

## Testing

### Running Tests

```bash
# All tests
cargo test --workspace --all-features

# Specific crate
cargo test --package dde-core --all-features

# Unit tests only (faster)
cargo test --workspace --all-features --lib

# With output
cargo test --workspace --all-features -- --nocapture
```

### Writing Tests

- Add tests for new functionality
- Place unit tests in the same file as the code
- Place integration tests in `tests/` directories
- Ensure tests are deterministic (use seeded RNG when randomness is needed)

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        let result = new_feature("input");
        assert_eq!(result, expected_output);
    }
}
```

## Code Style

### Formatting

We use `rustfmt` with a custom configuration (see `rustfmt.toml`):

- Max width: 100 characters
- Tab spaces: 4
- Use field init shorthand
- Reorder imports and modules

### Linting

We use `clippy` with warnings as errors. Common lints to watch for:

- Unused imports/variables
- Unnecessary clones
- Inefficient code patterns
- Safety issues

### Documentation

- Document all public APIs with doc comments (`///`)
- Include examples in documentation where helpful
- Keep documentation up to date with code changes

Example:
```rust
/// Calculates damage based on attacker and defender stats.
///
/// # Example
///
/// ```
/// let damage = calculate_damage(&attacker, &defender, 100);
/// assert!(damage > 0);
/// ```
pub fn calculate_damage(attacker: &Stats, defender: &Stats, base: i32) -> i32 {
    // ...
}
```

## Project Structure

```
dde-engine/
├── crates/              # Workspace crates
│   ├── dde-core/       # Core ECS and simulation
│   ├── dde-editor/     # Editor UI
│   ├── dde-render/     # Rendering pipeline
│   ├── dde-db/         # Database layer
│   ├── dde-ai/         # AI systems
│   ├── dde-battle/     # Battle system
│   ├── dde-audio/      # Audio engine
│   ├── dde-export/     # Export formats
│   ├── dde-sync/       # Collaboration
│   ├── dde-lua/        # Scripting
│   └── dde-asset-forge/# Asset management
├── .github/workflows/   # CI/CD configuration
├── rustfmt.toml        # Formatting configuration
└── justfile           # Development tasks
```

## Continuous Integration

Our CI pipeline runs on every PR and push to main:

1. **Format Check**: Ensures code is properly formatted
2. **Clippy**: Runs lint checks
3. **Build**: Builds on Linux, Windows, and macOS
4. **Test**: Runs full test suite on all platforms
5. **Documentation**: Builds and checks documentation
6. **Coverage**: Generates coverage reports (main branch only)

## Getting Help

- Check existing [issues](https://github.com/DocDamage/myrpgmakerpreplacement/issues)
- Join our Discord (coming soon)
- Ask questions in PRs or issues

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
