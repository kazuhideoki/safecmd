use assert_cmd::prelude::*;
use predicates;
use std::fs::{self, File};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn file_is_trashed() {
    // create a temporary file to delete
    let temp_dir = tempdir().expect("create tmp dir");
    let file_path = temp_dir.path().join("example.txt");
    File::create(&file_path).expect("create file");

    // run the binary
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg(&file_path)
        .assert()
        .success();

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
    Command::cargo_bin("safecmd")
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
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg("-d")
        .arg(&dir_path)
        .assert()
        .success();

    // directory should no longer exist
    assert!(!dir_path.exists(), "directory still exists at original path");
}

#[test]
fn non_empty_directory_with_d_flag_fails() {
    // create a directory with a file
    let temp_dir = tempdir().expect("create tmp dir");
    let dir_path = temp_dir.path().join("non_empty_dir");
    fs::create_dir(&dir_path).expect("create directory");
    File::create(dir_path.join("file.txt")).expect("create file");

    // run the binary with -d flag (should fail)
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg("-d")
        .arg(&dir_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("Directory not empty"));

    // directory should still exist
    assert!(dir_path.exists(), "non-empty directory was removed with -d flag");
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
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg("-r")
        .arg(&dir_path)
        .assert()
        .success();

    // directory should no longer exist
    assert!(!dir_path.exists(), "directory still exists at original path");
}

#[test]
fn non_existent_file_without_f_flag_fails() {
    // run the binary on a non-existent file without -f
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg("non_existent_file.txt")
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot remove"));
}

#[test]
fn non_existent_file_with_f_flag_succeeds() {
    // run the binary on a non-existent file with -f
    Command::cargo_bin("safecmd")
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
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg("-f")
        .arg(&existing_file)
        .arg("non_existent.txt")
        .arg("another_non_existent.txt")
        .assert()
        .success();

    // existing file should be removed
    assert!(!existing_file.exists(), "existing file was not removed");
}