//! The Suite window's facts: for each app, whether it is installed, which version, and which of
//! the things Porog asks of it it answers. Found by asking, not by guessing from a version
//! number, so a Sputnik built from a branch reports what it really does.

use fond_status::Status;

use crate::{
    installed_version, is_installed, now, search, status, update_hint, whats_new, App, Feature,
    APPS,
};

/// How one app stands on one thing Porog asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    Answers,
    /// The app is fine but has nothing to say yet: it has not been used since it learned to
    /// write a status file.
    NotYet,
    /// It does not answer; the text says what to do.
    Missing(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub app: &'static App,
    pub installed: bool,
    pub version: Option<String>,
    /// Each thing Porog asks of the app, in a fixed order.
    pub things: Vec<(&'static str, Standing)>,
}

/// Ask `app` everything. Blocks: runs `flatpak info` and a helper per thing, so call it from a
/// worker thread, one per app.
pub fn probe(app: &'static App) -> Report {
    if !is_installed(app) {
        return Report {
            app,
            installed: false,
            version: None,
            things: Vec::new(),
        };
    }
    let missing = |feature| Standing::Missing(update_hint(app, feature));
    let mut things = vec![(
        "Status file",
        match status::status_dir().map(|dir| Status::read(&dir, app.status_name)) {
            Some(Ok(Some(_))) => Standing::Answers,
            Some(Ok(None)) | None => Standing::NotYet,
            Some(Err(_)) => missing(Feature::Status),
        },
    )];
    if app.status_name == "sputnik" {
        let standing = if now::answers() {
            Standing::Answers
        } else {
            missing(Feature::Now)
        };
        things.push(("Today's Now card", standing));
    }
    things.push((
        "Search",
        if search::answers(app) {
            Standing::Answers
        } else {
            missing(Feature::Search)
        },
    ));
    things.push((
        "What's New",
        match whats_new::fetch(app) {
            whats_new::NewsView::Notes { .. } => Standing::Answers,
            whats_new::NewsView::Message(_) => missing(Feature::WhatsNew),
        },
    ));
    Report {
        app,
        installed: true,
        version: installed_version(app),
        things,
    }
}

/// Ask every app at once. Reports come back in [`APPS`] order.
pub fn probe_all() -> Vec<Report> {
    std::thread::scope(|scope| {
        let asked: Vec<_> = APPS
            .iter()
            .map(|app| scope.spawn(move || probe(app)))
            .collect();
        asked
            .into_iter()
            .zip(APPS.iter())
            .map(|(thread, app)| {
                thread.join().unwrap_or(Report {
                    app,
                    installed: false,
                    version: None,
                    things: Vec::new(),
                })
            })
            .collect()
    })
}
