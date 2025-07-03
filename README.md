# safecmd

Safe commands for rm, cp, mv.

## Features

- **Safe deletion**: Moves files to system trash instead of permanent deletion
- **rm compatibility**: Drop-in replacement for `rm` command
- **Execution control**: Restricts execution to allowed directories via config file
- **Protection**: Respects `.gitignore` patterns - prevents deletion of ignored files
  - Recursive deletion (`-r`) checks all files and subdirectories for protection
- **Override capability**: `.allowsafecmd` files can override `.gitignore` protection

## Usage

```bash
safecmd [OPTIONS] <PATH>...
```

### Options

- `-d`  Allow removing empty directories
- `-f`  Force removal, ignore non-existent files
- `-r`  Remove directories recursively

### Examples

```bash
# Remove an empty directory
safecmd -d empty_dir
# Remove a directory and its contents recursively
safecmd -r dir file.txt

# Force removal, ignore if files don't exist
safecmd -f non_existent.txt existing.txt

# .gitignore protected files cannot be deleted
safecmd build/output.bin  # Error if build/ is in .gitignore

# Recursive deletion also checks all contents
safecmd -r dist/  # Error if any file inside is protected

# Unless explicitly allowed in .allowsafecmd
echo "build/" > .allowsafecmd
safecmd -r build/  # Now allowed
```

## Configuration

SafeCmd requires a configuration file at `~/.config/safecmd/config.toml` to specify allowed execution directories. The file is automatically created on first run.

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

1. **Execution restriction**: SafeCmd can only run in directories listed in `config.toml`
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
