//! One app's row: its name, what it can resume, the button, and a quiet menu holding the
//! user's note and the app's What's New.
//!
//! ```text
//! Zerkalo   An Example Essay · today 16:40        ⋯   ( Resume )
//!           §2 Method · "…the second argument fails because"
//!           next: rewrite the intro · left 3 Nov
//! ```

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

/// What runs after a refresh, if anything.
type ChangedHook = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// The parts of an app's row that change after it is built.
#[derive(Clone)]
pub(crate) struct AppRow {
    app: &'static App,
    index: usize,
    /// The whole row, which a screen reader announces as one sentence.
    widget: gtk::ListBoxRow,
    /// The app's name. Always opens the app plainly, whatever the button says.
    name: gtk::Button,
    /// The row's menu: note, What's New, open without resuming. The same quiet icon on every
    /// row, whatever is in it (`PLAN.md` P5).
    menu: RowMenu,
    detail: gtk::Label,
    /// "Resume" when the row shows something that can be reopened, otherwise "Open".
    primary: gtk::Button,
    /// The second and third resume entries, shown only with `details` on.
    extras: gtk::Box,
    /// One line from the work itself ("Method · "…the second argument fails because""), under
    /// the first entry; hidden when the app gave none.
    context_line: gtk::Box,
    context_text: gtk::Label,
    state: Rc<RefCell<RowState>>,
    details: Rc<Cell<bool>>,
    toasts: adw::ToastOverlay,
    /// True from a press until the app is running or has failed, so a second press (or
    /// Alt+N, or Enter) cannot start it twice.
    starting: Rc<Cell<bool>>,
    /// Called after a refresh puts new state on the row (the tray menu follows the rows).
    changed: ChangedHook,
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
            .width_request(96)
            .css_classes(["porog-act", "porog-open"])
            .build();
        primary.connect_clicked(move |primary| {
            let _ = primary.activate_action("win.open-app", Some(&(index as i32).to_variant()));
        });

        let menu = RowMenu::new(app, index, &name, config, config_path, toasts);
        let (context_line, context_text) = indented_line(&name, &["fond-row-detail"]);

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        line.append(&name);
        line.append(&detail);
        line.append(&menu.button);
        line.append(&primary);
        let extras = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_top(2)
            .visible(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
        content.append(&line);
        content.append(&context_line);
        content.append(&menu.note_line);
        content.append(&extras);
        let widget = gtk::ListBoxRow::builder()
            .child(&content)
            .selectable(false)
            .css_classes(["fond-row"])
            .build();

        let row = Self {
            app,
            index,
            widget: widget.clone(),
            name,
            menu,
            detail,
            primary,
            extras,
            context_line,
            context_text,
            state: Rc::default(),
            details,
            toasts: toasts.clone(),
            starting: Rc::default(),
            changed: Rc::default(),
        };
        row.show(RowState {
            installed: true,
            ..RowState::default()
        });
        (widget, row)
    }

    /// Re-read the row's state off the main thread.
    async fn fetch_state(&self) -> RowState {
        let app = self.app;
        gio::spawn_blocking(move || {
            status::row_state(
                app,
                status::status_dir().as_deref(),
                porog_core::in_flatpak_sandbox(),
            )
        })
        .await
        .unwrap_or_default()
    }

    /// Re-read the row's state off the main thread, then show it.
    pub(crate) fn refresh(&self) {
        let row = self.clone();
        glib::spawn_future_local(async move {
            let state = row.fetch_state().await;
            row.show(state);
            let changed = row.changed.borrow().clone();
            if let Some(changed) = changed {
                changed();
            }
        });
    }

    /// Read the state afresh, then do what the button says: for a request that arrives before
    /// the row has ever been read (the tray menu, `--resume` on a first launch).
    pub(crate) fn activate_fresh(&self) {
        let row = self.clone();
        glib::spawn_future_local(async move {
            let state = row.fetch_state().await;
            row.show(state);
            row.activate();
        });
    }

    /// Run `f` after each refresh.
    pub(crate) fn on_change(&self, f: impl Fn() + 'static) {
        *self.changed.borrow_mut() = Some(Rc::new(f));
    }

    /// This row's place in the tray menu; `None` while the app is not installed.
    pub(crate) fn tray_line(&self) -> Option<crate::tray::Line> {
        let (text, resume) = status::tray_entry(self.app, &self.state.borrow())?;
        Some(crate::tray::Line {
            index: self.index,
            text,
            resume,
        })
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
                "Install…",
                format!("{} is not installed: show how to install it", app.name),
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
        // One sentence for a screen reader: "Zerkalo. An Example Essay · today 16:40. Resume."
        let spoken = format!("{}. {text}. {label}.", app.name);
        self.widget
            .update_property(&[gtk::accessible::Property::Label(&spoken)]);
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
            set_primary(&self.primary, label);
            self.primary.set_sensitive(true);
        }
        self.primary
            .set_tooltip_text(Some(&format!("{hint} ({shortcut})")));
        self.primary
            .update_property(&[gtk::accessible::Property::Label(&hint)]);
        self.name.set_sensitive(state.installed);
        self.menu.news.set_sensitive(state.installed);
        self.menu
            .plain
            .set_visible(state.installed && !state.resume.is_empty());

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
            .width_request(96)
            .tooltip_text(&hint)
            .css_classes(["flat", "porog-act-quiet"])
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
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
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
            let label = if row.state.borrow().resume.is_empty() {
                "Open"
            } else {
                "Resume"
            };
            set_primary(&row.primary, label);
            row.primary.set_sensitive(true);
        });
    }

    /// Open the note box, as the menu's "Leave a note…" does.
    pub(crate) fn edit_note(&self) {
        self.menu.edit_note();
    }

    /// Open this app's What's New window.
    pub(crate) fn show_news(&self) {
        if self.menu.news.is_sensitive() {
            show_news(&self.menu.button, self.app);
        }
    }

    /// The app is not there: say how to get it instead of only saying it is missing.
    fn say_not_installed(&self) {
        if let Some(parent) = self.primary.root().and_downcast::<gtk::Window>() {
            crate::install::show(&parent, self.app);
        }
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

/// The primary button's words, and its look: Resume is a soft filled button, Open and Install…
/// a hairline outline, so a row with somewhere to go back to can be told at a glance by shape,
/// not colour. "Starting…" keeps whichever look it had.
fn set_primary(button: &gtk::Button, label: &str) {
    button.set_label(label);
    let resume = label == "Resume";
    for (class, on) in [("porog-resume", resume), ("porog-open", !resume)] {
        if on {
            button.add_css_class(class);
        } else {
            button.remove_css_class(class);
        }
    }
}

fn show_news(widget: &impl IsA<gtk::Widget>, app: &'static App) {
    if let Some(parent) = widget.root().and_downcast::<gtk::Window>() {
        crate::news::show(&parent, app);
    }
}

/// A row's menu, behind one quiet "⋯" button: open without resuming, the note, What's New.
/// The note is written in the same popover, which turns into a one-line box: Enter keeps it,
/// an empty line removes it. The note is the user's own words, kept in Porog's config and
/// nowhere else, and shown under the row's first entry.
#[derive(Clone)]
struct RowMenu {
    button: gtk::MenuButton,
    popover: gtk::Popover,
    /// "menu" or "note".
    pages: gtk::Stack,
    entry: gtk::Entry,
    /// "Open Zerkalo without resuming", shown only while the row says Resume.
    plain: gtk::Button,
    news: gtk::Button,
    /// "Leave a note…" or "Change the note…".
    note: gtk::Button,
    clear: gtk::Button,
    note_line: gtk::Box,
    note_text: gtk::Label,
}

impl RowMenu {
    fn new(
        app: &'static App,
        index: usize,
        name: &gtk::Button,
        config: &Rc<RefCell<Config>>,
        config_path: &Path,
        toasts: &adw::ToastOverlay,
    ) -> Self {
        let plain = menu::menu_item(&format!("Open {} without resuming", app.name), None);
        let note = menu::menu_item("Leave a note…", Some("N"));
        let clear = menu::menu_item("Clear the note", None);
        let news = menu::menu_item(&format!("What's new in {}", app.name), Some("W"));
        let items = gtk::Box::new(gtk::Orientation::Vertical, 2);
        items.append(&plain);
        items.append(&note);
        items.append(&clear);
        items.append(&menu::popover_separator());
        items.append(&news);

        let hint = format!("Leave yourself a note for {}", app.name);
        let entry = gtk::Entry::builder()
            .placeholder_text("e.g. next: rewrite the intro")
            .max_length(NOTE_MAX as i32)
            .width_chars(34)
            .build();
        entry.update_property(&[gtk::accessible::Property::Label(&hint)]);
        let note_page = gtk::Box::new(gtk::Orientation::Vertical, 6);
        note_page.append(
            &gtk::Label::builder()
                .label(format!("A note to yourself, under {}", app.name))
                .xalign(0.0)
                .css_classes(["caption", "dim-label"])
                .build(),
        );
        note_page.append(&entry);
        note_page.append(
            &gtk::Label::builder()
                .label("Enter keeps it. An empty line removes it.")
                .xalign(0.0)
                .css_classes(["caption", "dim-label"])
                .build(),
        );

        // No sliding between the two pages: the box simply becomes the other one.
        let pages = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::None)
            .hhomogeneous(false)
            .vhomogeneous(false)
            .build();
        pages.add_named(&items, Some("menu"));
        pages.add_named(&note_page, Some("note"));
        let popover = gtk::Popover::builder().child(&pages).build();
        popover.add_css_class("porog-row-menu");

        let label = format!("More for {}: note, What's new", app.name);
        let button = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .valign(gtk::Align::Center)
            .popover(&popover)
            .tooltip_text(format!("{label} (N, W)"))
            .css_classes(["flat", "porog-more"])
            .build();
        button.update_property(&[gtk::accessible::Property::Label(&label)]);

        let (note_line, note_text) = indented_line(name, &["fond-row-detail", "porog-note"]);
        let menu = Self {
            button,
            popover: popover.clone(),
            pages: pages.clone(),
            entry: entry.clone(),
            plain: plain.clone(),
            news: news.clone(),
            note: note.clone(),
            clear: clear.clone(),
            note_line,
            note_text,
        };
        menu.show_note(&config.borrow(), app);

        // Each opening starts on the menu, unless it was opened to write the note.
        popover.connect_closed({
            let pages = pages.clone();
            move |_| pages.set_visible_child_name("menu")
        });
        popover.connect_show({
            let pages = pages.clone();
            let entry = entry.clone();
            move |_| {
                if pages.visible_child_name().is_some_and(|p| p == "note") {
                    entry.grab_focus();
                }
            }
        });
        plain.connect_clicked({
            let popover = popover.clone();
            move |plain| {
                popover.popdown();
                let _ = plain.activate_action("win.open-plain", Some(&(index as i32).to_variant()));
            }
        });
        note.connect_clicked({
            let menu = menu.clone();
            let config = config.clone();
            move |_| menu.show_note_page(&config.borrow(), app)
        });
        news.connect_clicked({
            let popover = popover.clone();
            let button = menu.button.clone();
            move |_| {
                popover.popdown();
                show_news(&button, app);
            }
        });
        let save = {
            let menu = menu.clone();
            let config = config.clone();
            let config_path = config_path.to_path_buf();
            let toasts = toasts.clone();
            move |text: &str| {
                config.borrow_mut().set_note(app, text);
                menu.show_note(&config.borrow(), app);
                menu.popover.popdown();
                menu::save_config(
                    &config,
                    &config_path,
                    &toasts,
                    "The note could not be saved. It lasts until Porog closes.",
                );
            }
        };
        entry.connect_activate({
            let save = save.clone();
            move |entry| save(&entry.text())
        });
        clear.connect_clicked(move |_| save(""));
        menu
    }

    /// Open the menu straight on the note box, for the N key.
    fn edit_note(&self) {
        self.pages.set_visible_child_name("note");
        self.button.popup();
    }

    fn show_note_page(&self, config: &Config, app: &App) {
        self.entry.set_text(config.note(app));
        self.pages.set_visible_child_name("note");
        self.entry.grab_focus();
    }

    /// The note under the row, and the menu's words for it.
    fn show_note(&self, config: &Config, app: &App) {
        let note = note_display(config, app);
        self.note_text.set_label(&note);
        self.note_text.set_tooltip_text(Some(&note));
        self.note_line.set_visible(!note.is_empty());
        self.clear.set_visible(!note.is_empty());
        if let Some(label) = menu::menu_item_label(&self.note) {
            label.set_label(if note.is_empty() {
                "Leave a note…"
            } else {
                "Change the note…"
            });
        }
        if !self.pages.visible_child_name().is_some_and(|p| p == "note") {
            self.entry.set_text(config.note(app));
        }
    }
}
