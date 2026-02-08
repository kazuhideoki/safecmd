use std::path::Path;

use crate::config::Config;
use handlers::{CopyKind, ProcessContext};

pub mod args;
pub mod handlers;

/// cp コマンド全体を実行し、各ソースの処理結果に応じて終了コードを決定する。
pub fn run(
    sources: Vec<String>,
    target: String,
    recursive: bool,
    _force: bool,
    config: Config,
) -> i32 {
    let target_path = Path::new(&target);
    let mut exit_code = 0;
    let context = ProcessContext::new(recursive, config);

    if sources.len() > 1 && !target_path.is_dir() {
        eprintln!("cp: target '{target}' is not a directory");
        return 1;
    }

    for source in sources {
        if let Err(msg) = process_source(&source, target_path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
        }
    }

    exit_code
}

/// 単一ソースの処理としてハンドラ選択・検証・実行を行う。
fn process_source(
    source: &str,
    target_path: &Path,
    context: &ProcessContext,
) -> Result<(), String> {
    let source_path = Path::new(source);
    let kind = determine_handler(source_path, context)?;
    let task = handlers::validate(kind, source, source_path, target_path, context)?;
    handlers::execute(&task, context)
}

/// ソース種別とオプションに応じてコピー処理種別を決定する。
fn determine_handler(source_path: &Path, context: &ProcessContext) -> Result<CopyKind, String> {
    if !source_path.exists() {
        return Err(format!(
            "cp: cannot stat '{}': No such file or directory",
            source_path.display()
        ));
    }

    if source_path.is_file() {
        Ok(CopyKind::File)
    } else if source_path.is_dir() {
        if context.recursive {
            Ok(CopyKind::RecursiveDirectory)
        } else {
            Ok(CopyKind::DirectoryWithoutRecursive)
        }
    } else {
        Ok(CopyKind::UnsupportedType)
    }
}
