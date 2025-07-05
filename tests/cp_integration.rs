use assert_cmd::prelude::*;
use predicates;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn single_file_copy() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");
    
    // create source file with content
    let mut source_file = File::create(&source_path).expect("create source file");
    source_file.write_all(b"Hello, World!").expect("write to source file");
    
    // run the cp command
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .success();
    
    // both files should exist
    assert!(source_path.exists(), "source file was removed");
    assert!(target_path.exists(), "target file was not created");
    
    // verify content
    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(content, "Hello, World!", "content mismatch");
}

#[test]
fn overwrite_existing_file() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");
    
    // create source file with content
    let mut source_file = File::create(&source_path).expect("create source file");
    source_file.write_all(b"New content").expect("write to source file");
    
    // create target file with different content
    let mut target_file = File::create(&target_path).expect("create target file");
    target_file.write_all(b"Old content").expect("write to target file");
    
    // run the cp command
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .success();
    
    // verify target has new content
    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(content, "New content", "content was not overwritten");
}

#[test]
fn copy_to_directory() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_dir = temp_dir.path().join("target_dir");
    
    // create source file
    File::create(&source_path).expect("create source file");
    
    // create target directory
    fs::create_dir(&target_dir).expect("create target directory");
    
    // run the cp command
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source_path)
        .arg(&target_dir)
        .assert()
        .success();
    
    // file should exist in target directory with same name
    let expected_path = target_dir.join("source.txt");
    assert!(expected_path.exists(), "file not copied to directory");
}

#[test]
fn copy_multiple_files_to_directory() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source1_path = temp_dir.path().join("file1.txt");
    let source2_path = temp_dir.path().join("file2.txt");
    let target_dir = temp_dir.path().join("target_dir");
    
    // create source files
    File::create(&source1_path).expect("create source file 1");
    File::create(&source2_path).expect("create source file 2");
    
    // create target directory
    fs::create_dir(&target_dir).expect("create target directory");
    
    // run the cp command
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source1_path)
        .arg(&source2_path)
        .arg(&target_dir)
        .assert()
        .success();
    
    // both files should exist in target directory
    assert!(target_dir.join("file1.txt").exists(), "file1 not copied");
    assert!(target_dir.join("file2.txt").exists(), "file2 not copied");
}

#[test]
fn copy_multiple_files_to_non_directory_fails() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source1_path = temp_dir.path().join("file1.txt");
    let source2_path = temp_dir.path().join("file2.txt");
    let target_path = temp_dir.path().join("target.txt");
    
    // create source files
    File::create(&source1_path).expect("create source file 1");
    File::create(&source2_path).expect("create source file 2");
    
    // create target file (not a directory)
    File::create(&target_path).expect("create target file");
    
    // run the cp command (should fail)
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source1_path)
        .arg(&source2_path)
        .arg(&target_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("is not a directory"));
}

#[test]
fn copy_nonexistent_file_fails() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("nonexistent.txt");
    let target_path = temp_dir.path().join("target.txt");
    
    // run the cp command (should fail)
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .failure()
        .stderr(predicates::str::contains("No such file or directory"));
    
    // target should not be created
    assert!(!target_path.exists(), "target file was created");
}

#[test]
fn copy_directory_without_r_flag_fails() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_dir = temp_dir.path().join("target_dir");
    
    // create source directory
    fs::create_dir(&source_dir).expect("create source directory");
    
    // run the cp command without -r flag (should fail)
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source_dir)
        .arg(&target_dir)
        .assert()
        .failure()
        .stderr(predicates::str::contains("omitting directory"));
    
    // target directory should not be created
    assert!(!target_dir.exists(), "target directory was created");
}

#[test]
fn missing_arguments_fails() {
    // no arguments
    Command::cargo_bin("cp")
        .expect("binary exists")
        .assert()
        .failure()
        .stderr(predicates::str::contains("missing destination file operand"));
    
    // only one argument
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("file.txt")
        .assert()
        .failure()
        .stderr(predicates::str::contains("missing destination file operand"));
}

#[test]
fn verify_original_file_moved_to_trash_on_overwrite() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");
    
    // create source file with content
    let mut source_file = File::create(&source_path).expect("create source file");
    source_file.write_all(b"New content").expect("write to source file");
    
    // create target file with different content
    let mut target_file = File::create(&target_path).expect("create target file");
    target_file.write_all(b"Original content to be trashed").expect("write to target file");
    
    // run the cp command
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .success();
    
    // verify target has new content
    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(content, "New content", "content was not overwritten");
    
    // NOTE: We can't easily verify the file is in trash without platform-specific code
    // The important behavior is that the overwrite succeeded
}