use std::collections::HashSet;
use std::path::PathBuf;

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimeCategory {
    pub mime_types: Vec<String>,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub video: MimeCategory,
    pub audio: MimeCategory,
}

impl Default for Config {
    fn default() -> Self {
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

    pub fn is_allowed(&self, path: &std::path::Path) -> bool {
        if let Ok(Some(inferred)) = infer::get_from_path(path) {
            let mime_type = inferred.mime_type();
            let allowed = self.allowed_mime_types();
            if allowed.contains(mime_type) {
                return true;
            }
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let allowed = self.allowed_extensions();
            if allowed.contains(&ext.to_lowercase()) {
                return true;
            }
        }

        false
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("yt-playlist").join("yt-playlist.toml"))
}

fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("yt-playlist"))
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
            let _ = std::fs::create_dir_all(&dir);
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

    #[test]
    fn config_default_contains_video_mime_types() {
        let config = Config::default();
        assert!(config.video.mime_types.contains(&"video/mp4".to_string()));
        assert!(config
            .video
            .mime_types
            .contains(&"video/x-matroska".to_string()));
    }

    #[test]
    fn config_default_contains_audio_mime_types() {
        let config = Config::default();
        assert!(config.audio.mime_types.contains(&"audio/mpeg".to_string()));
        assert!(config.audio.mime_types.contains(&"audio/flac".to_string()));
    }

    #[test]
    fn config_default_contains_video_extensions() {
        let config = Config::default();
        assert!(config.video.extensions.contains(&"mp4".to_string()));
        assert!(config.video.extensions.contains(&"mkv".to_string()));
    }

    #[test]
    fn config_default_contains_audio_extensions() {
        let config = Config::default();
        assert!(config.audio.extensions.contains(&"mp3".to_string()));
        assert!(config.audio.extensions.contains(&"flac".to_string()));
    }

    #[test]
    fn allowed_mime_types_combines_video_and_audio() {
        let config = Config::default();
        let allowed = config.allowed_mime_types();
        assert!(allowed.contains("video/mp4"));
        assert!(allowed.contains("audio/mpeg"));
    }

    #[test]
    fn allowed_extensions_combines_video_and_audio() {
        let config = Config::default();
        let allowed = config.allowed_extensions();
        assert!(allowed.contains("mp4"));
        assert!(allowed.contains("mp3"));
    }

    #[test]
    fn config_path_returns_some_path() {
        let path = config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("yt-playlist.toml"));
    }

    #[test]
    fn config_dir_returns_some_path() {
        let dir = config_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.to_string_lossy().contains("yt-playlist"));
    }
}
