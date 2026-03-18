use std::collections::HashSet;
use std::path::PathBuf;

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct ConfigError;

/// Definition of a MIME type category with associated extensions.
///
/// Groups related MIME types and file extensions together, optionally
/// specifying a command to use when opening files of this category.
/// Used to categorize media files as video or audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimeCategory {
    pub mime_types: Vec<String>,
    pub extensions: Vec<String>,
    pub cmd: Option<String>,
}

/// Application configuration for MIME types and commands.
///
/// Defines how the application categorizes and handles different media types.
/// Contains separate categories for video and audio files, each with their
/// associated MIME types, file extensions, and optional launch commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub video: MimeCategory,
    pub audio: MimeCategory,
}

impl Default for Config {
    fn default() -> Self {
        let mpv_cmd = "shownotes action mpv {{path}} --socket {{socket_path}}".to_string();
        Self {
            video: MimeCategory {
                mime_types: vec![
                    "video/mp4".to_string(),
                    "video/x-matroska".to_string(),
                    "video/x-msvideo".to_string(),
                    "video/webm".to_string(),
                    "video/quicktime".to_string(),
                    "video/x-flv".to_string(),
                    "video/x-ms-wmv".to_string(),
                ],
                extensions: vec![
                    "mp4".to_string(),
                    "mkv".to_string(),
                    "avi".to_string(),
                    "webm".to_string(),
                    "mov".to_string(),
                    "flv".to_string(),
                    "wmv".to_string(),
                ],
                cmd: Some(mpv_cmd.clone()),
            },
            audio: MimeCategory {
                mime_types: vec![
                    "audio/mpeg".to_string(),
                    "audio/flac".to_string(),
                    "audio/x-wav".to_string(),
                    "audio/ogg".to_string(),
                    "audio/mp4".to_string(),
                    "audio/x-aac".to_string(),
                    "audio/webm".to_string(),
                ],
                extensions: vec![
                    "mp3".to_string(),
                    "flac".to_string(),
                    "wav".to_string(),
                    "ogg".to_string(),
                    "m4a".to_string(),
                    "aac".to_string(),
                    "webm".to_string(),
                ],
                cmd: Some(mpv_cmd),
            },
        }
    }
}

impl Config {
    pub fn allowed_mime_types(&self) -> HashSet<String> {
        let mut types = self.video.mime_types.clone();
        types.extend(self.audio.mime_types.clone());
        types.into_iter().collect()
    }

    pub fn allowed_extensions(&self) -> HashSet<String> {
        let mut exts = self.video.extensions.clone();
        exts.extend(self.audio.extensions.clone());
        exts.into_iter().collect()
    }

    pub fn get_cmd(&self, path: &std::path::Path) -> Option<&str> {
        if Self::matches_category(&self.video, path) {
            return self.video.cmd.as_deref();
        }
        if Self::matches_category(&self.audio, path) {
            return self.audio.cmd.as_deref();
        }
        None
    }

    pub fn is_video_or_audio(&self, path: &std::path::Path) -> bool {
        Self::matches_category(&self.video, path) || Self::matches_category(&self.audio, path)
    }

    fn matches_category(category: &MimeCategory, path: &std::path::Path) -> bool {
        if let Ok(Some(inferred)) = infer::get_from_path(path) {
            if category
                .mime_types
                .contains(&inferred.mime_type().to_string())
            {
                return true;
            }
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if category.extensions.contains(&ext.to_lowercase()) {
                return true;
            }
        }
        false
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("shownotes").join("shownotes.toml"))
}

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("shownotes"))
}

/// Loads configuration from the config file, or creates a default one if it doesn't exist.
///
/// # Errors
///
/// Returns an error if the config file cannot be read or parsed.
pub fn load() -> Result<Config, Report<ConfigError>> {
    let path = config_path().ok_or_else(|| Report::new(ConfigError))?;

    if !path.exists() {
        let default_config = Config::default();
        let content = toml::to_string_pretty(&default_config)
            .change_context(ConfigError)
            .attach("failed to serialize default config")?;

        if let Some(dir) = config_dir() {
            std::fs::create_dir_all(&dir)
                .change_context(ConfigError)
                .attach("failed to create config directory")?;
        }

        std::fs::write(&path, content)
            .change_context(ConfigError)
            .attach("failed to write default config file")?;

        return Ok(default_config);
    }

    let content = std::fs::read_to_string(&path)
        .change_context(ConfigError)
        .attach("failed to read config file")?;

    let config: Config = toml::from_str(&content)
        .change_context(ConfigError)
        .attach("failed to parse config file")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use serial_test::serial;

    fn with_temp_home<F, R>(f: F) -> R
    where
        F: FnOnce(&PathBuf) -> R,
    {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let original_home = std::env::var("HOME").ok();
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();

        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::remove_var("XDG_CONFIG_HOME");
        }

        let expected_config_path = temp_dir.path().join(".config").join("shownotes");
        let result = f(&expected_config_path);

        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
        }

        result
    }

    #[test]
    #[serial]
    fn load_creates_default_config_when_not_exists() {
        with_temp_home(|expected_config_path| {
            // Given no config file exists.
            assert!(!expected_config_path.join("shownotes.toml").exists());

            // When loading config.
            let result = load();

            // Then a default config is returned and file is created.
            assert!(result.is_ok());
            let config = result.unwrap();
            assert!(config.video.mime_types.contains(&"video/mp4".to_string()));
            assert!(config.audio.mime_types.contains(&"audio/mpeg".to_string()));
            assert!(
                expected_config_path.join("shownotes.toml").exists(),
                "config file should be created at {:?}",
                expected_config_path
            );
        });
    }

    #[test]
    #[serial]
    fn load_parses_existing_config_file() {
        with_temp_home(|expected_config_path| {
            // Given an existing config file with custom values.
            std::fs::create_dir_all(expected_config_path).expect("failed to create config dir");
            let config_file = expected_config_path.join("shownotes.toml");

            let custom_config = r#"
[video]
mime_types = ["video/custom"]
extensions = ["custom"]
cmd = "custom-video-cmd"

[audio]
mime_types = ["audio/custom"]
extensions = ["cust"]
cmd = "custom-audio-cmd"
"#;
            let mut file =
                std::fs::File::create(&config_file).expect("failed to create config file");
            file.write_all(custom_config.as_bytes())
                .expect("failed to write config");

            // When loading config.
            let result = load();

            // Then the custom config values are parsed.
            assert!(result.is_ok());
            let config = result.unwrap();
            assert_eq!(config.video.mime_types, vec!["video/custom"]);
            assert_eq!(config.video.extensions, vec!["custom"]);
            assert_eq!(config.video.cmd, Some("custom-video-cmd".to_string()));
            assert_eq!(config.audio.mime_types, vec!["audio/custom"]);
            assert_eq!(config.audio.extensions, vec!["cust"]);
            assert_eq!(config.audio.cmd, Some("custom-audio-cmd".to_string()));
        });
    }

    #[test]
    #[serial]
    #[ignore = "dirs::config_dir() uses system password database as fallback, making this unreliable on Linux"]
    fn load_returns_error_when_config_dir_unavailable() {
        let original_home = std::env::var("HOME").ok();
        let original_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let original_user = std::env::var("USER").ok();
        let original_logname = std::env::var("LOGNAME").ok();

        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("USER");
            std::env::remove_var("LOGNAME");
        }

        // When loading config with no home directory set.
        let result = load();

        // Then an error is returned (config_dir returns None).
        assert!(
            result.is_err(),
            "expected error when config_dir is unavailable"
        );

        unsafe {
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            }
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            }
            if let Some(user) = original_user {
                std::env::set_var("USER", user);
            }
            if let Some(logname) = original_logname {
                std::env::set_var("LOGNAME", logname);
            }
        }
    }
}
