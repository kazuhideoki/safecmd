use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use std::process::Output;
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

/// rm 実行結果を返し、trash 非対応環境の削除失敗はスキップ可能として扱う。
fn run_rm_or_skip_for_trash(cmd: &mut Command) -> Option<Output> {
    let output = cmd.output().expect("run rm");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Error during a `trash` operation") {
        return None;
    }
    Some(output)
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

#[cfg(unix)]
#[test]
fn cp_recursive_no_clobber_denies_writes_via_symlink_under_destination() {
    // 再帰コピー時にコピー先配下のシンボリックリンク経由で許可範囲外へ書き込む経路を拒否することを確認する。
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let outside_dir = temp_path.join("outside");
    fs::create_dir(&workspace_dir).unwrap();
    fs::create_dir(&outside_dir).unwrap();

    let source_dir = workspace_dir.join("src");
    let source_nested_dir = source_dir.join("nested");
    let dest_dir = workspace_dir.join("dest");
    let existing_dest_subdir = dest_dir.join("src");
    let link_path = existing_dest_subdir.join("nested");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&source_nested_dir).unwrap();
    fs::create_dir(&dest_dir).unwrap();
    fs::create_dir(&existing_dest_subdir).unwrap();
    fs::write(source_nested_dir.join("payload.txt"), "payload").unwrap();
    symlink(&outside_dir, &link_path).unwrap();

    let config_path = write_config(temp_path, &[]);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cp"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg("-rn")
        .arg("src")
        .arg("dest")
        .assert()
        .failure()
        .stderr(predicate::str::contains("path is outside allowed scope"));

    assert!(
        !outside_dir.join("payload.txt").exists(),
        "symlink traversal must not create files outside allowed scope"
    );
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

#[test]
fn rm_recursive_continues_when_one_path_is_outside_allowed_scope() {
    // 許可対象と拒否対象が混在した -r 実行で、許可対象の削除を継続しつつ終了コード非0となることを確認する。
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let workspace_dir = temp_path.join("workspace");
    let allowed_dir = workspace_dir.join("allowed_recursive");
    let forbidden_dir = temp_path.join("forbidden_recursive");
    fs::create_dir_all(allowed_dir.join("nested")).unwrap();
    fs::create_dir_all(forbidden_dir.join("nested")).unwrap();
    fs::write(allowed_dir.join("nested").join("ok.txt"), "ok").unwrap();
    fs::write(forbidden_dir.join("nested").join("ng.txt"), "ng").unwrap();

    let config_path = write_config(temp_path, &[]);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rm"));
    cmd.env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .current_dir(&workspace_dir)
        .arg("-r")
        .arg(&forbidden_dir)
        .arg("allowed_recursive");

    let Some(output) = run_rm_or_skip_for_trash(&mut cmd) else {
        return;
    };

    assert!(!output.status.success(), "exit status should be non-zero");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("path is outside allowed scope"));
    assert!(
        !allowed_dir.exists(),
        "allowed directory should still be removed"
    );
    assert!(forbidden_dir.exists(), "forbidden directory should remain");
}
