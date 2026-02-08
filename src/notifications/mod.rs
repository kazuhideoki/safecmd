use std::env;
#[cfg(test)]
use std::{cell::Cell, thread_local};

/// 通知対象のコマンド種別を表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Rm,
    Cp,
}

impl CommandKind {
    /// 通知メッセージで使うコマンド名を返す。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rm => "rm",
            Self::Cp => "cp",
        }
    }
}

/// コマンド実行結果の集計情報を保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSummary {
    pub kind: CommandKind,
    pub success_count: usize,
    pub failure_count: usize,
}

impl CommandSummary {
    /// 成功有無を返す。
    pub fn is_success(&self) -> bool {
        self.failure_count == 0
    }
}

/// コマンド実行中の成功・失敗件数を集計する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResultCounter {
    kind: CommandKind,
    success_count: usize,
    failure_count: usize,
}

impl CommandResultCounter {
    /// コマンド種別に紐づく集計器を生成する。
    pub fn new(kind: CommandKind) -> Self {
        Self {
            kind,
            success_count: 0,
            failure_count: 0,
        }
    }

    /// 成功件数を1件加算する。
    pub fn record_success(&mut self) {
        self.success_count += 1;
    }

    /// 失敗件数を1件加算する。
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
    }

    /// 失敗件数を任意件数加算する。
    pub fn record_failures(&mut self, count: usize) {
        self.failure_count += count;
    }

    /// 現在の集計状態から通知用サマリを生成する。
    pub fn summary(&self) -> CommandSummary {
        CommandSummary {
            kind: self.kind,
            success_count: self.success_count,
            failure_count: self.failure_count,
        }
    }

    /// 現在の集計状態を通知する。
    pub fn notify(&self) {
        notify_command_result(&self.summary());
    }
}

/// 実行結果に応じた通知を発火する。
pub fn notify_command_result(summary: &CommandSummary) {
    #[cfg(test)]
    if let Some(override_fn) = test_override() {
        let _ = override_fn(summary);
        return;
    }

    if is_test_mode_enabled() {
        return;
    }

    let _ = dispatch(summary);
}

/// テストモード時は実通知を抑止する。
fn is_test_mode_enabled() -> bool {
    matches!(env::var("SAFECMD_TEST_MODE").as_deref(), Ok("1"))
}

#[cfg(target_os = "macos")]
fn dispatch(summary: &CommandSummary) -> Result<(), String> {
    use mac_notification_sys::{Notification, send_notification};

    let title = format!("safecmd {}", summary.kind.as_str());
    let subtitle = if summary.is_success() {
        "completed"
    } else {
        "failed"
    };
    let message = format!(
        "success: {}, failure: {}",
        summary.success_count, summary.failure_count
    );

    let mut options = Notification::new();
    options.asynchronous(true);

    send_notification(&title, Some(subtitle), &message, Some(&options))
        .map(|_| ())
        .map_err(|e| format!("notification delivery failed: {e}"))
}

#[cfg(test)]
type TestNotifier = fn(&CommandSummary) -> Result<(), String>;

#[cfg(test)]
thread_local! {
    static TEST_NOTIFIER_SLOT: Cell<Option<TestNotifier>> = const { Cell::new(None) };
}

#[cfg(test)]
fn test_override() -> Option<TestNotifier> {
    TEST_NOTIFIER_SLOT.with(|slot| slot.get())
}

#[cfg(test)]
pub(crate) fn with_test_notifier<T>(notifier: TestNotifier, f: impl FnOnce() -> T) -> T {
    // スコープ終了時に必ず元の notifier 設定へ戻す。
    struct RestoreGuard {
        previous: Option<TestNotifier>,
    }

    impl Drop for RestoreGuard {
        fn drop(&mut self) {
            TEST_NOTIFIER_SLOT.with(|slot| {
                slot.set(self.previous);
            });
        }
    }

    let previous = TEST_NOTIFIER_SLOT.with(|slot| {
        let previous = slot.get();
        slot.set(Some(notifier));
        previous
    });
    let _restore_guard = RestoreGuard { previous };
    let result = f();
    result
}

#[cfg(not(target_os = "macos"))]
fn dispatch(_summary: &CommandSummary) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::thread_local;

    fn capture_noop(_summary: &CommandSummary) -> Result<(), String> {
        Ok(())
    }

    thread_local! {
        static SUMMARY_STORE: RefCell<Vec<CommandSummary>> = const { RefCell::new(Vec::new()) };
    }

    fn capture_summary(summary: &CommandSummary) -> Result<(), String> {
        SUMMARY_STORE.with(|store| {
            store.borrow_mut().push(summary.clone());
        });
        Ok(())
    }

    #[test]
    fn command_result_counter_builds_summary_from_recorded_counts() {
        // 集計器へ記録した成功・失敗件数がサマリへ正しく反映されることを確認する。
        let mut counter = CommandResultCounter::new(CommandKind::Rm);
        counter.record_success();
        counter.record_success();
        counter.record_failure();
        counter.record_failures(3);

        assert_eq!(
            counter.summary(),
            CommandSummary {
                kind: CommandKind::Rm,
                success_count: 2,
                failure_count: 4,
            }
        );
    }

    #[test]
    fn command_result_counter_notifies_current_summary() {
        // 集計器の notify が現在の集計結果を通知処理へ渡すことを確認する。
        let mut counter = CommandResultCounter::new(CommandKind::Cp);
        counter.record_success();
        counter.record_failure();

        SUMMARY_STORE.with(|store| {
            store.borrow_mut().clear();
        });

        with_test_notifier(capture_summary, || {
            counter.notify();
        });

        let captured = SUMMARY_STORE.with(|store| store.borrow().clone());
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured[0],
            CommandSummary {
                kind: CommandKind::Cp,
                success_count: 1,
                failure_count: 1,
            }
        );
    }

    #[test]
    fn with_test_notifier_clears_override_when_callback_panics() {
        // コールバックが panic してもテスト通知上書き設定が残留しないことを確認する。
        let result = std::panic::catch_unwind(|| {
            with_test_notifier(capture_noop, || {
                panic!("panic in callback");
            });
        });

        assert!(result.is_err());
        assert!(test_override().is_none());
    }
}
