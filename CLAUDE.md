# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SafeCmd is a safety-focused replacement for the `rm` command written in Rust. It moves files to the system trash instead of permanently deleting them, preventing accidental data loss.

## Development Commands

**Essential commands for development:**

```bash
# Check code compiles
cargo check

# format code
cargo fmt

# Run linter (must pass with no warnings)
cargo clippy -- -D warnings

# Run all tests
cargo test

# Run a single test
cargo test --test trash_integration

# Build the project
cargo build

# Run the binary
cargo run -- [arguments]
```

**Important**: After making any code changes, you MUST run all three quality checks in order:

1. `cargo check`
2. `cargo fmt`
3. `cargo clippy -- -D warnings`
4. `cargo test`

## Architecture

The codebase follows a simple CLI architecture:

- `src/main.rs`: Entry point with argument parsing using `clap`
- Uses the `trash` crate for safe file deletion (moves to system trash)
- Integration tests in `tests/` verify trash functionality

## Key Design Decisions

1. **Safety First**: All deletions go through the system trash, never permanently delete
2. **rm Compatibility**: Aims to be a drop-in replacement for `rm` with compatible flags
3. **Error Handling**: Comprehensive error messages to prevent user confusion

## Testing Strategy

- Integration tests use `tempfile` for isolated test environments
- Tests verify files are actually moved to trash (XDG specification)
- Use `assert_cmd` and `predicates` for CLI testing
