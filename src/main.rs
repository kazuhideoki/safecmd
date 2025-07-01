use clap::Parser;
use std::path::{Path, PathBuf};

/// Move the specified file to the system trash.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Allow removing empty directories
    #[arg(short = 'd')]
    allow_dir: bool,
    /// Force removal without prompting, ignore non-existent files
    #[arg(short = 'f')]
    force: bool,
    /// Recursively remove directories
    #[arg(short = 'r')]
    recursive: bool,
    /// Paths to files or directories to trash
    path: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let mut exit_code = 0;

    for path in &args.path {
        if let Err(msg) = process_path(path, &args) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    std::process::exit(exit_code);
}

fn process_path(path: &Path, args: &Args) -> Result<(), String> {
    match std::fs::metadata(path) {
        Ok(meta) => {
            if meta.is_dir() {
                handle_directory(path, args)
            } else {
                handle_file(path)
            }
        }
        Err(e) => {
            if args.force && e.kind() == std::io::ErrorKind::NotFound {
                Ok(()) // With -f flag, ignore non-existent files
            } else {
                Err(format!("safecmd: cannot remove '{}': {}", path.display(), e))
            }
        }
    }
}

fn handle_directory(path: &Path, args: &Args) -> Result<(), String> {
    if args.recursive {
        handle_recursive(path)
    } else if args.allow_dir {
        handle_allow_dir(path)
    } else {
        Err(format!("safecmd: {}: is a directory", path.display()))
    }
}

fn handle_recursive(path: &Path) -> Result<(), String> {
    trash::delete(path)
        .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
}

fn handle_allow_dir(path: &Path) -> Result<(), String> {
    match std::fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_none() {
                trash::delete(path)
                    .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
            } else {
                Err(format!("safecmd: {}: Directory not empty", path.display()))
            }
        }
        Err(e) => Err(format!("safecmd: cannot access '{}': {}", path.display(), e)),
    }
}

fn handle_file(path: &Path) -> Result<(), String> {
    trash::delete(path)
        .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
}
