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
use crate::{is_installed, update_hint, App, Feature, APPS};

/// The highest `schema` of Sputnik's reply this Porog understands.
const SCHEMA: u32 = 1;
/// How long to wait for Sputnik before saying it did not answer.
const TIMEOUT: Duration = Duration::from_secs(5);

pub const NOT_INSTALLED: &str = "Sputnik is not installed.";
pub const TOO_OLD: &str = "This Sputnik is too old to show today here.";

/// [`TOO_OLD`] and what to do about it.
pub fn too_old_message() -> String {
    format!("{TOO_OLD} {}", update_hint(&APPS[0], Feature::Now))
}
pub const NO_ANSWER: &str = "Sputnik did not answer.";

/// A button Sputnik offers for its Now item: start `app` with `args`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowAction {
    pub label: String,
    pub app: &'static App,
    pub args: Vec<String>,
}

/// One of the next dated things after the Now item, in Sputnik's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaterItem {
    pub title: String,
    pub detail: String,
}

/// How many `later` lines are kept, whatever Sputnik sends.
const LATER_MAX: usize = 3;

/// How many `pieces` are kept, whatever Sputnik sends.
const PIECES_MAX: usize = 4;

/// One thing the Now item is made of, with its own button: "Draft · THM 501: Essay" opening
/// the document in Zerkalo, "Reading · …" opening a linked reading in Pereplyot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowPiece {
    pub label: String,
    pub title: String,
    pub app: &'static App,
    pub args: Vec<String>,
}

/// What the Now area shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NowView {
    /// Sputnik's card, in its words.
    Card {
        title: String,
        detail: String,
        /// "2 more today · 1 needs a decision", or nothing when both are zero.
        footer: Option<String>,
        /// "Start reading", "New essay": what Sputnik says can be done with the item.
        action: Option<NowAction>,
        /// What comes next by date, shown only with `details` on.
        later: Vec<LaterItem>,
        /// What the item is made of, shown only with `details` on.
        pieces: Vec<NowPiece>,
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
    action: Option<Action>,
    #[serde(default)]
    later: Vec<Later>,
    #[serde(default)]
    pieces: Vec<Piece>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct Piece {
    #[serde(default)]
    label: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    app: String,
    #[serde(default)]
    args: Vec<String>,
}

impl Piece {
    /// A piece needs a title and an app Porog knows; anything else is dropped, as with an
    /// [`Action`].
    fn into_piece(self) -> Option<NowPiece> {
        let app = APPS.iter().find(|a| a.status_name == self.app)?;
        let title = self.title.trim();
        (!title.is_empty()).then(|| NowPiece {
            label: self.label.trim().to_string(),
            title: title.to_string(),
            app,
            args: self.args,
        })
    }
}

#[derive(Deserialize)]
struct Later {
    #[serde(default)]
    title: String,
    #[serde(default)]
    detail: String,
}

#[derive(Deserialize)]
struct Action {
    #[serde(default)]
    label: String,
    #[serde(default)]
    app: String,
    #[serde(default)]
    args: Vec<String>,
}

impl Action {
    /// Only an action with a label, for an app Porog knows, can be shown. Anything else is
    /// dropped rather than guessed at.
    fn into_action(self) -> Option<NowAction> {
        let label = self.label.trim();
        let app = APPS.iter().find(|a| a.status_name == self.app)?;
        (!label.is_empty()).then(|| NowAction {
            label: label.to_string(),
            app,
            args: self.args,
        })
    }
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
        Outcome::Failed => NowView::Message(too_old_message()),
        Outcome::TimedOut => NowView::Message(NO_ANSWER.to_string()),
    }
}

/// Whether Sputnik answers `sputnik-now` at all. "Sputnik has no vault yet." is an answer.
pub fn answers() -> bool {
    match helper::run(&APPS[0], "sputnik-now", &[], TIMEOUT) {
        Outcome::Answer(text) => parse(&text) != NowView::Message(too_old_message()),
        _ => false,
    }
}

/// Turn Sputnik's reply into what to show. Anything that is not a reply Porog understands
/// reads as an old Sputnik: showing nothing is better than showing a guess.
pub fn parse(text: &str) -> NowView {
    let too_old = || NowView::Message(too_old_message());
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
        action: reply.action.and_then(Action::into_action),
        later: reply
            .later
            .into_iter()
            .filter(|l| !l.title.trim().is_empty())
            .take(LATER_MAX)
            .map(|l| LaterItem {
                title: l.title,
                detail: l.detail,
            })
            .collect(),
        pieces: reply
            .pieces
            .into_iter()
            .filter_map(Piece::into_piece)
            .take(PIECES_MAX)
            .collect(),
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
    fn the_shared_example_replies_are_understood() {
        let NowView::Card {
            title,
            action,
            later,
            pieces,
            footer,
            ..
        } = parse(fond_status::contracts::NOW_CARD)
        else {
            panic!("expected a card");
        };
        assert_eq!(title, "Final essay");
        assert_eq!(footer.as_deref(), Some("1 more today"));
        assert_eq!(action.unwrap().label, "Write essay");
        assert_eq!(later.len(), 1);
        assert_eq!(pieces.len(), 2);
        assert_eq!(
            parse(fond_status::contracts::NOW_ERROR),
            NowView::Message("Sputnik has no vault yet.".to_string())
        );
    }

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
                action: None,
                later: Vec::new(),
                pieces: Vec::new(),
            }
        );
    }

    #[test]
    fn later_items_are_kept_in_sputniks_words_and_capped() {
        let reply = r#"{"schema":1,"now":{"title":"x"},"later":[
            {"title":"Essay","detail":"THM 501 · due Fri Nov 20"},{"title":" "},
            {"title":"b"},{"title":"c"},{"title":"d"}]}"#;
        let NowView::Card { later, .. } = parse(reply) else {
            panic!("expected a card");
        };
        let titles: Vec<_> = later.iter().map(|l| l.title.as_str()).collect();
        assert_eq!(titles, ["Essay", "b", "c"]);
        assert_eq!(later[0].detail, "THM 501 · due Fri Nov 20");
    }

    #[test]
    fn pieces_are_kept_for_known_apps_and_capped() {
        let reply = r#"{"schema":1,"now":{"title":"Essay"},"pieces":[
            {"label":"Draft","title":"THM 501: Essay","app":"zerkalo","args":["--new-document=THM 501: Essay"]},
            {"label":"Reading","title":"Soskice","app":"pereplyot","args":["/a.pdf"]},
            {"label":"Reading","title":"Unknown app","app":"gimp","args":[]},
            {"label":"Reading","title":" ","app":"pereplyot","args":[]},
            {"title":"No label is fine","app":"kartoteka"},
            {"title":"c","app":"zerkalo"},{"title":"d","app":"zerkalo"}]}"#;
        let NowView::Card { pieces, .. } = parse(reply) else {
            panic!("expected a card");
        };
        let titles: Vec<_> = pieces.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(
            titles,
            ["THM 501: Essay", "Soskice", "No label is fine", "c"]
        );
        assert_eq!(pieces[0].label, "Draft");
        assert_eq!(pieces[1].app.name, "Pereplyot");
        assert_eq!(pieces[2].label, "");
    }

    #[test]
    fn a_reply_without_later_has_none() {
        let NowView::Card { later, .. } = parse(r#"{"schema":1,"now":{"title":"x"}}"#) else {
            panic!("expected a card");
        };
        assert!(later.is_empty());
    }

    #[test]
    fn sputniks_action_is_kept_for_a_known_app() {
        let reply = r#"{"schema":1,"now":{"title":"Essay","detail":"","more_today":0,"needs_decision":0},
            "action":{"label":"New essay","app":"zerkalo","args":["--new-document=THM 501: Essay"]}}"#;
        let NowView::Card { action, .. } = parse(reply) else {
            panic!("expected a card");
        };
        let action = action.unwrap();
        assert_eq!(action.label, "New essay");
        assert_eq!(action.app.name, "Zerkalo");
        assert_eq!(
            action.args,
            vec!["--new-document=THM 501: Essay".to_string()]
        );
    }

    #[test]
    fn an_action_for_an_unknown_app_or_without_a_label_is_dropped() {
        for action in [
            r#"{"label":"Go","app":"gimp","args":[]}"#,
            r#"{"label":" ","app":"zerkalo","args":[]}"#,
            r#"{"app":"zerkalo"}"#,
        ] {
            let reply = format!(r#"{{"schema":1,"now":{{"title":"x"}},"action":{action}}}"#);
            let NowView::Card { action, .. } = parse(&reply) else {
                panic!("expected a card");
            };
            assert_eq!(action, None, "{reply}");
        }
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
        let too_old = NowView::Message(too_old_message());
        assert_eq!(parse(""), too_old);
        assert_eq!(parse("not json"), too_old);
        assert_eq!(parse(r#"{"schema":2,"now":{"title":"x"}}"#), too_old);
        assert_eq!(parse(r#"{"schema":1}"#), too_old);
        assert_eq!(parse(r#"{"schema":1,"now":{"title":" "}}"#), too_old);
    }
}
