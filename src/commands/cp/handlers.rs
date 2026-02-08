use crate::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

/// cp 実行時に必要な設定とオプションを保持するコンテキスト。
pub struct ProcessContext {
    pub recursive: bool,
    pub config: Config,
}

impl ProcessContext {
    /// cp 実行に必要な情報をまとめたコンテキストを生成する。
    pub fn new(recursive: bool, config: Config) -> Self {
        Self { recursive, config }
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
    if task.final_target.exists() {
        trash::delete(&task.final_target)
            .map_err(|e| format!("cp: failed to move existing file to trash: {e}"))?;
    }

    match task.kind {
        CopyKind::File => fs::copy(&task.source, &task.final_target)
            .map(|_| ())
            .map_err(|e| {
                format!(
                    "cp: cannot copy '{}' to '{}': {}",
                    task.source_label,
                    task.final_target.display(),
                    e
                )
            }),
        CopyKind::RecursiveDirectory => {
            copy_dir_recursive(&task.source, &task.final_target, &context.config)
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
fn copy_dir_recursive(source: &Path, target: &Path, config: &Config) -> Result<(), String> {
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
            if target_path.exists() {
                trash::delete(&target_path)
                    .map_err(|e| format!("cp: failed to move existing file to trash: {e}"))?;
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
            copy_dir_recursive(&entry_path, &target_path, config)?;
        }
    }

    Ok(())
}
