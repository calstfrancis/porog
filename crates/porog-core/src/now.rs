//! Sputnik's Now item, asked for rather than worked out (`PLAN.md` P2, §3.2).
//!
//! A status file would be wrong after midnight, so Porog runs
//! `flatpak run --command=sputnik-now io.github.calstfrancis.Sputnik` each time its window
//! gains focus. Sputnik prints the Today tab's Now card as one line of JSON, in its own words;
//! Porog shows them unchanged and adds nothing of its own except the three literal messages
//! below for when there is no card to show.

use std::time::Duration;

use serde::Deserialize;

use crate::helper::{self, Outcome};
use crate::{is_installed, APPS};

/// The highest `schema` of Sputnik's reply this Porog understands.
const SCHEMA: u32 = 1;
/// How long to wait for Sputnik before saying it did not answer.
const TIMEOUT: Duration = Duration::from_secs(5);

pub const NOT_INSTALLED: &str = "Sputnik is not installed.";
pub const TOO_OLD: &str = "This Sputnik is too old to show today here.";
pub const NO_ANSWER: &str = "Sputnik did not answer.";

/// What the Now area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NowView {
    /// Sputnik's card, in its words.
    Card {
        title: String,
        detail: String,
        /// "2 more today · 1 needs a decision", or nothing when both are zero.
        footer: Option<String>,
    },
    /// One plain line: a literal outcome above, or Sputnik's own reason it has no card
    /// ("Sputnik has no vault yet.").
    Message(String),
}

#[derive(Deserialize)]
struct Reply {
    #[serde(default)]
    schema: u32,
    now: Option<Card>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct Card {
    title: String,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    more_today: usize,
    #[serde(default)]
    needs_decision: usize,
}

/// Ask Sputnik for its Now card. Blocks: runs `flatpak info` and the helper, so call it from
/// a worker thread.
pub fn fetch() -> NowView {
    if !is_installed(&APPS[0]) {
        return NowView::Message(NOT_INSTALLED.to_string());
    }
    match helper::run(&APPS[0], "sputnik-now", &[], TIMEOUT) {
        Outcome::Answer(text) => parse(&text),
        // A Sputnik without the command fails outright rather than answering.
        Outcome::Failed => NowView::Message(TOO_OLD.to_string()),
        Outcome::TimedOut => NowView::Message(NO_ANSWER.to_string()),
    }
}

/// Turn Sputnik's reply into what to show. Anything that is not a reply Porog understands
/// reads as an old Sputnik: showing nothing is better than showing a guess.
pub fn parse(text: &str) -> NowView {
    let too_old = || NowView::Message(TOO_OLD.to_string());
    let Ok(reply) = serde_json::from_str::<Reply>(text.trim()) else {
        return too_old();
    };
    if reply.schema == 0 || reply.schema > SCHEMA {
        return too_old();
    }
    if let Some(error) = reply.error.filter(|e| !e.trim().is_empty()) {
        return NowView::Message(error);
    }
    let Some(card) = reply.now.filter(|c| !c.title.trim().is_empty()) else {
        return too_old();
    };
    NowView::Card {
        footer: footer(card.more_today, card.needs_decision),
        title: card.title,
        detail: card.detail,
    }
}

/// "2 more today · 1 needs a decision". Plain text, exact numbers, nothing for zero.
fn footer(more_today: usize, needs_decision: usize) -> Option<String> {
    let mut parts = Vec::new();
    if more_today > 0 {
        parts.push(format!("{more_today} more today"));
    }
    match needs_decision {
        0 => {}
        1 => parts.push("1 needs a decision".to_string()),
        n => parts.push(format!("{n} need a decision")),
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sputniks_words_are_shown_unchanged() {
        let reply = r#"{"schema":1,"now":{"state":"work","title":"Example Reader, ch. 3",
            "detail":"THM 501 · free for 40 min","more_today":2,"needs_decision":1}}"#;
        assert_eq!(
            parse(reply),
            NowView::Card {
                title: "Example Reader, ch. 3".to_string(),
                detail: "THM 501 · free for 40 min".to_string(),
                footer: Some("2 more today · 1 needs a decision".to_string()),
            }
        );
    }

    #[test]
    fn a_quiet_day_has_no_footer() {
        let reply = r#"{"schema":1,"now":{"title":"Nothing left for today.","detail":"Rest counts.",
            "more_today":0,"needs_decision":0}}"#;
        let NowView::Card { footer, .. } = parse(reply) else {
            panic!("expected a card");
        };
        assert_eq!(footer, None);
        assert_eq!(footer_text(0, 3), "3 need a decision");
        assert_eq!(footer_text(1, 0), "1 more today");
    }

    fn footer_text(more: usize, decide: usize) -> String {
        footer(more, decide).unwrap()
    }

    #[test]
    fn sputniks_own_reason_for_having_no_card_is_shown() {
        assert_eq!(
            parse(r#"{"schema":1,"error":"Sputnik has no vault yet."}"#),
            NowView::Message("Sputnik has no vault yet.".to_string())
        );
    }

    #[test]
    fn anything_unreadable_or_newer_reads_as_too_old() {
        let too_old = NowView::Message(TOO_OLD.to_string());
        assert_eq!(parse(""), too_old);
        assert_eq!(parse("not json"), too_old);
        assert_eq!(parse(r#"{"schema":2,"now":{"title":"x"}}"#), too_old);
        assert_eq!(parse(r#"{"schema":1}"#), too_old);
        assert_eq!(parse(r#"{"schema":1,"now":{"title":" "}}"#), too_old);
    }
}
