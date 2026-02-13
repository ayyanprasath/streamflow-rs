# Contributing to Enterprise Data Processor

Thank you for your interest in contributing! This document provides guidelines for contributing to the project.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/enterprise-data-processor.git`
3. Create a feature branch: `git checkout -b feature/my-new-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Commit your changes: `git commit -am 'Add new feature'`
7. Push to your fork: `git push origin feature/my-new-feature`
8. Create a Pull Request

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo

### Building

```bash
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration_tests
```

### Running Benchmarks

```bash
cargo bench
```

### Code Formatting

```bash
# Check formatting
cargo fmt -- --check

# Apply formatting
cargo fmt
```

### Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Pull Request Guidelines

1. **Keep PRs focused**: One feature or bug fix per PR
2. **Write tests**: All new features should include tests
3. **Update documentation**: Update docs for API changes
4. **Follow style guide**: Use `rustfmt` and pass `clippy`
5. **Write clear commits**: Use descriptive commit messages
6. **Add changelog entry**: Update CHANGELOG.md with your changes

## Commit Message Format

Follow the conventional commits specification:

```
type(scope): subject

body

footer
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Example:
```
feat(processor): add batch processing support

Implement concurrent batch processing with configurable
worker pool size.

Closes #123
```

## Code Style

- Follow Rust naming conventions
- Keep functions focused and small
- Use meaningful variable names
- Add comments for complex logic
- Prefer explicit over implicit
- Use `Result` for error handling
- Add `#[derive(Debug)]` to structs

## Testing Guidelines

### Unit Tests

- Test each function in isolation
- Use descriptive test names
- Test edge cases and error conditions
- Use `#[should_panic]` for panic tests

```rust
#[test]
fn test_record_creation() {
    let record = Record::new("key", "value");
    assert_eq!(record.key, "key");
}

#[test]
#[should_panic(expected = "Invalid state")]
fn test_invalid_state() {
    // Test that should panic
}
```

### Integration Tests

- Test complete workflows
- Test interaction between components
- Place in `tests/` directory

### Async Tests

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

## Documentation

### Code Documentation

- Add doc comments (`///`) to public items
- Include examples in doc comments
- Document parameters and return values
- Add `# Examples` section

```rust
/// Process a single record asynchronously
///
/// # Arguments
///
/// * `record` - The record to process
///
/// # Returns
///
/// A `Result` containing the processing result
///
/// # Examples
///
/// ```
/// use enterprise_data_processor::{Processor, Record};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let processor = Processor::new(Default::default())?;
/// let record = Record::new("key", "value");
/// let result = processor.process(record).await?;
/// # Ok(())
/// # }
/// ```
pub async fn process(&self, record: Record) -> Result<ProcessingResult> {
    // Implementation
}
```

### README Updates

Update README.md when:
- Adding new features
- Changing public API
- Updating examples

## Performance Considerations

- Profile before optimizing
- Use benchmarks to measure impact
- Document performance characteristics
- Avoid unnecessary allocations
- Use appropriate data structures
- Consider async/await overhead

## Error Handling

- Use `Result` types
- Create specific error variants
- Provide helpful error messages
- Include context with errors
- Document error conditions

```rust
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Processing failed: {0}")]
    Processing(String),
}
```

## Adding New Features

1. **Discuss first**: Open an issue to discuss the feature
2. **Design API**: Consider ergonomics and consistency
3. **Implement**: Follow guidelines above
4. **Test thoroughly**: Add comprehensive tests
5. **Document**: Update docs and examples
6. **Benchmark**: Measure performance impact

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
4. Push tag: `git push origin v1.0.0`
5. Publish to crates.io: `cargo publish`

## Getting Help

- GitHub Issues: Bug reports and feature requests
- GitHub Discussions: Questions and discussions
- Documentation: https://docs.rs/enterprise-data-processor

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

## Recognition

Contributors will be recognized in:
- CHANGELOG.md
- GitHub contributors page
- Release notes

Thank you for contributing! 🎉
