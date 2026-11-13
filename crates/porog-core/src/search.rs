//! Suite search (`PLAN.md` Phase 4): one query, asked of each app that can answer, results
//! kept in the fixed app order.
//!
//! Porog reads no app's files. An app that can search ships a helper command that prints its
//! hits as one line of JSON, the way Sputnik's `sputnik-now` does: `<app>-search`, run through
//! `flatpak run --command=`. Every Fond app has one. Each app's answer is shown in its own
//! words, and the apps are asked at the same time, so a slow one does not hold up the rest.

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
    /// For an app whose hits name only a key: the scheme that makes the key a link the app
    /// opens (`kartoteka:KEY`). Other apps send each hit's `args` ready to use.
    scheme: Option<&'static str>,
    too_old: &'static str,
    no_answer: &'static str,
}

const SOURCES: [Source; 5] = [
    Source {
        app: 0,
        command: "sputnik-search",
        scheme: None,
        too_old: "This Sputnik is too old to search here.",
        no_answer: "Sputnik did not answer.",
    },
    Source {
        app: 1,
        command: "pereplyot-search",
        scheme: None,
        too_old: "This Pereplyot is too old to search here.",
        no_answer: "Pereplyot did not answer.",
    },
    Source {
        app: 2,
        command: "kartoteka-search",
        scheme: Some("kartoteka"),
        too_old: "This Kartoteka is too old to search here.",
        no_answer: "Kartoteka did not answer.",
    },
    Source {
        app: 3,
        command: "zerkalo-search",
        scheme: None,
        too_old: "This Zerkalo is too old to search here.",
        no_answer: "Zerkalo did not answer.",
    },
    Source {
        app: 4,
        command: "skrizhal-search",
        scheme: Some("skrizhal"),
        too_old: "This Skrizhal is too old to search here.",
        no_answer: "Skrizhal did not answer.",
    },
];

/// Whether `query` is worth asking: two or more characters once trimmed.
pub fn is_askable(query: &str) -> bool {
    query.trim().chars().count() >= 2
}

/// Ask every source, all at once. Blocks: runs `flatpak info` and a helper per source, so call
/// it from a worker thread. Groups come back in [`APPS`] order.
pub fn search(query: &str) -> Vec<Group> {
    let words: Vec<String> = query.split_whitespace().map(str::to_string).collect();
    std::thread::scope(|scope| {
        let asked: Vec<_> = SOURCES
            .iter()
            .map(|source| scope.spawn(|| ask(source, &words)))
            .collect();
        asked
            .into_iter()
            .zip(SOURCES.iter())
            .map(|(thread, source)| Group {
                app: &APPS[source.app],
                answer: thread
                    .join()
                    .unwrap_or_else(|_| Answer::Message(source.no_answer.to_string())),
            })
            .collect()
    })
}

fn ask(source: &Source, words: &[String]) -> Answer {
    let app = &APPS[source.app];
    if !is_installed(app) {
        return Answer::Message(format!("{} is not installed.", app.name));
    }
    match helper::run(app, source.command, words, TIMEOUT) {
        Outcome::Answer(text) => parse(source, &text),
        Outcome::Failed => Answer::Message(source.too_old.to_string()),
        Outcome::TimedOut => Answer::Message(source.no_answer.to_string()),
    }
}

#[derive(Deserialize)]
struct Reply {
    #[serde(default)]
    schema: u32,
    #[serde(default)]
    hits: Vec<ReplyHit>,
    error: Option<String>,
}

/// One hit as an app sends it. Kartoteka sends a `key` with `author` and `year`; the other
/// apps send `detail` and ready-made `args`.
#[derive(Deserialize)]
struct ReplyHit {
    #[serde(default)]
    key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    detail: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    year: String,
    #[serde(default)]
    args: Vec<String>,
}

fn parse(source: &Source, text: &str) -> Answer {
    let too_old = || Answer::Message(source.too_old.to_string());
    let Ok(reply) = serde_json::from_str::<Reply>(text.trim()) else {
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
            .filter_map(|h| {
                let args = if !h.args.is_empty() {
                    h.args
                } else if let (false, Some(scheme)) = (h.key.is_empty(), source.scheme) {
                    vec![format!("{scheme}:{}", h.key)]
                } else {
                    return None;
                };
                let detail = if h.detail.trim().is_empty() {
                    [h.author, h.year]
                        .into_iter()
                        .filter(|part| !part.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join(" · ")
                } else {
                    h.detail
                };
                let title = if h.title.trim().is_empty() {
                    h.key
                } else {
                    h.title
                };
                Some(Hit {
                    title,
                    detail,
                    args,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_points_at_the_app_it_names() {
        let names: Vec<_> = SOURCES.iter().map(|s| APPS[s.app].name).collect();
        assert_eq!(
            names,
            ["Sputnik", "Pereplyot", "Kartoteka", "Zerkalo", "Skrizhal"]
        );
    }

    #[test]
    fn kartotekas_hits_open_by_key() {
        let reply = r#"{"schema":1,"hits":[{"key":"example2019example","title":"An Example Study",
            "author":"Example, Ada","year":"2019"},{"key":"bare","title":"","author":"","year":""}]}"#;
        let Answer::Hits(hits) = parse(&SOURCES[2], reply) else {
            panic!("expected hits");
        };
        assert_eq!(hits[0].title, "An Example Study");
        assert_eq!(hits[0].detail, "Example, Ada · 2019");
        assert_eq!(hits[0].args, ["kartoteka:example2019example"]);
        assert_eq!(hits[1].title, "bare");
        assert_eq!(hits[1].detail, "");
    }

    #[test]
    fn other_apps_send_their_own_args_and_detail() {
        let reply = r#"{"schema":1,"hits":[{"title":"A quote","detail":"Doc · p. 3",
            "args":["/a/b.pdf","--annotation=hl-1"]},{"title":"no way to open it"}]}"#;
        let Answer::Hits(hits) = parse(&SOURCES[1], reply) else {
            panic!("expected hits");
        };
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].detail, "Doc · p. 3");
        assert_eq!(hits[0].args, ["/a/b.pdf", "--annotation=hl-1"]);
    }

    #[test]
    fn skrizhal_hits_open_by_key() {
        let reply = r#"{"schema":1,"hits":[{"key":"mdiv","title":"Master of Divinity",
            "detail":"Education · Example University"}]}"#;
        let Answer::Hits(hits) = parse(&SOURCES[4], reply) else {
            panic!("expected hits");
        };
        assert_eq!(hits[0].args, ["skrizhal:mdiv"]);
        assert_eq!(hits[0].detail, "Education · Example University");
    }

    #[test]
    fn no_hits_is_an_empty_list_and_an_error_is_its_own_words() {
        assert_eq!(
            parse(&SOURCES[2], r#"{"schema":1,"hits":[]}"#),
            Answer::Hits(Vec::new())
        );
        assert_eq!(
            parse(
                &SOURCES[2],
                r#"{"schema":1,"error":"Kartoteka has no library yet."}"#
            ),
            Answer::Message("Kartoteka has no library yet.".into())
        );
    }

    #[test]
    fn anything_else_reads_as_an_old_kartoteka() {
        let old = Answer::Message(SOURCES[2].too_old.to_string());
        assert_eq!(parse(&SOURCES[2], "not json"), old);
        assert_eq!(parse(&SOURCES[2], r#"{"schema":9,"hits":[]}"#), old);
        assert_eq!(parse(&SOURCES[2], r#"{"hits":[]}"#), old);
    }

    #[test]
    fn a_query_needs_two_characters() {
        assert!(!is_askable(" a "));
        assert!(is_askable("ab"));
    }
}
