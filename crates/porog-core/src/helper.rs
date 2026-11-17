//! Running an app's helper command (`flatpak run --command=<name> <app id> …`) with a time
//! limit. The Now card, search and What's New all ask an app a question this way.

use std::io::Read;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::{host_flatpak_command, in_flatpak_sandbox, App};

/// How often a waiting call looks at its cancel flag.
const POLL: Duration = Duration::from_millis(50);

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The helper exited successfully and printed this.
    Answer(String),
    /// It could not be started or exited with an error: an app that predates the helper.
    Failed,
    /// It did not finish in time, or the question was withdrawn. Either way it is killed.
    TimedOut,
}

/// Run `command` of `app` with `args`. Blocks, so call it from a worker thread.
pub fn run(app: &App, command: &str, args: &[String], timeout: Duration) -> Outcome {
    run_until(app, command, args, timeout, None)
}

/// Like [`run`], and also gives up (killing the helper) as soon as `cancel` is set: a search
/// that a newer query replaced does not leave its helpers running.
pub fn run_until(
    app: &App,
    command: &str,
    args: &[String],
    timeout: Duration,
    cancel: Option<&AtomicBool>,
) -> Outcome {
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
        let _ = tx.send(text);
    });

    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let cancelled = cancel.is_some_and(|c| c.load(Ordering::Relaxed));
        if left.is_zero() || cancelled {
            let _ = child.kill();
            let _ = child.wait();
            return Outcome::TimedOut;
        }
        match rx.recv_timeout(left.min(POLL)) {
            // The helper closed its output: it is finished or about to be.
            Ok(text) => {
                return match child.wait() {
                    Ok(status) if status.success() => Outcome::Answer(text),
                    _ => Outcome::Failed,
                };
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                return Outcome::Failed;
            }
        }
    }
}
