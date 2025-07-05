use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;

pub mod args;

pub fn run(sources: Vec<String>, target: String, config: Config) -> i32 {
    let target_path = Path::new(&target);
    let mut exit_code = 0;

    // 複数ソースの場合、ターゲットはディレクトリでなければならない
    if sources.len() > 1 && !target_path.is_dir() {
        eprintln!("cp: target '{target}' is not a directory");
        return 1;
    }

    for source in sources {
        if let Err(msg) = copy_item(&source, &target, &config) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    exit_code
}

fn copy_item(source: &str, target: &str, config: &Config) -> Result<(), String> {
    let source_path = Path::new(source);
    let target_path = Path::new(target);

    // ソースの存在確認
    if !source_path.exists() {
        return Err(format!(
            "cp: cannot stat '{source}': No such file or directory"
        ));
    }

    // パスの検証
    let canonical_source = source_path
        .canonicalize()
        .map_err(|_| format!("cp: cannot access '{source}': Permission denied"))?;

    if !config.is_path_allowed(&canonical_source) {
        return Err(format!(
            "cp: cannot copy '{source}': path is not in allowed directories"
        ));
    }

    // ターゲットパスの決定
    let final_target = if target_path.is_dir() {
        // ディレクトリの場合、ソースのファイル名を追加
        let file_name = source_path
            .file_name()
            .ok_or_else(|| format!("cp: invalid source path: '{source}'"))?;
        target_path.join(file_name)
    } else {
        PathBuf::from(target)
    };

    // ターゲットの検証（存在する場合）
    if final_target.exists() {
        let canonical_target = final_target.canonicalize().map_err(|_| {
            format!(
                "cp: cannot access '{}': Permission denied",
                final_target.display()
            )
        })?;

        if !config.is_path_allowed(&canonical_target) {
            return Err(format!(
                "cp: cannot copy to '{}': path is not in allowed directories",
                final_target.display()
            ));
        }

        // 既存ファイルをゴミ箱へ移動
        trash::delete(&final_target)
            .map_err(|e| format!("cp: failed to move existing file to trash: {e}"))?;
    }

    // ファイルコピー実行
    if source_path.is_file() {
        fs::copy(source_path, &final_target).map_err(|e| {
            format!(
                "cp: cannot copy '{}' to '{}': {}",
                source,
                final_target.display(),
                e
            )
        })?;
    } else {
        return Err(format!("cp: omitting directory '{source}'"));
    }

    Ok(())
}
