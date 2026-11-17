//! Ctrl+K: one box that asks every app that can answer, results grouped by app in the fixed
//! app order. Enter opens the highlighted result in its app. The window keeps one size, and
//! the line under the box is always there, so nothing moves while typing.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::search::{self, Answer, Hit};
use porog_core::{App, APPS};

use crate::window::launch;

/// How long typing must pause before the apps are asked: each question starts a process.
const PAUSE_MS: u32 = 450;

thread_local! {
    /// The last words typed, put back (and selected) when the box is opened again.
    static LAST_QUERY: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Where a result row goes: the row, the app, and the arguments that open it there.
type Target = (gtk::ListBoxRow, &'static App, Vec<String>);

/// The rows of each app's group in the list, in app order, so one group can be replaced when its
/// app answers without moving the others.
type Groups = Rc<RefCell<Vec<Vec<gtk::ListBoxRow>>>>;

pub fn show(parent: &adw::ApplicationWindow, toasts: &adw::ToastOverlay, query: &str) {
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search the suite")
        .hexpand(true)
        .build();
    let header = adw::HeaderBar::builder()
        .title_widget(&entry)
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();

    let line = gtk::Label::builder()
        .xalign(0.0)
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(6)
        .css_classes(["caption", "fond-quiet"])
        .label(HINT)
        .build();
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["fond-list"])
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.append(&line);
    body.append(&scroll);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&body));
    let window = adw::Window::builder()
        .title("Search")
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(420)
        .content(&view)
        .build();

    // Each question gets a number; an answer that is no longer the latest is dropped.
    let latest = Rc::new(Cell::new(0u32));
    let pending: Rc<Cell<Option<glib::SourceId>>> = Rc::new(Cell::new(None));
    // Set when the question it belongs to is replaced or the window closes: the helpers
    // still running for it are then killed instead of left to finish.
    let cancel: Rc<RefCell<Arc<AtomicBool>>> = Rc::default();
    let targets: Rc<RefCell<Vec<Target>>> = Rc::default();
    let groups: Groups = Rc::default();

    entry.connect_search_changed({
        let list = list.clone();
        let line = line.clone();
        let latest = latest.clone();
        let pending = pending.clone();
        let cancel = cancel.clone();
        let targets = targets.clone();
        let groups = groups.clone();
        move |entry| {
            if let Some(id) = pending.take() {
                id.remove();
            }
            cancel.borrow().store(true, Ordering::Relaxed);
            let token = Arc::new(AtomicBool::new(false));
            *cancel.borrow_mut() = token.clone();
            let query = entry.text().to_string();
            LAST_QUERY.with(|last| *last.borrow_mut() = query.clone());
            let number = latest.get() + 1;
            latest.set(number);
            clear(&list, &targets, &groups);
            if !search::is_askable(&query) {
                line.set_label(HINT);
                return;
            }
            line.set_label("Searching…");
            let id =
                glib::timeout_add_local_once(std::time::Duration::from_millis(PAUSE_MS.into()), {
                    let list = list.clone();
                    let line = line.clone();
                    let latest = latest.clone();
                    let pending = pending.clone();
                    let targets = targets.clone();
                    let groups = groups.clone();
                    move || {
                        pending.set(None);
                        ask_all(
                            &query, number, &token, &latest, &list, &line, &targets, &groups,
                        );
                    }
                });
            pending.set(Some(id));
        }
    });

    let open = {
        let window = window.clone();
        let toasts = toasts.clone();
        let targets = targets.clone();
        move |row: &gtk::ListBoxRow| {
            let target = targets
                .borrow()
                .iter()
                .find(|(r, _, _)| r == row)
                .map(|(_, app, args)| (*app, args.clone()));
            let Some((app, args)) = target else {
                return;
            };
            launch(app, args, &toasts);
            window.close();
        }
    };
    list.connect_row_activated({
        let open = open.clone();
        move |_, row| open(row)
    });
    // Enter in the box opens the first result; Down moves into the list.
    entry.connect_activate({
        let list = list.clone();
        let open = open.clone();
        move |_| {
            if let Some(row) = first_hit(&list) {
                open(&row);
            }
        }
    });
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed({
        let list = list.clone();
        move |_, key, _, _| {
            if key == gtk::gdk::Key::Down {
                if let Some(row) = first_hit(&list) {
                    list.select_row(Some(&row));
                    row.grab_focus();
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        }
    });
    entry.add_controller(keys);
    entry.connect_stop_search({
        let window = window.clone();
        move |_| window.close()
    });
    let escape = gtk::EventControllerKey::new();
    escape.connect_key_pressed({
        let window = window.clone();
        move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                window.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    window.add_controller(escape);
    window.connect_close_request({
        let cancel = cancel.clone();
        move |_| {
            cancel.borrow().store(true, Ordering::Relaxed);
            glib::Propagation::Proceed
        }
    });

    window.present();
    entry.grab_focus();
    // Words given up front (`porog --search=WORDS`) are asked at once; otherwise the last
    // words typed come back, selected, so typing replaces them and Enter asks again.
    let given = query.trim();
    if !given.is_empty() {
        entry.set_text(given);
        entry.set_position(-1);
    } else {
        let last = LAST_QUERY.with(|last| last.borrow().clone());
        if !last.is_empty() {
            entry.set_text(&last);
            entry.select_region(0, -1);
        }
    }
}

const HINT: &str = "Type two or more letters, then pause.";

/// Ask every app, showing each group as soon as its app answers. Until then a group says
/// "Asking…", so the groups are all there, in order, from the first moment.
#[allow(clippy::too_many_arguments)]
fn ask_all(
    query: &str,
    number: u32,
    token: &Arc<AtomicBool>,
    latest: &Rc<Cell<u32>>,
    list: &gtk::ListBox,
    line: &gtk::Label,
    targets: &Rc<RefCell<Vec<Target>>>,
    groups: &Groups,
) {
    for app in APPS.iter() {
        let rows = vec![heading_row(app.name), note_row("Asking…")];
        rows.iter().for_each(|row| list.append(row));
        groups.borrow_mut().push(rows);
    }
    let waiting = Rc::new(Cell::new(APPS.len()));
    let found = Rc::new(Cell::new(0usize));
    for index in 0..APPS.len() {
        let query = query.to_string();
        let token = token.clone();
        let latest = latest.clone();
        let (list, line) = (list.clone(), line.clone());
        let (targets, groups) = (targets.clone(), groups.clone());
        let (waiting, found) = (waiting.clone(), found.clone());
        glib::spawn_future_local(async move {
            let asked = query.clone();
            let group = gio::spawn_blocking(move || search::ask_app(index, &asked, &token))
                .await
                .ok();
            if latest.get() != number {
                return;
            }
            if let Some(group) = group {
                found.set(
                    found.get() + replace_group(index, &group, &query, &list, &targets, &groups),
                );
            }
            waiting.set(waiting.get() - 1);
            line.set_label(&summary(found.get(), waiting.get()));
        });
    }
}

fn summary(found: usize, waiting: usize) -> String {
    let still = if waiting > 0 { " Still asking…" } else { "" };
    match found {
        0 if waiting > 0 => "Searching…".to_string(),
        0 => "Nothing found.".to_string(),
        1 => format!("1 result. Enter opens it.{still}"),
        n => format!("{n} results. Enter opens the first; Down moves through them.{still}"),
    }
}

fn clear(list: &gtk::ListBox, targets: &Rc<RefCell<Vec<Target>>>, groups: &Groups) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }
    targets.borrow_mut().clear();
    groups.borrow_mut().clear();
}

fn first_hit(list: &gtk::ListBox) -> Option<gtk::ListBoxRow> {
    (0..)
        .map_while(|i| list.row_at_index(i))
        .find(|row| row.is_activatable())
}

/// Swap one group's rows for its app's answer, in place. Returns how many results it has.
fn replace_group(
    index: usize,
    group: &search::Group,
    query: &str,
    list: &gtk::ListBox,
    targets: &Rc<RefCell<Vec<Target>>>,
    groups: &Groups,
) -> usize {
    let mut groups = groups.borrow_mut();
    let Some(old) = groups.get(index).cloned() else {
        return 0;
    };
    let position: i32 = groups[..index].iter().map(|g| g.len() as i32).sum();
    for row in &old {
        list.remove(row);
    }
    let mut rows = vec![heading_row(group.app.name)];
    let mut count = 0;
    match &group.answer {
        Answer::Hits(hits) if hits.is_empty() => rows.push(note_row("Nothing found.")),
        Answer::Hits(hits) => {
            count = hits.len();
            for hit in hits {
                let row = hit_row(hit, query);
                targets
                    .borrow_mut()
                    .push((row.clone(), group.app, hit.args.clone()));
                rows.push(row);
            }
        }
        Answer::Message(message) => rows.push(note_row(message)),
    }
    for (offset, row) in rows.iter().enumerate() {
        list.insert(row, position + offset as i32);
    }
    groups[index] = rows;
    count
}

fn heading_row(name: &str) -> gtk::ListBoxRow {
    let label = gtk::Label::builder()
        .label(name)
        .xalign(0.0)
        .margin_start(12)
        .margin_top(8)
        .margin_bottom(2)
        .css_classes(["heading"])
        .build();
    gtk::ListBoxRow::builder()
        .child(&label)
        .activatable(false)
        .selectable(false)
        .build()
}

fn note_row(text: &str) -> gtk::ListBoxRow {
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(6)
        .css_classes(["fond-row-detail"])
        .build();
    gtk::ListBoxRow::builder()
        .child(&label)
        .activatable(false)
        .selectable(false)
        .build()
}

/// A result: its title, and its detail, with the words searched for in bold.
fn hit_row(hit: &Hit, query: &str) -> gtk::ListBoxRow {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(4);
    content.set_margin_bottom(4);
    content.append(
        &gtk::Label::builder()
            .use_markup(true)
            .label(search::highlight_markup(&hit.title, query))
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["fond-row-title"])
            .build(),
    );
    if !hit.detail.is_empty() {
        content.append(
            &gtk::Label::builder()
                .use_markup(true)
                .label(search::highlight_markup(&hit.detail, query))
                .xalign(0.0)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .css_classes(["fond-row-detail"])
                .build(),
        );
    }
    let row = gtk::ListBoxRow::builder().child(&content).build();
    let spoken = if hit.detail.is_empty() {
        hit.title.clone()
    } else {
        format!("{}. {}", hit.title, hit.detail)
    };
    row.update_property(&[gtk::accessible::Property::Label(&spoken)]);
    row
}
