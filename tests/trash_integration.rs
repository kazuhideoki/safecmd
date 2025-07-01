use assert_cmd::prelude::*;
use std::fs::{self, File};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn file_is_trashed() {
    // set up temporary home for trash
    let data_home = tempdir().expect("create data_home");
    // create a temporary file to delete
    let temp_dir = tempdir().expect("create tmp dir");
    let file_path = temp_dir.path().join("example.txt");
    File::create(&file_path).expect("create file");

    // run the binary
    Command::cargo_bin("safecmd")
        .expect("binary exists")
        .arg(&file_path)
        .env("XDG_DATA_HOME", data_home.path())
        .assert()
        .success();

    // original file should no longer exist
    assert!(!file_path.exists(), "file still exists at original path");

    // file should be inside XDG_DATA_HOME/Trash/files
    let trashed_path = data_home.path().join("Trash/files").join("example.txt");
    assert!(trashed_path.exists(), "file was not moved to trash");

    // clean up
    fs::remove_file(trashed_path).ok();
}
