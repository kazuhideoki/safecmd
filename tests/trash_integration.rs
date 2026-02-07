use assert_cmd::prelude::*;
use predicates;
use std::fs::{self, File};
use std::process::Command;
use tempfile::tempdir;

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
fn file_is_trashed() {
    // create a temporary file to delete
    let temp_dir = tempdir().expect("create tmp dir");
    let file_path = temp_dir.path().join("example.txt");
    File::create(&file_path).expect("create file");

    // run the binary
    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg(&file_path);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    // original file should no longer exist
    assert!(!file_path.exists(), "file still exists at original path");
}

#[test]
fn directory_without_flags_fails() {
    // create a directory
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("some_dir");
    fs::create_dir(&dir_path).expect("create directory");

    // run the binary without flags (should fail)
    Command::cargo_bin("rm")
        .expect("binary exists")
        .arg(&dir_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("is a directory"));

    // directory should still exist
    assert!(dir_path.exists(), "directory was removed without flags");
}

#[test]
fn empty_directory_with_d_flag() {
    // create an empty directory
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("empty_dir");
    fs::create_dir(&dir_path).expect("create directory");

    // run the binary with -d flag
    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-d").arg(&dir_path);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    // directory should no longer exist
    assert!(
        !dir_path.exists(),
        "directory still exists at original path"
    );
}

#[test]
fn non_empty_directory_with_d_flag_fails() {
    // create a directory with a file
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("non_empty_dir");
    fs::create_dir(&dir_path).expect("create directory");
    File::create(dir_path.join("file.txt")).expect("create file");

    // run the binary with -d flag (should fail)
    Command::cargo_bin("rm")
        .expect("binary exists")
        .arg("-d")
        .arg(&dir_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("Directory not empty"));

    // directory should still exist
    assert!(
        dir_path.exists(),
        "non-empty directory was removed with -d flag"
    );
}

#[test]
fn directory_with_r_flag() {
    // create a directory with files
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("dir_with_files");
    fs::create_dir(&dir_path).expect("create directory");

    // create files inside the directory
    File::create(dir_path.join("file1.txt")).expect("create file1");
    File::create(dir_path.join("file2.txt")).expect("create file2");

    // create subdirectory with file
    let sub_dir = dir_path.join("subdir");
    fs::create_dir(&sub_dir).expect("create subdirectory");
    File::create(sub_dir.join("file3.txt")).expect("create file3");

    // run the binary with -r flag
    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-r").arg(&dir_path);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    // directory should no longer exist
    assert!(
        !dir_path.exists(),
        "directory still exists at original path"
    );
}

#[test]
fn non_existent_file_without_f_flag_fails() {
    // run the binary on a non-existent file without -f
    Command::cargo_bin("rm")
        .expect("binary exists")
        .arg("non_existent_file.txt")
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot remove"));
}

#[test]
fn non_existent_file_with_f_flag_succeeds() {
    // run the binary on a non-existent file with -f
    Command::cargo_bin("rm")
        .expect("binary exists")
        .arg("-f")
        .arg("non_existent_file.txt")
        .assert()
        .success();
}

#[test]
fn mixed_files_with_f_flag() {
    // create a temporary file
    let temp_dir = tempdir().expect("create tmp dir");
    let existing_file = temp_dir.path().join("existing.txt");
    File::create(&existing_file).expect("create file");

    // run the binary with -f on both existing and non-existent files
    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-f")
        .arg(&existing_file)
        .arg("non_existent.txt")
        .arg("another_non_existent.txt");
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    // existing file should be removed
    assert!(!existing_file.exists(), "existing file was not removed");
}

#[test]
fn combined_flags_rf() {
    // test -rf combined flag
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("dir_to_remove");
    fs::create_dir(&dir_path).expect("create directory");
    File::create(dir_path.join("file.txt")).expect("create file");

    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-rf").arg(&dir_path);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(!dir_path.exists(), "directory was not removed with -rf");
}

#[test]
fn combined_flags_fr() {
    // test -fr combined flag (opposite order)
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("dir_to_remove");
    fs::create_dir(&dir_path).expect("create directory");
    File::create(dir_path.join("file.txt")).expect("create file");

    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-fr").arg(&dir_path);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(!dir_path.exists(), "directory was not removed with -fr");
}

#[test]
fn combined_flags_df() {
    // test -df combined flag on empty directory
    let temp_dir = tempdir().expect("create tmp dir");
    let empty_dir = temp_dir.path().join("empty_dir");
    fs::create_dir(&empty_dir).expect("create directory");

    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-df").arg(&empty_dir);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(
        !empty_dir.exists(),
        "empty directory was not removed with -df"
    );
}

#[test]
fn combined_flags_drf() {
    // test -drf combined flag (all flags)
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("dir_with_files");
    fs::create_dir(&dir_path).expect("create directory");
    File::create(dir_path.join("file.txt")).expect("create file");

    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-drf").arg(&dir_path);
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(!dir_path.exists(), "directory was not removed with -drf");
}

#[test]
fn combined_flags_frd() {
    // test -frd combined flag (different order)
    let temp_dir = tempdir().expect("create tmp dir");
    let existing_dir = temp_dir.path().join("existing");
    fs::create_dir(&existing_dir).expect("create directory");

    let mut cmd = Command::cargo_bin("rm").expect("binary exists");
    cmd.arg("-frd").arg(&existing_dir).arg("non_existent_dir");
    if !assert_rm_success_or_skip(&mut cmd) {
        return;
    }

    assert!(
        !existing_dir.exists(),
        "directory was not removed with -frd"
    );
}

#[test]
fn disable_allowed_directories_env_var() {
    // create a directory that is NOT in current directory scope
    let temp_dir = tempdir().expect("create tmp dir");
    let file_path = temp_dir.path().join("test.txt");

    // create a minimal config so the binary does not generate one
    let config_dir = temp_dir.path().join(".config");
    fs::create_dir(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, "[additional_allowed_directories]\npaths=[]").unwrap();

    // Without the environment variable, this should fail due to directory restriction
    Command::cargo_bin("rm")
        .expect("binary exists")
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .env("SAFECMD_CONFIG_PATH", &config_path)
        .arg("-f")
        .arg(&file_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("path is outside allowed scope"));

    // With SAFECMD_DISABLE_ALLOWED_DIRECTORIES=1, it should succeed
    Command::cargo_bin("rm")
        .expect("binary exists")
        .env("SAFECMD_DISABLE_TEST_MODE", "1")
        .env("SAFECMD_CONFIG_PATH", &config_path)
        .env("SAFECMD_DISABLE_ALLOWED_DIRECTORIES", "1")
        .arg("-f")
        .arg(&file_path)
        .assert()
        .success();
}
