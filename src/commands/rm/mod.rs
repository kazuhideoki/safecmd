pub mod args;
pub mod handlers;

use crate::config::Config;
use crate::notifications::{self, CommandKind, CommandSummary};
use args::Args;
use handlers::{ProcessContext, RemovalKind};
use std::path::Path;

/// rm コマンド全体を実行し、各パスの処理結果に応じて終了コードを決定する。
pub fn run(args: Args, config: Config) -> i32 {
    let mut exit_code = 0;
    let mut success_count = 0usize;
    let mut failure_count = 0usize;
    let context = ProcessContext::new(args, config);

    for path in &context.args.path {
        if let Err(msg) = process_path(path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
            failure_count += 1;
        } else {
            success_count += 1;
        }
    }

    notifications::notify_command_result(&CommandSummary {
        kind: CommandKind::Rm,
        success_count,
        failure_count,
    });

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

    if std::fs::symlink_metadata(path).is_err() {
        if context.args.force {
            return Ok(());
        }
        return Err(format!(
            "rm: cannot remove '{}': No such file or directory",
            path.display()
        ));
    }

    let handler = determine_handler(path, context)?;
    handlers::validate(&handler, path, context)?;
    handlers::execute(&handler, path, context)
}

/// 対象パスの種類とオプションに応じて適切な削除ハンドラを選択する。
fn determine_handler(path: &Path, context: &ProcessContext) -> Result<RemovalKind, String> {
    use RemovalKind::*;

    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                Ok(File)
            } else if meta.is_dir() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AdditionalAllowedDirectories, Config};
    use crate::notifications::{self, CommandKind, CommandSummary};
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    /// テスト用の最小コンテキストを生成する。
    fn build_context(recursive: bool) -> ProcessContext {
        ProcessContext::new(
            Args {
                allow_dir: false,
                force: false,
                recursive,
                path: vec![],
            },
            Config {
                additional_allowed_directories: AdditionalAllowedDirectories {
                    paths: vec![std::path::PathBuf::from("/")],
                },
            },
        )
    }

    #[cfg(unix)]
    #[test]
    fn determine_handler_treats_directory_symlink_as_file_without_r() {
        // ディレクトリへのシンボリックリンクは通常時に File と判定されることを確認する。
        let temp_dir = TempDir::new().unwrap();
        let linked_dir = temp_dir.path().join("linked");
        std::fs::create_dir(&linked_dir).unwrap();
        let symlink_path = temp_dir.path().join("dir_link");
        symlink(&linked_dir, &symlink_path).unwrap();

        let context = build_context(false);
        let kind = determine_handler(&symlink_path, &context).unwrap();

        assert!(matches!(kind, RemovalKind::File));
    }

    #[cfg(unix)]
    #[test]
    fn determine_handler_treats_directory_symlink_as_file_with_r() {
        // -r 指定時でもシンボリックリンクは File と判定されることを確認する。
        let temp_dir = TempDir::new().unwrap();
        let linked_dir = temp_dir.path().join("linked");
        std::fs::create_dir(&linked_dir).unwrap();
        let symlink_path = temp_dir.path().join("dir_link");
        symlink(&linked_dir, &symlink_path).unwrap();

        let context = build_context(true);
        let kind = determine_handler(&symlink_path, &context).unwrap();

        assert!(matches!(kind, RemovalKind::File));
    }

    fn notification_store() -> &'static Mutex<Vec<CommandSummary>> {
        static STORE: OnceLock<Mutex<Vec<CommandSummary>>> = OnceLock::new();
        STORE.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn capture_notification(summary: &CommandSummary) -> Result<(), String> {
        notification_store()
            .lock()
            .expect("lock notification store")
            .push(summary.clone());
        Ok(())
    }

    #[test]
    fn run_notifies_summary_when_success() {
        // rm 実行成功時に通知へ集計結果を渡すことを確認する。
        let path = Path::new("missing-file-for-notify-success");
        let context_args = Args {
            allow_dir: false,
            force: true,
            recursive: false,
            path: vec![path.to_path_buf()],
        };
        let config = Config {
            additional_allowed_directories: AdditionalAllowedDirectories {
                paths: vec![std::path::PathBuf::from("/")],
            },
        };

        notification_store()
            .lock()
            .expect("lock notification store")
            .clear();
        notifications::with_test_notifier(capture_notification, || {
            let exit_code = run(context_args, config);
            assert_eq!(exit_code, 0);
        });

        let captured = notification_store()
            .lock()
            .expect("lock notification store");
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured[0],
            CommandSummary {
                kind: CommandKind::Rm,
                success_count: 1,
                failure_count: 0,
            }
        );
    }
}
