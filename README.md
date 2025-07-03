# safecmd

Safe replacement for the `rm` command that moves files to the system trash instead of permanently deleting them.

## Features

- **Safe deletion**: Moves files to system trash instead of permanent deletion
- **rm compatibility**: Drop-in replacement for `rm` command
- **Execution control**: Restricts execution to allowed directories via config file
- **Protection**: Respects `.gitignore` patterns - prevents deletion of ignored files
  - Recursive deletion (`-r`) checks all files and subdirectories for protection
- **Override capability**: `.allowsafecmd` files can override `.gitignore` protection

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

# .gitignore protected files cannot be deleted
rm build/output.bin  # Error if build/ is in .gitignore

# Recursive deletion also checks all contents
rm -r dist/  # Error if any file inside is protected

# Unless explicitly allowed in .allowsafecmd
echo "build/" > .allowsafecmd
rm -r build/  # Now allowed
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

### Security Model

1. **Execution restriction**: The `rm` command can only run in directories listed in `config.toml`
2. **File protection priority**: 
   - `config.toml` (execution allowed) → `.allowsafecmd` (deletion allowed) → `.gitignore` (deletion denied)

### Protection Files

- **`.gitignore`**: Files/directories matching patterns are protected from deletion
- **`.allowsafecmd`**: Overrides `.gitignore` protection for specific patterns (same syntax as `.gitignore`)

Example `.allowsafecmd`:
```
# Allow deletion of specific protected files
node_modules/
*.log
build/
```
