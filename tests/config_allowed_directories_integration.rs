use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_allowed_directories_restriction() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a custom config.toml that only allows a specific subdirectory
    let allowed_dir = temp_path.join("allowed");
    fs::create_dir(&allowed_dir).unwrap();

    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = format!(
        r#"[allowed_directories]
paths = ["{}"]
"#,
        allowed_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create test files in allowed and disallowed directories
    fs::write(allowed_dir.join("test.txt"), "allowed").unwrap();
    fs::write(temp_path.join("disallowed.txt"), "disallowed").unwrap();

    // Test: allowed directory - should succeed
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg("test.txt")
        .assert()
        .success();

    assert!(!allowed_dir.join("test.txt").exists());

    // Test: disallowed directory - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("disallowed.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "current directory is not in the allowed directories list",
        ));

    assert!(temp_path.join("disallowed.txt").exists());
}

#[test]
fn test_allowed_directories_with_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create directory structure
    let project_dir = temp_path.join("projects");
    let subproject_dir = project_dir.join("subproject");
    fs::create_dir_all(&subproject_dir).unwrap();

    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = format!(
        r#"[allowed_directories]
paths = ["{}"]
"#,
        project_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create test files
    fs::write(subproject_dir.join("file.txt"), "content").unwrap();

    // Test: subdirectory of allowed directory - should succeed
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&subproject_dir)
        .arg("file.txt")
        .assert()
        .success();

    assert!(!subproject_dir.join("file.txt").exists());
}

#[test]
fn test_multiple_allowed_directories() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create multiple allowed directories
    let dir1 = temp_path.join("workspace1");
    let dir2 = temp_path.join("workspace2");
    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();

    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = format!(
        r#"[allowed_directories]
paths = ["{}", "{}"]
"#,
        dir1.display(),
        dir2.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create test files
    fs::write(dir1.join("file1.txt"), "content1").unwrap();
    fs::write(dir2.join("file2.txt"), "content2").unwrap();

    // Test both directories
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&dir1)
        .arg("file1.txt")
        .assert()
        .success();

    assert!(!dir1.join("file1.txt").exists());

    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&dir2)
        .arg("file2.txt")
        .assert()
        .success();

    assert!(!dir2.join("file2.txt").exists());
}

#[test]
fn test_absolute_path_outside_allowed_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create allowed and disallowed directories
    let allowed_dir = temp_path.join("allowed");
    let disallowed_dir = temp_path.join("disallowed");
    fs::create_dir(&allowed_dir).unwrap();
    fs::create_dir(&disallowed_dir).unwrap();

    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = format!(
        r#"[allowed_directories]
paths = ["{}"]
"#,
        allowed_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create test file in disallowed directory
    let disallowed_file = disallowed_dir.join("secret.txt");
    fs::write(&disallowed_file, "secret").unwrap();

    // Test: try to delete file outside allowed directory using absolute path
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg(&disallowed_file)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "path is not in allowed directories",
        ));

    assert!(disallowed_file.exists());
}

#[test]
fn test_empty_allowed_directories() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = r#"[allowed_directories]
paths = []
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create test file
    fs::write(temp_path.join("file.txt"), "content").unwrap();

    // Test: empty allowed directories - should fail
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "current directory is not in the allowed directories list",
        ));

    assert!(temp_path.join("file.txt").exists());
}

#[test]
fn test_config_creation_error_message() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let config_path = temp_path.join("nonexistent").join("config.toml");

    // Test: config file doesn't exist and can't be created - should show error
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Created default configuration file at:",
        ))
        .stderr(predicate::str::contains(
            "Please add allowed directories to the config file and try again",
        ));
}

#[test]
fn test_symlink_handling() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create real directory and symlink
    let real_dir = temp_path.join("real");
    let symlink_dir = temp_path.join("symlink");
    fs::create_dir(&real_dir).unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_dir, &symlink_dir).unwrap();

    #[cfg(not(unix))]
    {
        // On non-Unix systems, just create a regular directory
        fs::create_dir(&symlink_dir).unwrap();
    }

    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let config_content = format!(
        r#"[allowed_directories]
paths = ["{}"]
"#,
        real_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create test file
    fs::write(symlink_dir.join("file.txt"), "content").unwrap();

    // Test: accessing through symlink when real path is allowed - should succeed
    let mut cmd = Command::cargo_bin("safecmd").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&symlink_dir)
        .arg("file.txt")
        .assert()
        .success();

    assert!(!symlink_dir.join("file.txt").exists());
}
