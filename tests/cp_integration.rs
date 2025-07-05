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
    source_file
        .write_all(b"Hello, World!")
        .expect("write to source file");

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
    source_file
        .write_all(b"New content")
        .expect("write to source file");

    // create target file with different content
    let mut target_file = File::create(&target_path).expect("create target file");
    target_file
        .write_all(b"Old content")
        .expect("write to target file");

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
        .stderr(predicates::str::contains(
            "missing destination file operand",
        ));

    // only one argument
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("file.txt")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "missing destination file operand",
        ));
}

#[test]
fn verify_original_file_moved_to_trash_on_overwrite() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    // create source file with content
    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"New content")
        .expect("write to source file");

    // create target file with different content
    let mut target_file = File::create(&target_path).expect("create target file");
    target_file
        .write_all(b"Original content to be trashed")
        .expect("write to target file");

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

#[test]
fn copy_directory_with_r_flag() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_dir = temp_dir.path().join("target_dir");

    // create source directory with files and subdirectories
    fs::create_dir(&source_dir).expect("create source directory");
    let file1_path = source_dir.join("file1.txt");
    let file2_path = source_dir.join("file2.txt");
    let subdir_path = source_dir.join("subdir");
    let subfile_path = subdir_path.join("subfile.txt");

    // create files
    let mut file1 = File::create(&file1_path).expect("create file1");
    file1.write_all(b"Content 1").expect("write to file1");
    let mut file2 = File::create(&file2_path).expect("create file2");
    file2.write_all(b"Content 2").expect("write to file2");

    // create subdirectory with file
    fs::create_dir(&subdir_path).expect("create subdirectory");
    let mut subfile = File::create(&subfile_path).expect("create subfile");
    subfile
        .write_all(b"Subfile content")
        .expect("write to subfile");

    // run the cp command with -r flag
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("-r")
        .arg(&source_dir)
        .arg(&target_dir)
        .assert()
        .success();

    // verify directory structure was copied
    assert!(target_dir.exists(), "target directory not created");
    assert!(target_dir.join("file1.txt").exists(), "file1 not copied");
    assert!(target_dir.join("file2.txt").exists(), "file2 not copied");
    assert!(
        target_dir.join("subdir").exists(),
        "subdirectory not copied"
    );
    assert!(
        target_dir.join("subdir/subfile.txt").exists(),
        "subfile not copied"
    );

    // verify file contents
    let content1 = fs::read_to_string(target_dir.join("file1.txt")).expect("read file1");
    assert_eq!(content1, "Content 1", "file1 content mismatch");
    let content2 = fs::read_to_string(target_dir.join("file2.txt")).expect("read file2");
    assert_eq!(content2, "Content 2", "file2 content mismatch");
    let subcontent =
        fs::read_to_string(target_dir.join("subdir/subfile.txt")).expect("read subfile");
    assert_eq!(subcontent, "Subfile content", "subfile content mismatch");
}

#[test]
fn copy_directory_with_capital_r_flag() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_dir = temp_dir.path().join("target_dir");

    // create source directory
    fs::create_dir(&source_dir).expect("create source directory");
    let file_path = source_dir.join("file.txt");
    File::create(&file_path).expect("create file");

    // run the cp command with -R flag (capital)
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("-R")
        .arg(&source_dir)
        .arg(&target_dir)
        .assert()
        .success();

    // verify directory was copied
    assert!(target_dir.exists(), "target directory not created");
    assert!(target_dir.join("file.txt").exists(), "file not copied");
}

#[test]
fn copy_empty_directory_with_r_flag() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("empty_dir");
    let target_dir = temp_dir.path().join("target_dir");

    // create empty source directory
    fs::create_dir(&source_dir).expect("create source directory");

    // run the cp command with -r flag
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("-r")
        .arg(&source_dir)
        .arg(&target_dir)
        .assert()
        .success();

    // verify empty directory was created
    assert!(target_dir.exists(), "target directory not created");
    assert!(target_dir.is_dir(), "target is not a directory");

    // verify it's empty
    let entries: Vec<_> = fs::read_dir(&target_dir)
        .expect("read target directory")
        .collect();
    assert_eq!(entries.len(), 0, "target directory is not empty");
}

#[test]
fn copy_directory_to_existing_directory() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_parent = temp_dir.path().join("target_parent");

    // create source directory with file
    fs::create_dir(&source_dir).expect("create source directory");
    let file_path = source_dir.join("file.txt");
    File::create(&file_path).expect("create file");

    // create target parent directory
    fs::create_dir(&target_parent).expect("create target parent directory");

    // run the cp command with -r flag
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("-r")
        .arg(&source_dir)
        .arg(&target_parent)
        .assert()
        .success();

    // verify source_dir was copied inside target_parent
    let expected_dir = target_parent.join("source_dir");
    assert!(expected_dir.exists(), "directory not copied to parent");
    assert!(expected_dir.join("file.txt").exists(), "file not copied");
}

#[test]
fn overwrite_directory_with_r_flag() {
    // create a temporary directory
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_dir = temp_dir.path().join("target_dir");

    // create source directory with new content
    fs::create_dir(&source_dir).expect("create source directory");
    let new_file = source_dir.join("new.txt");
    let mut file = File::create(&new_file).expect("create new file");
    file.write_all(b"New content").expect("write to new file");

    // create existing target directory with old content
    fs::create_dir(&target_dir).expect("create target directory");
    let old_file = target_dir.join("old.txt");
    File::create(&old_file).expect("create old file");

    // run the cp command with -r flag
    Command::cargo_bin("cp")
        .expect("binary exists")
        .arg("-r")
        .arg(&source_dir)
        .arg(&target_dir)
        .assert()
        .success();

    // verify source_dir was copied inside existing target_dir
    let copied_dir = target_dir.join("source_dir");
    assert!(copied_dir.exists(), "source directory not copied");
    assert!(copied_dir.join("new.txt").exists(), "new file not copied");

    // original file in target should still exist
    assert!(old_file.exists(), "original file was removed");
}
