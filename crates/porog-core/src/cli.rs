//! Porog's own command line: ways to reach a row or the search box without clicking, for a
//! keyboard shortcut or a script (`PLAN.md` Phase 11).
//!
//! `porog --resume=zerkalo` does what that row's button says, `--open=zerkalo` opens the app
//! plainly, `--search-box` opens the search box and `--search=WORDS` asks the apps at once. The
//! app is named by its status name, as in the status files: `sputnik`, `pereplyot`,
//! `kartoteka`, `zerkalo`, `skrizhal`.

use crate::APPS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cli {
    /// No options: show the window.
    Show,
    /// Do what row N's button says.
    Resume(usize),
    /// Open row N's app plainly.
    Open(usize),
    /// Open the search box, with these words already typed (maybe none).
    Search(String),
}

/// What the program's arguments (without the program name) ask for, or why they cannot.
pub fn parse(args: &[String]) -> Result<Cli, String> {
    let mut parsed = Cli::Show;
    for arg in args {
        parsed = match arg.split_once('=') {
            Some(("--resume", app)) => Cli::Resume(row_of(app)?),
            Some(("--open", app)) => Cli::Open(row_of(app)?),
            Some(("--search", words)) => Cli::Search(words.to_string()),
            None if arg == "--search" || arg == "--search-box" => Cli::Search(String::new()),
            _ => return Err(format!("Unknown option {arg}")),
        };
    }
    Ok(parsed)
}

fn row_of(name: &str) -> Result<usize, String> {
    APPS.iter()
        .position(|app| app.status_name == name)
        .ok_or_else(|| {
            let known: Vec<_> = APPS.iter().map(|a| a.status_name).collect();
            format!(
                "{name:?} is not a Fond app. Use one of: {}",
                known.join(", ")
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(args: &[&str]) -> Result<Cli, String> {
        parse(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn each_option_names_a_row_or_the_search() {
        assert_eq!(parse_all(&[]), Ok(Cli::Show));
        assert_eq!(parse_all(&["--resume=zerkalo"]), Ok(Cli::Resume(3)));
        assert_eq!(parse_all(&["--open=sputnik"]), Ok(Cli::Open(0)));
        assert_eq!(parse_all(&["--search-box"]), Ok(Cli::Search(String::new())));
        assert_eq!(
            parse_all(&["--search=grace and law"]),
            Ok(Cli::Search("grace and law".to_string()))
        );
    }

    #[test]
    fn anything_else_is_an_error_that_says_what_would_work() {
        assert!(parse_all(&["--resume=gimp"])
            .unwrap_err()
            .contains("zerkalo"));
        assert!(parse_all(&["--resume"]).is_err());
        assert!(parse_all(&["--frobnicate"]).is_err());
    }
}
