use std::env;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

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
fn notifier_slot() -> &'static Mutex<Option<TestNotifier>> {
    static SLOT: OnceLock<Mutex<Option<TestNotifier>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn test_override() -> Option<TestNotifier> {
    notifier_slot().lock().ok().and_then(|guard| *guard)
}

#[cfg(test)]
pub(crate) fn with_test_notifier<T>(notifier: TestNotifier, f: impl FnOnce() -> T) -> T {
    *notifier_slot().lock().expect("lock notifier slot") = Some(notifier);
    let result = f();
    *notifier_slot().lock().expect("lock notifier slot") = None;
    result
}

#[cfg(not(target_os = "macos"))]
fn dispatch(_summary: &CommandSummary) -> Result<(), String> {
    Ok(())
}
