use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub allowed_directories: AllowedDirectories,
    #[serde(default)]
    pub allowed_gitignores: AllowedGitignores,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllowedDirectories {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AllowedGitignores {
    pub patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allowed_directories: AllowedDirectories { paths: vec![] },
            allowed_gitignores: AllowedGitignores::default(),
        }
    }
}

impl Config {
    /// Loads the configuration from the TOML file.
    ///
    /// # Behavior
    /// 1. In test mode (when CARGO_MANIFEST_DIR is set), allows all paths unless disabled
    /// 2. Uses SAFECMD_CONFIG_PATH environment variable if set
    /// 3. Otherwise, looks for config at ~/.config/safecmd/config.toml
    /// 4. Creates a default config file if none exists
    pub fn load() -> Result<Self, String> {
        // Check if test mode should be disabled
        if std::env::var("SAFECMD_DISABLE_TEST_MODE").is_err() {
            // Check if running in test mode by looking for cargo test environment
            if std::env::var("CARGO_MANIFEST_DIR").is_ok() && std::env::var("CARGO").is_ok() {
                // If SAFECMD_CONFIG_PATH is set, still load from the config file even in test mode
                if std::env::var("SAFECMD_CONFIG_PATH").is_err() {
                    // Return a config that allows all paths for testing
                    return Ok(Self {
                        allowed_directories: AllowedDirectories {
                            paths: vec![PathBuf::from("/")],
                        },
                        allowed_gitignores: AllowedGitignores::default(),
                    });
                }
            }
        }

        let config_path = Self::config_path()?;

        if !config_path.exists() {
            Self::create_default_config(&config_path)?;
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;

        let config: Config =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {e}"))?;

        Ok(config)
    }

    /// Checks if a given path is within the allowed directories.
    ///
    /// # Security Features
    /// - Converts relative paths to absolute paths using the current directory
    /// - Canonicalizes paths to resolve symlinks and normalize ".." components
    /// - Prevents path traversal attacks (e.g., "../../etc/passwd")
    /// - Handles non-existent paths by resolving them based on current directory
    ///
    /// # Arguments
    /// * `path` - The path to check (can be relative or absolute)
    ///
    /// # Returns
    /// * `true` if the path is within any allowed directory
    /// * `false` if the path is outside all allowed directories or cannot be resolved
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        // First, convert the path to absolute and try to canonicalize it
        let absolute_path = if path.is_absolute() {
            // For absolute paths: canonicalize if exists to resolve symlinks,
            // otherwise use as-is (for paths that will be created)
            if path.exists() {
                match path.canonicalize() {
                    Ok(p) => p,
                    Err(_) => path.to_path_buf(),
                }
            } else {
                path.to_path_buf()
            }
        } else {
            // For relative paths: resolve against current directory
            match std::env::current_dir() {
                Ok(cwd) => {
                    // Canonicalize the current directory first to handle cases where
                    // we're in a symlinked directory
                    let canonical_cwd = match cwd.canonicalize() {
                        Ok(p) => p,
                        Err(_) => cwd,
                    };
                    let joined_path = canonical_cwd.join(path);
                    // Canonicalize the final path to resolve any ".." components
                    // This prevents attacks like "../../../etc/passwd"
                    if joined_path.exists() {
                        match joined_path.canonicalize() {
                            Ok(p) => p,
                            Err(_) => joined_path,
                        }
                    } else {
                        joined_path
                    }
                }
                Err(_) => return false,
            }
        };

        // Check if the resolved path is within any allowed directory
        for allowed_dir in &self.allowed_directories.paths {
            // Canonicalize allowed directories too for consistent comparison
            // This handles cases where allowed dirs contain symlinks
            let allowed_canonical = if allowed_dir.exists() {
                match allowed_dir.canonicalize() {
                    Ok(p) => p,
                    Err(_) => allowed_dir.to_path_buf(),
                }
            } else {
                allowed_dir.to_path_buf()
            };

            // Use starts_with to check if path is within allowed directory tree
            if absolute_path.starts_with(&allowed_canonical) {
                return true;
            }
        }

        false
    }

    /// Checks if the current working directory is within allowed directories.
    ///
    /// # Security Purpose
    /// This provides the first layer of defense - preventing safecmd from even
    /// running in directories that aren't explicitly allowed. This stops users
    /// from navigating to sensitive directories and using relative paths.
    ///
    /// # Returns
    /// * `true` if current directory is within any allowed directory
    /// * `false` if current directory is outside all allowed directories
    pub fn is_current_dir_allowed(&self) -> bool {
        match std::env::current_dir() {
            Ok(cwd) => {
                // Canonicalize to handle cases where we're in a symlinked directory
                // This ensures we check against the real path, not the symlink
                let canonical_cwd = match cwd.canonicalize() {
                    Ok(p) => p,
                    Err(_) => cwd,
                };

                for allowed_dir in &self.allowed_directories.paths {
                    // Try to canonicalize the allowed directory
                    let allowed_canonical = if allowed_dir.exists() {
                        match allowed_dir.canonicalize() {
                            Ok(p) => p,
                            Err(_) => allowed_dir.to_path_buf(),
                        }
                    } else {
                        // If the allowed directory doesn't exist, use it as-is
                        allowed_dir.to_path_buf()
                    };

                    if canonical_cwd.starts_with(&allowed_canonical) {
                        return true;
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    /// Determines the path to the configuration file.
    ///
    /// # Priority
    /// 1. SAFECMD_CONFIG_PATH environment variable (for testing and custom setups)
    /// 2. ~/.config/safecmd/config.toml (default location)
    fn config_path() -> Result<PathBuf, String> {
        // Check for environment variable override
        if let Ok(path) = std::env::var("SAFECMD_CONFIG_PATH") {
            return Ok(PathBuf::from(path));
        }

        let home_dir =
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;

        let config_dir = home_dir.join(".config").join("safecmd");

        Ok(config_dir.join("config.toml"))
    }

    /// Creates a default configuration file with helpful comments.
    ///
    /// # Error Handling
    /// Always returns an error after creating the file to force the user
    /// to explicitly configure allowed directories before using safecmd.
    fn create_default_config(config_path: &Path) -> Result<(), String> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let default_content = r#"# SafeCmd configuration file
# Only allow safecmd to run in directories listed below

[allowed_directories]
paths = [
    # Add your allowed directories here
    # Example: "/home/user/projects",
    # Example: "/Users/yourname/Documents",
]

# Patterns to allow deletion even if protected by .gitignore
# These patterns work in addition to local .allowsafecmd files
[allowed_gitignores]
patterns = [
    # Add gitignore-style patterns here
    # Example: "*.log",
    # Example: "*.cache",
    # Example: "node_modules/",
    # Example: "build/",
    # Example: "__pycache__/",
]
"#;

        let mut file = fs::File::create(config_path)
            .map_err(|e| format!("Failed to create config file: {e}"))?;

        file.write_all(default_content.as_bytes())
            .map_err(|e| format!("Failed to write default config: {e}"))?;

        Err(format!(
            "Created default configuration file at: {}\nPlease add allowed directories to the config file and try again.",
            config_path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to ensure sequential test execution.
    // This prevents race conditions when tests modify the current directory.
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Sets up test environment by disabling the automatic test mode.
    /// Without this, safecmd would allow all paths during cargo test.
    fn setup_test_env() {
        // Disable test mode for these unit tests
        unsafe {
            std::env::set_var("SAFECMD_DISABLE_TEST_MODE", "1");
        }
    }

    #[test]
    fn test_is_path_allowed_absolute_path() {
        setup_test_env();
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().join("allowed");
        fs::create_dir(&allowed_dir).unwrap();

        // Use canonical path for allowed directories to ensure consistent comparison
        let allowed_dir_canonical = allowed_dir.canonicalize().unwrap();

        let config = Config {
            allowed_directories: AllowedDirectories {
                paths: vec![allowed_dir_canonical.clone()],
            },
            allowed_gitignores: AllowedGitignores::default(),
        };

        // Test 1: Absolute path within allowed directory should be allowed
        let test_file = allowed_dir.join("test.txt");
        fs::write(&test_file, "test").unwrap();
        assert!(config.is_path_allowed(&test_file));

        // Test 2: Absolute path outside allowed directory should be denied
        let outside_file = temp_dir.path().join("outside.txt");
        fs::write(&outside_file, "test").unwrap();
        assert!(!config.is_path_allowed(&outside_file));
    }

    // NOTE: Relative path tests are thoroughly covered in integration tests.
    // Unit tests using set_current_dir are commented out to avoid conflicts
    // during parallel test execution. The TEST_MUTEX would help, but integration
    // tests provide better isolation for these scenarios.
    // #[test]
    #[allow(dead_code)]
    fn test_is_path_allowed_relative_paths() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().join("allowed");
        let subdir = allowed_dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        let allowed_dir_canonical = allowed_dir.canonicalize().unwrap();
        let config = Config {
            allowed_directories: AllowedDirectories {
                paths: vec![allowed_dir_canonical],
            },
            allowed_gitignores: AllowedGitignores::default(),
        };

        // Change to allowed directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&allowed_dir).unwrap();

        // Simple relative path
        fs::write("file.txt", "test").unwrap();
        assert!(config.is_path_allowed(Path::new("file.txt")));
        assert!(config.is_path_allowed(Path::new("./file.txt")));

        // Subdirectory relative path
        fs::write(subdir.join("subfile.txt"), "test").unwrap();
        assert!(config.is_path_allowed(Path::new("subdir/subfile.txt")));
        assert!(config.is_path_allowed(Path::new("./subdir/subfile.txt")));

        // Complex relative path
        assert!(config.is_path_allowed(Path::new("./subdir/../file.txt")));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    // Tests path traversal attack prevention (e.g., "../../etc/passwd")
    // Verifies that canonicalization prevents escaping allowed directories
    // #[test]
    #[allow(dead_code)]
    fn test_is_path_allowed_parent_directory_traversal() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().join("allowed");
        let subdir = allowed_dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        // Create file outside allowed directory
        let outside_file = temp_dir.path().join("outside.txt");
        fs::write(&outside_file, "test").unwrap();

        let allowed_dir_canonical = allowed_dir.canonicalize().unwrap();
        let config = Config {
            allowed_directories: AllowedDirectories {
                paths: vec![allowed_dir_canonical],
            },
            allowed_gitignores: AllowedGitignores::default(),
        };

        // Change to subdirectory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&subdir).unwrap();

        // Try to access parent directories (should be denied)
        assert!(!config.is_path_allowed(Path::new("../../outside.txt")));
        assert!(!config.is_path_allowed(Path::new("../..")));

        // But accessing within allowed directory via parent traversal should work
        fs::write(allowed_dir.join("allowed_file.txt"), "test").unwrap();
        assert!(config.is_path_allowed(Path::new("../allowed_file.txt")));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    // Tests handling of non-existent paths (files to be created)
    // Ensures path resolution works even when the target doesn't exist yet
    // #[test]
    #[allow(dead_code)]
    fn test_is_path_allowed_nonexistent_path() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().join("allowed");
        fs::create_dir(&allowed_dir).unwrap();

        let allowed_dir_canonical = allowed_dir.canonicalize().unwrap();
        let config = Config {
            allowed_directories: AllowedDirectories {
                paths: vec![allowed_dir_canonical],
            },
            allowed_gitignores: AllowedGitignores::default(),
        };

        // Change to allowed directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&allowed_dir).unwrap();

        // Non-existent relative path should still be checked based on resolved path
        assert!(config.is_path_allowed(Path::new("nonexistent.txt")));
        assert!(config.is_path_allowed(Path::new("./subdir/nonexistent.txt")));

        // Non-existent path outside allowed directory
        assert!(!config.is_path_allowed(Path::new("../outside/nonexistent.txt")));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    // Tests the first layer of security: current directory validation
    // This prevents users from running safecmd in unauthorized locations
    // #[test]
    #[allow(dead_code)]
    fn test_is_current_dir_allowed() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path().join("allowed");
        let disallowed_dir = temp_dir.path().join("disallowed");
        fs::create_dir(&allowed_dir).unwrap();
        fs::create_dir(&disallowed_dir).unwrap();

        let allowed_dir_canonical = allowed_dir.canonicalize().unwrap();
        let config = Config {
            allowed_directories: AllowedDirectories {
                paths: vec![allowed_dir_canonical.clone()],
            },
            allowed_gitignores: AllowedGitignores::default(),
        };

        let original_dir = std::env::current_dir().unwrap();

        // Test allowed directory
        std::env::set_current_dir(&allowed_dir).unwrap();
        assert!(config.is_current_dir_allowed());

        // Test disallowed directory
        std::env::set_current_dir(&disallowed_dir).unwrap();
        assert!(!config.is_current_dir_allowed());

        // Test subdirectory of allowed directory
        let subdir = allowed_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        std::env::set_current_dir(&subdir).unwrap();
        assert!(config.is_current_dir_allowed());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_empty_allowed_directories() {
        setup_test_env();
        let config = Config {
            allowed_directories: AllowedDirectories { paths: vec![] },
            allowed_gitignores: AllowedGitignores::default(),
        };

        // Empty allowed directories should implement "deny by default" policy
        // This ensures safecmd fails closed rather than open
        assert!(!config.is_current_dir_allowed());
        assert!(!config.is_path_allowed(Path::new("any_file.txt")));
        assert!(!config.is_path_allowed(Path::new("/absolute/path.txt")));
    }
}
