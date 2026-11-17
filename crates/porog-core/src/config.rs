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

/// One note, and when it was left. Files written before notes were dated hold a plain string;
/// that reads as a note with no date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "NoteRepr")]
pub struct Note {
    pub text: String,
    /// RFC 3339 with an offset; `None` for a note from before they were dated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NoteRepr {
    Plain(String),
    Dated {
        text: String,
        #[serde(default)]
        at: Option<String>,
    },
}

impl From<NoteRepr> for Note {
    fn from(repr: NoteRepr) -> Note {
        match repr {
            NoteRepr::Plain(text) => Note { text, at: None },
            NoteRepr::Dated { text, at } => Note { text, at },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    /// Whether each row lists its last three resume entries instead of one.
    pub details: bool,
    /// Whether Porog keeps an icon in the system tray, and keeps running when its window is
    /// closed. Off by default.
    pub tray: bool,
    /// The version whose Welcome / What's New window was last shown; empty on a first run.
    pub last_seen_version: String,
    /// One line the user left for themselves per app, keyed by the app's status name
    /// ("next: rewrite the intro"). Only Porog reads or writes these.
    pub notes: BTreeMap<String, Note>,
}

/// The longest note kept, in characters. It is one line on a row, not a document.
pub const NOTE_MAX: usize = 160;

impl Config {
    /// The note left for `app`, or an empty string.
    pub fn note(&self, app: &App) -> &str {
        self.notes
            .get(app.status_name)
            .map_or("", |note| note.text.as_str())
    }

    /// When the note for `app` was left, if it has a date.
    pub fn note_at(&self, app: &App) -> Option<&str> {
        self.notes.get(app.status_name)?.at.as_deref()
    }

    /// Keep `text` as `app`'s note, dated now: see [`Config::set_note_at`].
    pub fn set_note(&mut self, app: &App, text: &str) {
        let now = chrono::Local::now().to_rfc3339();
        self.set_note_at(app, text, &now);
    }

    /// Keep `text` as `app`'s note: one line, trimmed and cut at [`NOTE_MAX`] characters, left
    /// at `at`. An empty note removes it. Saving the same words again keeps the first date: the
    /// date says when the note was written, not when it was last looked at.
    pub fn set_note_at(&mut self, app: &App, text: &str, at: &str) {
        let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let line: String = line.chars().take(NOTE_MAX).collect();
        if line.is_empty() {
            self.notes.remove(app.status_name);
            return;
        }
        if let Some(existing) = self.notes.get(app.status_name) {
            if existing.text == line && existing.at.is_some() {
                return;
            }
        }
        self.notes.insert(
            app.status_name.to_string(),
            Note {
                text: line,
                at: Some(at.to_string()),
            },
        );
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
    fn a_note_carries_the_date_it_was_left_and_keeps_it_when_unchanged() {
        let app = &crate::APPS[3];
        let mut config = Config::default();
        config.set_note_at(app, "next: the intro", "2026-11-03T10:00:00-04:00");
        assert_eq!(config.note_at(app), Some("2026-11-03T10:00:00-04:00"));
        config.set_note_at(app, " next:  the intro ", "2026-11-09T10:00:00-04:00");
        assert_eq!(config.note_at(app), Some("2026-11-03T10:00:00-04:00"));
        config.set_note_at(app, "next: the outline", "2026-11-09T10:00:00-04:00");
        assert_eq!(config.note_at(app), Some("2026-11-09T10:00:00-04:00"));
    }

    #[test]
    fn notes_from_before_they_were_dated_still_load() {
        let path = scratch("plain-notes");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"notes":{"zerkalo":"next: the intro"}}"#).unwrap();
        let config = Config::load(&path);
        assert_eq!(config.note(&crate::APPS[3]), "next: the intro");
        assert_eq!(config.note_at(&crate::APPS[3]), None);
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path), config);
        let _ = fs::remove_dir_all(path.parent().unwrap());
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
