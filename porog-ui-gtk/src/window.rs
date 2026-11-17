//! The main window: the five fixed app rows and the status bar.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::config::{Config, NOTE_MAX};
use porog_core::now::{self, LaterItem, NowView};
use porog_core::status::{self, ResumeEntry, RowState};
use porog_core::{App, APPS};

use crate::{changelog, menu, search, welcome, VERSION};

/// The parts of an app's row that change after it is built.
#[derive(Clone)]
struct AppRow {
    app: &'static App,
    index: usize,
    /// The app's name. Always opens the app plainly, whatever the button says.
    name: gtk::Button,
    detail: gtk::Label,
    /// "Resume" when the row shows something that can be reopened, otherwise "Open".
    primary: gtk::Button,
    /// The second and third resume entries, shown only with `details` on.
    extras: gtk::Box,
    /// The note the user left for this app, under the first entry; hidden when there is none.
    note_line: gtk::Box,
    note_text: gtk::Label,
    state: Rc<RefCell<RowState>>,
    details: Rc<Cell<bool>>,
    toasts: adw::ToastOverlay,
}

impl AppRow {
    fn new(
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

        let (edit, note_line, note_text) = note_widgets(app, &name, config, config_path, toasts);

        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.append(&name);
        line.append(&detail);
        line.append(&edit);
        line.append(&primary);
        let extras = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .visible(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
        content.append(&line);
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
            detail,
            primary,
            extras,
            note_line,
            note_text,
            state: Rc::default(),
            details,
            toasts: toasts.clone(),
        };
        row.show_note(config.borrow().note(app));
        row.show(RowState {
            installed: true,
            ..RowState::default()
        });
        (widget, row)
    }

    /// Re-read the row's state off the main thread, then show it.
    fn refresh(&self) {
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
        self.primary.set_label(label);
        self.primary
            .set_tooltip_text(Some(&format!("{hint} ({shortcut})")));
        self.primary
            .update_property(&[gtk::accessible::Property::Label(&hint)]);
        self.primary.set_sensitive(state.installed);
        self.name.set_sensitive(state.installed);

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
            move |_| launch(app, args.clone(), &toasts)
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
        line
    }

    /// Put the user's own note for this app under its first entry, or hide the line.
    fn show_note(&self, note: &str) {
        self.note_text.set_label(note);
        self.note_text.set_tooltip_text(Some(note));
        self.note_line.set_visible(!note.is_empty());
    }

    /// Show or hide the older entries to match the `details` toggle.
    fn show_details(&self) {
        self.extras
            .set_visible(self.details.get() && self.extras.first_child().is_some());
    }

    /// What the row's button does right now: Resume the first entry, or open the app.
    fn activate(&self) {
        let state = self.state.borrow();
        if !state.installed {
            self.say_not_installed();
            return;
        }
        let args = state.resume.first().map(|entry| entry.args.clone());
        launch(self.app, args.unwrap_or_default(), &self.toasts);
    }

    /// Open the app with nothing to resume.
    fn open_plain(&self) {
        if self.state.borrow().installed {
            launch(self.app, Vec::new(), &self.toasts);
        } else {
            self.say_not_installed();
        }
    }

    fn say_not_installed(&self) {
        self.toasts.add_toast(adw::Toast::new(&format!(
            "{} is not installed.",
            self.app.name
        )));
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
            let note = config.borrow().note(app).to_string();
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

/// The Now area above the rows: Sputnik's Now card in Sputnik's words (`PLAN.md` P2). It
/// keeps its height whatever it shows, so the rows below never move (P1).
#[derive(Clone)]
struct NowArea {
    title: gtk::Label,
    detail: gtk::Label,
    footer: gtk::Label,
    action: gtk::Button,
    /// What the action button starts; set with each answer from Sputnik.
    target: ActionTarget,
    /// What is due next, in Sputnik's words; shown only with `details` on.
    later: gtk::Box,
    details: Rc<Cell<bool>>,
}

type ActionTarget = Rc<RefCell<Option<(&'static App, Vec<String>)>>>;

impl NowArea {
    fn new(toasts: &adw::ToastOverlay, details: Rc<Cell<bool>>) -> (gtk::Box, Self) {
        let eyebrow = gtk::Label::builder()
            .label("NOW  (from Sputnik)")
            .xalign(0.0)
            .css_classes(["caption-heading", "fond-quiet"])
            .build();
        let title = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["title-4"])
            .build();
        let detail = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["fond-row-detail"])
            .build();
        let footer = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["fond-row-detail"])
            .build();
        let text = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let later = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .visible(false)
            .build();
        for label in [&eyebrow, &title, &detail, &footer] {
            text.append(label);
        }
        text.append(&later);
        let open = gtk::Button::builder()
            .label("Open Sputnik")
            .valign(gtk::Align::Center)
            .tooltip_text("Open Sputnik")
            .action_name("win.open-plain")
            .action_target(&0i32.to_variant())
            .build();
        let action = gtk::Button::builder()
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        let target = ActionTarget::default();
        action.connect_clicked({
            let target = target.clone();
            let toasts = toasts.clone();
            move |_| {
                if let Some((app, args)) = target.borrow().clone() {
                    launch(app, args, &toasts);
                }
            }
        });
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.append(&text);
        line.append(&action);
        line.append(&open);
        let card = gtk::Box::builder()
            .height_request(104)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        card.append(&line);
        line.set_hexpand(true);
        line.set_margin_start(12);
        line.set_margin_end(12);
        line.set_margin_top(8);
        line.set_margin_bottom(8);
        card.add_css_class("card");
        let area = Self {
            title,
            detail,
            footer,
            action,
            target,
            later,
            details,
        };
        area.show(&NowView::Message("Looking…".to_string()));
        (card, area)
    }

    fn show(&self, view: &NowView) {
        let (title, detail, footer, action, later) = match view {
            NowView::Card {
                title,
                detail,
                footer,
                action,
                later,
            } => (
                title.as_str(),
                detail.as_str(),
                footer.as_deref(),
                action.as_ref(),
                later.as_slice(),
            ),
            NowView::Message(message) => (message.as_str(), "", None, None, &[][..]),
        };
        self.show_later(later);
        self.action.set_visible(action.is_some());
        if let Some(action) = action {
            self.action.set_label(&action.label);
            self.action
                .set_tooltip_text(Some(&format!("{} in {}", action.label, action.app.name)));
        }
        *self.target.borrow_mut() = action.map(|a| (a.app, a.args.clone()));
        self.title.set_label(title);
        self.detail.set_label(detail);
        self.detail.set_visible(!detail.is_empty());
        self.footer.set_label(footer.unwrap_or(""));
        self.footer.set_visible(footer.is_some());
    }

    /// Replace the "what is due next" lines, one label each.
    fn show_later(&self, later: &[LaterItem]) {
        while let Some(child) = self.later.first_child() {
            self.later.remove(&child);
        }
        for item in later {
            let text = if item.detail.is_empty() {
                item.title.clone()
            } else {
                format!("{} · {}", item.title, item.detail)
            };
            self.later.append(
                &gtk::Label::builder()
                    .label(&text)
                    .tooltip_text(&text)
                    .xalign(0.0)
                    .wrap(true)
                    .css_classes(["fond-row-detail"])
                    .build(),
            );
        }
        self.show_details();
    }

    /// Show or hide the "due next" lines to match the `details` toggle.
    fn show_details(&self) {
        self.later
            .set_visible(self.details.get() && self.later.first_child().is_some());
    }

    /// Ask Sputnik again, off the main thread.
    fn refresh(&self) {
        let area = self.clone();
        glib::spawn_future_local(async move {
            let view = gio::spawn_blocking(now::fetch)
                .await
                .unwrap_or_else(|_| NowView::Message(now::NO_ANSWER.to_string()));
            area.show(&view);
        });
    }
}

/// Start `app` off the main thread, and say so plainly if it does not start.
pub fn launch(app: &'static App, args: Vec<String>, toasts: &adw::ToastOverlay) {
    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let started = gio::spawn_blocking(move || porog_core::launch(app, &args)).await;
        if !matches!(started, Ok(Ok(()))) {
            toasts.add_toast(adw::Toast::new(&format!("{} did not start.", app.name)));
        }
    });
}

/// A file dropped on the window goes to the app that opens it, or Porog says it has none.
/// One file at a time: several would mean several windows from one gesture.
fn open_dropped(files: &[gio::File], toasts: &adw::ToastOverlay) {
    let [file] = files else {
        toasts.add_toast(adw::Toast::new("Drop one file at a time."));
        return;
    };
    let Some(path) = file.path() else {
        toasts.add_toast(adw::Toast::new(
            "Porog can only open files on this computer.",
        ));
        return;
    };
    match porog_core::app_for_file(&path) {
        Some(app) => launch(app, vec![path.to_string_lossy().into_owned()], toasts),
        None => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            toasts.add_toast(adw::Toast::new(&format!(
                "Porog cannot open {name}. It handles PDF, EPUB and .typ files."
            )));
        }
    }
}

/// Mark a status-bar word toggle on or off: weight for the eye, pressed state for a
/// screen reader.
fn set_toggle(button: &gtk::Button, on: bool) {
    if on {
        button.add_css_class("fond-toggle-active");
    } else {
        button.remove_css_class("fond-toggle-active");
    }
    button.update_state(&[gtk::accessible::State::Pressed(if on {
        gtk::AccessibleTristate::True
    } else {
        gtk::AccessibleTristate::False
    })]);
}

pub fn build(app: &adw::Application) {
    let config_path = glib::user_config_dir().join("porog").join("config.json");
    let config = Rc::new(RefCell::new(Config::load(&config_path)));
    menu::apply_theme(config.borrow().theme);
    let details = Rc::new(Cell::new(config.borrow().details));

    let toasts = adw::ToastOverlay::new();

    // Enter (or a double click) on a row does what its button says; a single click acts
    // only on a button.
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .activate_on_single_click(false)
        .vexpand(true)
        .css_classes(["fond-list", "porog-rows"])
        .build();
    let rows: Rc<Vec<AppRow>> = Rc::new(
        APPS.iter()
            .enumerate()
            .map(|(index, app)| {
                let (widget, row) =
                    AppRow::new(app, index, details.clone(), &toasts, &config, &config_path);
                list.append(&widget);
                row
            })
            .collect(),
    );

    let (now_card, now_area) = NowArea::new(&toasts, details.clone());
    let details_toggle = gtk::Button::builder()
        .label("details")
        .tooltip_text(
            "Show the last three things each app can resume, and what Sputnik says is due next",
        )
        .css_classes(["flat", "caption", "fond-quiet"])
        .build();
    set_toggle(&details_toggle, details.get());
    details_toggle.connect_clicked({
        let rows = rows.clone();
        let now_area = now_area.clone();
        let details = details.clone();
        let config = config.clone();
        let config_path = config_path.clone();
        let toasts = toasts.clone();
        move |toggle| {
            let on = !details.get();
            details.set(on);
            set_toggle(toggle, on);
            rows.iter().for_each(AppRow::show_details);
            now_area.show_details();
            config.borrow_mut().details = on;
            menu::save_config(
                &config,
                &config_path,
                &toasts,
                "The details choice could not be saved. It lasts until Porog closes.",
            );
        }
    });
    let version = gtk::Button::builder()
        .label(format!("v{VERSION}"))
        .tooltip_text("View changelog")
        .action_name("win.changelog")
        .halign(gtk::Align::End)
        .hexpand(true)
        .css_classes(["flat", "caption", "fond-quiet"])
        .build();
    let status = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status.add_css_class("fond-statusbar");
    status.add_css_class("fond-edge-top");
    status.append(&details_toggle);
    status.append(&version);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.append(&now_card);
    body.append(&scroll);
    body.append(&status);
    toasts.set_child(Some(&body));

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Menu")
        .build();
    let header = adw::HeaderBar::new();
    header.pack_end(&menu_button);
    header.pack_start(
        &gtk::Button::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search the suite (Ctrl+K)")
            .action_name("win.search")
            .build(),
    );

    let drop = gtk::DropTarget::new(
        gtk::gdk::FileList::static_type(),
        gtk::gdk::DragAction::COPY,
    );
    drop.connect_drop({
        let toasts = toasts.clone();
        move |_, value, _, _| match value.get::<gtk::gdk::FileList>() {
            Ok(list) => {
                open_dropped(&list.files(), &toasts);
                true
            }
            Err(_) => false,
        }
    });
    toasts.add_controller(drop);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&toasts));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Porog")
        .default_width(560)
        .default_height(560)
        .content(&view)
        .build();
    menu_button.set_popover(Some(&menu::build(
        &window,
        &toasts,
        config.clone(),
        config_path.clone(),
    )));

    // One action does what a row's button says, by row index: the buttons, Enter on a row
    // and Alt+1…5 all go through it, so they cannot behave differently.
    let row_action = |name: &str, act: fn(&AppRow)| {
        let action = gio::SimpleAction::new(name, Some(glib::VariantTy::INT32));
        action.connect_activate({
            let rows = rows.clone();
            move |_, index| {
                let row = index
                    .and_then(|index| index.get::<i32>())
                    .and_then(|index| rows.get(usize::try_from(index).ok()?));
                if let Some(row) = row {
                    act(row);
                }
            }
        });
        window.add_action(&action);
        action
    };
    let open_app = row_action("open-app", AppRow::activate);
    row_action("open-plain", AppRow::open_plain);
    list.connect_row_activated(move |_, row| open_app.activate(Some(&row.index().to_variant())));

    let show_changelog = gio::SimpleAction::new("changelog", None);
    show_changelog.connect_activate({
        let window = window.clone();
        move |_, _| changelog::show_changelog(&window)
    });
    window.add_action(&show_changelog);
    let search = gio::SimpleAction::new("search", None);
    search.connect_activate({
        let window = window.clone();
        let toasts = toasts.clone();
        move |_, _| search::show(&window, &toasts)
    });
    window.add_action(&search);
    app.set_accels_for_action("win.search", &["<Control>k"]);
    let whats_new = gio::SimpleAction::new("whats-new", None);
    whats_new.connect_activate({
        let window = window.clone();
        move |_, _| welcome::show(&window, false)
    });
    window.add_action(&whats_new);
    let show_about = gio::SimpleAction::new("about", None);
    show_about.connect_activate({
        let window = window.clone();
        move |_, _| menu::show_about(&window)
    });
    window.add_action(&show_about);

    // Rows are refreshed when the window gains focus, never while it is being looked at.
    window.connect_is_active_notify(move |window| {
        if window.is_active() {
            now_area.refresh();
            rows.iter().for_each(AppRow::refresh);
        }
    });
    // Start on the first row, so Enter and the arrow keys work at once.
    if let Some(first) = list.row_at_index(0) {
        GtkWindowExt::set_focus(&window, Some(&first));
    }
    window.present();
    welcome::maybe_show(&window, &config, &config_path, &toasts);
}
