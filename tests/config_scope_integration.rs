use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// 設定ファイルを作成する。
fn write_config(
    temp_path: &std::path::Path,
    additional_paths: &[std::path::PathBuf],
) -> std::path::PathBuf {
    let config_dir = temp_path.join(".config");
    fs::create_dir(&config_dir).unwrap();

    let paths = additional_paths
        .iter()
        .map(|p| format!("\"{}\"", p.display()))
        .collect::<Vec<_>>()
        .join(", ");

    let config_content = format!(
        r#"[additional_allowed_directories]
paths = [{}]
"#,
        paths
    );

    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content).unwrap();
    config_path
}

/// rm 実行結果を確認し、trash が使えない環境では成功系テストをスキップ扱いにする。
fn assert_rm_success_or_skip(cmd: &mut Command) -> bool {
    let output = cmd.output().expect("run rm");
    if output.status.success() {
        return true;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Error during a `trash` operation") {
        return false;
    }

    panic!("rm failed unexpectedly: {stderr}");
}

#[test]
fn rm_allows_current_directory_scope_without_additional_paths() {
    // カレントディレクトリ配下は追加設定なしでも削除できることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    let config_path = write_config(temp_path, &[]);
    let target_file = temp_path.join("target.txt");
    fs::write(&target_file, "content").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rm"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("-f")
        .arg("target.txt");
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(!target_file.exists());
}

#[test]
fn rm_allows_path_in_additional_allowed_directories() {
    // additional_allowed_directories 配下は絶対パス指定でも削除できることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let external_dir = temp_path.join("external");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&external_dir).unwrap();

    let external_file = external_dir.join("from_external.txt");
    fs::write(&external_file, "content").unwrap();

    let config_path = write_config(temp_path, std::slice::from_ref(&external_dir));

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rm"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg("-f")
        .arg(&external_file);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(!external_file.exists());
}

#[test]
fn rm_denies_path_outside_current_and_additional_scopes() {
    // カレントと追加許可のどちらにも属さないパスは拒否されることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let additional_dir = temp_path.join("additional");
    let forbidden_dir = temp_path.join("forbidden");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&additional_dir).unwrap();
    fs::create_dir(&forbidden_dir).unwrap();

    let forbidden_file = forbidden_dir.join("secret.txt");

    let config_path = write_config(temp_path, std::slice::from_ref(&additional_dir));

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rm"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg("-f")
        .arg(&forbidden_file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("path is outside allowed scope"));
}

#[test]
fn rm_denies_parent_traversal_outside_current_directory_scope() {
    // ../ を使ってカレント配下外へ出る操作が拒否されることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let subdir = workspace_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();

    let config_path = write_config(temp_path, &[]);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rm"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&subdir)
        .arg("-f")
        .arg("../../outside.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("path is outside allowed scope"));
}

#[test]
fn cp_allows_source_in_additional_allowed_directories() {
    // additional_allowed_directories の source をカレント配下へコピーできることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let external_dir = temp_path.join("external");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&external_dir).unwrap();

    let source_file = external_dir.join("from_external.txt");
    fs::write(&source_file, "external-content").unwrap();

    let config_path = write_config(temp_path, std::slice::from_ref(&external_dir));

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cp"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg(source_file.as_os_str())
        .arg("copied.txt")
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(workspace_dir.join("copied.txt")).unwrap(),
        "external-content"
    );
}

#[test]
fn cp_allows_target_in_additional_allowed_directories() {
    // カレント配下の source を additional_allowed_directories 配下へコピーできることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let external_dir = temp_path.join("external");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&external_dir).unwrap();

    let source_file = workspace_dir.join("source.txt");
    fs::write(&source_file, "workspace-content").unwrap();

    let external_target = external_dir.join("copied.txt");
    let config_path = write_config(temp_path, std::slice::from_ref(&external_dir));

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cp"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg("source.txt")
        .arg(&external_target)
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&external_target).unwrap(),
        "workspace-content"
    );
}

#[test]
fn cp_denies_source_outside_current_and_additional_scopes() {
    // カレント配下外かつ追加許可外の source を拒否することを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let additional_dir = temp_path.join("additional");
    let forbidden_dir = temp_path.join("forbidden");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&additional_dir).unwrap();
    fs::create_dir(&forbidden_dir).unwrap();

    let forbidden_source = forbidden_dir.join("secret.txt");
    fs::write(&forbidden_source, "secret-content").unwrap();

    let config_path = write_config(temp_path, std::slice::from_ref(&additional_dir));

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cp"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg(&forbidden_source)
        .arg("copied.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("path is outside allowed scope"));

    assert!(!workspace_dir.join("copied.txt").exists());
}

#[test]
fn cp_denies_target_outside_allowed_scope() {
    // カレント配下外かつ追加許可外の target を拒否することを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let forbidden_dir = temp_path.join("forbidden");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&forbidden_dir).unwrap();

    let source_file = workspace_dir.join("source.txt");
    let forbidden_target = forbidden_dir.join("copied.txt");
    fs::write(&source_file, "content").unwrap();

    let config_path = write_config(temp_path, &[]);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cp"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg("source.txt")
        .arg(&forbidden_target)
        .assert()
        .failure()
        .stderr(predicate::str::contains("path is outside allowed scope"));

    assert!(!forbidden_target.exists());
}

#[test]
fn rm_continues_after_creating_default_config() {
    // 設定ファイル未作成時に自動生成後そのまま処理継続できることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let config_path = temp_path.join("nonexistent").join("config.toml");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rm"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("-f")
        .arg("file.txt")
        .assert()
        .success();

    assert!(config_path.exists());
    let content = fs::read_to_string(config_path).unwrap();
    assert!(content.contains("[additional_allowed_directories]"));
}

#[test]
fn rm_fails_when_config_contains_relative_additional_path() {
    // 設定ファイルに相対パスが含まれる場合は設定エラーで失敗することを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    let config_path = temp_path.join("config.toml");
    fs::write(
        &config_path,
        r#"[additional_allowed_directories]
paths = ["relative/path"]
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rm").unwrap();
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(temp_path)
        .arg("-f")
        .arg("file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be an absolute path"));
}
