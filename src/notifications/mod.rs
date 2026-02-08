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

    fn capture_noop(_summary: &CommandSummary) -> Result<(), String> {
        Ok(())
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
