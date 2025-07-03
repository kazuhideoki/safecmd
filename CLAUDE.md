# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SafeCmd is a safety-focused package written in Rust that provides safe replacements for dangerous commands. The `rm` binary moves files to the system trash instead of permanently deleting them, preventing accidental data loss.

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

# Run the rm binary
cargo run --bin rm -- [arguments]
```

**Important**: After making any code changes, you MUST run all three quality checks in order:

1. `cargo check`
2. `cargo fmt`
3. `cargo clippy -- -D warnings`
4. `cargo test`

## Architecture

The codebase follows a simple CLI architecture:

- `src/bin/rm.rs`: Entry point for the `rm` binary with argument parsing using `clap`
- Uses the `trash` crate for safe file deletion (moves to system trash)
- Integration tests in `tests/` verify trash functionality

## Key Design Decisions

1. **Safety First**: All deletions go through the system trash, never permanently delete
2. **rm Compatibility**: Aims to be a drop-in replacement for `rm` with compatible flags
3. **Error Handling**: Comprehensive error messages to prevent user confusion

## Planned Features

- `-v` (verbose) flag: Display each file as it's moved to trash

## Testing Strategy

- TDD を t-wada の方法で実践すること
  - 失敗するテストケースを書く
  - 実行して 🔴 Red であることを確認する
  - コードを修正する
  - 実行して 🟢 Green であることを確認する
- Integration tests use `tempfile` for isolated test environments
- Tests verify files are actually moved to trash (XDG specification)
- Use `assert_cmd` and `predicates` for CLI testing
