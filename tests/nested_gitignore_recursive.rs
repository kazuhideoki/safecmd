use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_recursive_deletion_respects_nested_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore at root that ignores dist/
    let gitignore_content = "dist/\n";
    fs::write(temp_path.join(".gitignore"), gitignore_content).unwrap();

    // Create dist/some_dir with files
    fs::create_dir_all(temp_path.join("dist/some_dir")).unwrap();
    fs::write(temp_path.join("dist/some_dir/file1.txt"), "content1").unwrap();
    fs::write(temp_path.join("dist/some_dir/file2.txt"), "content2").unwrap();

    // Also create a non-ignored directory for comparison
    fs::create_dir_all(temp_path.join("allowed/some_dir")).unwrap();
    fs::write(temp_path.join("allowed/some_dir/file1.txt"), "content1").unwrap();

    // Test: Try to recursively delete a subdirectory of gitignored directory - should fail
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-r")
        .arg("dist/some_dir")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Verify the directory and files still exist
    assert!(temp_path.join("dist/some_dir").exists());
    assert!(temp_path.join("dist/some_dir/file1.txt").exists());
    assert!(temp_path.join("dist/some_dir/file2.txt").exists());

    // Test: Non-ignored directory can be deleted
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-r")
        .arg("allowed/some_dir")
        .assert()
        .success();

    // Verify non-ignored directory was deleted
    assert!(!temp_path.join("allowed/some_dir").exists());
}

#[test]
fn test_recursive_deletion_checks_all_contents() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a directory with mixed content
    fs::create_dir_all(temp_path.join("project/src")).unwrap();
    fs::write(temp_path.join("project/src/main.rs"), "fn main() {}").unwrap();
    fs::write(temp_path.join("project/README.md"), "# Project").unwrap();

    // Create .gitignore inside the project directory
    let gitignore_content = "src/\n";
    fs::write(temp_path.join("project/.gitignore"), gitignore_content).unwrap();

    // Test: Try to recursively delete the project directory - should fail
    // because it contains gitignored content
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-r")
        .arg("project")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Verify directory still exists
    assert!(temp_path.join("project").exists());
    assert!(temp_path.join("project/src/main.rs").exists());
}
