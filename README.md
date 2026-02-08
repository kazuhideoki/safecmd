# safecmd

Safer wrappers for `rm` and `cp` with trash-first behavior and directory-scope controls.

## Features

- **Safe deletion**: Moves files to system trash instead of permanent deletion
- **GNU-like interface**: Supports familiar `rm` / `cp` flags with a focused subset
- **Execution control**: Allows operations in current directory tree and optionally in additional directories via config

## Usage

```bash
rm [OPTIONS] <PATH>...
```

### Options

- `-d`  Allow removing empty directories
- `-f`  Force removal, ignore non-existent files
- `-r`  Remove directories recursively

### Examples

```bash
# Remove an empty directory
rm -d empty_dir
# Remove a directory and its contents recursively
rm -r dir file.txt

# Force removal, ignore if files don't exist
rm -f non_existent.txt existing.txt

```

## Configuration

The `rm` command (from safecmd package) requires a configuration file at `~/.config/safecmd/config.toml` to specify additional allowed directories. The file is automatically created on first run.
You can also start from `config.example.toml` in this repository.

### Configuration Format

```toml
[additional_allowed_directories]
paths = [
    "/home/user/shared",
    "/Users/yourname/Documents",
]

```

## Environment Variables

SafeCmd supports several environment variables for configuration and testing:

### `SAFECMD_CONFIG_PATH`
- **Purpose**: Override the default configuration file location
- **Default**: `~/.config/safecmd/config.toml`
- **Example**: `SAFECMD_CONFIG_PATH=/custom/path/config.toml rm file.txt`

### `SAFECMD_DISABLE_TEST_MODE`
- **Purpose**: Disable automatic test mode detection
- **Effect**: Prevents allowing all paths when running under `cargo test`
- **Example**: `SAFECMD_DISABLE_TEST_MODE=1 cargo test`

## Flag Behavior vs GNU Coreutils

The tables below summarize implemented behavior in `safecmd` versus GNU `rm` and GNU `cp`.

### `rm` flags

| Flag | `safecmd rm` behavior | GNU `rm` behavior | Notes |
| --- | --- | --- | --- |
| `-d` | Removes only empty directories by moving them to trash | Removes empty directories permanently | Same condition, different deletion target (trash vs permanent) |
| `-f` | Ignores missing paths and suppresses that error | Ignores missing paths and suppresses prompts/errors | Similar for missing files; no interactive prompt mode in `safecmd` |
| `-r` | Recursively removes directories by moving them to trash | Recursively removes directories permanently | Same recursion intent, different deletion target |
| `-R` | Alias of `-r` | Alias of `-r` | Equivalent in both |
| Unsupported (for example `-i`, `-I`, `--one-file-system`) | Not available | Available depending on flag | `safecmd rm` intentionally supports a smaller safe subset |

### `cp` flags

| Flag | `safecmd cp` behavior | GNU `cp` behavior | Notes |
| --- | --- | --- | --- |
| `-r` | Enables recursive directory copy | Enables recursive directory copy | Recursion enabled |
| `-R` | Alias of `-r` | Alias of recursive copy | Recursion enabled |
| `--recursive` | Enables recursive directory copy | Enables recursive directory copy | Recursion enabled |
| `-n` | Skips overwrite when destination is an existing regular file | `--no-clobber`: does not overwrite existing files | `safecmd cp` keeps type-conflict errors (for example file-to-directory) |
| No recursive flag (directory source) | Fails with `omitting directory` | Fails with `-r not specified; omitting directory` | Same outcome; wording differs |
| Overwrite existing target | Moves existing target to trash, then copies | Overwrites destination directly | `safecmd cp` adds a trash-first safety step |
| Unsupported (for example `-a`, `-p`, `--preserve`) | Not available | Available depending on flag | `safecmd cp` currently supports a focused subset |
