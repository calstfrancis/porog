//! What each row says: the app's status file, checked against what can still be honoured.
//!
//! Porog never opens an app's own files. It reads the status file the app wrote (see the
//! `fond-status` crate) and keeps only the resume entries it can actually hand back.

use std::path::Path;
use std::process::{Command, Stdio};

use chrono::{DateTime, FixedOffset};
use fond_status::Status;
pub use fond_status::{status_dir, ResumeEntry};

use crate::App;

/// Everything a row shows, gathered in one blocking pass off the main thread.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowState {
    pub installed: bool,
    /// The app's own one-line summary, if it wrote one.
    pub summary: Option<String>,
    /// Entries that can be resumed right now, newest first.
    pub resume: Vec<ResumeEntry>,
}

/// Read `app`'s row state. Blocks: runs `flatpak info`, reads a file, and checks each
/// resume target still exists.
pub fn row_state(app: &App, status_dir: Option<&Path>, sandboxed: bool) -> RowState {
    let installed = crate::is_installed(app);
    // A missing, corrupt or too-new status file is the same to the user as none at all: the
    // row falls back to plain Open.
    let status = status_dir
        .and_then(|dir| Status::read(dir, app.status_name).ok().flatten())
        .filter(|status| status.app == app.status_name);
    let Some(status) = status else {
        return RowState {
            installed,
            ..RowState::default()
        };
    };
    RowState {
        installed,
        summary: current_summary(
            status.summary,
            status.summary_until.as_deref(),
            chrono::Utc::now().fixed_offset(),
        ),
        resume: usable(status.resume, |path| path_exists(path, sandboxed)),
    }
}

/// The summary, unless it is blank or its app said it stops being true at a time that has
/// passed. An expiry that cannot be read counts as passed: showing nothing is better than
/// showing a line that may be wrong.
fn current_summary(
    summary: Option<String>,
    until: Option<&str>,
    now: DateTime<FixedOffset>,
) -> Option<String> {
    let summary = summary.filter(|s| !s.trim().is_empty())?;
    match until {
        None => Some(summary),
        Some(until) => DateTime::parse_from_rfc3339(until)
            .ok()
            .filter(|until| now < *until)
            .map(|_| summary),
    }
}

/// Keep the entries Resume can honour: something to pass back, a title to show, and, when
/// the first argument is a file path, a file that is still there.
fn usable(entries: Vec<ResumeEntry>, exists: impl Fn(&str) -> bool) -> Vec<ResumeEntry> {
    entries
        .into_iter()
        .filter(|entry| !entry.title.trim().is_empty())
        .filter(|entry| match entry.args.first() {
            None => false,
            Some(first) if first.starts_with('/') => exists(first),
            Some(_) => true,
        })
        .collect()
}

/// Whether `path` exists on the host. Inside the sandbox Porog cannot see the user's files,
/// so it asks the host instead of being granted the whole home directory.
fn path_exists(path: &str, sandboxed: bool) -> bool {
    if !sandboxed {
        return Path::new(path).exists();
    }
    Command::new("flatpak-spawn")
        .args(["--host", "test", "-e", path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// A moment with its UTC offset: what "today" and "yesterday" are measured from.
pub type Now = DateTime<FixedOffset>;

/// The current local time, for [`describe`].
pub fn now() -> Now {
    chrono::Local::now().fixed_offset()
}

/// The row text for one resume entry: its title, then its detail and when it was last
/// worked on, where the app gave them.
pub fn describe(entry: &ResumeEntry, now: DateTime<FixedOffset>) -> String {
    let mut parts = vec![entry.title.trim().to_string()];
    if let Some(detail) = entry.detail.as_deref().map(str::trim) {
        if !detail.is_empty() {
            parts.push(detail.to_string());
        }
    }
    if let Some(when) = entry.at.as_deref().and_then(|at| describe_when(at, now)) {
        parts.push(when);
    }
    parts.join(" · ")
}

/// An RFC 3339 time as plain words in `now`'s timezone: "today 16:40", "yesterday 16:40",
/// "3 Nov", or "3 Nov 2025" for another year. `None` when it does not parse.
pub fn describe_when(at: &str, now: DateTime<FixedOffset>) -> Option<String> {
    let at = DateTime::parse_from_rfc3339(at)
        .ok()?
        .with_timezone(now.offset());
    let days = (now.date_naive() - at.date_naive()).num_days();
    Some(match days {
        0 => format!("today {}", at.format("%H:%M")),
        1 => format!("yesterday {}", at.format("%H:%M")),
        _ if at.format("%Y").to_string() == now.format("%Y").to_string() => {
            at.format("%-d %b").to_string()
        }
        _ => at.format("%-d %b %Y").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, args: &[&str]) -> ResumeEntry {
        ResumeEntry {
            title: title.to_string(),
            detail: None,
            at: None,
            args: args.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn now() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-11-10T09:15:00-04:00").unwrap()
    }

    #[test]
    fn resume_is_offered_only_for_what_can_be_reopened() {
        let entries = vec![
            entry("gone.pdf", &["/tmp/example/gone.pdf"]),
            entry("here.pdf", &["/tmp/example/here.pdf", "--page=4"]),
            entry("nothing to pass back", &[]),
            entry("  ", &["/tmp/example/here.pdf"]),
            entry("By key", &["--key=example2026"]),
        ];
        let kept = usable(entries, |path| path == "/tmp/example/here.pdf");
        let titles: Vec<_> = kept.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["here.pdf", "By key"]);
    }

    #[test]
    fn times_are_said_in_plain_words() {
        assert_eq!(
            describe_when("2026-11-10T08:05:00-04:00", now()).as_deref(),
            Some("today 08:05")
        );
        assert_eq!(
            describe_when("2026-11-09T16:40:00-04:00", now()).as_deref(),
            Some("yesterday 16:40")
        );
        assert_eq!(
            describe_when("2026-11-03T16:40:00-04:00", now()).as_deref(),
            Some("3 Nov")
        );
        assert_eq!(
            describe_when("2025-12-24T10:00:00-04:00", now()).as_deref(),
            Some("24 Dec 2025")
        );
        assert_eq!(describe_when("last Tuesday", now()), None);
    }

    #[test]
    fn a_time_written_in_another_zone_is_shown_in_this_one() {
        // 01:30 UTC on the 10th is 21:30 on the 9th at -04:00.
        assert_eq!(
            describe_when("2026-11-10T01:30:00Z", now()).as_deref(),
            Some("yesterday 21:30")
        );
    }

    #[test]
    fn a_row_line_joins_what_the_app_gave() {
        let mut e = entry("Example Reader", &["/tmp/example/reader.pdf"]);
        assert_eq!(describe(&e, now()), "Example Reader");
        e.detail = Some("p. 47".to_string());
        e.at = Some("2026-11-09T16:40:00-04:00".to_string());
        assert_eq!(
            describe(&e, now()),
            "Example Reader · p. 47 · yesterday 16:40"
        );
    }

    #[test]
    fn a_dated_summary_is_dropped_once_its_day_is_over() {
        let line = || Some("Week 6 · Example Term".to_string());
        let at = |t: &str| DateTime::parse_from_rfc3339(t).unwrap();
        let until = Some("2026-11-11T00:00:00-04:00");
        assert_eq!(
            current_summary(line(), None, at("2030-01-01T00:00:00Z")),
            line()
        );
        assert_eq!(
            current_summary(line(), until, at("2026-11-11T03:59:59Z")),
            line()
        );
        assert_eq!(
            current_summary(line(), until, at("2026-11-11T04:00:00Z")),
            None
        );
        assert_eq!(
            current_summary(line(), Some("tomorrow"), at("2026-11-10T12:00:00Z")),
            None
        );
        assert_eq!(
            current_summary(Some("  ".to_string()), None, at("2026-11-10T12:00:00Z")),
            None
        );
    }

    #[test]
    fn a_status_file_for_another_app_is_not_trusted() {
        let dir = std::env::temp_dir().join(format!("porog-status-{}", std::process::id()));
        let mut wrong = Status::new("zerkalo", "");
        wrong.summary = Some("not Sputnik's".to_string());
        wrong.write(&dir).unwrap();
        std::fs::rename(dir.join("zerkalo.json"), dir.join("sputnik.json")).unwrap();

        let state = row_state(&crate::APPS[0], Some(&dir), false);
        assert_eq!(state.summary, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
