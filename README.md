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

# Global patterns to allow deletion even if protected by .gitignore
[allowed_gitignores]
patterns = [
    # Glob patterns (like .gitignore syntax)
    "*.log",           # All .log files in any directory
    "*.cache",         # All .cache files
    "node_modules/",   # node_modules directories
    "build/",          # build directories
    "__pycache__/",    # Python cache directories
    "dist/*",          # All files in dist directories
    "src/*.tmp",       # .tmp files only in src directories
    "**/*.bak",        # .bak files at any depth
]
```

### Security Model

1. **Execution restriction**: The `rm` command can only run in directories listed in `config.toml`
2. **File protection priority**: 
   - `config.toml` `allowed_gitignores` patterns (globally allowed)
   - `.allowsafecmd` files (locally allowed, overrides `.gitignore`)
   - `.gitignore` files (protected by default)

### Protection Files

- **`.gitignore`**: Files/directories matching patterns are protected from deletion
- **`.allowsafecmd`**: Overrides `.gitignore` protection for specific patterns locally (same syntax as `.gitignore`)
- **`config.toml` `allowed_gitignores`**: Global patterns that override `.gitignore` protection across all directories

### Pattern Matching

Patterns in `allowed_gitignores` follow standard gitignore syntax:
- Patterns are relative to the current working directory
- `*.ext` matches files with that extension in any directory
- `dir/` matches directories named "dir"
- `dir/*.ext` matches files with that extension only in "dir"
- `**/file` matches "file" at any depth
- Files within allowed directories are automatically allowed

Example `.allowsafecmd`:
```
# Allow deletion of specific protected files
node_modules/
*.log
build/
```
