use crate::{allowlist::AllowlistChecker, args::Args, config::Config, gitignore::GitignoreChecker};
use std::path::Path;

pub trait RemovalStrategy {
    fn validate(&self, path: &Path, context: &ProcessContext) -> Result<(), String>;
    fn execute(&self, path: &Path, context: &ProcessContext) -> Result<(), String>;
}

pub struct ProcessContext {
    pub args: Args,
    pub config: Config,
    pub gitignore_checker: GitignoreChecker,
    pub allowlist_checker: AllowlistChecker,
}

impl ProcessContext {
    pub fn new(args: Args, config: Config) -> Self {
        let allowlist_checker = AllowlistChecker::with_config(&config);
        Self {
            args,
            config,
            gitignore_checker: GitignoreChecker::new(),
            allowlist_checker,
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
    fn validate(&self, path: &Path, context: &ProcessContext) -> Result<(), String> {
        // Check if any file or directory within the target directory is protected by gitignore
        Self::check_directory_recursively(path, context)
    }

    fn execute(&self, path: &Path, _context: &ProcessContext) -> Result<(), String> {
        trash::delete(path)
            .map_err(|e| format!("safecmd: failed to remove '{}': {}", path.display(), e))
    }
}

impl RecursiveDirectoryStrategy {
    fn check_directory_recursively(path: &Path, context: &ProcessContext) -> Result<(), String> {
        // First check the directory itself
        if context.gitignore_checker.is_ignored(path) && !context.allowlist_checker.is_allowed(path)
        {
            return Err(format!(
                "safecmd: cannot remove '{}': directory is protected by .gitignore",
                path.display()
            ));
        }

        // Then check all contents recursively
        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let entry_path = entry.path();

                            // Check if this entry is protected by gitignore
                            if context.gitignore_checker.is_ignored(&entry_path)
                                && !context.allowlist_checker.is_allowed(&entry_path)
                            {
                                let path_type = if entry_path.is_dir() {
                                    "directory"
                                } else {
                                    "file"
                                };
                                return Err(format!(
                                    "safecmd: cannot remove '{}': contains {} '{}' protected by .gitignore",
                                    path.display(),
                                    path_type,
                                    entry_path.display()
                                ));
                            }

                            // If it's a directory, check recursively
                            if entry_path.is_dir() {
                                Self::check_directory_recursively(&entry_path, context)?;
                            }
                        }
                        Err(e) => {
                            return Err(format!(
                                "safecmd: error reading directory '{}': {}",
                                path.display(),
                                e
                            ));
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(format!(
                "safecmd: cannot access '{}': {}",
                path.display(),
                e
            )),
        }
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
