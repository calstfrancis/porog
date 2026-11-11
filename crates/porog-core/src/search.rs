//! Suite search (`PLAN.md` Phase 4): one query, asked of each app that can answer, results
//! kept in the fixed app order.
//!
//! Porog reads no app's files. An app that can search ships a helper command that prints its
//! hits as one line of JSON, the way Sputnik's `sputnik-now` does. Today only Kartoteka has
//! one (`kartoteka-search`); an app without a source is simply absent from the results, not
//! shown as failed. Each app's answer is shown in its own words.

use std::time::Duration;

use serde::Deserialize;

use crate::helper::{self, Outcome};
use crate::{is_installed, App, APPS};

/// The highest `schema` of an app's reply this Porog understands.
const SCHEMA: u32 = 1;
const TIMEOUT: Duration = Duration::from_secs(5);

/// One thing found in an app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub title: String,
    pub detail: String,
    /// Handed to the app unchanged to open this hit.
    pub args: Vec<String>,
}

/// What one app had to say about the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Hits(Vec<Hit>),
    /// Why there are no results to show ("Kartoteka is not installed.").
    Message(String),
}

/// An app's results.
#[derive(Debug, Clone)]
pub struct Group {
    pub app: &'static App,
    pub answer: Answer,
}

/// How to ask one app.
struct Source {
    /// Index into [`APPS`].
    app: usize,
    command: &'static str,
    too_old: &'static str,
    no_answer: &'static str,
    parse: fn(&str) -> Answer,
}

const SOURCES: [Source; 1] = [Source {
    app: 2,
    command: "kartoteka-search",
    too_old: "This Kartoteka is too old to search here.",
    no_answer: "Kartoteka did not answer.",
    parse: parse_kartoteka,
}];

/// Whether `query` is worth asking: two or more characters once trimmed.
pub fn is_askable(query: &str) -> bool {
    query.trim().chars().count() >= 2
}

/// Ask every source. Blocks: runs `flatpak info` and a helper per source, so call it from a
/// worker thread. Groups come back in [`APPS`] order.
pub fn search(query: &str) -> Vec<Group> {
    let words: Vec<String> = query.split_whitespace().map(str::to_string).collect();
    SOURCES
        .iter()
        .map(|source| {
            let app = &APPS[source.app];
            let answer = if !is_installed(app) {
                Answer::Message(format!("{} is not installed.", app.name))
            } else {
                match helper::run(app, source.command, &words, TIMEOUT) {
                    Outcome::Answer(text) => (source.parse)(&text),
                    Outcome::Failed => Answer::Message(source.too_old.to_string()),
                    Outcome::TimedOut => Answer::Message(source.no_answer.to_string()),
                }
            };
            Group { app, answer }
        })
        .collect()
}

#[derive(Deserialize)]
struct KartotekaReply {
    #[serde(default)]
    schema: u32,
    #[serde(default)]
    hits: Vec<KartotekaHit>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct KartotekaHit {
    key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    year: String,
}

fn parse_kartoteka(text: &str) -> Answer {
    let too_old = || Answer::Message(SOURCES[0].too_old.to_string());
    let Ok(reply) = serde_json::from_str::<KartotekaReply>(text.trim()) else {
        return too_old();
    };
    if reply.schema == 0 || reply.schema > SCHEMA {
        return too_old();
    }
    if let Some(error) = reply.error.filter(|e| !e.trim().is_empty()) {
        return Answer::Message(error);
    }
    Answer::Hits(
        reply
            .hits
            .into_iter()
            .filter(|h| !h.key.is_empty())
            .map(|h| Hit {
                title: if h.title.trim().is_empty() {
                    h.key.clone()
                } else {
                    h.title
                },
                detail: [h.author, h.year]
                    .into_iter()
                    .filter(|part| !part.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                args: vec![format!("kartoteka:{}", h.key)],
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_points_at_the_app_it_names() {
        assert_eq!(APPS[SOURCES[0].app].name, "Kartoteka");
    }

    #[test]
    fn kartotekas_hits_open_by_key() {
        let reply = r#"{"schema":1,"hits":[{"key":"example2019example","title":"An Example Study",
            "author":"Example, Ada","year":"2019"},{"key":"bare","title":"","author":"","year":""}]}"#;
        let Answer::Hits(hits) = parse_kartoteka(reply) else {
            panic!("expected hits");
        };
        assert_eq!(hits[0].title, "An Example Study");
        assert_eq!(hits[0].detail, "Example, Ada · 2019");
        assert_eq!(hits[0].args, ["kartoteka:example2019example"]);
        assert_eq!(hits[1].title, "bare");
        assert_eq!(hits[1].detail, "");
    }

    #[test]
    fn no_hits_is_an_empty_list_and_an_error_is_its_own_words() {
        assert_eq!(
            parse_kartoteka(r#"{"schema":1,"hits":[]}"#),
            Answer::Hits(Vec::new())
        );
        assert_eq!(
            parse_kartoteka(r#"{"schema":1,"error":"Kartoteka has no library yet."}"#),
            Answer::Message("Kartoteka has no library yet.".into())
        );
    }

    #[test]
    fn anything_else_reads_as_an_old_kartoteka() {
        let old = Answer::Message(SOURCES[0].too_old.to_string());
        assert_eq!(parse_kartoteka("not json"), old);
        assert_eq!(parse_kartoteka(r#"{"schema":9,"hits":[]}"#), old);
        assert_eq!(parse_kartoteka(r#"{"hits":[]}"#), old);
    }

    #[test]
    fn a_query_needs_two_characters() {
        assert!(!is_askable(" a "));
        assert!(is_askable("ab"));
    }
}
