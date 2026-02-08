use crate::config::Config;
use std::fs;
use std::io;
use std::path::Component;
use std::path::{Path, PathBuf};

/// 既存ターゲットを安全に退避する処理を抽象化した結果型。
type TrashResult = Result<(), String>;

/// cp 実行時に必要な設定とオプションを保持するコンテキスト。
pub struct ProcessContext {
    pub recursive: bool,
    pub no_clobber: bool,
    pub config: Config,
}

impl ProcessContext {
    /// cp 実行に必要な情報をまとめたコンテキストを生成する。
    pub fn new(recursive: bool, no_clobber: bool, config: Config) -> Self {
        Self {
            recursive,
            no_clobber,
            config,
        }
    }
}

/// コピー対象の種別を表す。
#[derive(Clone, Copy)]
pub enum CopyKind {
    File,
    RecursiveDirectory,
    DirectoryWithoutRecursive,
    UnsupportedType,
}

/// 実行フェーズで使うコピータスク情報を保持する。
pub struct CopyTask {
    pub kind: CopyKind,
    pub source: PathBuf,
    pub source_label: String,
    pub final_target: PathBuf,
}

/// コピー実行前にパス許可範囲と最終ターゲットを検証し、実行タスクを構築する。
pub fn validate(
    kind: CopyKind,
    source: &str,
    source_path: &Path,
    target_path: &Path,
    context: &ProcessContext,
) -> Result<CopyTask, String> {
    let canonical_source = source_path
        .canonicalize()
        .map_err(|_| format!("cp: cannot access '{source}': Permission denied"))?;

    if !context.config.is_path_allowed(&canonical_source) {
        return Err(format!(
            "cp: cannot copy '{source}': path is outside allowed scope"
        ));
    }

    let final_target = if target_path.is_dir() {
        let file_name = source_path
            .file_name()
            .ok_or_else(|| format!("cp: invalid source path: '{source}'"))?;
        target_path.join(file_name)
    } else {
        PathBuf::from(target_path)
    };

    if final_target.exists() {
        let canonical_target = final_target.canonicalize().map_err(|_| {
            format!(
                "cp: cannot access '{}': Permission denied",
                final_target.display()
            )
        })?;

        if !context.config.is_path_allowed(&canonical_target) {
            return Err(format!(
                "cp: cannot copy to '{}': path is outside allowed scope",
                final_target.display()
            ));
        }
    } else if !context.config.is_path_allowed(&final_target) {
        return Err(format!(
            "cp: cannot copy to '{}': path is outside allowed scope",
            final_target.display()
        ));
    }

    Ok(CopyTask {
        kind,
        source: source_path.to_path_buf(),
        source_label: source.to_string(),
        final_target,
    })
}

/// コピー種別に応じた実処理を行う。
pub fn execute(task: &CopyTask, context: &ProcessContext) -> Result<(), String> {
    match task.kind {
        CopyKind::File => {
            if task.final_target.exists() {
                if context.no_clobber && task.final_target.is_file() {
                    return Ok(());
                }
                if !context.no_clobber {
                    move_existing_file_to_trash(&task.final_target)?;
                }
            }

            fs::copy(&task.source, &task.final_target)
                .map(|_| ())
                .map_err(|e| {
                    format!(
                        "cp: cannot copy '{}' to '{}': {}",
                        task.source_label,
                        task.final_target.display(),
                        e
                    )
                })
        }
        CopyKind::RecursiveDirectory => {
            if task.final_target.exists() && !context.no_clobber {
                move_existing_file_to_trash(&task.final_target)?;
            }

            copy_dir_recursive(
                &task.source,
                &task.final_target,
                &context.config,
                context.no_clobber,
            )
        }
        CopyKind::DirectoryWithoutRecursive => {
            Err(format!("cp: omitting directory '{}'", task.source_label))
        }
        CopyKind::UnsupportedType => Err(format!(
            "cp: cannot copy '{}': Not a regular file",
            task.source_label
        )),
    }
}

/// ディレクトリを再帰的に走査し、配下を同構造でコピーする。
fn copy_dir_recursive(
    source: &Path,
    target: &Path,
    config: &Config,
    no_clobber: bool,
) -> Result<(), String> {
    ensure_target_path_allowed_for_write(target, config)?;

    fs::create_dir_all(target)
        .map_err(|e| format!("cp: cannot create directory '{}': {}", target.display(), e))?;

    let entries = fs::read_dir(source)
        .map_err(|e| format!("cp: cannot read directory '{}': {}", source.display(), e))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| format!("cp: error reading directory '{}': {}", source.display(), e))?;

        let entry_path = entry.path();
        let file_name = entry.file_name();
        let target_path = target.join(&file_name);

        let canonical_entry = entry_path.canonicalize().map_err(|_| {
            format!(
                "cp: cannot access '{}': Permission denied",
                entry_path.display()
            )
        })?;

        if !config.is_path_allowed(&canonical_entry) {
            return Err(format!(
                "cp: cannot copy '{}': path is outside allowed scope",
                entry_path.display()
            ));
        }

        if entry_path.is_file() {
            ensure_target_path_allowed_for_write(&target_path, config)?;

            if target_path.exists() {
                if no_clobber {
                    if target_path.is_file() {
                        continue;
                    }
                    return Err(format!(
                        "cp: cannot copy '{}' to '{}': destination is not a file",
                        entry_path.display(),
                        target_path.display()
                    ));
                }
                move_existing_file_to_trash(&target_path)?;
            }

            fs::copy(&entry_path, &target_path).map_err(|e| {
                format!(
                    "cp: cannot copy '{}' to '{}': {}",
                    entry_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        } else if entry_path.is_dir() {
            ensure_target_path_allowed_for_write(&target_path, config)?;
            copy_dir_recursive(&entry_path, &target_path, config, no_clobber)?;
        }
    }

    Ok(())
}

/// 既存ターゲットをゴミ箱へ移動し、失敗時はフォールバック移動を試みる。
fn move_existing_file_to_trash(target: &Path) -> TrashResult {
    move_existing_file_to_trash_with_fallback(
        target,
        |path| {
            trash::delete(path)
                .map_err(|e| format!("cp: failed to move existing file to trash: {e}"))
        },
        resolve_fallback_trash_dir,
    )
}

/// 既存ターゲットを退避し、主経路失敗時はフォールバック先へ一意名で移動する。
fn move_existing_file_to_trash_with_fallback<F, G>(
    target: &Path,
    primary_delete: F,
    fallback_dir_resolver: G,
) -> TrashResult
where
    F: Fn(&Path) -> TrashResult,
    G: Fn() -> Result<PathBuf, String>,
{
    match primary_delete(target) {
        Ok(()) => Ok(()),
        Err(primary_err) => {
            let fallback_dir = fallback_dir_resolver()?;
            fs::create_dir_all(&fallback_dir).map_err(|e| {
                format!("{primary_err}; cp: failed to prepare fallback trash directory: {e}")
            })?;

            let fallback_path = build_unique_fallback_path(&fallback_dir, target)?;
            move_to_fallback_path(target, &fallback_path).map_err(|e| {
                format!(
                    "{primary_err}; cp: failed to move existing file to fallback trash '{}': {}",
                    fallback_path.display(),
                    e
                )
            })
        }
    }
}

/// フォールバック先への移動を行う。別デバイス間ではコピー+削除へ退避する。
fn move_to_fallback_path(source: &Path, destination: &Path) -> io::Result<()> {
    move_to_fallback_path_with_rename(source, destination, rename_path)
}

/// フォールバック先への移動を行う。rename 失敗時の分岐を注入可能にする。
fn move_to_fallback_path_with_rename(
    source: &Path,
    destination: &Path,
    rename_fn: fn(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    match rename_fn(source, destination) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            copy_and_remove(source, destination)
        }
        Err(err) => Err(err),
    }
}

/// `fs::rename` を関数ポインタとして扱うための薄いラッパー。
fn rename_path(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

/// デバイス跨ぎ時にコピーして元を削除する。
fn copy_and_remove(source: &Path, destination: &Path) -> io::Result<()> {
    let source_meta = fs::symlink_metadata(source)?;
    let source_type = source_meta.file_type();

    if source_type.is_symlink() {
        copy_symlink_entry(source, destination)?;
        fs::remove_file(source)?;
        return Ok(());
    }

    if source_type.is_file() {
        fs::copy(source, destination)?;
        fs::remove_file(source)?;
        return Ok(());
    }

    if source_type.is_dir() {
        copy_dir_all(source, destination)?;
        fs::remove_dir_all(source)?;
        return Ok(());
    }

    Err(io::Error::other(
        "unsupported source type for cross-device move",
    ))
}

/// ディレクトリを再帰コピーする。
fn copy_dir_all(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = destination.join(entry.file_name());
        let entry_meta = fs::symlink_metadata(&entry_path)?;
        let entry_type = entry_meta.file_type();
        if entry_type.is_symlink() {
            copy_symlink_entry(&entry_path, &target_path)?;
        } else if entry_type.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else if entry_type.is_file() {
            fs::copy(&entry_path, &target_path)?;
        } else {
            return Err(io::Error::other("unsupported entry type in directory copy"));
        }
    }
    Ok(())
}

/// シンボリックリンクを辿らずリンクとして複製する。
#[cfg(unix)]
fn copy_symlink_entry(source: &Path, destination: &Path) -> io::Result<()> {
    let link_target = fs::read_link(source)?;
    std::os::unix::fs::symlink(link_target, destination)
}

/// シンボリックリンク複製の非Unix向けスタブ。
#[cfg(not(unix))]
fn copy_symlink_entry(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::other(
        "symlink copy is not supported on this platform",
    ))
}

/// フォールバック用のゴミ箱ディレクトリを解決する。
fn resolve_fallback_trash_dir() -> Result<PathBuf, String> {
    let Some(home_dir) = dirs::home_dir() else {
        return Err(
            "cp: failed to resolve fallback trash directory: home directory not found".to_string(),
        );
    };

    if cfg!(target_os = "macos") {
        return Ok(home_dir.join(".Trash"));
    }

    Ok(home_dir
        .join(".local")
        .join("share")
        .join("Trash")
        .join("files"))
}

/// フォールバック先で衝突しない退避パスを生成する。
fn build_unique_fallback_path(fallback_dir: &Path, target: &Path) -> Result<PathBuf, String> {
    let file_name = target.file_name().ok_or_else(|| {
        format!(
            "cp: failed to build fallback trash path for '{}': invalid file name",
            target.display()
        )
    })?;

    let safe_name = sanitize_file_name(file_name);
    let mut candidate = fallback_dir.join(&safe_name);
    if !path_slot_is_occupied(&candidate) {
        return Ok(candidate);
    }

    for index in 1..=9999 {
        let candidate_name = format!("{safe_name}.{index}");
        candidate = fallback_dir.join(candidate_name);
        if !path_slot_is_occupied(&candidate) {
            return Ok(candidate);
        }
    }

    Err(format!(
        "cp: failed to build fallback trash path for '{}': too many name collisions",
        target.display()
    ))
}

/// 壊れたシンボリックリンクも衝突として扱うため、symlink_metadata で占有判定する。
fn path_slot_is_occupied(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(err) => err.kind() != io::ErrorKind::NotFound,
    }
}

/// ファイル名として安全に扱える文字列へ変換する。
fn sanitize_file_name(file_name: &std::ffi::OsStr) -> String {
    let raw = file_name.to_string_lossy();
    let mut sanitized = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c == '/' || c == '\\' {
            sanitized.push('_');
        } else {
            sanitized.push(c);
        }
    }

    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        return "unnamed".to_string();
    }

    // 念のためパストラバーサルに繋がるセグメントを除去する。
    let normalized = Path::new(&sanitized)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("_");

    if normalized.is_empty() {
        return "unnamed".to_string();
    }

    normalized
}

/// コピー先への書き込み前に、許可範囲外パスとシンボリックリンク経由を拒否する。
fn ensure_target_path_allowed_for_write(path: &Path, config: &Config) -> Result<(), String> {
    if let Ok(meta) = fs::symlink_metadata(path)
        && meta.file_type().is_symlink()
    {
        return Err(format!(
            "cp: cannot copy to '{}': path is outside allowed scope",
            path.display()
        ));
    }

    if !config.is_path_allowed(path) {
        return Err(format!(
            "cp: cannot copy to '{}': path is outside allowed scope",
            path.display()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::io::Write;
    #[cfg(unix)]
    use std::{fs::symlink_metadata, os::unix::fs::symlink};
    use tempfile::tempdir;

    #[test]
    fn move_existing_file_to_trash_with_fallback_succeeds_when_primary_fails() {
        // 主経路のゴミ箱移動が失敗しても、フォールバックで退避できれば成功することを確認する。
        let temp_dir = tempdir().expect("create temp dir");
        let fallback_dir = temp_dir.path().join(".trash");
        let target = temp_dir.path().join("target.txt");
        let mut file = fs::File::create(&target).expect("create target");
        file.write_all(b"old").expect("write target");

        let result = move_existing_file_to_trash_with_fallback(
            &target,
            |_| Err("cp: failed to move existing file to trash: primary failed".to_string()),
            || Ok(fallback_dir.clone()),
        );

        assert!(
            result.is_ok(),
            "fallback should make trash move succeed even if primary fails"
        );
        assert!(
            !target.exists(),
            "target should be moved away after fallback succeeds"
        );
        assert!(
            fallback_dir.join("target.txt").exists(),
            "fallback trash should contain moved file"
        );
    }

    #[test]
    fn move_existing_file_to_trash_with_fallback_generates_unique_name_when_collision_exists() {
        // フォールバック先に同名があっても、一意な退避名を採番して成功することを確認する。
        let temp_dir = tempdir().expect("create temp dir");
        let fallback_dir = temp_dir.path().join(".trash");
        fs::create_dir_all(&fallback_dir).expect("create fallback dir");
        let target = temp_dir.path().join("report.txt");
        let mut file = fs::File::create(&target).expect("create target");
        file.write_all(b"old").expect("write target");

        let collision = fallback_dir.join("report.txt");
        let mut collision_file = fs::File::create(&collision).expect("create collision");
        collision_file
            .write_all(b"already exists")
            .expect("write collision");

        let result = move_existing_file_to_trash_with_fallback(
            &target,
            |_| Err("cp: failed to move existing file to trash: primary failed".to_string()),
            || Ok(fallback_dir.clone()),
        );

        assert!(
            result.is_ok(),
            "fallback should resolve name collisions and still succeed"
        );
        assert!(
            fallback_dir.join("report.txt.1").exists(),
            "fallback should pick a suffixed file name when the base name already exists"
        );
    }

    #[test]
    fn move_existing_file_to_trash_with_fallback_returns_error_when_all_paths_fail() {
        // 主経路とフォールバックの両方が失敗した場合のみエラーを返すことを確認する。
        let temp_dir = tempdir().expect("create temp dir");
        let target = temp_dir.path().join("target.txt");
        let mut file = fs::File::create(&target).expect("create target");
        file.write_all(b"old").expect("write target");

        let result = move_existing_file_to_trash_with_fallback(
            &target,
            |_| Err("cp: failed to move existing file to trash: primary failed".to_string()),
            || {
                Err(
                    "cp: failed to resolve fallback trash directory: test injected failure"
                        .to_string(),
                )
            },
        );

        assert!(result.is_err(), "should fail only when fallback also fails");
    }

    #[test]
    fn move_to_fallback_path_with_rename_copies_on_cross_device_for_file() {
        // rename が EXDEV になる場合でも、ファイルをコピー+削除で移動できることを確認する。
        let temp_dir = tempdir().expect("create temp dir");
        let source = temp_dir.path().join("source.txt");
        let destination = temp_dir.path().join("dest.txt");
        let mut file = fs::File::create(&source).expect("create source");
        file.write_all(b"payload").expect("write source");

        let result =
            move_to_fallback_path_with_rename(&source, &destination, mock_cross_device_rename);

        assert!(
            result.is_ok(),
            "cross-device fallback should succeed for files"
        );
        assert!(
            !source.exists(),
            "source should be removed after cross-device move fallback"
        );
        assert_eq!(
            fs::read_to_string(&destination).expect("read destination"),
            "payload"
        );
    }

    #[cfg(unix)]
    #[test]
    fn move_to_fallback_path_with_rename_preserves_symlink_entry_in_directory() {
        // EXDEV 時のディレクトリ退避で、symlink を辿らずリンクとして複製することを確認する。
        let temp_dir = tempdir().expect("create temp dir");
        let source_dir = temp_dir.path().join("source_dir");
        let destination_dir = temp_dir.path().join("destination_dir");
        let external_file = temp_dir.path().join("external.txt");
        fs::create_dir_all(&source_dir).expect("create source dir");
        fs::write(&external_file, "external").expect("create external file");
        symlink(&external_file, source_dir.join("link.txt")).expect("create symlink");

        let result = move_to_fallback_path_with_rename(
            &source_dir,
            &destination_dir,
            mock_cross_device_rename,
        );

        assert!(
            result.is_ok(),
            "cross-device fallback should succeed for directory"
        );
        assert!(
            !source_dir.exists(),
            "source directory should be removed after fallback move"
        );

        let moved_link = destination_dir.join("link.txt");
        let moved_meta = symlink_metadata(&moved_link).expect("read moved link metadata");
        assert!(
            moved_meta.file_type().is_symlink(),
            "moved entry should remain symlink instead of copied target content"
        );
        assert_eq!(
            fs::read_link(&moved_link).expect("read moved symlink target"),
            external_file
        );
    }

    #[cfg(unix)]
    #[test]
    fn build_unique_fallback_path_treats_dangling_symlink_as_occupied() {
        // 壊れた symlink があっても占有済みと判定し、別名を採番することを確認する。
        let temp_dir = tempdir().expect("create temp dir");
        let fallback_dir = temp_dir.path().join(".trash");
        fs::create_dir_all(&fallback_dir).expect("create fallback dir");
        let dangling = fallback_dir.join("report.txt");
        symlink("missing-target", &dangling).expect("create dangling symlink");
        let target = temp_dir.path().join("report.txt");
        fs::write(&target, "payload").expect("create target");

        let selected =
            build_unique_fallback_path(&fallback_dir, &target).expect("select fallback path");

        assert_eq!(selected, fallback_dir.join("report.txt.1"));
    }

    /// EXDEV を返す rename 失敗を模擬する。
    fn mock_cross_device_rename(_from: &Path, _to: &Path) -> io::Result<()> {
        Err(io::Error::new(ErrorKind::CrossesDevices, "exdev"))
    }
}
