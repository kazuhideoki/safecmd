# safecmd

Safe replacement for the `rm` command that moves files to the system trash instead of permanently deleting them.

## Features

- **Safe deletion**: Moves files to system trash instead of permanent deletion
- **rm compatibility**: Drop-in replacement for `rm` command
- **Execution control**: Restricts execution to allowed directories via config file

## Usage

```bash
rm [OPTIONS] <PATH>...
```

### Options

- `-d`  Allow removing empty directories
- `-f`  Force removal, ignore non-existent files
- `-r`  Remove directories recursively
- `-v`  Verbose mode - display each file as it's moved to trash (planned)

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

The `rm` command (from safecmd package) requires a configuration file at `~/.config/safecmd/config.toml` to specify allowed execution directories. The file is automatically created on first run.

### Configuration Format

```toml
[allowed_directories]
paths = [
    "/home/user/projects",
    "/home/user/tmp",
    "/Users/yourname/Documents",
]

```

## Environment Variables

SafeCmd supports several environment variables for configuration and testing:

### `SAFECMD_CONFIG_PATH`
- **Purpose**: Override the default configuration file location
- **Default**: `~/.config/safecmd/config.toml`
- **Example**: `SAFECMD_CONFIG_PATH=/custom/path/config.toml rm file.txt`

### `SAFECMD_DISABLE_ALLOWED_DIRECTORIES`
- **Purpose**: Disable directory restriction checks (use with caution)
- **Effect**: When set (any value), allows execution in any directory
- **Example**: `SAFECMD_DISABLE_ALLOWED_DIRECTORIES=1 rm file.txt`

### `SAFECMD_DISABLE_TEST_MODE`
- **Purpose**: Disable automatic test mode detection
- **Effect**: Prevents allowing all paths when running under `cargo test`
- **Example**: `SAFECMD_DISABLE_TEST_MODE=1 cargo test`

