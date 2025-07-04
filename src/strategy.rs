use crate::{args::Args, config::Config};
use std::path::Path;

pub trait RemovalStrategy {
    fn validate(&self, path: &Path, context: &ProcessContext) -> Result<(), String>;
    fn execute(&self, path: &Path, context: &ProcessContext) -> Result<(), String>;
}

pub struct ProcessContext {
    pub args: Args,
    pub config: Config,
}

impl ProcessContext {
    pub fn new(args: Args, config: Config) -> Self {
        Self { args, config }
    }
}

pub struct FileStrategy;

impl RemovalStrategy for FileStrategy {
    fn validate(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path).map_err(|e| format!("rm: failed to remove '{}': {}", path.display(), e))
    }
}

pub struct RecursiveDirectoryStrategy;

impl RemovalStrategy for RecursiveDirectoryStrategy {
    fn validate(&self, _path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path).map_err(|e| format!("rm: failed to remove '{}': {}", path.display(), e))
    }
}

impl RecursiveDirectoryStrategy {}

pub struct EmptyDirectoryStrategy;

impl RemovalStrategy for EmptyDirectoryStrategy {
    fn validate(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        match std::fs::read_dir(path) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    Err(format!("rm: {}: Directory not empty", path.display()))
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(format!("rm: cannot access '{}': {}", path.display(), e)),
        }
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path).map_err(|e| format!("rm: failed to remove '{}': {}", path.display(), e))
    }
}

pub struct DirectoryErrorStrategy;

impl RemovalStrategy for DirectoryErrorStrategy {
    fn validate(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        Err(format!("rm: {}: is a directory", path.display()))
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
