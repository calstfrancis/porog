//! One app's row: its name, what it can resume, the button and the user's note.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::config::{Config, NOTE_MAX};
use porog_core::status::{self, ResumeEntry, RowState};
use porog_core::App;

use crate::menu;
use crate::window::launch_then;

/// The parts of an app's row that change after it is built.
#[derive(Clone)]
pub(crate) struct AppRow {
    app: &'static App,
    index: usize,
    /// The app's name. Always opens the app plainly, whatever the button says.
    name: gtk::Button,
    /// Opens that app's What's New. The same icon on every row, whatever is in it.
    news: gtk::Button,
    detail: gtk::Label,
    /// "Resume" when the row shows something that can be reopened, otherwise "Open".
    primary: gtk::Button,
    /// The second and third resume entries, shown only with `details` on.
    extras: gtk::Box,
    /// One line from the work itself ("Method · "…the second argument fails because""), under
    /// the first entry; hidden when the app gave none.
    context_line: gtk::Box,
    context_text: gtk::Label,
    /// The note the user left for this app, under the first entry; hidden when there is none.
    note_line: gtk::Box,
    note_text: gtk::Label,
    state: Rc<RefCell<RowState>>,
    details: Rc<Cell<bool>>,
    toasts: adw::ToastOverlay,
    /// True from a press until the app is running or has failed, so a second press (or
    /// Alt+N, or Enter) cannot start it twice.
    starting: Rc<Cell<bool>>,
}

impl AppRow {
    pub(crate) fn new(
        app: &'static App,
        index: usize,
        details: Rc<Cell<bool>>,
        toasts: &adw::ToastOverlay,
        config: &Rc<RefCell<Config>>,
        config_path: &Path,
    ) -> (gtk::ListBoxRow, Self) {
        let name_label = gtk::Label::builder()
            .label(app.name)
            .xalign(0.0)
            .width_chars(10)
            .css_classes(["fond-row-title"])
            .build();
        let name = gtk::Button::builder()
            .child(&name_label)
            .valign(gtk::Align::Center)
            .tooltip_text(format!("Open {}", app.name))
            .css_classes(["flat", "porog-name"])
            .build();
        name.connect_clicked(move |name| {
            let _ = name.activate_action("win.open-plain", Some(&(index as i32).to_variant()));
        });
        let detail = gtk::Label::builder()
            .label(app.purpose)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["fond-row-detail"])
            .build();
        // A fixed width, so the text beside it does not shift when the label changes.
        let primary = gtk::Button::builder()
            .label("Open")
            .valign(gtk::Align::Center)
            .width_request(88)
            .build();
        primary.connect_clicked(move |primary| {
            let _ = primary.activate_action("win.open-app", Some(&(index as i32).to_variant()));
        });

        let news_hint = format!("What's new in {}", app.name);
        let news = gtk::Button::builder()
            .icon_name("help-about-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(&news_hint)
            .css_classes(["flat"])
            .build();
        news.update_property(&[gtk::accessible::Property::Label(&news_hint)]);
        news.connect_clicked(move |news| {
            let _ = news.activate_action("win.app-news", Some(&(index as i32).to_variant()));
        });
        let (edit, note_line, note_text) = note_widgets(app, &name, config, config_path, toasts);
        let (context_line, context_text) = indented_line(&name, &["fond-row-detail"]);

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.append(&name);
        line.append(&detail);
        line.append(&edit);
        line.append(&news);
        line.append(&primary);
        let extras = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .visible(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
        content.append(&line);
        content.append(&context_line);
        content.append(&note_line);
        content.append(&extras);
        let widget = gtk::ListBoxRow::builder()
            .child(&content)
            .selectable(false)
            .css_classes(["fond-row"])
            .build();

        let row = Self {
            app,
            index,
            name,
            news,
            detail,
            primary,
            extras,
            context_line,
            context_text,
            note_line,
            note_text,
            state: Rc::default(),
            details,
            toasts: toasts.clone(),
            starting: Rc::default(),
        };
        row.show_note(&note_display(&config.borrow(), app));
        row.show(RowState {
            installed: true,
            ..RowState::default()
        });
        (widget, row)
    }

    /// Re-read the row's state off the main thread, then show it.
    pub(crate) fn refresh(&self) {
        let row = self.clone();
        glib::spawn_future_local(async move {
            let app = row.app;
            let state = gio::spawn_blocking(move || {
                status::row_state(
                    app,
                    status::status_dir().as_deref(),
                    porog_core::in_flatpak_sandbox(),
                )
            })
            .await
            .unwrap_or_default();
            row.show(state);
        });
    }

    /// Put `state` on the row. The button says Resume only when the first entry can be
    /// reopened; in every other case it says Open, and opens the app.
    fn show(&self, state: RowState) {
        let app = self.app;
        let shortcut = format!("Alt+{}", self.index + 1);
        let now = status::now();

        let (text, label, hint) = match state.resume.first() {
            _ if !state.installed => (
                "Not installed".to_string(),
                "Open",
                format!("{} is not installed", app.name),
            ),
            Some(first) => (
                status::describe(first, now),
                "Resume",
                format!("Resume {} in {}", first.title.trim(), app.name),
            ),
            None => (
                state
                    .summary
                    .clone()
                    .unwrap_or_else(|| app.purpose.to_string()),
                "Open",
                format!("Open {}", app.name),
            ),
        };
        self.detail.set_label(&text);
        self.detail.set_tooltip_text(Some(&text));
        let context = state
            .resume
            .first()
            .filter(|_| state.installed)
            .and_then(|first| first.context.as_deref())
            .unwrap_or("");
        self.context_text.set_label(context);
        self.context_text.set_tooltip_text(Some(context));
        self.context_line.set_visible(!context.is_empty());
        let hint = if context.is_empty() {
            hint
        } else {
            format!("{hint}: {context}")
        };
        if !self.starting.get() {
            self.primary.set_label(label);
            self.primary.set_sensitive(state.installed);
        }
        self.primary
            .set_tooltip_text(Some(&format!("{hint} ({shortcut})")));
        self.primary
            .update_property(&[gtk::accessible::Property::Label(&hint)]);
        self.name.set_sensitive(state.installed);
        self.news.set_sensitive(state.installed);

        while let Some(child) = self.extras.first_child() {
            self.extras.remove(&child);
        }
        if state.installed {
            for entry in state.resume.iter().skip(1) {
                self.extras.append(&self.extra_line(entry, now));
            }
        }
        *self.state.borrow_mut() = state;
        self.show_details();
    }

    /// One older resume entry: its text and its own Resume button.
    fn extra_line(&self, entry: &ResumeEntry, now: status::Now) -> gtk::Box {
        let text = status::describe(entry, now);
        let label = gtk::Label::builder()
            .label(&text)
            .tooltip_text(&text)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["fond-row-detail"])
            .build();
        let hint = format!("Resume {} in {}", entry.title.trim(), self.app.name);
        let resume = gtk::Button::builder()
            .label("Resume")
            .valign(gtk::Align::Center)
            .width_request(88)
            .tooltip_text(&hint)
            .css_classes(["flat"])
            .build();
        resume.update_property(&[gtk::accessible::Property::Label(&hint)]);
        resume.connect_clicked({
            let app = self.app;
            let args = entry.args.clone();
            let toasts = self.toasts.clone();
            move |button| {
                button.set_label("Starting…");
                button.set_sensitive(false);
                let button = button.clone();
                launch_then(app, args.clone(), &toasts, move || {
                    button.set_label("Resume");
                    button.set_sensitive(true);
                });
            }
        });
        // An empty box as wide as the app's name, so this text starts under the first entry's.
        let indent = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let widths = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
        widths.add_widget(&self.name);
        widths.add_widget(&indent);
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.append(&indent);
        line.append(&label);
        line.append(&resume);
        let Some(context) = entry.context.as_deref().filter(|c| !c.trim().is_empty()) else {
            return line;
        };
        let (context_line, context_text) = indented_line(&self.name, &["fond-row-detail"]);
        context_text.set_label(context);
        context_text.set_tooltip_text(Some(context));
        context_line.set_visible(true);
        let both = gtk::Box::new(gtk::Orientation::Vertical, 2);
        both.append(&line);
        both.append(&context_line);
        both
    }

    /// Put the user's own note for this app under its first entry, or hide the line.
    fn show_note(&self, note: &str) {
        self.note_text.set_label(note);
        self.note_text.set_tooltip_text(Some(note));
        self.note_line.set_visible(!note.is_empty());
    }

    /// Show or hide the older entries to match the `details` toggle.
    pub(crate) fn show_details(&self) {
        self.extras
            .set_visible(self.details.get() && self.extras.first_child().is_some());
    }

    /// What the row's button does right now: Resume the first entry, or open the app.
    pub(crate) fn activate(&self) {
        let args = {
            let state = self.state.borrow();
            if !state.installed {
                drop(state);
                self.say_not_installed();
                return;
            }
            state.resume.first().map(|entry| entry.args.clone())
        };
        self.start(args.unwrap_or_default());
    }

    /// Open the app with nothing to resume.
    pub(crate) fn open_plain(&self) {
        if self.state.borrow().installed {
            self.start(Vec::new());
        } else {
            self.say_not_installed();
        }
    }

    /// Start the app, with the button saying so until it is running: a launch takes a second or
    /// two, and a button that seems to do nothing gets pressed again.
    fn start(&self, args: Vec<String>) {
        if self.starting.replace(true) {
            return;
        }
        self.primary.set_label("Starting…");
        self.primary.set_sensitive(false);
        let row = self.clone();
        launch_then(self.app, args, &self.toasts, move || {
            row.starting.set(false);
            let state = row.state.borrow();
            row.primary
                .set_label(if state.resume.is_empty() { "Open" } else { "Resume" });
            row.primary.set_sensitive(state.installed);
        });
    }

    /// Open this app's What's New window.
    pub(crate) fn show_news(&self) {
        if let Some(parent) = self.news.root().and_downcast::<gtk::Window>() {
            crate::news::show(&parent, self.app);
        }
    }

    fn say_not_installed(&self) {
        self.toasts.add_toast(adw::Toast::new(&format!(
            "{} is not installed.",
            self.app.name
        )));
    }
}

/// A line of text under a row's first entry: an empty box as wide as the app's name, so the
/// text starts under the entry's own, and the label beside it. Hidden until it has text.
fn indented_line(name: &gtk::Button, classes: &[&str]) -> (gtk::Box, gtk::Label) {
    let indent = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let widths = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
    widths.add_widget(name);
    widths.add_widget(&indent);
    let text = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(classes.to_vec())
        .build();
    let line = gtk::Box::builder().spacing(12).visible(false).build();
    line.append(&indent);
    line.append(&text);
    (line, text)
}

/// The note as the row shows it: its words, then when it was left ("next: the intro · left
/// 3 Nov") so it can be judged by its age without anything telling the user it is old.
fn note_display(config: &Config, app: &App) -> String {
    let note = config.note(app);
    if note.is_empty() {
        return String::new();
    }
    match config
        .note_at(app)
        .and_then(|at| status::describe_when(at, status::now()))
    {
        Some(when) => format!("{note} · left {when}"),
        None => note.to_string(),
    }
}

/// The note button on a row, the line that shows its note, and the label inside that line.
///
/// The button opens a small box holding one line of text: Enter keeps it, an empty line
/// removes it. The note is the user's own words, kept in Porog's config and nowhere else.
fn note_widgets(
    app: &'static App,
    name: &gtk::Button,
    config: &Rc<RefCell<Config>>,
    config_path: &Path,
    toasts: &adw::ToastOverlay,
) -> (gtk::MenuButton, gtk::Box, gtk::Label) {
    let hint = format!("Leave yourself a note for {}", app.name);
    let entry = gtk::Entry::builder()
        .placeholder_text("e.g. next: rewrite the intro")
        .max_length(NOTE_MAX as i32)
        .width_chars(34)
        .build();
    entry.update_property(&[gtk::accessible::Property::Label(&hint)]);
    let popover = gtk::Popover::builder().child(&entry).build();
    let edit = gtk::MenuButton::builder()
        .icon_name("document-edit-symbolic")
        .valign(gtk::Align::Center)
        .popover(&popover)
        .tooltip_text(&hint)
        .css_classes(["flat"])
        .build();
    edit.update_property(&[gtk::accessible::Property::Label(&hint)]);

    // An empty box as wide as the app's name, so the note starts under the first entry's text.
    let indent = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let widths = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
    widths.add_widget(name);
    widths.add_widget(&indent);
    let text = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["fond-row-detail", "porog-note"])
        .build();
    let line = gtk::Box::builder().spacing(12).visible(false).build();
    line.append(&indent);
    line.append(&text);

    popover.connect_show({
        let entry = entry.clone();
        let config = config.clone();
        move |_| {
            entry.set_text(config.borrow().note(app));
            entry.grab_focus();
        }
    });
    entry.connect_activate({
        let popover = popover.clone();
        let config = config.clone();
        let config_path = config_path.to_path_buf();
        let toasts = toasts.clone();
        let text = text.clone();
        let line = line.clone();
        move |entry| {
            config.borrow_mut().set_note(app, &entry.text());
            let note = note_display(&config.borrow(), app);
            text.set_label(&note);
            text.set_tooltip_text(Some(&note));
            line.set_visible(!note.is_empty());
            popover.popdown();
            menu::save_config(
                &config,
                &config_path,
                &toasts,
                "The note could not be saved. It lasts until Porog closes.",
            );
        }
    });
    (edit, line, text)
}
