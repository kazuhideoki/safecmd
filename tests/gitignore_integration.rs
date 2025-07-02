use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_gitignore_prevents_deletion() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore file
    let gitignore_content = "important.txt\n*.log\nbuild/\n";
    fs::write(temp_path.join(".gitignore"), gitignore_content).unwrap();

    // Create files that should be protected
    fs::write(temp_path.join("important.txt"), "important data").unwrap();
    fs::write(temp_path.join("app.log"), "log data").unwrap();
    fs::create_dir(temp_path.join("build")).unwrap();
    fs::write(temp_path.join("build/output.bin"), "binary data").unwrap();

    // Create a file that should NOT be protected
    fs::write(temp_path.join("regular.txt"), "regular data").unwrap();

    // Test: Try to delete a gitignored file - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("important.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Try to delete a gitignored pattern - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("app.log")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Try to delete a gitignored directory - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-r")
        .arg("build")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Even with -f flag, gitignored files should not be deleted
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-f")
        .arg("important.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Regular file can be deleted
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("regular.txt")
        .assert()
        .success();

    // Verify regular.txt was deleted
    assert!(!temp_path.join("regular.txt").exists());

    // Verify protected files still exist
    assert!(temp_path.join("important.txt").exists());
    assert!(temp_path.join("app.log").exists());
    assert!(temp_path.join("build").exists());
}

#[test]
fn test_gitignore_prevents_deletion_of_files_in_ignored_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore that ignores entire directories
    let gitignore_content = "build/\ncache/\n";
    fs::write(temp_path.join(".gitignore"), gitignore_content).unwrap();

    // Create ignored directories with files
    fs::create_dir(temp_path.join("build")).unwrap();
    fs::write(temp_path.join("build/output.bin"), "binary data").unwrap();
    fs::write(temp_path.join("build/debug.log"), "debug info").unwrap();

    fs::create_dir(temp_path.join("cache")).unwrap();
    fs::write(temp_path.join("cache/temp.dat"), "temp data").unwrap();

    // Create a non-ignored directory with file
    fs::create_dir(temp_path.join("src")).unwrap();
    fs::write(temp_path.join("src/main.rs"), "source code").unwrap();

    // Test: Try to delete a file inside gitignored directory - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("build/output.bin")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Try to delete another file in gitignored directory - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("build/debug.log")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Try to delete file in different gitignored directory - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("cache/temp.dat")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Even with -f flag, files in gitignored directories should not be deleted
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-f")
        .arg("build/output.bin")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: File in non-ignored directory can be deleted
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("src/main.rs")
        .assert()
        .success();

    // Verify results
    assert!(temp_path.join("build/output.bin").exists());
    assert!(temp_path.join("build/debug.log").exists());
    assert!(temp_path.join("cache/temp.dat").exists());
    assert!(!temp_path.join("src/main.rs").exists());
}

#[test]
fn test_gitignore_with_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create nested structure with .gitignore
    let gitignore_content = "secrets/\n";
    fs::write(temp_path.join(".gitignore"), gitignore_content).unwrap();

    // Create nested directories
    fs::create_dir_all(temp_path.join("secrets/deep")).unwrap();
    fs::write(temp_path.join("secrets/password.txt"), "secret").unwrap();
    fs::write(temp_path.join("secrets/deep/key.pem"), "key").unwrap();

    // Test: Try to delete gitignored nested directory
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&temp_path)
        .arg("-r")
        .arg("secrets")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Verify directory still exists
    assert!(temp_path.join("secrets").exists());
    assert!(temp_path.join("secrets/password.txt").exists());
}

#[test]
fn test_gitignore_respects_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create parent .gitignore
    let gitignore_content = "*.secret\n";
    fs::write(temp_path.join(".gitignore"), gitignore_content).unwrap();

    // Create subdirectory
    let subdir = temp_path.join("subdir");
    fs::create_dir(&subdir).unwrap();

    // Create files in subdirectory
    fs::write(subdir.join("data.secret"), "secret data").unwrap();
    fs::write(subdir.join("data.txt"), "normal data").unwrap();

    // Test: Parent .gitignore should be respected
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&subdir)
        .arg("data.secret")
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Test: Non-gitignored file can be deleted
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.current_dir(&subdir).arg("data.txt").assert().success();

    // Verify results
    assert!(subdir.join("data.secret").exists());
    assert!(!subdir.join("data.txt").exists());
}
