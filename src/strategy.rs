use crate::{Args, gitignore::GitignoreChecker};
use std::path::Path;

pub trait RemovalStrategy {
    fn validate(&self, path: &Path, context: &ProcessContext) -> Result<(), String>;
    fn execute(&self, path: &Path, context: &ProcessContext) -> Result<(), String>;
}

pub struct ProcessContext {
    pub args: Args,
    pub gitignore_checker: GitignoreChecker,
}

impl ProcessContext {
    pub fn new(args: Args) -> Self {
        Self {
            args,
            gitignore_checker: GitignoreChecker::new(),
        }
    }
}

pub struct FileStrategy;

impl RemovalStrategy for FileStrategy {
    fn validate(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path)
            .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
    }
}

pub struct RecursiveDirectoryStrategy;

impl RemovalStrategy for RecursiveDirectoryStrategy {
    fn validate(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path)
            .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
    }
}

pub struct EmptyDirectoryStrategy;

impl RemovalStrategy for EmptyDirectoryStrategy {
    fn validate(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        match std::fs::read_dir(path) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    Err(format!("safecmd: {}: Directory not empty", path.display()))
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(format!(
                "safecmd: cannot access '{}': {}",
                path.display(),
                e
            )),
        }
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path)
            .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
    }
}

pub struct DirectoryErrorStrategy;

impl RemovalStrategy for DirectoryErrorStrategy {
    fn validate(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Err(format!("safecmd: {}: is a directory", path.display()))
    }

    fn execute(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        unreachable!("DirectoryErrorStrategy should fail at validation")
    }
}

pub struct NonExistentFileStrategy;

impl RemovalStrategy for NonExistentFileStrategy {
    fn validate(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Ok(())
    }
}
