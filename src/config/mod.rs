use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub additional_allowed_directories: AdditionalAllowedDirectories,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdditionalAllowedDirectories {
    pub paths: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            additional_allowed_directories: AdditionalAllowedDirectories { paths: vec![] },
        }
    }
}

impl Config {
    /// 設定ファイルを読み込み、実行時の制約設定を構築する。
    ///
    /// # Behavior
    /// 1. In test mode (when CARGO_MANIFEST_DIR is set), allows all paths unless disabled
    /// 2. Uses SAFECMD_CONFIG_PATH environment variable if set
    /// 3. Otherwise, looks for config at ~/.config/safecmd/config.toml
    /// 4. Creates a default config file if none exists
    pub fn load() -> Result<Self, String> {
        // Check if test mode should be disabled
        if std::env::var("SAFECMD_DISABLE_TEST_MODE").is_err() {
            // Check if running in test mode by looking for cargo test environment
            if std::env::var("CARGO_MANIFEST_DIR").is_ok() && std::env::var("CARGO").is_ok() {
                // If SAFECMD_CONFIG_PATH is set, still load from the config file even in test mode
                if std::env::var("SAFECMD_CONFIG_PATH").is_err() {
                    // Return a config that allows all paths for testing
                    return Ok(Self {
                        additional_allowed_directories: AdditionalAllowedDirectories {
                            paths: vec![PathBuf::from("/")],
                        },
                    });
                }
            }
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

        let default_content = r#"# SafeCmd configuration file
# Current working directory is always allowed.
# Add extra allowed directories below if needed.

[additional_allowed_directories]
paths = [
    # Add your additional allowed directories here
    # Example: "/home/user/shared",
    # Example: "/Users/yourname/Documents",
]

"#;

        let mut file = fs::File::create(config_path)
            .map_err(|e| format!("Failed to create config file: {e}"))?;

        file.write_all(default_content.as_bytes())
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
        };

        assert!(!config.is_path_allowed(&forbidden_file));

        std::env::set_current_dir(original).unwrap();
    }

<<<<<<< HEAD
    #[cfg(unix)]
    #[test]
    fn test_is_path_allowed_uses_symlink_path_instead_of_target() {
        // シンボリックリンクはリンク先ではなくリンク自身の配置場所で許可判定することを確認する。
        use std::os::unix::fs::symlink;

=======
    #[test]
    fn test_load_allows_empty_additional_paths() {
        // `paths = []` を許容し、追加許可なし設定として読み込めることを確認する。
>>>>>>> main
        let _guard = TEST_MUTEX.lock().unwrap();
        setup_test_env();

        let temp_dir = TempDir::new().unwrap();
<<<<<<< HEAD
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
=======
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
>>>>>>> main
    }
}
