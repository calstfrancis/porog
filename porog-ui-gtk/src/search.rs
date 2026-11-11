//! Ctrl+K: one box that asks every app that can answer, results grouped by app in the fixed
//! app order. Enter opens the highlighted result in its app. The window keeps one size, and
//! the line under the box is always there, so nothing moves while typing.

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::search::{self, Answer, Group, Hit};
use porog_core::App;

use crate::window::launch;

/// How long typing must pause before the apps are asked: each question starts a process.
const PAUSE_MS: u32 = 450;

pub fn show(parent: &adw::ApplicationWindow, toasts: &adw::ToastOverlay) {
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
        .label("Type two or more letters, then pause.")
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

    entry.connect_search_changed({
        let list = list.clone();
        let line = line.clone();
        let latest = latest.clone();
        let pending = pending.clone();
        move |entry| {
            if let Some(id) = pending.take() {
                id.remove();
            }
            let query = entry.text().to_string();
            let number = latest.get() + 1;
            latest.set(number);
            clear(&list);
            if !search::is_askable(&query) {
                line.set_label("Type two or more letters, then pause.");
                return;
            }
            line.set_label("Searching…");
            let id =
                glib::timeout_add_local_once(std::time::Duration::from_millis(PAUSE_MS.into()), {
                    let list = list.clone();
                    let line = line.clone();
                    let latest = latest.clone();
                    let pending = pending.clone();
                    move || {
                        pending.set(None);
                        glib::spawn_future_local(async move {
                            let groups = gio::spawn_blocking(move || search::search(&query))
                                .await
                                .unwrap_or_default();
                            if latest.get() == number {
                                fill(&list, &line, &groups);
                            }
                        });
                    }
                });
            pending.set(Some(id));
        }
    });

    let open = {
        let window = window.clone();
        let toasts = toasts.clone();
        move |row: &gtk::ListBoxRow| {
            let Some(target) = row_target(row) else {
                return;
            };
            launch(target.0, target.1, &toasts);
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

    window.present();
    entry.grab_focus();
}

fn clear(list: &gtk::ListBox) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }
}

/// What a result row opens: stored on the row so activation needs no lookup table.
fn row_target(row: &gtk::ListBoxRow) -> Option<(&'static App, Vec<String>)> {
    // SAFETY: only `append_hit` sets this key, always to a `(&'static App, Vec<String>)`.
    unsafe {
        row.data::<(&'static App, Vec<String>)>("porog-target")
            .map(|target| target.as_ref().clone())
    }
}

fn first_hit(list: &gtk::ListBox) -> Option<gtk::ListBoxRow> {
    (0..)
        .map_while(|i| list.row_at_index(i))
        .find(|row| row.is_activatable())
}

fn fill(list: &gtk::ListBox, line: &gtk::Label, groups: &[Group]) {
    clear(list);
    let mut found = 0;
    for group in groups {
        append_heading(list, group.app.name);
        match &group.answer {
            Answer::Hits(hits) if hits.is_empty() => append_note(list, "Nothing found."),
            Answer::Hits(hits) => {
                found += hits.len();
                hits.iter().for_each(|hit| append_hit(list, group.app, hit));
            }
            Answer::Message(message) => append_note(list, message),
        }
    }
    line.set_label(&match found {
        0 => "Nothing found.".to_string(),
        1 => "1 result. Enter opens it.".to_string(),
        n => format!("{n} results. Enter opens the first; Down moves through them."),
    });
}

fn append_heading(list: &gtk::ListBox, name: &str) {
    let label = gtk::Label::builder()
        .label(name)
        .xalign(0.0)
        .margin_start(12)
        .margin_top(8)
        .margin_bottom(2)
        .css_classes(["heading"])
        .build();
    let row = gtk::ListBoxRow::builder()
        .child(&label)
        .activatable(false)
        .selectable(false)
        .build();
    list.append(&row);
}

fn append_note(list: &gtk::ListBox, text: &str) {
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(6)
        .css_classes(["fond-row-detail"])
        .build();
    let row = gtk::ListBoxRow::builder()
        .child(&label)
        .activatable(false)
        .selectable(false)
        .build();
    list.append(&row);
}

fn append_hit(list: &gtk::ListBox, app: &'static App, hit: &Hit) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(4);
    content.set_margin_bottom(4);
    content.append(
        &gtk::Label::builder()
            .label(&hit.title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(["fond-row-title"])
            .build(),
    );
    if !hit.detail.is_empty() {
        content.append(
            &gtk::Label::builder()
                .label(&hit.detail)
                .xalign(0.0)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .css_classes(["fond-row-detail"])
                .build(),
        );
    }
    let row = gtk::ListBoxRow::builder().child(&content).build();
    // SAFETY: read back only by `row_target`, with the same type.
    unsafe { row.set_data("porog-target", (app, hit.args.clone())) };
    list.append(&row);
}
