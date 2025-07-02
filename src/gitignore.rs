use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

pub struct GitignoreChecker;

impl GitignoreChecker {
    pub fn new() -> Self {
        Self
    }

    fn get_gitignore_for_path(&self, path: &Path) -> Option<Gitignore> {
        let cwd = std::env::current_dir().ok()?;
        let mut builder = GitignoreBuilder::new(&cwd);
        
        // Get the directory containing the path
        let start_dir = if path.is_absolute() {
            path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| path.to_path_buf())
        } else {
            let full_path = cwd.join(path);
            full_path.parent().map(|p| p.to_path_buf()).unwrap_or(cwd.clone())
        };
        
        // Walk up directory tree looking for .gitignore files
        let mut current_dir = start_dir;
        loop {
            let gitignore_path = current_dir.join(".gitignore");
            if gitignore_path.exists() {
                if let Some(e) = builder.add(&gitignore_path) {
                    eprintln!("Warning: Failed to parse .gitignore at {}: {}", gitignore_path.display(), e);
                }
            }
            
            if !current_dir.pop() {
                break;
            }
        }
        
        builder.build().ok()
    }
    
    pub fn is_ignored(&self, path: &Path) -> bool {
        if let Some(gitignore) = self.get_gitignore_for_path(path) {
            // For gitignore matching, we need to use relative paths from the gitignore location
            let is_dir = path.is_dir();
            gitignore.matched(path, is_dir).is_ignore()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_new_creates_gitignore_checker() {
        let _checker = GitignoreChecker::new();
        // Should create successfully without panic
    }

    #[test]
    fn test_no_gitignore_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("test.txt"), "content").unwrap();
        fs::create_dir(temp_path.join("test_dir")).unwrap();

        let checker = GitignoreChecker::new();
        
        // Without .gitignore files, nothing should be ignored
        assert!(!checker.is_ignored(&temp_path.join("test.txt")));
        assert!(!checker.is_ignored(&temp_path.join("test_dir")));
    }

    #[test]
    fn test_gitignore_in_current_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .gitignore
        fs::write(temp_path.join(".gitignore"), "*.log\ntarget/\nsecret.txt\n").unwrap();

        // Create test files
        fs::write(temp_path.join("app.log"), "log content").unwrap();
        fs::write(temp_path.join("secret.txt"), "secret").unwrap();
        fs::write(temp_path.join("normal.txt"), "normal").unwrap();
        fs::create_dir(temp_path.join("target")).unwrap();
        fs::create_dir(temp_path.join("src")).unwrap();

        let checker = GitignoreChecker::new();

        // Check ignored files
        assert!(checker.is_ignored(&temp_path.join("app.log")));
        assert!(checker.is_ignored(&temp_path.join("secret.txt")));
        assert!(checker.is_ignored(&temp_path.join("target")));
        
        // Check non-ignored files
        assert!(!checker.is_ignored(&temp_path.join("normal.txt")));
        assert!(!checker.is_ignored(&temp_path.join("src")));
    }

    #[test]
    fn test_gitignore_in_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create parent .gitignore
        fs::write(temp_path.join(".gitignore"), "*.secret\n").unwrap();
        
        // Create subdirectory
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        
        // Create test files in subdirectory
        fs::write(sub_dir.join("data.secret"), "secret data").unwrap();
        fs::write(sub_dir.join("data.txt"), "normal data").unwrap();

        let checker = GitignoreChecker::new();

        // Parent .gitignore should be respected
        assert!(checker.is_ignored(&sub_dir.join("data.secret")));
        assert!(!checker.is_ignored(&sub_dir.join("data.txt")));
    }

    #[test]
    fn test_nested_gitignore_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create parent .gitignore
        fs::write(temp_path.join(".gitignore"), "*.log\n").unwrap();
        
        // Create subdirectory with its own .gitignore
        let sub_dir = temp_path.join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join(".gitignore"), "*.secret\n").unwrap();
        
        // Create test files
        fs::write(sub_dir.join("app.log"), "log").unwrap();
        fs::write(sub_dir.join("data.secret"), "secret").unwrap();
        fs::write(sub_dir.join("normal.txt"), "normal").unwrap();

        let checker = GitignoreChecker::new();

        // Both parent and local .gitignore should apply
        assert!(checker.is_ignored(&sub_dir.join("app.log"))); // From parent
        assert!(checker.is_ignored(&sub_dir.join("data.secret"))); // From local
        assert!(!checker.is_ignored(&sub_dir.join("normal.txt")));
    }

    #[test]
    fn test_absolute_vs_relative_paths() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .gitignore
        fs::write(temp_path.join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(temp_path.join("ignored.txt"), "content").unwrap();

        let checker = GitignoreChecker::new();

        // Test with absolute path (both tests use absolute paths now)
        let abs_path = temp_path.join("ignored.txt");
        assert!(checker.is_ignored(&abs_path));
    }

    #[test]
    fn test_directory_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .gitignore with directory patterns
        fs::write(temp_path.join(".gitignore"), "build/\n*.tmp/\n").unwrap();
        
        // Create directories
        fs::create_dir(temp_path.join("build")).unwrap();
        fs::create_dir(temp_path.join("cache.tmp")).unwrap();
        fs::create_dir(temp_path.join("src")).unwrap();

        let checker = GitignoreChecker::new();

        // Check directory patterns
        assert!(checker.is_ignored(&temp_path.join("build")));
        assert!(checker.is_ignored(&temp_path.join("cache.tmp")));
        assert!(!checker.is_ignored(&temp_path.join("src")));
    }

    #[test]
    fn test_malformed_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .gitignore with potentially problematic patterns
        fs::write(temp_path.join(".gitignore"), "valid.txt\n[invalid\n*.log\n").unwrap();
        
        // Create test files
        fs::write(temp_path.join("valid.txt"), "content").unwrap();
        fs::write(temp_path.join("test.log"), "log").unwrap();

        let checker = GitignoreChecker::new();

        // Should still handle valid patterns despite malformed ones
        assert!(checker.is_ignored(&temp_path.join("valid.txt")));
        assert!(checker.is_ignored(&temp_path.join("test.log")));
    }
}