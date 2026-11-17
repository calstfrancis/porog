//! Porog's GTK-free core.
//!
//! Porog links none of the Fond apps. It knows each app only as an installed flatpak it can
//! start as a program, the same relationship Kartoteka and Sputnik have with Pereplyot. Every
//! function here that runs a command blocks, so the UI calls them from a worker thread.

pub mod cli;
pub mod config;
pub mod helper;
pub mod now;
pub mod search;
pub mod status;
pub mod suite;
pub mod whats_new;

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// One Fond app, as Porog shows and launches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct App {
    /// The name shown on the app's row.
    pub name: &'static str,
    /// What the app is for, in plain words. Shown when there is nothing to resume.
    pub purpose: &'static str,
    pub flatpak_id: &'static str,
    /// What the row will show once the app has been used, for the first-run welcome:
    /// "your last document", not a promise about anything else.
    pub first_use: &'static str,
    /// The app's name in its status file: `~/.local/share/fond/status/<status_name>.json`.
    pub status_name: &'static str,
}

/// The five Fond apps, in the order their rows are shown: the order the work itself runs in
/// (see what is due, read, cite, write), with the CV last. The order is part of the design:
/// a row never moves, whatever was used most recently, so don't sort this at runtime.
pub const APPS: [App; 5] = [
    App {
        name: "Sputnik",
        purpose: "Courses and schedule",
        flatpak_id: "io.github.calstfrancis.Sputnik",
        first_use: "the week you are in, and what is due next",
        status_name: "sputnik",
    },
    App {
        name: "Pereplyot",
        purpose: "Reading",
        flatpak_id: "io.github.calstfrancis.Pereplyot",
        first_use: "the last document you read, and the page you were on",
        status_name: "pereplyot",
    },
    App {
        name: "Kartoteka",
        purpose: "References",
        flatpak_id: "io.github.calstfrancis.Kartoteka",
        first_use: "how many references your library holds",
        status_name: "kartoteka",
    },
    App {
        name: "Zerkalo",
        purpose: "Writing",
        flatpak_id: "io.github.calstfrancis.Zerkalo",
        first_use: "the last document you wrote, and where in it you were",
        status_name: "zerkalo",
    },
    App {
        name: "Skrizhal",
        purpose: "CV elements",
        flatpak_id: "io.github.calstfrancis.Skrizhal",
        first_use: "how many CV entries you have",
        status_name: "skrizhal",
    },
];

/// Something Porog asks an app for, which an older release of that app cannot answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// The status file the rows read.
    Status,
    /// Sputnik's `sputnik-now`.
    Now,
    /// The app's `<app>-search` helper.
    Search,
    /// The app's `<app>-whats-new` helper.
    WhatsNew,
}

/// The first release of each app that can answer each [`Feature`], by `status_name`. Checked
/// against the app's CHANGELOG when written; add a row here whenever Porog starts asking an
/// app for something new, in the same change (see `CLAUDE.md`). An app not listed for a
/// feature has no version to name yet, and the message just says to update it.
const MIN_VERSIONS: &[(&str, Feature, &str)] = &[
    ("sputnik", Feature::Status, "0.24.2"),
    ("pereplyot", Feature::Status, "0.23.2"),
    ("kartoteka", Feature::Status, "0.24.0"),
    ("zerkalo", Feature::Status, "0.57.2"),
    ("skrizhal", Feature::Status, "0.4.1"),
    ("sputnik", Feature::Now, "0.24.2"),
    ("sputnik", Feature::Search, "0.25.0"),
    ("pereplyot", Feature::Search, "0.24.0"),
    ("kartoteka", Feature::Search, "0.24.0"),
    ("zerkalo", Feature::Search, "0.58.0"),
    ("skrizhal", Feature::Search, "0.5.0"),
];

/// What to do about an app that cannot answer `feature`: "Update Sputnik to 0.25.0 or later."
pub fn update_hint(app: &App, feature: Feature) -> String {
    let version = MIN_VERSIONS
        .iter()
        .find(|(name, f, _)| *name == app.status_name && *f == feature)
        .map(|(_, _, version)| *version);
    match version {
        Some(version) => format!("Update {} to {version} or later.", app.name),
        None => format!("Update {}.", app.name),
    }
}

/// The app that opens a dropped file, by its extension: PDFs and EPUBs in Pereplyot, Typst
/// files in Zerkalo. Anything else has no app, and Porog says so instead of guessing.
pub fn app_for_file(path: &Path) -> Option<&'static App> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let status_name = match extension.as_str() {
        "pdf" | "epub" => "pereplyot",
        "typ" => "zerkalo",
        _ => return None,
    };
    APPS.iter().find(|app| app.status_name == status_name)
}

/// Whether this process is running inside a flatpak sandbox — `flatpak-spawn --host` only
/// works, and is only needed, there; a non-flatpak install can just exec `flatpak` directly.
pub fn in_flatpak_sandbox() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// A `flatpak` invocation, routed through the host when sandboxed — the same pattern
/// Sputnik's and Kartoteka's Pereplyot launch uses. Callers append their own subcommand.
fn host_flatpak_command(sandboxed: bool) -> Command {
    if sandboxed {
        let mut cmd = Command::new("flatpak-spawn");
        cmd.arg("--host").arg("flatpak");
        cmd
    } else {
        Command::new("flatpak")
    }
}

/// The command that starts `app`, with `args` handed to the app unchanged.
pub fn launch_command(app: &App, args: &[String], sandboxed: bool) -> Command {
    let mut cmd = host_flatpak_command(sandboxed);
    cmd.arg("run").arg(app.flatpak_id).args(args);
    cmd
}

/// The remote Fond apps are installed from.
pub const REMOTE_URL: &str = "https://calstfrancis.github.io/flatpak/calstfrancis.flatpakrepo";

/// The two commands that install `app` on a machine that has not got it, for the "How to
/// install" dialog to show and copy. The first is harmless to run twice.
pub fn install_commands(app: &App) -> String {
    format!(
        "flatpak remote-add --user --if-not-exists calstfrancis {REMOTE_URL}\nflatpak install --user calstfrancis {}",
        app.flatpak_id
    )
}

/// The version of `app` that is installed: the `Version:` line of `flatpak info`. `None` when
/// it is not installed or the line is missing. Blocks, so call it from a worker thread.
pub fn installed_version(app: &App) -> Option<String> {
    let output = host_flatpak_command(in_flatpak_sandbox())
        .args(["info", app.flatpak_id])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then(|| version_in(&String::from_utf8_lossy(&output.stdout)))?
}

/// The value of the `Version:` line in `flatpak info` output.
fn version_in(info: &str) -> Option<String> {
    info.lines()
        .find_map(|line| line.trim().strip_prefix("Version:"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Whether `app`'s flatpak is installed. Blocks while `flatpak info` runs.
pub fn is_installed(app: &App) -> bool {
    host_flatpak_command(in_flatpak_sandbox())
        .args(["info", app.flatpak_id])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// How long [`launch`] watches a new process for an early failure before calling it started.
pub const LAUNCH_GRACE: Duration = Duration::from_secs(3);

/// Start `app` and return once it is clearly running. `flatpak run` that fails (the app is
/// missing, the arguments are refused) exits within moments with an error; that is reported as
/// an `Err`. One still running after [`LAUNCH_GRACE`], or that has exited successfully, counts
/// as started. Blocks for at most the grace period, so call it from a worker thread.
pub fn launch(app: &App, args: &[String]) -> io::Result<()> {
    let mut child = launch_command(app, args, in_flatpak_sandbox())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + LAUNCH_GRACE;
    loop {
        match child.try_wait()? {
            Some(status) if status.success() => return Ok(()),
            Some(status) => {
                return Err(io::Error::other(format!("flatpak run exited with {status}")))
            }
            None if Instant::now() >= deadline => break,
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    // Reap the child when it exits, so a long Porog session doesn't collect zombies.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(cmd: &Command) -> Vec<String> {
        std::iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(|s| s.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn rows_keep_their_order() {
        let names: Vec<_> = APPS.iter().map(|app| app.name).collect();
        assert_eq!(
            names,
            ["Sputnik", "Pereplyot", "Kartoteka", "Zerkalo", "Skrizhal"]
        );
    }

    #[test]
    fn the_hint_names_the_version_when_one_is_known() {
        assert_eq!(
            update_hint(&APPS[0], Feature::Search),
            "Update Sputnik to 0.25.0 or later."
        );
        assert_eq!(update_hint(&APPS[1], Feature::Now), "Update Pereplyot.");
    }

    #[test]
    fn the_install_commands_add_the_remote_then_the_app() {
        let commands = install_commands(&APPS[3]);
        let lines: Vec<_> = commands.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("flatpak remote-add --user --if-not-exists calstfrancis https://"));
        assert_eq!(
            lines[1],
            "flatpak install --user calstfrancis io.github.calstfrancis.Zerkalo"
        );
    }

    #[test]
    fn the_version_is_read_from_flatpak_info() {
        let info = "\nZerkalo - A Typst editor\n\n          ID: io.github.calstfrancis.Zerkalo\n         Ref: app/x/x86_64/stable\n     Version: 0.59.0\n      Branch: stable\n";
        assert_eq!(version_in(info).as_deref(), Some("0.59.0"));
        assert_eq!(version_in("ID: x\n"), None);
    }

    #[test]
    fn a_dropped_file_goes_to_the_app_that_opens_it() {
        let app = |name: &str| app_for_file(Path::new(name)).map(|a| a.name);
        assert_eq!(app("/home/me/Soskice.PDF"), Some("Pereplyot"));
        assert_eq!(app("book.epub"), Some("Pereplyot"));
        assert_eq!(app("essay-2.typ"), Some("Zerkalo"));
        assert_eq!(app("refs.bib"), None);
        assert_eq!(app("README"), None);
    }

    #[test]
    fn launches_directly_outside_a_sandbox() {
        let cmd = launch_command(&APPS[0], &[], false);
        assert_eq!(
            argv(&cmd),
            ["flatpak", "run", "io.github.calstfrancis.Sputnik"]
        );
    }

    #[test]
    fn launches_through_the_host_inside_a_sandbox() {
        let cmd = launch_command(&APPS[1], &["/tmp/a.pdf".to_string()], true);
        assert_eq!(
            argv(&cmd),
            [
                "flatpak-spawn",
                "--host",
                "flatpak",
                "run",
                "io.github.calstfrancis.Pereplyot",
                "/tmp/a.pdf"
            ]
        );
    }
}
