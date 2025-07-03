use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_allowlist_overrides_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore that protects *.log files
    fs::write(temp_path.join(".gitignore"), "*.log\n").unwrap();

    // Create .allowsafecmd that allows specific log files
    fs::write(temp_path.join(".allowsafecmd"), "debug.log\n").unwrap();

    // Create test files
    fs::write(temp_path.join("debug.log"), "debug content").unwrap();
    fs::write(temp_path.join("error.log"), "error content").unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // debug.log should be removable (allowed by .allowsafecmd)
    cmd.arg(temp_path.join("debug.log")).assert().success();

    // Verify file was removed
    assert!(!temp_path.join("debug.log").exists());

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // error.log should still be protected (not in .allowsafecmd)
    cmd.arg(temp_path.join("error.log"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    // Verify file still exists
    assert!(temp_path.join("error.log").exists());
}

#[test]
fn test_allowlist_directory_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore that protects build directories
    fs::write(temp_path.join(".gitignore"), "build/\n").unwrap();

    // Create .allowsafecmd that allows build directories
    fs::write(temp_path.join(".allowsafecmd"), "build/\n").unwrap();

    // Create build directory with files
    fs::create_dir(temp_path.join("build")).unwrap();
    fs::write(temp_path.join("build/output.bin"), "binary").unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // build directory should be removable with -r flag
    cmd.arg("-r")
        .arg(temp_path.join("build"))
        .assert()
        .success();

    // Verify directory was removed
    assert!(!temp_path.join("build").exists());
}

#[test]
fn test_allowlist_wildcard_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore that protects all temp files
    fs::write(temp_path.join(".gitignore"), "*.tmp\n").unwrap();

    // Create .allowsafecmd that allows cache.tmp specifically
    fs::write(temp_path.join(".allowsafecmd"), "cache.tmp\n*.cache\n").unwrap();

    // Create test files
    fs::write(temp_path.join("cache.tmp"), "cache").unwrap();
    fs::write(temp_path.join("data.tmp"), "data").unwrap();
    fs::write(temp_path.join("index.cache"), "index").unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // cache.tmp should be removable (explicitly allowed)
    cmd.arg(temp_path.join("cache.tmp")).assert().success();

    assert!(!temp_path.join("cache.tmp").exists());

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // data.tmp should be protected (not in allowlist)
    cmd.arg(temp_path.join("data.tmp"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    assert!(temp_path.join("data.tmp").exists());

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // index.cache should be removable (matches *.cache pattern)
    cmd.arg(temp_path.join("index.cache")).assert().success();

    assert!(!temp_path.join("index.cache").exists());
}

#[test]
fn test_nested_allowlist_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create parent .gitignore
    fs::write(temp_path.join(".gitignore"), "*.secret\n").unwrap();

    // Create parent .allowsafecmd
    fs::write(temp_path.join(".allowsafecmd"), "test.secret\n").unwrap();

    // Create subdirectory
    let sub_dir = temp_path.join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    // Create subdirectory .allowsafecmd with additional patterns
    fs::write(sub_dir.join(".allowsafecmd"), "data.secret\n").unwrap();

    // Create test files
    fs::write(sub_dir.join("test.secret"), "test").unwrap();
    fs::write(sub_dir.join("data.secret"), "data").unwrap();
    fs::write(sub_dir.join("key.secret"), "key").unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // Both test.secret and data.secret should be removable
    cmd.arg(sub_dir.join("test.secret")).assert().success();

    assert!(!sub_dir.join("test.secret").exists());

    let mut cmd = Command::cargo_bin("rm").unwrap();

    cmd.arg(sub_dir.join("data.secret")).assert().success();

    assert!(!sub_dir.join("data.secret").exists());

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // key.secret should still be protected
    cmd.arg(sub_dir.join("key.secret"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("protected by .gitignore"));

    assert!(sub_dir.join("key.secret").exists());
}

#[test]
fn test_allowlist_without_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .allowsafecmd without corresponding .gitignore
    fs::write(temp_path.join(".allowsafecmd"), "*.log\n").unwrap();

    // Create test file
    fs::write(temp_path.join("app.log"), "log").unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // File should be removable (no gitignore protection)
    cmd.arg(temp_path.join("app.log")).assert().success();

    assert!(!temp_path.join("app.log").exists());
}

#[test]
fn test_allowlist_files_in_gitignored_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create .gitignore that protects node_modules
    fs::write(temp_path.join(".gitignore"), "node_modules/\n").unwrap();

    // Create .allowsafecmd that allows specific files in node_modules
    fs::write(
        temp_path.join(".allowsafecmd"),
        "node_modules/\nnode_modules/**\n",
    )
    .unwrap();

    // Create node_modules directory with files
    let node_modules = temp_path.join("node_modules");
    fs::create_dir(&node_modules).unwrap();
    fs::write(node_modules.join("package.json"), "{}").unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();

    // Files in allowed directory should be removable
    cmd.arg(node_modules.join("package.json"))
        .assert()
        .success();

    assert!(!node_modules.join("package.json").exists());

    // Directory itself should also be removable with -r
    let mut cmd = Command::cargo_bin("rm").unwrap();

    cmd.arg("-r").arg(&node_modules).assert().success();

    assert!(!node_modules.exists());
}
