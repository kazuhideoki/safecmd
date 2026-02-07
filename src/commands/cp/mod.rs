use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;

pub mod args;

pub fn run(sources: Vec<String>, target: String, recursive: bool, config: Config) -> i32 {
    let target_path = Path::new(&target);
    let mut exit_code = 0;

    // 複数ソースの場合、ターゲットはディレクトリでなければならない
    if sources.len() > 1 && !target_path.is_dir() {
        eprintln!("cp: target '{target}' is not a directory");
        return 1;
    }

    for source in sources {
        if let Err(msg) = copy_item(&source, &target, recursive, &config) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    exit_code
}

fn copy_item(source: &str, target: &str, recursive: bool, config: &Config) -> Result<(), String> {
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
            "cp: cannot copy '{source}': path is outside allowed scope"
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

    // ターゲットの検証
    if final_target.exists() {
        let canonical_target = final_target.canonicalize().map_err(|_| {
            format!(
                "cp: cannot access '{}': Permission denied",
                final_target.display()
            )
        })?;

        if !config.is_path_allowed(&canonical_target) {
            return Err(format!(
                "cp: cannot copy to '{}': path is outside allowed scope",
                final_target.display()
            ));
        }

        // 既存ファイルをゴミ箱へ移動
        trash::delete(&final_target)
            .map_err(|e| format!("cp: failed to move existing file to trash: {e}"))?;
    } else if !config.is_path_allowed(&final_target) {
        return Err(format!(
            "cp: cannot copy to '{}': path is outside allowed scope",
            final_target.display()
        ));
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
    } else if source_path.is_dir() {
        if !recursive {
            return Err(format!("cp: omitting directory '{source}'"));
        }
        // 再帰的ディレクトリコピー
        copy_dir_recursive(source_path, &final_target, config)?;
    } else {
        return Err(format!("cp: cannot copy '{source}': Not a regular file"));
    }

    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path, config: &Config) -> Result<(), String> {
    // ターゲットディレクトリを作成
    fs::create_dir_all(target)
        .map_err(|e| format!("cp: cannot create directory '{}': {}", target.display(), e))?;

    // ディレクトリ内のエントリを走査
    let entries = fs::read_dir(source)
        .map_err(|e| format!("cp: cannot read directory '{}': {}", source.display(), e))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| format!("cp: error reading directory '{}': {}", source.display(), e))?;

        let entry_path = entry.path();
        let file_name = entry.file_name();
        let target_path = target.join(&file_name);

        // 各エントリのパスを検証
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

        // エントリの種類に応じて処理
        if entry_path.is_file() {
            // ファイルをコピー
            if target_path.exists() {
                // 既存ファイルをゴミ箱へ移動
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
            // サブディレクトリを再帰的にコピー
            copy_dir_recursive(&entry_path, &target_path, config)?;
        }
    }

    Ok(())
}
