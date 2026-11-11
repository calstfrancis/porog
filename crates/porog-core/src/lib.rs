//! Porog's GTK-free core.
//!
//! Porog links none of the Fond apps. It knows each app only as an installed flatpak it can
//! start as a program, the same relationship Kartoteka and Sputnik have with Pereplyot. Every
//! function here that runs a command blocks, so the UI calls them from a worker thread.

pub mod config;
pub mod helper;
pub mod now;
pub mod search;
pub mod status;

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// One Fond app, as Porog shows and launches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct App {
    /// The name shown on the app's row.
    pub name: &'static str,
    /// What the app is for, in plain words. Shown when there is nothing to resume.
    pub purpose: &'static str,
    pub flatpak_id: &'static str,
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
        status_name: "sputnik",
    },
    App {
        name: "Pereplyot",
        purpose: "Reading",
        flatpak_id: "io.github.calstfrancis.Pereplyot",
        status_name: "pereplyot",
    },
    App {
        name: "Kartoteka",
        purpose: "References",
        flatpak_id: "io.github.calstfrancis.Kartoteka",
        status_name: "kartoteka",
    },
    App {
        name: "Zerkalo",
        purpose: "Writing",
        flatpak_id: "io.github.calstfrancis.Zerkalo",
        status_name: "zerkalo",
    },
    App {
        name: "Skrizhal",
        purpose: "CV elements",
        flatpak_id: "io.github.calstfrancis.Skrizhal",
        status_name: "skrizhal",
    },
];

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

/// Start `app` and return once it has been spawned; the app outlives this call.
pub fn launch(app: &App, args: &[String]) -> io::Result<()> {
    let mut child = launch_command(app, args, in_flatpak_sandbox())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
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
