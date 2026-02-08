use crate::{commands::rm::args::Args, config::Config};
use std::path::Path;

pub struct ProcessContext {
    pub args: Args,
    pub config: Config,
}

impl ProcessContext {
    /// rm 実行に必要な引数と設定をまとめたコンテキストを生成する。
    pub fn new(args: Args, config: Config) -> Self {
        Self { args, config }
    }
}

/// 削除対象の種類とオプションに応じた処理種別を表す。
pub enum RemovalKind {
    File,
    RecursiveDirectory,
    EmptyDirectory,
    DirectoryError,
    NonExistentFile,
}

/// 処理種別ごとの前提条件を検証し、実行可否を判定する。
pub fn validate(kind: &RemovalKind, path: &Path, _context: &ProcessContext) -> Result<(), String> {
    match kind {
        RemovalKind::File => Ok(()),
        RemovalKind::RecursiveDirectory => Ok(()),
        RemovalKind::EmptyDirectory => match std::fs::read_dir(path) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    Err(format!("rm: {}: Directory not empty", path.display()))
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(format!("rm: cannot access '{}': {}", path.display(), e)),
        },
        RemovalKind::DirectoryError => Err(format!("rm: {}: is a directory", path.display())),
        RemovalKind::NonExistentFile => Ok(()),
    }
}

/// 処理種別に応じて実際の削除処理を実行する。
pub fn execute(kind: &RemovalKind, path: &Path, _context: &ProcessContext) -> Result<(), String> {
    match kind {
        RemovalKind::File | RemovalKind::RecursiveDirectory | RemovalKind::EmptyDirectory => {
            trash::delete(path)
                .map_err(|e| format!("rm: failed to remove '{}': {}", path.display(), e))
        }
        RemovalKind::DirectoryError => {
            unreachable!("DirectoryError should fail at validation")
        }
        RemovalKind::NonExistentFile => Ok(()),
    }
}
