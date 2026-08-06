# Contributing to Gemini SDK

Thank you for your interest in contributing! This document outlines the
process for submitting issues, feature requests, and pull requests.

## Code of Conduct

Be respectful, inclusive, and constructive in all interactions.

## How to Contribute

1. **Fork the repository** and create a feature branch.
2. **Write tests** for new functionality.
3. **Run the full check suite** before submitting:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo doc --no-deps
   ```
4. **Update documentation** if you change public APIs.
5. **Submit a pull request** with a clear description.

## Coding Standards

- Follow `rustfmt.toml` and `clippy.toml`.
- Use meaningful variable names.
- Document all public items with doc comments.
- Keep functions small and focused.
- Prefer `Result` and strong error types over panics.

## Reporting Issues

When reporting bugs, please include:

- Rust version (`rustc --version`).
- Library version from `Cargo.toml`.
- Steps to reproduce.
- Expected vs. actual behavior.
- Minimal code example if possible.

## Security

If you discover a security-sensitive issue, please contact the author directly
rather than opening a public issue.
