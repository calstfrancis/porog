//! The hamburger popover: hand-built rows, not a `gio::Menu` — house style (root
//! `CLAUDE.md`'s UI design standard).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::config::{Config, Theme};

use crate::tray::{Line, Tray};

/// A flat, left-aligned popover row — same shape as every other Fond app's helper of the
/// same name.
fn popover_button(label: &str) -> gtk::Button {
    menu_item(label, None)
}

/// A popover row: its label flush left and, when it has one, its key flush right in a dim
/// caption (Zerkalo's `make_menu_item`).
pub(crate) fn menu_item(label: &str, shortcut: Option<&str>) -> gtk::Button {
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    let text = gtk::Label::builder()
        .label(label)
        .xalign(0.0)
        .hexpand(true)
        .build();
    line.append(&text);
    if let Some(shortcut) = shortcut {
        line.append(
            &gtk::Label::builder()
                .label(shortcut)
                .css_classes(["caption", "dim-label"])
                .build(),
        );
    }
    gtk::Button::builder()
        .child(&line)
        .css_classes(["flat", "porog-menu-item"])
        .build()
}

/// The label inside a [`menu_item`], to change its words later.
pub(crate) fn menu_item_label(item: &gtk::Button) -> Option<gtk::Label> {
    item.child()?.first_child()?.downcast().ok()
}

pub(crate) fn popover_separator() -> gtk::Separator {
    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    sep
}

const TRAY_NOTE: &str =
    "An icon with Resume and Open for each app. Closing the window then keeps Porog running.";
const TRAY_NO_HOST: &str = "Your desktop shows no tray icons. On GNOME, install the AppIndicator extension. Closing the window then keeps Porog running.";

pub fn apply_theme(theme: Theme) {
    adw::StyleManager::default().set_color_scheme(match theme {
        Theme::System => adw::ColorScheme::Default,
        Theme::Light => adw::ColorScheme::ForceLight,
        Theme::Dark => adw::ColorScheme::ForceDark,
    });
}

/// Write the settings off the main thread, and say `failure` if that does not work.
pub fn save_config(
    config: &Rc<RefCell<Config>>,
    config_path: &Path,
    toasts: &adw::ToastOverlay,
    failure: &'static str,
) {
    let config = config.borrow().clone();
    let config_path = config_path.to_path_buf();
    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let saved = gio::spawn_blocking(move || config.save(&config_path)).await;
        if !matches!(saved, Ok(Ok(()))) {
            toasts.add_toast(adw::Toast::new(failure));
        }
    });
}

/// Turn the tray icon on or off and keep the choice, saying so if it cannot be done.
pub fn set_tray(
    on: bool,
    tray: &Rc<Tray>,
    lines: &dyn Fn() -> Vec<Line>,
    config: &Rc<RefCell<Config>>,
    config_path: &Path,
    toasts: &adw::ToastOverlay,
) {
    if on {
        let failed_toasts = toasts.clone();
        let failed_config = config.clone();
        tray.start(lines(), move || {
            failed_config.borrow_mut().tray = false;
            failed_toasts.add_toast(adw::Toast::new("Porog could not put an icon in the tray."));
        });
    } else {
        tray.stop();
    }
    config.borrow_mut().tray = on;
    save_config(
        config,
        config_path,
        toasts,
        "The tray choice could not be saved. It lasts until Porog closes.",
    );
}

pub fn build(
    window: &adw::ApplicationWindow,
    toasts: &adw::ToastOverlay,
    config: Rc<RefCell<Config>>,
    config_path: PathBuf,
    tray: Rc<Tray>,
    lines: Rc<dyn Fn() -> Vec<Line>>,
) -> gtk::Popover {
    let current = config.borrow().theme;
    let rows = gtk::Box::new(gtk::Orientation::Vertical, 2);
    rows.set_margin_top(6);
    rows.set_margin_bottom(6);
    rows.set_margin_start(6);
    rows.set_margin_end(6);
    rows.set_width_request(200);

    let popover = gtk::Popover::new();
    popover.set_child(Some(&rows));

    // Theme: three inline rows, the current one marked by `.fond-toggle-active`'s bold.
    let theme_rows: Rc<Vec<(Theme, gtk::Button)>> = Rc::new(
        Theme::ALL
            .into_iter()
            .map(|theme| {
                let row = popover_button(theme.label());
                if theme == current {
                    row.add_css_class("fond-toggle-active");
                }
                rows.append(&row);
                (theme, row)
            })
            .collect(),
    );
    for (theme, row) in theme_rows.iter() {
        let theme = *theme;
        let popover = popover.clone();
        let all = theme_rows.clone();
        let toasts = toasts.clone();
        let config = config.clone();
        let config_path = config_path.clone();
        row.connect_clicked(move |_| {
            popover.popdown();
            apply_theme(theme);
            for (other, button) in all.iter() {
                if *other == theme {
                    button.add_css_class("fond-toggle-active");
                } else {
                    button.remove_css_class("fond-toggle-active");
                }
            }
            config.borrow_mut().theme = theme;
            save_config(
                &config,
                &config_path,
                &toasts,
                "The theme choice could not be saved. It lasts until Porog closes.",
            );
        });
    }
    rows.append(&popover_separator());

    // The tray icon: a word that is bold when on, like the status bar's toggles, and under it
    // what it does, or why it will not show.
    let tray_row = popover_button("Show in system tray");
    if config.borrow().tray {
        tray_row.add_css_class("fond-toggle-active");
    }
    let tray_note = gtk::Label::builder()
        .label(TRAY_NOTE)
        .xalign(0.0)
        .wrap(true)
        .max_width_chars(28)
        .margin_start(10)
        .margin_end(10)
        .css_classes(["caption", "fond-quiet"])
        .build();
    tray_row.connect_clicked({
        let config = config.clone();
        let config_path = config_path.clone();
        let toasts = toasts.clone();
        let tray = tray.clone();
        let lines = lines.clone();
        move |row| {
            let on = !config.borrow().tray;
            if on {
                row.add_css_class("fond-toggle-active");
            } else {
                row.remove_css_class("fond-toggle-active");
            }
            set_tray(on, &tray, &*lines, &config, &config_path, &toasts);
        }
    });
    // Whether anything is watching for tray icons is asked each time the menu opens.
    popover.connect_show({
        let tray_note = tray_note.clone();
        move |_| {
            let tray_note = tray_note.clone();
            glib::spawn_future_local(async move {
                let present = gio::spawn_blocking(crate::tray::host_present)
                    .await
                    .unwrap_or(false);
                tray_note.set_label(if present { TRAY_NOTE } else { TRAY_NO_HOST });
            });
        }
    });
    rows.append(&tray_row);
    rows.append(&tray_note);
    rows.append(&popover_separator());

    let suite = popover_button("The suite");
    let shortcuts = menu_item("Keyboard shortcuts", Some("Ctrl+?"));
    let whats_new = popover_button("What's New");
    let changelog = popover_button("Changelog");
    let about = popover_button("About Porog");
    for (row, action) in [
        (&suite, "suite"),
        (&shortcuts, "shortcuts"),
        (&whats_new, "whats-new"),
        (&changelog, "changelog"),
        (&about, "about"),
    ] {
        let popover = popover.clone();
        let window = window.clone();
        row.connect_clicked(move |_| {
            popover.popdown();
            gio::prelude::ActionGroupExt::activate_action(&window, action, None);
        });
        rows.append(row);
    }

    popover
}

pub fn show_about(window: &adw::ApplicationWindow) {
    let about = gtk::AboutDialog::builder()
        .program_name("Porog")
        .version(crate::VERSION)
        .comments("The way back into the Fond suite — порог, \"threshold\"")
        .website("https://github.com/calstfrancis/porog")
        .transient_for(window)
        .modal(true)
        .build();
    about.present();
}
