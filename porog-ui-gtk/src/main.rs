//! Porog — the way back into the Fond suite.
//!
//! One window: a fixed row per Fond app. See `PLAN.md` for what each later phase adds
//! (Resume, the Now card, Ctrl+K search) and for the rules the layout follows.

mod changelog;
mod menu;
mod search;
mod window;

use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::APPS;

const APP_ID: &str = "io.github.calstfrancis.Porog";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const FOND_CSS: &str = include_str!("../style/fond.css");
const GLOBAL_CSS: &str = "
.porog-rows { margin: 12px; }
.porog-rows row.fond-row > box { padding-left: 4px; padding-right: 8px; }
.porog-name { padding-left: 8px; padding-right: 0; }
";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|app| {
        load_css();
        // Alt+1…5 open the five apps, in row order.
        for index in 0..APPS.len() {
            app.set_accels_for_action(
                &format!("win.open-app({index})"),
                &[&format!("<Alt>{}", index + 1)],
            );
        }
    });
    app.connect_activate(|app| match app.active_window() {
        Some(window) => window.present(),
        None => window::build(app),
    });
    app.run()
}

fn load_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let css = gtk::CssProvider::new();
    css.load_from_data(&format!("{FOND_CSS}\n{GLOBAL_CSS}"));
    gtk::style_context_add_provider_for_display(
        &display,
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
