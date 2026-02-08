use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::process::Command;
use tempfile::tempdir;

/// mv バイナリ実行時に明示テストモードを付与したコマンドを生成する。
fn mv_command() -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("mv"));
    cmd.env("SAFECMD_TEST_MODE", "1");
    cmd
}

#[test]
fn single_file_move() {
    // 単一ファイル移動で source が消え、target に内容が移ることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"Hello from mv")
        .expect("write to source file");

    mv_command()
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .success();

    assert!(!source_path.exists(), "source file still exists after move");
    assert!(target_path.exists(), "target file was not created");

    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(content, "Hello from mv", "target content mismatch");
}

#[test]
fn overwrite_existing_target_by_trashing_target_first() {
    // 既存 target がある場合、target を trash 後に source 内容へ置き換わることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"new content")
        .expect("write source content");

    let mut target_file = File::create(&target_path).expect("create target file");
    target_file
        .write_all(b"old content")
        .expect("write target content");

    let output = mv_command()
        .arg(&source_path)
        .arg(&target_path)
        .output()
        .expect("run mv");

    if !output.status.success() {
        // GUI が使えない環境では trash 実装が Finder に接続できず失敗するため許容する。
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("failed to move existing file to trash") {
            return;
        }
        panic!("mv failed unexpectedly: {stderr}");
    }

    assert!(
        !source_path.exists(),
        "source file still exists after overwrite move"
    );
    let content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(content, "new content", "target content was not replaced");
}

#[test]
fn no_clobber_skips_existing_target() {
    // -n 指定時は既存 target を上書きせず、source も残ることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");
    let target_path = temp_dir.path().join("target.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"new content")
        .expect("write source content");

    let mut target_file = File::create(&target_path).expect("create target file");
    target_file
        .write_all(b"old content")
        .expect("write target content");

    mv_command()
        .arg("-n")
        .arg(&source_path)
        .arg(&target_path)
        .assert()
        .success();

    assert!(source_path.exists(), "source file should remain with -n");
    let source_content = fs::read_to_string(&source_path).expect("read source file");
    assert_eq!(
        source_content, "new content",
        "source content changed unexpectedly"
    );

    let target_content = fs::read_to_string(&target_path).expect("read target file");
    assert_eq!(
        target_content, "old content",
        "target should not be overwritten with -n"
    );
}

#[test]
fn same_path_move_does_not_delete_source() {
    // 同一実体への移動は失敗させつつ、source が消えないことを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"same path content")
        .expect("write source content");

    mv_command()
        .arg(&source_path)
        .arg(&source_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("are the same file"));

    assert!(source_path.exists(), "source file was deleted unexpectedly");
    let content = fs::read_to_string(&source_path).expect("read source file");
    assert_eq!(
        content, "same path content",
        "source content changed unexpectedly"
    );
}

#[test]
fn no_clobber_same_path_succeeds_without_changes() {
    // -n 指定時は同一パス移動をスキップし、source を維持することを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_path = temp_dir.path().join("source.txt");

    let mut source_file = File::create(&source_path).expect("create source file");
    source_file
        .write_all(b"same path content")
        .expect("write source content");

    mv_command()
        .arg("-n")
        .arg(&source_path)
        .arg(&source_path)
        .assert()
        .success();

    assert!(source_path.exists(), "source file was deleted unexpectedly");
    let content = fs::read_to_string(&source_path).expect("read source file");
    assert_eq!(
        content, "same path content",
        "source content changed unexpectedly"
    );
}

#[test]
fn directory_to_existing_file_fails_without_trashing_target() {
    // ディレクトリを既存ファイルへ移動する型衝突では、target を保持して失敗することを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("source_dir");
    let target_file = temp_dir.path().join("target.txt");

    fs::create_dir(&source_dir).expect("create source dir");
    let mut target = File::create(&target_file).expect("create target file");
    target
        .write_all(b"target content")
        .expect("write target content");

    mv_command()
        .arg(&source_dir)
        .arg(&target_file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot overwrite non-directory"));

    assert!(source_dir.exists(), "source dir should remain on failure");
    assert!(target_file.exists(), "target file should remain on failure");
    let content = fs::read_to_string(&target_file).expect("read target file");
    assert_eq!(
        content, "target content",
        "target content changed unexpectedly"
    );
}

#[cfg(unix)]
#[test]
fn moving_symlink_over_another_symlink_to_same_target_succeeds() {
    // 同一実体を指す別シンボリックリンク同士は移動可能であることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let real_path = temp_dir.path().join("real.txt");
    let link1_path = temp_dir.path().join("link1");
    let link2_path = temp_dir.path().join("link2");

    let mut real = File::create(&real_path).expect("create real file");
    real.write_all(b"real content").expect("write real content");
    symlink(&real_path, &link1_path).expect("create link1");
    symlink(&real_path, &link2_path).expect("create link2");

    let output = mv_command()
        .arg(&link1_path)
        .arg(&link2_path)
        .output()
        .expect("run mv");

    if !output.status.success() {
        // GUI が使えない環境では trash 実装が Finder に接続できず失敗するため許容する。
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("failed to move existing file to trash") {
            return;
        }
        panic!("mv failed unexpectedly: {stderr}");
    }

    assert!(!link1_path.exists(), "source symlink should be moved");
    assert!(link2_path.exists(), "destination symlink should exist");

    let resolved = fs::read_link(&link2_path).expect("read destination symlink");
    assert_eq!(resolved, real_path, "destination symlink target mismatch");
}

#[test]
fn directory_move_over_existing_empty_directory_succeeds() {
    // 既存の空ディレクトリへ同名ディレクトリを移動できることを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("src_dir");
    let destination_root = temp_dir.path().join("dst");
    let final_target = destination_root.join("src_dir");
    let nested_file = source_dir.join("note.txt");

    fs::create_dir_all(&source_dir).expect("create source dir");
    fs::create_dir_all(&destination_root).expect("create destination root");
    fs::create_dir_all(&final_target).expect("create empty final target");
    fs::write(&nested_file, "hello").expect("write nested file");

    mv_command()
        .arg(&source_dir)
        .arg(&destination_root)
        .assert()
        .success();

    assert!(!source_dir.exists(), "source dir should be moved");
    assert!(final_target.exists(), "final target should exist");
    let moved_content = fs::read_to_string(final_target.join("note.txt")).expect("read moved file");
    assert_eq!(moved_content, "hello", "moved file content mismatch");
}

#[test]
fn directory_move_over_existing_non_empty_directory_fails() {
    // 既存の非空ディレクトリへ同名ディレクトリを移動すると失敗することを確認する。
    let temp_dir = tempdir().expect("create tmp dir");
    let source_dir = temp_dir.path().join("src_dir");
    let destination_root = temp_dir.path().join("dst");
    let final_target = destination_root.join("src_dir");
    let source_file = source_dir.join("from_source.txt");
    let existing_file = final_target.join("existing.txt");

    fs::create_dir_all(&source_dir).expect("create source dir");
    fs::create_dir_all(&final_target).expect("create non-empty final target");
    fs::write(&source_file, "source").expect("write source file");
    fs::write(&existing_file, "existing").expect("write existing file");

    mv_command()
        .arg(&source_dir)
        .arg(&destination_root)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Directory not empty"));

    assert!(source_dir.exists(), "source dir should remain on failure");
    assert!(existing_file.exists(), "existing file should remain");
}
