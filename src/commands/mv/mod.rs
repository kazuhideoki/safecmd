use crate::config::Config;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use trash::macos::{DeleteMethod, TrashContextExtMacos};

pub mod args;

/// 既存ターゲットの解決結果に応じた最終移動アクションを表す。
enum DestinationAction {
    RenameDirectly,
    ReplaceEmptyDirectory,
}

/// mv コマンド全体を実行し、各ソースの処理結果に応じて終了コードを決定する。
pub fn run(
    sources: Vec<String>,
    target: String,
    _force: bool,
    no_clobber: bool,
    config: Config,
) -> i32 {
    let target_path = Path::new(&target);
    let mut exit_code = 0;

    if sources.len() > 1 && !target_path.is_dir() {
        eprintln!("mv: target '{target}' is not a directory");
        return 1;
    }

    for source in &sources {
        if let Err(msg) = process_source(source, target_path, no_clobber, &config) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    exit_code
}

/// 単一ソースの移動を検証付きで実行する。
fn process_source(
    source: &str,
    target_path: &Path,
    no_clobber: bool,
    config: &Config,
) -> Result<(), String> {
    let source_path = Path::new(source);

    let source_meta = fs::symlink_metadata(source_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "mv: cannot stat '{}': No such file or directory",
                source_path.display()
            )
        } else {
            format!("mv: cannot access '{}': {}", source_path.display(), e)
        }
    })?;

    if !config.is_path_allowed(source_path) {
        return Err(format!(
            "mv: cannot move '{}': path is outside allowed scope",
            source_path.display()
        ));
    }

    let final_target = resolve_final_target(source_path, target_path)?;
    validate_target_scope(&final_target, config)?;
    if no_clobber && path_entry_exists(&final_target) {
        return Ok(());
    }
    ensure_not_same_file(source_path, &final_target)?;

    let staged_source = stage_source_for_destination(source_path, &final_target)?;
    if let Err(e) = finalize_move(&staged_source, source_path, &final_target, &source_meta) {
        rollback_staged_source(&staged_source, source_path);
        return Err(e);
    }

    Ok(())
}

/// ソースと最終ターゲットが同一実体かを判定し、同一ならエラーにする。
fn ensure_not_same_file(source_path: &Path, final_target: &Path) -> Result<(), String> {
    let target_meta = match fs::symlink_metadata(final_target) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(format!(
                "mv: cannot access '{}': {}",
                final_target.display(),
                e
            ));
        }
    };

    let source_meta = fs::symlink_metadata(source_path)
        .map_err(|e| format!("mv: cannot access '{}': {}", source_path.display(), e))?;

    #[cfg(unix)]
    if source_meta.dev() == target_meta.dev() && source_meta.ino() == target_meta.ino() {
        return Err(format!(
            "mv: '{}' and '{}' are the same file",
            source_path.display(),
            final_target.display()
        ));
    }

    #[cfg(not(unix))]
    if source_path == final_target {
        return Err(format!(
            "mv: '{}' and '{}' are the same file",
            source_path.display(),
            final_target.display()
        ));
    }

    Ok(())
}

/// 一時退避経由でターゲットへの最終移動を完了させる。
fn finalize_move(
    staged_source: &Path,
    source_path: &Path,
    final_target: &Path,
    source_meta: &fs::Metadata,
) -> Result<(), String> {
    match handle_existing_target(source_path, final_target, source_meta)? {
        DestinationAction::RenameDirectly => {}
        DestinationAction::ReplaceEmptyDirectory => {
            fs::remove_dir(final_target).map_err(|e| {
                format!(
                    "mv: cannot move '{}' to '{}': {}",
                    source_path.display(),
                    final_target.display(),
                    e
                )
            })?;
        }
    }

    fs::rename(staged_source, final_target).map_err(|e| {
        format!(
            "mv: cannot move '{}' to '{}': {}",
            source_path.display(),
            final_target.display(),
            e
        )
    })
}

/// 既存ターゲットの衝突解決として trash を実行する。
fn handle_existing_target(
    source_path: &Path,
    final_target: &Path,
    source_meta: &fs::Metadata,
) -> Result<DestinationAction, String> {
    let target_meta = match fs::symlink_metadata(final_target) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DestinationAction::RenameDirectly);
        }
        Err(e) => {
            return Err(format!(
                "mv: cannot access '{}': {}",
                final_target.display(),
                e
            ));
        }
    };

    if source_meta.is_dir() && !target_meta.file_type().is_dir() {
        return Err(format!(
            "mv: cannot overwrite non-directory '{}' with directory '{}'",
            final_target.display(),
            source_path.display()
        ));
    }

    if target_meta.file_type().is_dir() {
        if source_meta.is_dir() {
            let mut entries = fs::read_dir(final_target)
                .map_err(|e| format!("mv: cannot access '{}': {}", final_target.display(), e))?;
            if entries.next().is_some() {
                return Err(format!(
                    "mv: cannot move '{}' to '{}': Directory not empty",
                    source_path.display(),
                    final_target.display()
                ));
            }
            return Ok(DestinationAction::ReplaceEmptyDirectory);
        }
        return Err(format!(
            "mv: cannot move '{}' to '{}': destination is a directory",
            source_path.display(),
            final_target.display()
        ));
    }

    move_existing_target_to_trash(final_target)?;
    Ok(DestinationAction::RenameDirectly)
}

/// 既存ターゲットをシステムのゴミ箱へ移動する。
fn move_existing_target_to_trash(target: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Finder 経由の削除は権限ダイアログを誘発しうるため、テスト時も安定する実装を使う。
        let mut context = trash::TrashContext::new();
        context.set_delete_method(DeleteMethod::NsFileManager);
        context
            .delete(target)
            .map_err(|e| format!("mv: failed to move existing file to trash: {e}"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        trash::delete(target).map_err(|e| format!("mv: failed to move existing file to trash: {e}"))
    }
}

/// ソースを最終ターゲットの親ディレクトリへ一時退避する。
fn stage_source_for_destination(
    source_path: &Path,
    final_target: &Path,
) -> Result<PathBuf, String> {
    let staged_source = build_staging_path(final_target)?;
    fs::rename(source_path, &staged_source).map_err(|e| {
        format!(
            "mv: cannot move '{}' to '{}': {}",
            source_path.display(),
            final_target.display(),
            e
        )
    })?;
    Ok(staged_source)
}

/// 一時退避後の失敗時に元ソース位置への巻き戻しを試みる。
fn rollback_staged_source(staged_source: &Path, source_path: &Path) {
    let _ = fs::rename(staged_source, source_path);
}

/// ソースとターゲット指定から最終移動先を決定する。
fn resolve_final_target(source_path: &Path, target_path: &Path) -> Result<PathBuf, String> {
    if target_path.is_dir() {
        let file_name = source_path
            .file_name()
            .ok_or_else(|| format!("mv: invalid source path: '{}'", source_path.display()))?;
        Ok(target_path.join(file_name))
    } else {
        Ok(target_path.to_path_buf())
    }
}

/// 移動先パスが許可範囲内かを検証する。
fn validate_target_scope(final_target: &Path, config: &Config) -> Result<(), String> {
    if !config.is_path_allowed(final_target) {
        return Err(format!(
            "mv: cannot move to '{}': path is outside allowed scope",
            final_target.display()
        ));
    }
    Ok(())
}

/// ターゲット候補パスの存在をシンボリックリンクを含めて判定する。
fn path_entry_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// 最終ターゲットの親ディレクトリ配下に一時退避先を作る。
fn build_staging_path(final_target: &Path) -> Result<PathBuf, String> {
    let parent = final_target.parent().ok_or_else(|| {
        format!(
            "mv: cannot move to '{}': invalid destination path",
            final_target.display()
        )
    })?;

    let process_id = std::process::id();
    for attempt in 0..256 {
        let candidate = parent.join(format!(".safecmd-mv-stage-{process_id}-{attempt}"));
        if !path_entry_exists(&candidate) {
            return Ok(candidate);
        }
    }

    Err(format!(
        "mv: cannot move to '{}': failed to allocate staging path",
        final_target.display()
    ))
}
