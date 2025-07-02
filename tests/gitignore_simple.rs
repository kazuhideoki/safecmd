use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_gitignore_basic_functionality() {
    // Get the current directory before test
    let original_dir = std::env::current_dir().unwrap();
    
    // Create temp directory and change to it
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    std::env::set_current_dir(&temp_path).unwrap();
    
    // Create .gitignore file
    fs::write(".gitignore", "important.txt\n*.log\nbuild/\n").unwrap();
    
    // Create files
    fs::write("important.txt", "important data").unwrap();
    fs::write("app.log", "log data").unwrap();
    fs::write("regular.txt", "regular data").unwrap();
    fs::create_dir("build").unwrap();
    
    // Build the path to safecmd binary
    let safecmd_bin = original_dir.join("target/debug/safecmd");
    
    // Test: Try to delete a gitignored file - should fail
    Command::new(&safecmd_bin)
        .arg("important.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));
    
    // Test: Try to delete a gitignored pattern - should fail
    Command::new(&safecmd_bin)
        .arg("app.log")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));
    
    // Test: Try to delete a gitignored directory - should fail
    Command::new(&safecmd_bin)
        .arg("-r")
        .arg("build")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));
    
    // Test: Even with -f flag, gitignored files should not be deleted
    Command::new(&safecmd_bin)
        .arg("-f")
        .arg("important.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));
    
    // Test: Regular file can be deleted
    Command::new(&safecmd_bin)
        .arg("regular.txt")
        .assert()
        .success();
    
    // Verify regular.txt was deleted (moved to trash)
    assert!(!std::path::Path::new("regular.txt").exists());
    
    // Verify protected files still exist
    assert!(std::path::Path::new("important.txt").exists());
    assert!(std::path::Path::new("app.log").exists());
    assert!(std::path::Path::new("build").exists());
    
    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}