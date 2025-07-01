# safecmd

Safe commands for rm, cp, mv.

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
```
