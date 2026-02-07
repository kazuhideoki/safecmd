pub mod args;
pub mod strategy;

use crate::config::Config;
use args::Args;
use std::path::Path;
use strategy::{ProcessContext, RemovalStrategy};

pub fn run(args: Args, config: Config) -> i32 {
    let mut exit_code = 0;
    let context = ProcessContext::new(args, config);

    for path in &context.args.path {
        if let Err(msg) = process_path(path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    exit_code
}

fn process_path(path: &Path, context: &ProcessContext) -> Result<(), String> {
    if !context.config.is_path_allowed(path) {
        return Err(format!(
            "rm: cannot remove '{}': path is outside allowed scope",
            path.display()
        ));
    }

    if !path.exists() {
        if context.args.force {
            return Ok(());
        } else {
            return Err(format!(
                "rm: cannot remove '{}': No such file or directory",
                path.display()
            ));
        }
    }

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
                Err(format!("rm: cannot remove '{}': {e}", path.display()))
            }
        }
    }
}
