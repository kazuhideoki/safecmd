use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../../config.example.toml");

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub additional_allowed_directories: AdditionalAllowedDirectories,
    #[serde(default)]
    pub notify: NotifyConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdditionalAllowedDirectories {
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NotifyConfig {
    pub macos_notify: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            additional_allowed_directories: AdditionalAllowedDirectories { paths: vec![] },
            notify: NotifyConfig::default(),
        }
    }
}

impl Config {
    /// 設定ファイルを読み込み、実行時の制約設定を構築する。
    ///
    /// # 判定ルール
    /// 1. `SAFECMD_TEST_MODE=1` かつ `SAFECMD_DISABLE_TEST_MODE` 未指定なら全許可モード
    /// 2. それ以外は `SAFECMD_CONFIG_PATH` または `~/.config/safecmd/config.toml` を使用
    /// 3. 設定ファイルが存在しない場合はデフォルト設定を作成
    pub fn load() -> Result<Self, String> {
        if Self::is_explicit_allow_all_test_mode_enabled() {
            return Ok(Self {
                additional_allowed_directories: AdditionalAllowedDirectories {
                    paths: vec![PathBuf::from("/")],
                },
                notify: NotifyConfig::default(),
            });
        }

        let config_path = Self::config_path()?;

        if !config_path.exists() {
            Self::create_default_config(&config_path)?;
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;

        let config: Config =
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {e}"))?;
        config.validate()?;

        Ok(config)
    }

    /// 明示指定されたテストモード（全許可）の有効化可否を判定する。
    ///
    /// `CARGO_*` の自動推測は行わず、`SAFECMD_TEST_MODE=1` のみを受け付ける。
    fn is_explicit_allow_all_test_mode_enabled() -> bool {
        if std::env::var("SAFECMD_DISABLE_TEST_MODE").is_ok() {
            return false;
        }

        matches!(std::env::var("SAFECMD_TEST_MODE").as_deref(), Ok("1"))
    }

    /// 読み込んだ設定値の整合性を検証する。
    ///
    /// `additional_allowed_directories.paths` には絶対パスのみを許可する。
    fn validate(&self) -> Result<(), String> {
        for (index, path) in self.additional_allowed_directories.paths.iter().enumerate() {
            if !path.is_absolute() {
                return Err(format!(
                    "Invalid config: additional_allowed_directories.paths[{index}] must be an absolute path: {}",
                    path.display()
                ));
            }
        }

        Ok(())
    }

    /// 指定パスが操作可能範囲に含まれるかを判定する。
    ///
    /// # 判定ルール
    /// - 実行時のカレントディレクトリ配下は常に許可
    /// - `additional_allowed_directories.paths` 配下は追加で許可
    pub fn is_path_allowed(&self, path: &Path) -> bool {
        let Some(resolved_target) = Self::resolve_target_path_without_symlink_resolution(path)
        else {
            return false;
        };

        for scope in self.allowed_scopes() {
            if resolved_target.starts_with(scope) {
                return true;
            }
        }

        false
    }

    /// 判定対象パスを絶対パスへ解決する。
    fn resolve_target_path_without_symlink_resolution(path: &Path) -> Option<PathBuf> {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            let cwd = std::env::current_dir().ok()?;
            cwd.join(path)
        };

        Some(Self::canonicalize_preserving_symlink_leaf(&absolute_path))
    }

    /// 最終要素がシンボリックリンクの場合はリンク先へ辿らず、親ディレクトリのみ実体解決する。
    fn canonicalize_preserving_symlink_leaf(path: &Path) -> PathBuf {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let Some(name) = path.file_name() else {
                    return Self::normalize_lexically(path);
                };
                let Some(parent) = path.parent() else {
                    return Self::normalize_lexically(path);
                };
                let resolved_parent = Self::canonicalize_with_missing(parent);
                Self::normalize_lexically(&resolved_parent.join(name))
            }
            _ => Self::canonicalize_with_missing(path),
        }
    }

    /// 非存在パスを含む場合でも、既存部分を基準に正規化する。
    fn canonicalize_with_missing(path: &Path) -> PathBuf {
        if path.exists() {
            return path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        }

        let mut existing = path;
        let mut missing_tail: Vec<std::ffi::OsString> = Vec::new();

        while !existing.exists() {
            let Some(name) = existing.file_name() else {
                break;
            };
            missing_tail.push(name.to_os_string());

            let Some(parent) = existing.parent() else {
                break;
            };
            existing = parent;
        }

        let mut resolved = if existing.exists() {
            existing
                .canonicalize()
                .unwrap_or_else(|_| existing.to_path_buf())
        } else {
            existing.to_path_buf()
        };

        for part in missing_tail.iter().rev() {
            resolved.push(part);
        }

        Self::normalize_lexically(&resolved)
    }

    /// 許可された操作スコープ一覧を構築する。
    fn allowed_scopes(&self) -> Vec<PathBuf> {
        let mut scopes = Vec::new();

        if let Ok(cwd) = std::env::current_dir() {
            scopes.push(cwd.canonicalize().unwrap_or(cwd));
        }

        for dir in &self.additional_allowed_directories.paths {
            let resolved = if dir.exists() {
                dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())
            } else {
                dir.to_path_buf()
            };
            scopes.push(resolved);
        }

        scopes
    }

    /// `.` と `..` を語彙的に解決し、比較可能なパスへ正規化する。
    fn normalize_lexically(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                Component::RootDir => normalized.push(Path::new("/")),
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::Normal(name) => normalized.push(name),
            }
        }

        normalized
    }

    /// Determines the path to the configuration file.
    ///
    /// # Priority
    /// 1. SAFECMD_CONFIG_PATH environment variable (for testing and custom setups)
    /// 2. ~/.config/safecmd/config.toml (default location)
    fn config_path() -> Result<PathBuf, String> {
        // Check for environment variable override
        if let Ok(path) = std::env::var("SAFECMD_CONFIG_PATH") {
            return Ok(PathBuf::from(path));
        }

        let home_dir =
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;

        let config_dir = home_dir.join(".config").join("safecmd");

        Ok(config_dir.join("config.toml"))
    }

    /// デフォルト設定ファイルを作成する。
    fn create_default_config(config_path: &Path) -> Result<(), String> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        fs::write(config_path, DEFAULT_CONFIG_TEMPLATE)
            .map_err(|e| format!("Failed to write default config: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // set_current_dir を使うテストの競合を防ぐため逐次実行する。
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// テスト時の自動全許可モードを無効化する。
    fn setup_test_env() {
        // Disable test mode for these unit tests
        unsafe {
            std::env::set_var("SAFECMD_DISABLE_TEST_MODE", "1");
        }
    }

    /// テストで使用する環境変数をクリアし、相互干渉を防ぐ。
    fn clear_test_related_env() {
        unsafe {
            std::env::remove_var("SAFECMD_DISABLE_TEST_MODE");
            std::env::remove_var("SAFECMD_TEST_MODE");
            std::env::remove_var("SAFECMD_CONFIG_PATH");
            std::env::remove_var("CARGO_MANIFEST_DIR");
            std::env::remove_var("CARGO");
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_is_path_allowed_with_current_directory_scope() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().join("workspace");
        fs::create_dir(&cwd).unwrap();
        fs::write(cwd.join("target.txt"), "content").unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&cwd).unwrap();

        let config = Config::default();
        assert!(config.is_path_allowed(Path::new("target.txt")));

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn test_is_path_allowed_with_additional_directory_scope() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().join("workspace");
        let external = temp_dir.path().join("external");

        fs::create_dir(&cwd).unwrap();
        fs::create_dir(&external).unwrap();

        let external_file = external.join("extra.txt");
        fs::write(&external_file, "content").unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&cwd).unwrap();

        let config = Config {
            additional_allowed_directories: AdditionalAllowedDirectories {
                paths: vec![external.clone()],
            },
            notify: NotifyConfig::default(),
        };

        assert!(config.is_path_allowed(&external_file));

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn test_is_path_allowed_denies_outside_scopes() {
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().join("workspace");
        let external = temp_dir.path().join("external");
        let forbidden = temp_dir.path().join("forbidden");

        fs::create_dir(&cwd).unwrap();
        fs::create_dir(&external).unwrap();
        fs::create_dir(&forbidden).unwrap();

        let forbidden_file = forbidden.join("secret.txt");
        fs::write(&forbidden_file, "secret").unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&cwd).unwrap();

        let config = Config {
            additional_allowed_directories: AdditionalAllowedDirectories {
                paths: vec![external],
            },
            notify: NotifyConfig::default(),
        };

        assert!(!config.is_path_allowed(&forbidden_file));

        std::env::set_current_dir(original).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_is_path_allowed_uses_symlink_path_instead_of_target() {
        // シンボリックリンクはリンク先ではなくリンク自身の配置場所で許可判定することを確認する。
        use std::os::unix::fs::symlink;
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().join("workspace");
        let outside = temp_dir.path().join("outside");

        fs::create_dir(&cwd).unwrap();
        fs::create_dir(&outside).unwrap();

        let outside_file = outside.join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        let link_in_cwd = cwd.join("secret-link.txt");
        symlink(&outside_file, &link_in_cwd).unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&cwd).unwrap();

        let config = Config::default();
        assert!(config.is_path_allowed(Path::new("secret-link.txt")));

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn test_load_allows_empty_additional_paths() {
        // `paths = []` を許容し、追加許可なし設定として読み込めることを確認する。
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"[additional_allowed_directories]
paths = []
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("SAFECMD_CONFIG_PATH", &config_path);
        }

        let loaded = Config::load().unwrap();
        assert!(loaded.additional_allowed_directories.paths.is_empty());
        assert!(!loaded.notify.macos_notify);
    }

    #[test]
    fn test_load_accepts_notify_macos_notify_setting() {
        // notify.macos_notify を設定ファイルから読み込めることを確認する。
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"[additional_allowed_directories]
paths = []

[notify]
macos_notify = true
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("SAFECMD_CONFIG_PATH", &config_path);
        }

        let loaded = Config::load().unwrap();
        assert!(loaded.notify.macos_notify);
    }

    #[test]
    fn test_load_rejects_relative_additional_path() {
        // 相対パス指定を設定エラーとして拒否することを確認する。
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"[additional_allowed_directories]
paths = ["relative/path"]
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("SAFECMD_CONFIG_PATH", &config_path);
        }

        let err = Config::load().unwrap_err();
        assert!(err.contains("must be an absolute path"));
    }

    #[test]
    fn test_load_does_not_enable_allow_all_from_cargo_env_only() {
        // CARGO 環境変数だけでは全許可モードにならないことを確認する。
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_test_related_env();

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().join("home");
        let config_dir = home_dir.join(".config").join("safecmd");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            r#"[additional_allowed_directories]
paths = []
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home_dir);
            std::env::set_var("CARGO_MANIFEST_DIR", temp_dir.path());
            std::env::set_var("CARGO", "cargo");
        }

        let loaded = Config::load().unwrap();
        assert!(
            loaded
                .additional_allowed_directories
                .paths
                .iter()
                .all(|path| path != Path::new("/")),
            "allow-all scope must not be enabled by CARGO_* environment only"
        );
    }

    #[test]
    fn test_load_enables_allow_all_only_with_explicit_test_mode() {
        // 明示的なテストモード指定時のみ全許可モードを有効化できることを確認する。
        let _guard = TEST_MUTEX.lock().unwrap();
        clear_test_related_env();

        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path().join("home");
        let config_dir = home_dir.join(".config").join("safecmd");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            r#"[additional_allowed_directories]
paths = []
"#,
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home_dir);
            std::env::set_var("SAFECMD_TEST_MODE", "1");
        }

        let loaded = Config::load().unwrap();
        assert_eq!(
            loaded.additional_allowed_directories.paths,
            vec![PathBuf::from("/")],
            "allow-all scope should be enabled only by explicit SAFECMD_TEST_MODE=1"
        );
    }

    #[test]
    fn test_create_default_config_uses_example_template() {
        // 既定設定の生成内容が `config.example.toml` と一致することを確認する。
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("generated").join("config.toml");

        Config::create_default_config(&config_path).unwrap();

        let generated = fs::read_to_string(&config_path).unwrap();
        let template_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.toml");
        let expected = fs::read_to_string(template_path).unwrap();
        assert_eq!(generated, expected);
    }
}
