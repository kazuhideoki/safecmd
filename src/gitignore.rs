use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

pub struct GitignoreChecker {
    builder: GitignoreBuilder,
}

impl GitignoreChecker {
    pub fn new() -> Self {
        let builder = GitignoreBuilder::new(".");
        Self { builder }
    }

    fn get_gitignore_for_path(&self, path: &Path) -> Option<Gitignore> {
        let mut builder = self.builder.clone();
        
        // Get the directory containing the path
        let start_dir = if path.is_absolute() {
            path.parent().map(|p| p.to_path_buf())
        } else {
            std::env::current_dir().ok().map(|cwd| {
                let full_path = cwd.join(path);
                full_path.parent().map(|p| p.to_path_buf()).unwrap_or(cwd)
            })
        }?;
        
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