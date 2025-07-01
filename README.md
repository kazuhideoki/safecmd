# safecmd

Safe commands for rm, cp, mv.

## Usage

```bash
safecmd [OPTIONS] <PATH>...
```

### Options

- `-d`  Allow removing empty directories
- `-r`  Remove directories recursively

### Examples

```bash
# Remove an empty directory
safecmd -d empty_dir
# Remove a directory and its contents recursively
safecmd -r dir file.txt
```
