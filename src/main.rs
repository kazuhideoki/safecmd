use clap::Parser;
use std::path::{Path, PathBuf};

mod allowlist;
mod config;
mod gitignore;
mod strategy;
use config::Config;
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

    // Load configuration
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("safecmd: {}", e);
            std::process::exit(1);
        }
    };

    // Check if current directory is allowed
    if !config.is_current_dir_allowed() {
        eprintln!("safecmd: current directory is not in the allowed directories list");
        std::process::exit(1);
    }

    let mut exit_code = 0;
    let context = ProcessContext::new(args, config);

    for path in &context.args.path {
        if let Err(msg) = process_path(path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    std::process::exit(exit_code);
}

fn process_path(path: &Path, context: &ProcessContext) -> Result<(), String> {
    // 1. Check if path is in allowed directories
    if !context.config.is_path_allowed(path) {
        return Err(format!(
            "safecmd: cannot remove '{}': path is not in allowed directories",
            path.display()
        ));
    }

    // 2. Check if file exists (considering -f flag)
    if !path.exists() {
        if context.args.force {
            // With -f flag, silently succeed for non-existent files
            return Ok(());
        } else {
            return Err(format!(
                "safecmd: cannot remove '{}': No such file or directory",
                path.display()
            ));
        }
    }

    // 3. Protection checks (only for existing files)
    // Priority: config.toml > .allowsafecmd > .gitignore

    // 4. Check gitignore protection
    if context.gitignore_checker.is_ignored(path) {
        // 5. Check if explicitly allowed by .allowsafecmd
        if !context.allowlist_checker.is_allowed(path) {
            let path_type = if path.is_dir() { "directory" } else { "file" };
            return Err(format!(
                "safecmd: cannot remove '{}': {} is protected by .gitignore",
                path.display(),
                path_type
            ));
        }
    }

    // 6. Proceed with removal
    let strategy = determine_strategy(path, context)?;
    strategy.validate(path, context)?;
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
