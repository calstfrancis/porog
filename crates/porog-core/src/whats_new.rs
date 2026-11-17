//! An app's What's New, asked for rather than stored (`PLAN.md` Phase 10).
//!
//! Each Fond app ships `<app>-whats-new`, which prints its newest release's name and what
//! changed as one line of JSON (`fond_status::whats_new`) and exits without opening a window.
//! Porog shows those words unchanged in its own window. An app that cannot answer says so,
//! and says what to do about it.

use std::time::Duration;

use fond_status::whats_new::{WhatsNew, SCHEMA};

use crate::helper::{self, Outcome};
use crate::{is_installed, update_hint, App, Feature};

const TIMEOUT: Duration = Duration::from_secs(5);

/// What the What's New window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewsView {
    /// The release and what changed in it, in the app's words.
    Notes {
        /// "What's new in Zerkalo 0.59.0 “Clean Slate”".
        heading: String,
        /// Headline and detail for each change; the detail may be empty.
        items: Vec<(String, String)>,
    },
    /// One plain line: the app is not installed, is too old, or did not answer.
    Message(String),
}

/// Ask `app` for its What's New. Blocks: runs `flatpak info` and the helper, so call it from a
/// worker thread.
pub fn fetch(app: &App) -> NewsView {
    if !is_installed(app) {
        return NewsView::Message(format!("{} is not installed.", app.name));
    }
    let command = format!("{}-whats-new", app.status_name);
    match helper::run(app, &command, &[], TIMEOUT) {
        Outcome::Answer(text) => parse(app, &text),
        Outcome::Failed => too_old(app),
        Outcome::TimedOut => NewsView::Message(format!("{} did not answer.", app.name)),
    }
}

fn too_old(app: &App) -> NewsView {
    NewsView::Message(format!(
        "This {} is too old to show what's new here. {}",
        app.name,
        update_hint(app, Feature::WhatsNew)
    ))
}

/// Turn an app's reply into what to show. Anything that is not a reply Porog understands reads
/// as an old app: showing nothing is better than showing a guess.
pub fn parse(app: &App, text: &str) -> NewsView {
    let Ok(reply) = serde_json::from_str::<WhatsNew>(text.trim()) else {
        return too_old(app);
    };
    if reply.schema == 0 || reply.schema > SCHEMA || reply.version.trim().is_empty() {
        return too_old(app);
    }
    let heading = match reply.release_name.as_deref().filter(|n| !n.trim().is_empty()) {
        Some(name) => format!("What's new in {} {} “{name}”", app.name, reply.version),
        None => format!("What's new in {} {}", app.name, reply.version),
    };
    NewsView::Notes {
        heading,
        items: reply
            .items
            .into_iter()
            .filter(|item| !item.headline.trim().is_empty())
            .map(|item| (item.headline, item.detail))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::APPS;

    #[test]
    fn the_apps_words_are_shown_under_a_heading_naming_the_release() {
        let reply = r#"{"schema":1,"version":"0.59.0","release_name":"Clean Slate",
            "items":[{"headline":"New document","detail":"Opens it if it is there."},
                     {"headline":" "},{"headline":"Alone"}]}"#;
        assert_eq!(
            parse(&APPS[3], reply),
            NewsView::Notes {
                heading: "What's new in Zerkalo 0.59.0 “Clean Slate”".to_string(),
                items: vec![
                    ("New document".into(), "Opens it if it is there.".into()),
                    ("Alone".into(), String::new()),
                ],
            }
        );
    }

    #[test]
    fn a_release_without_a_name_has_a_plain_heading() {
        let NewsView::Notes { heading, items } =
            parse(&APPS[2], r#"{"schema":1,"version":"0.24.0","items":[]}"#)
        else {
            panic!("expected notes");
        };
        assert_eq!(heading, "What's new in Kartoteka 0.24.0");
        assert!(items.is_empty());
    }

    #[test]
    fn anything_unreadable_or_newer_reads_as_too_old() {
        let old = too_old(&APPS[1]);
        assert_eq!(parse(&APPS[1], ""), old);
        assert_eq!(parse(&APPS[1], "not json"), old);
        assert_eq!(parse(&APPS[1], r#"{"schema":2,"version":"1","items":[]}"#), old);
        assert_eq!(parse(&APPS[1], r#"{"schema":1,"version":"","items":[]}"#), old);
        let NewsView::Message(text) = old else {
            panic!("expected a message");
        };
        assert!(text.starts_with("This Pereplyot is too old to show what's new here."));
        assert!(text.ends_with("Update Pereplyot."));
    }
}
