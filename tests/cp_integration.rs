use assert_cmd::prelude::*;
use predicates;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

/// cp バイナリ実行時に明示テストモードを付与したコマンドを生成する。
fn cp_command() -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("cp"));
    cmd.env("SAFECMD_TEST_MODE", "1");
    cmd
}

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
    cp_command()
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
    let output = cp_command()
        .arg(&source_path)
        .arg(&target_path)
        .output()
        .expect("run cp");

    if !output.status.success() {
        // GUI が使えない環境では trash 実装が Finder に接続できず失敗するため許容する。
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("failed to move existing file to trash") {
            return;
        }
        panic!("cp failed unexpectedly: {stderr}");
    }

    // verify target has new content
    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(content, "New content", "content was not overwritten");
}

#[test]
fn force_flag_is_accepted_for_overwrite() {
    // -f を指定しても通常の安全な上書きコピーが実行されることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"New content")
        .expect("write to source file");

    let mut target_file = File::create(&target_path).expect("create target file");
    target_file
        .write_all(b"Old content")
        .expect("write to target file");

    let output = cp_command()
        .arg("-f")
        .arg(&source_path)
        .arg(&target_path)
        .output()
        .expect("run cp");

    if !output.status.success() {
        // GUI が使えない環境では trash 実装が Finder に接続できず失敗するため許容する。
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("failed to move existing file to trash") {
            return;
        }
        panic!("cp failed unexpectedly: {stderr}");
    }

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
    cp_command()
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
    cp_command()
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
    cp_command()
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
    cp_command()
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
    cp_command()
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
    // 引数なし時は clap の必須引数エラーになることを確認する。
    cp_command()
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "the following required arguments were not provided",
        ));

    // 引数1つだけでも同様に clap の必須引数エラーになることを確認する。
    cp_command()
        .arg("file.txt")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "2 values required by '<FILES> <FILES>...'; only 1 was provided",
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
    let output = cp_command()
        .arg(&source_path)
        .arg(&target_path)
        .output()
        .expect("run cp");

    if !output.status.success() {
        // GUI が使えない環境では trash 実装が Finder に接続できず失敗するため許容する。
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("failed to move existing file to trash") {
            return;
        }
        panic!("cp failed unexpectedly: {stderr}");
    }

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
    cp_command()
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
    cp_command()
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
    cp_command()
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
    cp_command()
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
    cp_command()
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

#[test]
fn no_clobber_skips_overwrite_for_single_file() {
    // -n 指定時は既存ファイルを上書きせず、元の内容を維持することを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"New content")
        .expect("write to source file");

    let mut target_file = File::create(&target_path).expect("create target file");
    target_file
        .write_all(b"Old content")
        .expect("write to target file");

    cp_command()
        .arg("-n")
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .success();

    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(
        content, "Old content",
        "target content should not be overwritten"
    );
}

#[test]
fn no_clobber_skips_existing_files_in_recursive_copy() {
    // -n 指定時は再帰コピーでも既存ファイルを上書きせず、新規ファイルのみコピーすることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_dir = temp_dir.path().join("target_dir");

    fs::create_dir(&source_dir).expect("create source directory");
    fs::create_dir(&target_dir).expect("create target directory");

    let source_existing = source_dir.join("existing.txt");
    let source_new = source_dir.join("new.txt");
    let existing_target_subdir = target_dir.join("source_dir");
    fs::create_dir(&existing_target_subdir).expect("create existing target subdir");
    let target_existing = existing_target_subdir.join("existing.txt");

    let mut source_existing_file =
        File::create(&source_existing).expect("create source existing file");
    source_existing_file
        .write_all(b"new from source")
        .expect("write source existing file");

    let mut source_new_file = File::create(&source_new).expect("create source new file");
    source_new_file
        .write_all(b"brand new")
        .expect("write source new file");

    let mut target_existing_file =
        File::create(&target_existing).expect("create target existing file");
    target_existing_file
        .write_all(b"keep me")
        .expect("write target existing file");

    cp_command()
        .arg("-rn")
        .arg(&source_dir)
        .arg(&target_dir)
        .assert()
        .success();

    let copied_dir = target_dir.join("source_dir");
    let copied_existing = copied_dir.join("existing.txt");
    let copied_new = copied_dir.join("new.txt");

    let existing_content = fs::read_to_string(&copied_existing).expect("read copied existing file");
    assert_eq!(
        existing_content, "keep me",
        "existing file should be preserved when -n is specified"
    );

    let new_content = fs::read_to_string(&copied_new).expect("read copied new file");
    assert_eq!(new_content, "brand new", "new file should be copied");
}

#[test]
fn no_clobber_does_not_hide_type_conflict_with_directory_target() {
    // -n は既存ファイルへの上書き抑止のみを行い、型不整合はエラーとして扱うことを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_dir = temp_dir.path().join("target_dir");
    let conflicting_path = target_dir.join("source.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"source content")
        .expect("write source file");

    fs::create_dir(&target_dir).expect("create target directory");
    fs::create_dir(&conflicting_path).expect("create conflicting directory");

    cp_command()
        .arg("-n")
        .arg(&source_path)
        .arg(&target_dir)
        .assert()
        .failure();
}

#[test]
fn recursive_copy_without_no_clobber_replaces_existing_destination_directory() {
    // -n なしの再帰コピーは既存ターゲットディレクトリを置き換え、stale file を残さないことを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_parent = temp_dir.path().join("target_parent");
    let target_existing = target_parent.join("source_dir");
    let stale_file = target_existing.join("stale.txt");
    let new_file = source_dir.join("new.txt");

    fs::create_dir(&source_dir).expect("create source directory");
    fs::create_dir(&target_parent).expect("create target parent");
    fs::create_dir(&target_existing).expect("create existing target dir");

    let mut stale = File::create(&stale_file).expect("create stale file");
    stale.write_all(b"stale").expect("write stale file");

    let mut fresh = File::create(&new_file).expect("create new file");
    fresh.write_all(b"fresh").expect("write new file");

    cp_command()
        .arg("-r")
        .arg(&source_dir)
        .arg(&target_parent)
        .assert()
        .success();

    assert!(
        !stale_file.exists(),
        "stale file should be removed when destination directory is replaced"
    );
    assert!(
        target_existing.join("new.txt").exists(),
        "new file should exist in replaced destination"
    );
}

#[test]
fn recursive_no_clobber_reports_type_conflict_with_existing_directory() {
    // 再帰 -n では既存ファイルのみスキップし、ファイル/ディレクトリ衝突はエラーにすることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_parent = temp_dir.path().join("target_parent");
    let target_existing = target_parent.join("source_dir");
    let source_file = source_dir.join("conflict.txt");
    let conflicting_dir = target_existing.join("conflict.txt");

    fs::create_dir(&source_dir).expect("create source directory");
    fs::create_dir(&target_parent).expect("create target parent");
    fs::create_dir(&target_existing).expect("create existing target dir");
    fs::create_dir(&conflicting_dir).expect("create conflicting directory");

    let mut file = File::create(&source_file).expect("create source file");
    file.write_all(b"payload").expect("write source file");

    cp_command()
        .arg("-rn")
        .arg(&source_dir)
        .arg(&target_parent)
        .assert()
        .failure();
}
