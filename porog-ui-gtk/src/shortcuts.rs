//! The keyboard shortcuts window (Ctrl+?): every key Porog answers to, in plain words.

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;

use crate::welcome::section;

const KEYS: &[(&str, &str)] = &[
    ("Up / Down", "Move between the app rows"),
    ("Enter", "Do what the focused row's button says (Resume, Open or Install…)"),
    ("Alt+1 … Alt+5", "The same for Sputnik, Pereplyot, Kartoteka, Zerkalo, Skrizhal"),
    ("N", "Leave yourself a note on the focused row"),
    ("W", "What's new in the focused row's app"),
    ("Ctrl+K", "Search the suite"),
    ("Ctrl+?", "This list"),
    ("Esc", "Close a window or the search"),
];

pub fn show(parent: &adw::ApplicationWindow) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_margin_start(24);
    body.set_margin_end(24);
    body.set_margin_top(16);
    body.set_margin_bottom(20);
    body.append(&section("Keyboard shortcuts"));
    let grid = gtk::Grid::builder()
        .column_spacing(18)
        .row_spacing(8)
        .build();
    for (row, (keys, what)) in KEYS.iter().enumerate() {
        grid.attach(
            &gtk::Label::builder()
                .label(*keys)
                .xalign(0.0)
                .valign(gtk::Align::Start)
                .css_classes(["fond-row-title"])
                .build(),
            0,
            row as i32,
            1,
            1,
        );
        grid.attach(
            &gtk::Label::builder()
                .label(*what)
                .xalign(0.0)
                .wrap(true)
                .hexpand(true)
                .css_classes(["fond-row-detail"])
                .build(),
            1,
            row as i32,
            1,
            1,
        );
    }
    body.append(&grid);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(520);
    clamp.set_child(Some(&body));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&clamp));
    let window = adw::Window::builder()
        .title("Keyboard shortcuts")
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .content(&toolbar)
        .build();
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
}
