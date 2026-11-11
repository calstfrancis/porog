//! Welcome (first run) and What's New (after an update). `RELEASE_NAME` is the release's name
//! in `CHANGELOG.md`, the metainfo and the commit — updated at release time with the bullet list.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::config::Config;

use crate::VERSION;

pub(crate) const RELEASE_NAME: &str = "Open Door";

/// Show the window if this version has not been seen: the welcome the first time ever,
/// What's New after an update.
pub fn maybe_show(
    window: &adw::ApplicationWindow,
    config: &Rc<RefCell<Config>>,
    config_path: &Path,
    toasts: &adw::ToastOverlay,
) {
    if std::env::var_os("POROG_NO_WELCOME").is_some() {
        return;
    }
    let seen = config.borrow().last_seen_version.clone();
    if seen == VERSION {
        return;
    }
    config.borrow_mut().last_seen_version = VERSION.to_string();
    crate::menu::save_config(
        config,
        config_path,
        toasts,
        "Porog could not remember that you have seen this.",
    );
    show(window, seen.is_empty());
}

pub fn show(parent: &adw::ApplicationWindow, first_run: bool) {
    let window = adw::Window::builder()
        .title(if first_run {
            "Welcome to Porog"
        } else {
            "What's New"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .default_height(520)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_start(24);
    body.set_margin_end(24);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(450);
    clamp.set_child(Some(&body));
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_child(Some(&clamp));

    let title = gtk::Label::new(Some("Porog"));
    title.add_css_class("title-1");
    let sub = gtk::Label::new(Some(&format!("Version {VERSION} “{RELEASE_NAME}”")));
    sub.add_css_class("dim-label");
    body.append(&title);
    body.append(&sub);
    body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    if first_run {
        body.append(&section("What Porog is"));
        let about = gtk::Label::new(Some(
            "The way back into the Fond suite. One window with a fixed row for each app, \
             in the same order every time, showing where you were and a button to go back there.",
        ));
        style(&about);
        body.append(&about);
        body.append(&section("Try this"));
        for (headline, detail) in TRY {
            body.append(&bullet(headline, detail));
        }
    } else {
        body.append(&section("What's new"));
        for (headline, detail) in NEW {
            body.append(&bullet(headline, detail));
        }
    }
    let close = gtk::Button::with_label("Get started");
    close.add_css_class("suggested-action");
    close.set_halign(gtk::Align::Center);
    close.set_margin_top(12);
    body.append(&close);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroll));
    window.set_content(Some(&toolbar));
    close.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });
    window.present();
}

/// Updated at release time, with `RELEASE_NAME`.
const NEW: &[(&str, &str)] = &[
    (
        "A fixed board for the suite",
        "Sputnik, Pereplyot, Kartoteka, Zerkalo and Skrizhal each keep one row, in the same place, whatever you used last.",
    ),
    (
        "Resume",
        "Each row says what you were last doing there, and its button reopens exactly that when it can. The details word shows the last three.",
    ),
    (
        "Today, from Sputnik",
        "The top of the window shows Sputnik's Now card in Sputnik's own words, refreshed when you come back to the window.",
    ),
    (
        "Search the suite",
        "Ctrl+K searches your Kartoteka library; Enter opens the result there. The other apps will follow.",
    ),
];

const TRY: &[(&str, &str)] = &[
    (
        "Press Enter, or Alt+1 to 5",
        "Does what the row's button says: Resume or Open.",
    ),
    (
        "Click an app's name",
        "Opens the app plainly, without resuming anything.",
    ),
    ("Press Ctrl+K", "Search your Kartoteka library from here."),
];

fn section(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class("heading");
    l.set_xalign(0.0);
    l.set_margin_top(8);
    l
}

fn style(l: &gtk::Label) {
    l.set_wrap(true);
    l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    l.set_xalign(0.0);
    l.set_hexpand(true);
}

fn bullet(headline: &str, detail: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let dot = gtk::Label::new(Some("•"));
    dot.add_css_class("dim-label");
    dot.set_valign(gtk::Align::Start);
    row.append(&dot);
    let col = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let h = gtk::Label::new(None);
    h.set_markup(&format!(
        "<b>{}</b>",
        gtk::glib::markup_escape_text(headline)
    ));
    style(&h);
    col.append(&h);
    let d = gtk::Label::new(Some(detail));
    d.add_css_class("dim-label");
    style(&d);
    col.append(&d);
    row.append(&col);
    row
}
