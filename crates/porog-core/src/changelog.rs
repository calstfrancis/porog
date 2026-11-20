//! `CHANGELOG.md`, read into releases for the changelog window. The window shows the file's
//! own words; this only finds where each release, section and bullet starts and ends.

use chrono::NaiveDate;

/// One `## [0.6.0] "Open Latch" — 2026-11-18 — The tray icon is a list of your apps` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    /// The release name, without its quotes. Absent for an unreleased entry.
    pub name: Option<String>,
    /// "18 Nov 2026".
    pub date: Option<String>,
    /// The short description after the date.
    pub title: Option<String>,
    pub sections: Vec<Section>,
}

/// "Added", "Changed", "Fixed", or none for bullets that come before any `###` heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub heading: Option<String>,
    pub items: Vec<Item>,
}

/// A bullet. `**Headline.** detail` splits into the two; any other bullet is all `text`.
/// Both keep their inline Markdown (`**`, `*`, backticks); see [`inline_markup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub headline: Option<String>,
    pub text: String,
}

/// Every release in the file, newest first as written. Wrapped lines are joined, so a bullet
/// reflows to the window's width instead of the file's.
pub fn parse(md: &str) -> Vec<Release> {
    let mut releases: Vec<Release> = Vec::new();
    let mut bullet: Option<String> = None;
    let flush = |releases: &mut Vec<Release>, bullet: &mut Option<String>| {
        let Some(text) = bullet.take() else { return };
        let Some(release) = releases.last_mut() else {
            return;
        };
        if release.sections.is_empty() {
            release.sections.push(Section {
                heading: None,
                items: Vec::new(),
            });
        }
        if let Some(section) = release.sections.last_mut() {
            section.items.push(split_headline(&text));
        }
    };
    for line in md.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            flush(&mut releases, &mut bullet);
            releases.push(parse_heading(heading));
        } else if let Some(heading) = trimmed.strip_prefix("### ") {
            flush(&mut releases, &mut bullet);
            if let Some(release) = releases.last_mut() {
                release.sections.push(Section {
                    heading: Some(heading.trim().to_string()),
                    items: Vec::new(),
                });
            }
        } else if let Some(text) = trimmed.strip_prefix("- ") {
            flush(&mut releases, &mut bullet);
            bullet = Some(text.trim().to_string());
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            flush(&mut releases, &mut bullet);
        } else if let Some(text) = bullet.as_mut() {
            text.push(' ');
            text.push_str(trimmed);
        } else if !releases.is_empty() {
            // A paragraph under a release reads as a bullet of its own.
            bullet = Some(trimmed.to_string());
        }
    }
    flush(&mut releases, &mut bullet);
    releases
}

/// `[0.6.0] "Open Latch" — 2026-11-18 — Title`, any part after the version optional.
fn parse_heading(heading: &str) -> Release {
    let heading = heading.trim();
    let (version, rest) = match heading.strip_prefix('[').and_then(|h| h.split_once(']')) {
        Some((version, rest)) => (version.trim().to_string(), rest.trim()),
        None => (heading.to_string(), ""),
    };
    let mut rest = rest;
    let mut name = None;
    if let Some(after) = rest.strip_prefix('"') {
        if let Some((quoted, tail)) = after.split_once('"') {
            name = Some(quoted.trim().to_string()).filter(|n| !n.is_empty());
            rest = tail.trim();
        }
    }
    let mut date = None;
    let mut title = None;
    for part in rest
        .split('—')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match NaiveDate::parse_from_str(part, "%Y-%m-%d") {
            Ok(day) if date.is_none() => date = Some(day.format("%-d %b %Y").to_string()),
            _ => match &mut title {
                None => title = Some(part.to_string()),
                Some(title) => {
                    title.push_str(" — ");
                    title.push_str(part);
                }
            },
        }
    }
    Release {
        version,
        name,
        date,
        title,
        sections: Vec::new(),
    }
}

/// `**Headline.** the rest` → headline "Headline." and text "the rest".
fn split_headline(text: &str) -> Item {
    if let Some(after) = text.strip_prefix("**") {
        if let Some((headline, rest)) = after.split_once("**") {
            if !headline.trim().is_empty() {
                return Item {
                    headline: Some(headline.trim().to_string()),
                    text: rest.trim().to_string(),
                };
            }
        }
    }
    Item {
        headline: None,
        text: text.to_string(),
    }
}

/// Inline Markdown as Pango markup: `**bold**`, `*italic*`, `` `code` ``, everything else
/// escaped. A span left open at the end is closed, so the result always parses.
pub fn inline_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let (mut bold, mut italic, mut code) = (false, false, false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => {
                out.push_str(if code { "</tt>" } else { "<tt>" });
                code = !code;
            }
            '*' if !code && chars.peek() == Some(&'*') => {
                chars.next();
                out.push_str(if bold { "</b>" } else { "<b>" });
                bold = !bold;
            }
            '*' if !code => {
                out.push_str(if italic { "</i>" } else { "<i>" });
                italic = !italic;
            }
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    for (open, tag) in [(code, "</tt>"), (italic, "</i>"), (bold, "</b>")] {
        if open {
            out.push_str(tag);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# Changelog

## [Unreleased] — Calmer window

### Changed
- One line.

## [0.6.0] "Open Latch" — 2026-11-18 — The tray icon is a list of your apps

### Changed
- **The tray icon's left click drops down the list.** Click an app's name to open it or
  switch to it; a line under it resumes.

### Fixed
- A plain bullet with `code` in it.
"#;

    #[test]
    fn reads_releases_sections_and_wrapped_bullets() {
        let releases = parse(SAMPLE);
        assert_eq!(releases.len(), 2);
        let unreleased = &releases[0];
        assert_eq!(unreleased.version, "Unreleased");
        assert_eq!(unreleased.name, None);
        assert_eq!(unreleased.date, None);
        assert_eq!(unreleased.title.as_deref(), Some("Calmer window"));

        let release = &releases[1];
        assert_eq!(release.version, "0.6.0");
        assert_eq!(release.name.as_deref(), Some("Open Latch"));
        assert_eq!(release.date.as_deref(), Some("18 Nov 2026"));
        assert_eq!(
            release.title.as_deref(),
            Some("The tray icon is a list of your apps")
        );
        assert_eq!(release.sections.len(), 2);
        let changed = &release.sections[0];
        assert_eq!(changed.heading.as_deref(), Some("Changed"));
        assert_eq!(
            changed.items,
            vec![Item {
                headline: Some("The tray icon's left click drops down the list.".to_string()),
                text: "Click an app's name to open it or switch to it; a line under it resumes."
                    .to_string(),
            }]
        );
        assert_eq!(release.sections[1].items[0].headline, None);
    }

    #[test]
    fn the_real_changelog_parses_and_every_bullet_makes_valid_markup() {
        let releases = parse(include_str!("../../../CHANGELOG.md"));
        assert!(!releases.is_empty());
        for release in &releases {
            assert!(!release.version.is_empty());
            assert!(release.sections.iter().all(|s| !s.items.is_empty()));
            for item in release.sections.iter().flat_map(|s| &s.items) {
                assert!(!item.text.contains("\n"));
                for text in item.headline.iter().chain([&item.text]) {
                    let markup = inline_markup(text);
                    assert!(balanced(&markup), "{markup}");
                }
            }
        }
    }

    /// Each opening tag has its closer, in order.
    fn balanced(markup: &str) -> bool {
        let mut stack = Vec::new();
        let mut rest = markup;
        while let Some(start) = rest.find('<') {
            let end = rest[start..].find('>').map(|e| start + e);
            let Some(end) = end else { return false };
            let tag = &rest[start + 1..end];
            if let Some(name) = tag.strip_prefix('/') {
                if stack.pop() != Some(name.to_string()) {
                    return false;
                }
            } else {
                stack.push(tag.to_string());
            }
            rest = &rest[end + 1..];
        }
        stack.is_empty()
    }

    #[test]
    fn inline_markup_escapes_and_styles() {
        assert_eq!(
            inline_markup("**Bold** and *it* and `a<b>` & \"q\""),
            "<b>Bold</b> and <i>it</i> and <tt>a&lt;b&gt;</tt> &amp; &quot;q&quot;"
        );
        // Asterisks inside code stay literal; an unclosed span is closed.
        assert_eq!(inline_markup("`*x*`"), "<tt>*x*</tt>");
        assert_eq!(inline_markup("**open"), "<b>open</b>");
    }

    #[test]
    fn a_heading_without_name_or_date_keeps_its_title() {
        let release = parse_heading("[0.1.0] — Open Door");
        assert_eq!(release.version, "0.1.0");
        assert_eq!(release.title.as_deref(), Some("Open Door"));
        assert_eq!(release.date, None);
    }
}
