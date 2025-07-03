use crate::config::Config;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

pub struct AllowlistChecker {
    config_patterns: Option<Gitignore>,
}

impl AllowlistChecker {
    pub fn new() -> Self {
        Self {
            config_patterns: None,
        }
    }

    pub fn with_config(config: &Config) -> Self {
        let config_patterns = if !config.allowed_gitignores.patterns.is_empty() {
            // Use root "/" as the base directory for config patterns
            let mut builder = GitignoreBuilder::new("/");
            for pattern in &config.allowed_gitignores.patterns {
                let _ = builder.add_line(None, pattern);
            }
            builder.build().ok()
        } else {
            None
        };

        Self { config_patterns }
    }

    fn get_allowlist_for_path(&self, path: &Path) -> Option<(Gitignore, std::path::PathBuf)> {
        let cwd = std::env::current_dir().ok()?;

        // Convert to absolute path if necessary
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };

        // Get the directory containing the path
        let start_dir = abs_path.parent()?.to_path_buf();

        // Walk up directory tree collecting .allowsafecmd paths
        let mut current_dir = start_dir.clone();
        let mut allowlist_paths = Vec::new();
        let mut outermost_allowlist_dir = None;

        loop {
            let allowlist_path = current_dir.join(".allowsafecmd");
            if allowlist_path.exists() {
                allowlist_paths.push(allowlist_path);
                outermost_allowlist_dir = Some(current_dir.clone());
            }

            if !current_dir.pop() {
                break;
            }
        }

        if let Some(root) = outermost_allowlist_dir {
            let mut builder = GitignoreBuilder::new(&root);

            // Add allowlist files in reverse order (from root to local)
            for allowlist_path in allowlist_paths.into_iter().rev() {
                if let Some(e) = builder.add(&allowlist_path) {
                    eprintln!(
                        "Warning: Failed to parse .allowsafecmd at {}: {}",
                        allowlist_path.display(),
                        e
                    );
                }
            }

            builder.build().ok().map(|gi| (gi, root))
        } else {
            None
        }
    }

    pub fn is_allowed(&self, path: &Path) -> bool {
        let cwd = match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(_) => return false,
        };

        // Convert to absolute path
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };

        let is_dir = abs_path.is_dir();

        // First check config patterns
        if let Some(ref config_patterns) = self.config_patterns {
            // For config patterns, create a simple relative path from filename
            if let Some(file_name) = abs_path.file_name() {
                let file_name_path = Path::new(file_name);

                // Check the pattern match
                if config_patterns.matched(file_name_path, is_dir).is_ignore() {
                    return true;
                }

                // For directories, also check with trailing slash
                if is_dir {
                    let mut dir_name_with_slash = file_name.to_os_string();
                    dir_name_with_slash.push("/");
                    if config_patterns
                        .matched(Path::new(&dir_name_with_slash), true)
                        .is_ignore()
                    {
                        return true;
                    }
                }
            }
            
            // For files, check if any parent directory is allowed by config patterns
            if !is_dir {
                let mut current = abs_path.as_path();
                while let Some(parent) = current.parent() {
                    if let Some(parent_name) = parent.file_name() {
                        let parent_name_path = Path::new(parent_name);
                        
                        // Check if parent directory matches pattern
                        if config_patterns.matched(parent_name_path, true).is_ignore() {
                            return true;
                        }
                        
                        // Also check with trailing slash
                        let mut parent_name_with_slash = parent_name.to_os_string();
                        parent_name_with_slash.push("/");
                        if config_patterns
                            .matched(Path::new(&parent_name_with_slash), true)
                            .is_ignore()
                        {
                            return true;
                        }
                    }
                    current = parent;
                }
            }
        }

        // Then check local .allowsafecmd files
        if let Some((allowlist, allowlist_root)) = self.get_allowlist_for_path(path) {
            // Get relative path from the allowlist root
            if let Ok(rel_path) = abs_path.strip_prefix(&allowlist_root) {
                // Check if the path itself is matched (allowed)
                if allowlist.matched(rel_path, is_dir).is_ignore() {
                    return true;
                }

                // For files, also check if any parent directory is allowed
                if !is_dir {
                    let mut current = rel_path;
                    while let Some(parent) = current.parent() {
                        if !parent.as_os_str().is_empty()
                            && allowlist.matched(parent, true).is_ignore()
                        {
                            return true;
                        }
                        current = parent;
                    }
                }
            }
        }

        false
    }
}

impl Default for AllowlistChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AllowedDirectories, AllowedGitignores};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_new_creates_allowlist_checker() {
        let _checker = AllowlistChecker::new();
        // Should create successfully without panic
    }

    #[test]
    fn test_with_config_creates_allowlist_checker() {
        let config = Config {
            allowed_directories: AllowedDirectories { paths: vec![] },
            allowed_gitignores: AllowedGitignores {
                patterns: vec!["*.log".to_string(), "*.cache".to_string()],
            },
        };
        let _checker = AllowlistChecker::with_config(&config);
        // Should create successfully without panic
    }

    #[test]
    fn test_config_patterns_match_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create config with patterns
        let config = Config {
            allowed_directories: AllowedDirectories { paths: vec![] },
            allowed_gitignores: AllowedGitignores {
                patterns: vec![
                    "*.log".to_string(),
                    "*.cache".to_string(),
                    "build/".to_string(),
                ],
            },
        };
        let checker = AllowlistChecker::with_config(&config);

        // Create test files
        fs::write(temp_path.join("app.log"), "log").unwrap();
        fs::write(temp_path.join("data.cache"), "cache").unwrap();
        fs::write(temp_path.join("normal.txt"), "text").unwrap();
        fs::create_dir(temp_path.join("build")).unwrap();
        fs::create_dir(temp_path.join("src")).unwrap();

        // Test pattern matching
        assert!(checker.is_allowed(&temp_path.join("app.log")));
        assert!(checker.is_allowed(&temp_path.join("data.cache")));
        assert!(checker.is_allowed(&temp_path.join("build")));
        assert!(!checker.is_allowed(&temp_path.join("normal.txt")));
        assert!(!checker.is_allowed(&temp_path.join("src")));
    }

    #[test]
    fn test_no_allowlist_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("test.txt"), "content").unwrap();
        fs::create_dir(temp_path.join("test_dir")).unwrap();

        let checker = AllowlistChecker::new();

        // Without .allowsafecmd files, nothing should be explicitly allowed
        assert!(!checker.is_allowed(&temp_path.join("test.txt")));
        assert!(!checker.is_allowed(&temp_path.join("test_dir")));
    }

    #[test]
    fn test_allowlist_in_current_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .allowsafecmd
        fs::write(
            temp_path.join(".allowsafecmd"),
            "*.log\ntarget/\nbuild.tmp\n",
        )
        .unwrap();

        // Create test files
        fs::write(temp_path.join("app.log"), "log content").unwrap();
        fs::write(temp_path.join("build.tmp"), "temp").unwrap();
        fs::write(temp_path.join("normal.txt"), "normal").unwrap();
        fs::create_dir(temp_path.join("target")).unwrap();
        fs::create_dir(temp_path.join("src")).unwrap();

        let checker = AllowlistChecker::new();

        // Check allowed files
        assert!(checker.is_allowed(&temp_path.join("app.log")));
        assert!(checker.is_allowed(&temp_path.join("build.tmp")));
        assert!(checker.is_allowed(&temp_path.join("target")));

        // Check non-allowed files
        assert!(!checker.is_allowed(&temp_path.join("normal.txt")));
        assert!(!checker.is_allowed(&temp_path.join("src")));
    }

    #[test]
    fn test_allowlist_in_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create parent .allowsafecmd
        fs::write(temp_path.join(".allowsafecmd"), "*.tmp\n").unwrap();

        // Create subdirectory
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        // Create test files in subdirectory
        fs::write(sub_dir.join("cache.tmp"), "temp data").unwrap();
        fs::write(sub_dir.join("data.txt"), "normal data").unwrap();

        let checker = AllowlistChecker::new();

        // Parent .allowsafecmd should be respected
        assert!(checker.is_allowed(&sub_dir.join("cache.tmp")));
        assert!(!checker.is_allowed(&sub_dir.join("data.txt")));
    }

    #[test]
    fn test_nested_allowlist_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create parent .allowsafecmd
        fs::write(temp_path.join(".allowsafecmd"), "*.log\n").unwrap();

        // Create subdirectory with its own .allowsafecmd
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join(".allowsafecmd"), "*.cache\n").unwrap();

        // Create test files
        fs::write(sub_dir.join("app.log"), "log").unwrap();
        fs::write(sub_dir.join("data.cache"), "cache").unwrap();
        fs::write(sub_dir.join("normal.txt"), "normal").unwrap();

        let checker = AllowlistChecker::new();

        // Both parent and local .allowsafecmd should apply
        assert!(checker.is_allowed(&sub_dir.join("app.log"))); // From parent
        assert!(checker.is_allowed(&sub_dir.join("data.cache"))); // From local
        assert!(!checker.is_allowed(&sub_dir.join("normal.txt")));
    }

    #[test]
    fn test_directory_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .allowsafecmd with directory patterns
        fs::write(temp_path.join(".allowsafecmd"), "build/\n*.tmp/\n").unwrap();

        // Create directories
        fs::create_dir(temp_path.join("build")).unwrap();
        fs::create_dir(temp_path.join("cache.tmp")).unwrap();
        fs::create_dir(temp_path.join("src")).unwrap();

        let checker = AllowlistChecker::new();

        // Check directory patterns
        assert!(checker.is_allowed(&temp_path.join("build")));
        assert!(checker.is_allowed(&temp_path.join("cache.tmp")));
        assert!(!checker.is_allowed(&temp_path.join("src")));
    }

    #[test]
    fn test_files_in_allowed_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .allowsafecmd that allows entire directories
        fs::write(temp_path.join(".allowsafecmd"), "build/\ndist/\n").unwrap();

        // Create allowed directories with files
        fs::create_dir(temp_path.join("build")).unwrap();
        fs::write(temp_path.join("build/output.bin"), "binary").unwrap();
        fs::write(temp_path.join("build/debug.log"), "log").unwrap();

        fs::create_dir(temp_path.join("dist")).unwrap();
        fs::write(temp_path.join("dist/app.js"), "code").unwrap();

        // Create non-allowed directory with file
        fs::create_dir(temp_path.join("src")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "source").unwrap();

        let checker = AllowlistChecker::new();

        // Files in allowed directories should be allowed
        assert!(checker.is_allowed(&temp_path.join("build/output.bin")));
        assert!(checker.is_allowed(&temp_path.join("build/debug.log")));
        assert!(checker.is_allowed(&temp_path.join("dist/app.js")));

        // Files in non-allowed directories should not be allowed
        assert!(!checker.is_allowed(&temp_path.join("src/main.rs")));
    }
}
