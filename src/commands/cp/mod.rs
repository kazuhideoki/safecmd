use std::path::Path;

use crate::config::Config;
use crate::notifications::{self, CommandKind, CommandSummary};
use handlers::{CopyKind, ProcessContext};

pub mod args;
pub mod handlers;

/// cp コマンド全体を実行し、各ソースの処理結果に応じて終了コードを決定する。
pub fn run(
    sources: Vec<String>,
    target: String,
    recursive: bool,
    _force: bool,
    no_clobber: bool,
    config: Config,
) -> i32 {
    let target_path = Path::new(&target);
    let mut exit_code = 0;
    let mut success_count = 0usize;
    let mut failure_count = 0usize;
    let context = ProcessContext::new(recursive, no_clobber, config);

    if sources.len() > 1 && !target_path.is_dir() {
        eprintln!("cp: target '{target}' is not a directory");
        notifications::notify_command_result(&CommandSummary {
            kind: CommandKind::Cp,
            success_count: 0,
            failure_count: sources.len(),
        });
        return 1;
    }

    for source in sources {
        if let Err(msg) = process_source(&source, target_path, &context) {
            eprintln!("{msg}");
            exit_code = 1;
            failure_count += 1;
        } else {
            success_count += 1;
        }
    }

    notifications::notify_command_result(&CommandSummary {
        kind: CommandKind::Cp,
        success_count,
        failure_count,
    });

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AdditionalAllowedDirectories, Config};
    use crate::notifications::{self, CommandKind, CommandSummary};
    use std::cell::RefCell;
    use std::fs;
    use std::thread_local;
    use tempfile::TempDir;

    thread_local! {
        static NOTIFICATION_STORE: RefCell<Vec<CommandSummary>> = const { RefCell::new(Vec::new()) };
    }

    fn capture_notification(summary: &CommandSummary) -> Result<(), String> {
        NOTIFICATION_STORE.with(|store| {
            store.borrow_mut().push(summary.clone());
        });
        Ok(())
    }

    fn allow_all_config() -> Config {
        Config {
            additional_allowed_directories: AdditionalAllowedDirectories {
                paths: vec![std::path::PathBuf::from("/")],
            },
        }
    }

    #[test]
    fn run_notifies_summary_when_success() {
        // cp 実行成功時に通知へ集計結果を渡すことを確認する。
        let temp_dir = TempDir::new().expect("create temp dir");
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");
        fs::write(&source, "hello").expect("write source");

        NOTIFICATION_STORE.with(|store| {
            store.borrow_mut().clear();
        });
        notifications::with_test_notifier(capture_notification, || {
            let exit_code = run(
                vec![source.to_string_lossy().to_string()],
                target.to_string_lossy().to_string(),
                false,
                false,
                false,
                allow_all_config(),
            );
            assert_eq!(exit_code, 0);
        });

        let captured = NOTIFICATION_STORE.with(|store| store.borrow().clone());
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured[0],
            CommandSummary {
                kind: CommandKind::Cp,
                success_count: 1,
                failure_count: 0,
            }
        );
    }

    #[test]
    fn run_notifies_all_sources_as_failure_when_multi_source_target_is_not_directory() {
        // 複数ソース指定でターゲットがディレクトリでない場合に全ソースを失敗件数へ計上することを確認する。
        let temp_dir = TempDir::new().expect("create temp dir");
        let source1 = temp_dir.path().join("source1.txt");
        let source2 = temp_dir.path().join("source2.txt");
        let target = temp_dir.path().join("target.txt");
        fs::write(&source1, "one").expect("write source1");
        fs::write(&source2, "two").expect("write source2");
        fs::write(&target, "target").expect("write target");

        NOTIFICATION_STORE.with(|store| {
            store.borrow_mut().clear();
        });
        notifications::with_test_notifier(capture_notification, || {
            let exit_code = run(
                vec![
                    source1.to_string_lossy().to_string(),
                    source2.to_string_lossy().to_string(),
                ],
                target.to_string_lossy().to_string(),
                false,
                false,
                false,
                allow_all_config(),
            );
            assert_eq!(exit_code, 1);
        });

        let captured = NOTIFICATION_STORE.with(|store| store.borrow().clone());
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured[0],
            CommandSummary {
                kind: CommandKind::Cp,
                success_count: 0,
                failure_count: 2,
            }
        );
    }
}
