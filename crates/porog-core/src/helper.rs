//! Running an app's helper command (`flatpak run --command=<name> <app id> …`) with a time
//! limit. Both the Now card and search ask an app a question this way.

use std::io::Read;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use crate::{host_flatpak_command, in_flatpak_sandbox, App};

pub enum Outcome {
    /// The helper exited successfully and printed this.
    Answer(String),
    /// It could not be started or exited with an error: an app that predates the helper.
    Failed,
    /// It did not finish in time.
    TimedOut,
}

/// Run `command` of `app` with `args`. Blocks, so call it from a worker thread.
pub fn run(app: &App, command: &str, args: &[String], timeout: Duration) -> Outcome {
    let mut cmd = host_flatpak_command(in_flatpak_sandbox());
    cmd.arg("run")
        .arg(format!("--command={command}"))
        .arg(app.flatpak_id)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let Ok(mut child) = cmd.spawn() else {
        return Outcome::Failed;
    };
    let mut stdout = child.stdout.take();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(out) = stdout.as_mut() {
            let _ = out.read_to_string(&mut text);
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        let _ = tx.send((ok, text));
    });
    match rx.recv_timeout(timeout) {
        Ok((true, text)) => Outcome::Answer(text),
        Ok((false, _)) => Outcome::Failed,
        Err(_) => Outcome::TimedOut,
    }
}
