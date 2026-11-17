//! Porog's own settings, kept in `~/.config/porog/config.json`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::App;

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
    /// One line the user left for themselves per app, keyed by the app's status name
    /// ("next: rewrite the intro"). Only Porog reads or writes these.
    pub notes: BTreeMap<String, String>,
}

/// The longest note kept, in characters. It is one line on a row, not a document.
pub const NOTE_MAX: usize = 160;

impl Config {
    /// The note left for `app`, or an empty string.
    pub fn note(&self, app: &App) -> &str {
        self.notes.get(app.status_name).map_or("", String::as_str)
    }

    /// Keep `text` as `app`'s note: one line, trimmed and cut at [`NOTE_MAX`] characters.
    /// An empty note removes it.
    pub fn set_note(&mut self, app: &App, text: &str) {
        let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let line: String = line.chars().take(NOTE_MAX).collect();
        if line.is_empty() {
            self.notes.remove(app.status_name);
        } else {
            self.notes.insert(app.status_name.to_string(), line);
        }
    }

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
            ..Config::default()
        };
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path), config);
        assert!(fs::read_to_string(&path).unwrap().contains("\"dark\""));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_note_is_one_trimmed_line_and_empty_removes_it() {
        let app = &crate::APPS[3];
        let mut config = Config::default();
        assert_eq!(config.note(app), "");
        config.set_note(app, "  next:\n rewrite   the intro \n");
        assert_eq!(config.note(app), "next: rewrite the intro");
        assert_eq!(config.note(&crate::APPS[0]), "");
        config.set_note(app, "   ");
        assert_eq!(config.note(app), "");
        assert!(config.notes.is_empty());
    }

    #[test]
    fn a_long_note_is_cut_and_notes_survive_a_save() {
        let app = &crate::APPS[1];
        let mut config = Config::default();
        config.set_note(app, &"é".repeat(NOTE_MAX + 50));
        assert_eq!(config.note(app).chars().count(), NOTE_MAX);
        let path = scratch("notes");
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path), config);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn an_old_file_without_notes_still_loads() {
        let path = scratch("old");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"theme":"dark","details":true,"last_seen_version":"0.3.0"}"#,
        )
        .unwrap();
        let config = Config::load(&path);
        assert_eq!(config.theme, Theme::Dark);
        assert!(config.notes.is_empty());
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
