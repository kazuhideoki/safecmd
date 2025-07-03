use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_allowed_gitignores_allows_protected_files() {
    let temp_dir = TempDir::new().unwrap();
    let allowed_dir = temp_dir.path();

    // Create a config file with allowed_gitignores patterns
    let config_path = allowed_dir.join("config.toml");
    let config_content = format!(
        r#"
[allowed_directories]
paths = ["{allowed}"]

[allowed_gitignores]
patterns = ["*.log", "build/", "*.cache"]
"#,
        allowed = allowed_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create .gitignore that protects these files
    fs::write(allowed_dir.join(".gitignore"), "*.log\nbuild/\n*.cache\n").unwrap();

    // Create test files
    fs::write(allowed_dir.join("app.log"), "log content").unwrap();
    fs::write(allowed_dir.join("data.cache"), "cache content").unwrap();
    fs::create_dir(allowed_dir.join("build")).unwrap();
    fs::write(allowed_dir.join("build/output.bin"), "binary").unwrap();

    // These files should be removable despite being in .gitignore
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("app.log")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("data.cache")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("-r")
        .arg("build")
        .assert()
        .success();

    // Verify files were deleted
    assert!(!allowed_dir.join("app.log").exists());
    assert!(!allowed_dir.join("data.cache").exists());
    assert!(!allowed_dir.join("build").exists());
}

#[test]
fn test_config_and_local_allowsafecmd_combined() {
    let temp_dir = TempDir::new().unwrap();
    let allowed_dir = temp_dir.path();

    // Create a config file with some allowed patterns
    let config_path = allowed_dir.join("config.toml");
    let config_content = format!(
        r#"
[allowed_directories]
paths = ["{allowed}"]

[allowed_gitignores]
patterns = ["*.log"]
"#,
        allowed = allowed_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create .gitignore
    fs::write(allowed_dir.join(".gitignore"), "*.log\n*.cache\n*.tmp\n").unwrap();

    // Create local .allowsafecmd with additional patterns
    fs::write(allowed_dir.join(".allowsafecmd"), "*.cache\n").unwrap();

    // Create test files
    fs::write(allowed_dir.join("app.log"), "log").unwrap();
    fs::write(allowed_dir.join("data.cache"), "cache").unwrap();
    fs::write(allowed_dir.join("temp.tmp"), "temp").unwrap();

    // app.log should be removable (allowed by config)
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("app.log")
        .assert()
        .success();

    // data.cache should be removable (allowed by local .allowsafecmd)
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("data.cache")
        .assert()
        .success();

    // temp.tmp should NOT be removable (not in either allowlist)
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("temp.tmp")
        .assert()
        .failure()
        .stderr(contains("protected by .gitignore"));

    // Verify correct files were deleted
    assert!(!allowed_dir.join("app.log").exists());
    assert!(!allowed_dir.join("data.cache").exists());
    assert!(allowed_dir.join("temp.tmp").exists());
}

#[test]
fn test_config_allowed_gitignores_with_subdirectories() {
    let temp_dir = TempDir::new().unwrap();
    let allowed_dir = temp_dir.path();
    let sub_dir = allowed_dir.join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    // Create a config file with directory pattern
    let config_path = allowed_dir.join("config.toml");
    let config_content = format!(
        r#"
[allowed_directories]
paths = ["{allowed}"]

[allowed_gitignores]
patterns = ["node_modules/", "__pycache__/"]
"#,
        allowed = allowed_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create .gitignore in subdirectory
    fs::write(sub_dir.join(".gitignore"), "node_modules/\n__pycache__/\n").unwrap();

    // Create test directories
    fs::create_dir(sub_dir.join("node_modules")).unwrap();
    fs::write(sub_dir.join("node_modules/package.json"), "{}").unwrap();
    fs::create_dir(sub_dir.join("__pycache__")).unwrap();
    fs::write(sub_dir.join("__pycache__/cache.pyc"), "bytecode").unwrap();

    // Should be able to remove these directories
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&sub_dir)
        .arg("-r")
        .arg("node_modules")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&sub_dir)
        .arg("-r")
        .arg("__pycache__")
        .assert()
        .success();

    // Verify directories were deleted
    assert!(!sub_dir.join("node_modules").exists());
    assert!(!sub_dir.join("__pycache__").exists());
}

#[test]
fn test_empty_config_allowed_gitignores() {
    let temp_dir = TempDir::new().unwrap();
    let allowed_dir = temp_dir.path();

    // Create a config file with empty allowed_gitignores
    let config_path = allowed_dir.join("config.toml");
    let config_content = format!(
        r#"
[allowed_directories]
paths = ["{allowed}"]

[allowed_gitignores]
patterns = []
"#,
        allowed = allowed_dir.display()
    );
    fs::write(&config_path, config_content).unwrap();

    // Create .gitignore
    fs::write(allowed_dir.join(".gitignore"), "*.log\n").unwrap();

    // Create test file
    fs::write(allowed_dir.join("app.log"), "log").unwrap();

    // Should NOT be removable (empty config patterns, no local .allowsafecmd)
    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .current_dir(&allowed_dir)
        .arg("app.log")
        .assert()
        .failure()
        .stderr(contains("protected by .gitignore"));

    // File should still exist
    assert!(allowed_dir.join("app.log").exists());
}
