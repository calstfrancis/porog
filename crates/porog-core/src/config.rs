//! Porog's own settings, kept in `~/.config/porog/config.json`.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];

    /// The wording shown in the menu.
    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    /// Whether each row lists its last three resume entries instead of one.
    pub details: bool,
    /// The version whose Welcome / What's New window was last shown; empty on a first run.
    pub last_seen_version: String,
}

impl Config {
    /// The saved settings, or the defaults when the file is missing or unreadable.
    pub fn load(path: &Path) -> Config {
        fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    /// Write the settings by temp file and rename, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("porog-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("config.json")
    }

    #[test]
    fn a_missing_file_is_the_defaults() {
        assert_eq!(Config::load(&scratch("missing")), Config::default());
        assert_eq!(Config::default().theme, Theme::System);
    }

    #[test]
    fn a_saved_theme_is_read_back() {
        let path = scratch("roundtrip");
        let config = Config {
            theme: Theme::Dark,
            details: true,
            last_seen_version: "0.1.0".into(),
        };
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path), config);
        assert!(fs::read_to_string(&path).unwrap().contains("\"dark\""));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_unreadable_file_is_the_defaults() {
        let path = scratch("corrupt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Config::load(&path), Config::default());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
