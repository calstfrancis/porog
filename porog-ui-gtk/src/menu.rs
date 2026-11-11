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

/// A flat, left-aligned popover row — same shape as every other Fond app's helper of the
/// same name.
fn popover_button(label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("flat");
    let lbl = gtk::Label::new(Some(label));
    lbl.set_xalign(0.0);
    lbl.set_halign(gtk::Align::Start);
    button.set_child(Some(&lbl));
    button
}

fn popover_separator() -> gtk::Separator {
    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    sep
}

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

pub fn build(
    window: &adw::ApplicationWindow,
    toasts: &adw::ToastOverlay,
    config: Rc<RefCell<Config>>,
    config_path: PathBuf,
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

    let whats_new = popover_button("What's New");
    let changelog = popover_button("Changelog");
    let about = popover_button("About Porog");
    for (row, action) in [
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
