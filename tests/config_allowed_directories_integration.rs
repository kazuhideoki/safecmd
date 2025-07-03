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
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg("test.txt")
        .assert()
        .success();

    assert!(!allowed_dir.join("test.txt").exists());

    // Test: disallowed directory - should fail
    let mut cmd = Command::cargo_bin("rm").unwrap();
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
    let mut cmd = Command::cargo_bin("rm").unwrap();
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
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&dir1)
        .arg("file1.txt")
        .assert()
        .success();

    assert!(!dir1.join("file1.txt").exists());

    let mut cmd = Command::cargo_bin("rm").unwrap();
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
    let mut cmd = Command::cargo_bin("rm").unwrap();
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
    let mut cmd = Command::cargo_bin("rm").unwrap();
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
    let mut cmd = Command::cargo_bin("rm").unwrap();
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
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&symlink_dir)
        .arg("file.txt")
        .assert()
        .success();

    assert!(!symlink_dir.join("file.txt").exists());
}

#[test]
fn test_relative_paths_within_allowed_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let allowed_dir = temp_path.join("allowed");
    let subdir = allowed_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();

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

    // Create test files
    fs::write(allowed_dir.join("file1.txt"), "content1").unwrap();
    fs::write(subdir.join("file2.txt"), "content2").unwrap();

    // Test: simple relative path
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg("file1.txt")
        .assert()
        .success();

    assert!(!allowed_dir.join("file1.txt").exists());

    // Test: relative path with ./
    fs::write(allowed_dir.join("file3.txt"), "content3").unwrap();
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg("./file3.txt")
        .assert()
        .success();

    assert!(!allowed_dir.join("file3.txt").exists());

    // Test: relative path to subdirectory
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg("subdir/file2.txt")
        .assert()
        .success();

    assert!(!subdir.join("file2.txt").exists());
}

#[test]
fn test_relative_paths_to_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let allowed_dir = temp_path.join("allowed");
    let subdir = allowed_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();

    // Create file outside allowed directory
    let outside_file = temp_path.join("outside.txt");
    fs::write(&outside_file, "secret").unwrap();

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

    // Test: try to access parent directory file with ../
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&subdir)
        .arg("../../outside.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "path is not in allowed directories",
        ));

    assert!(outside_file.exists());

    // But accessing files within allowed directory via .. should work
    fs::write(allowed_dir.join("allowed_file.txt"), "content").unwrap();
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&subdir)
        .arg("../allowed_file.txt")
        .assert()
        .success();

    assert!(!allowed_dir.join("allowed_file.txt").exists());
}

#[test]
fn test_complex_relative_paths() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let allowed_dir = temp_path.join("allowed");
    let subdir1 = allowed_dir.join("subdir1");
    let subdir2 = allowed_dir.join("subdir2");
    fs::create_dir_all(&subdir1).unwrap();
    fs::create_dir_all(&subdir2).unwrap();

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

    // Create test files
    fs::write(allowed_dir.join("root_file.txt"), "root").unwrap();
    fs::write(subdir2.join("target.txt"), "target").unwrap();

    // Test: complex path like ./subdir1/../root_file.txt
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&allowed_dir)
        .arg("./subdir1/../root_file.txt")
        .assert()
        .success();

    assert!(!allowed_dir.join("root_file.txt").exists());

    // Test: accessing sibling directory from subdirectory
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&subdir1)
        .arg("../subdir2/target.txt")
        .assert()
        .success();

    assert!(!subdir2.join("target.txt").exists());
}

#[test]
fn test_relative_paths_from_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let allowed_dir = temp_path.join("allowed");
    let deep_subdir = allowed_dir.join("level1").join("level2");
    fs::create_dir_all(&deep_subdir).unwrap();

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

    // Create test file in deep subdirectory
    fs::write(deep_subdir.join("deep_file.txt"), "deep").unwrap();

    // Test: relative path from deep subdirectory
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&deep_subdir)
        .arg("deep_file.txt")
        .assert()
        .success();

    assert!(!deep_subdir.join("deep_file.txt").exists());
}

#[test]
fn test_disallowed_current_directory_with_allowed_target() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

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

    // Create test file in allowed directory
    let allowed_file = allowed_dir.join("allowed.txt");
    fs::write(&allowed_file, "allowed").unwrap();

    // Test: try to delete allowed file from disallowed directory
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&disallowed_dir)
        .arg(&allowed_file)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "current directory is not in the allowed directories list",
        ));

    // File should still exist because command was rejected early
    assert!(allowed_file.exists());
}
