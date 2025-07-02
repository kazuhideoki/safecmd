use clap::Parser;
use std::path::{Path, PathBuf};

mod strategy;
use strategy::{ProcessContext, RemovalStrategy};

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
    let context = ProcessContext::new(args);

    for path in &context.args.path {
        if let Err(msg) = process_path(path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    std::process::exit(exit_code);
}

fn process_path(path: &Path, context: &ProcessContext) -> Result<(), String> {
    // Determine strategy based on path type and flags
    let strategy = determine_strategy(path, context)?;

    // Validate the operation
    strategy.validate(path, context)?;

    // Execute the removal
    strategy.execute(path, context)
}

fn determine_strategy(
    path: &Path,
    context: &ProcessContext,
) -> Result<Box<dyn RemovalStrategy>, String> {
    use strategy::*;

    match std::fs::metadata(path) {
        Ok(meta) => {
            if meta.is_dir() {
                if context.args.recursive {
                    Ok(Box::new(RecursiveDirectoryStrategy))
                } else if context.args.allow_dir {
                    Ok(Box::new(EmptyDirectoryStrategy))
                } else {
                    Ok(Box::new(DirectoryErrorStrategy))
                }
            } else {
                Ok(Box::new(FileStrategy))
            }
        }
        Err(e) => {
            if context.args.force && e.kind() == std::io::ErrorKind::NotFound {
                Ok(Box::new(NonExistentFileStrategy))
            } else {
                Err(format!(
                    "safecmd: cannot remove '{}': {}",
                    path.display(),
                    e
                ))
            }
        }
    }
}
