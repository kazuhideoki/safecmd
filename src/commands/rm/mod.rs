pub mod args;
pub mod handlers;

use crate::config::Config;
use args::Args;
use handlers::{ProcessContext, RemovalKind};
use std::path::Path;

/// rm コマンド全体を実行し、各パスの処理結果に応じて終了コードを決定する。
pub fn run(args: Args, config: Config) -> i32 {
    let mut exit_code = 0;
    let context = ProcessContext::new(args, config);

    for path in &context.args.path {
        if let Err(msg) = process_path(path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    exit_code
}

/// 単一パスに対して許可範囲確認・ハンドラ選択・実行までを一貫して行う。
fn process_path(path: &Path, context: &ProcessContext) -> Result<(), String> {
    if !context.config.is_path_allowed(path) {
        return Err(format!(
            "rm: cannot remove '{}': path is outside allowed scope",
            path.display()
        ));
    }

    if !path.exists() {
        if context.args.force {
            return Ok(());
        } else {
            return Err(format!(
                "rm: cannot remove '{}': No such file or directory",
                path.display()
            ));
        }
    }

    let handler = determine_handler(path, context)?;
    handlers::validate(&handler, path, context)?;
    handlers::execute(&handler, path, context)
}

/// 対象パスの種類とオプションに応じて適切な削除ハンドラを選択する。
fn determine_handler(path: &Path, context: &ProcessContext) -> Result<RemovalKind, String> {
    use RemovalKind::*;

    match std::fs::metadata(path) {
        Ok(meta) => {
            if meta.is_dir() {
                if context.args.recursive {
                    Ok(RecursiveDirectory)
                } else if context.args.allow_dir {
                    Ok(EmptyDirectory)
                } else {
                    Ok(DirectoryError)
                }
            } else {
                Ok(File)
            }
        }
        Err(e) => {
            if context.args.force && e.kind() == std::io::ErrorKind::NotFound {
                Ok(NonExistentFile)
            } else {
                Err(format!("rm: cannot remove '{}': {e}", path.display()))
            }
        }
    }
}
