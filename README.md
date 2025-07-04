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

