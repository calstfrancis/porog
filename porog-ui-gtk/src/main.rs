//! Porog — the way back into the Fond suite.
//!
//! One window: a fixed row per Fond app. See `PLAN.md` for what each later phase adds
//! (Resume, the Now card, Ctrl+K search) and for the rules the layout follows.

mod changelog;
mod install;
mod menu;
mod news;
mod portal;
mod now_area;
mod row;
mod search;
mod suite;
mod tray;
mod welcome;
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
.porog-note { font-style: italic; }
";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
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
    app.connect_activate(|app| match app.windows().first() {
        Some(window) => window.present(),
        None => window::build(app),
    });
    // Registered so GLib accepts them; the values come back in the command line's options.
    // GLib does not allow an optional value on a string option, so the search box with
    // nothing typed is its own flag.
    for (name, help, arg) in [
        ("resume", "Do what an app's row button says: Resume or Open", "APP"),
        ("open", "Open an app plainly", "APP"),
        ("search", "Search the suite for WORDS", "WORDS"),
    ] {
        app.add_main_option(
            name,
            glib::Char(0),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            help,
            Some(arg),
        );
    }
    app.add_main_option(
        "search-box",
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Open the search box",
        None,
    );
    app.connect_command_line(|app, line| {
        let options = line.options_dict();
        let mut args: Vec<String> = ["resume", "open", "search"]
            .into_iter()
            .filter_map(|name| {
                let value = options.lookup_value(name, None)?;
                let value = value.str().map(str::to_string).unwrap_or_default();
                Some(format!("--{name}={value}"))
            })
            .collect();
        if options.contains("search-box") {
            args.push("--search".to_string());
        }
        let cli = match porog_core::cli::parse(&args) {
            Ok(cli) => cli,
            Err(message) => {
                eprintln!("porog: {message}");
                return glib::ExitCode::from(2);
            }
        };
        run_cli(app, cli);
        glib::ExitCode::SUCCESS
    });
    app.run()
}

/// Do what the command line asked, in the running Porog or a new one. A request for a row
/// does not raise an existing window: the point is to get to the app, not to Porog.
fn run_cli(app: &adw::Application, cli: porog_core::cli::Cli) {
    use porog_core::cli::Cli;
    let existing = app.windows().into_iter().next();
    if existing.is_none() || matches!(cli, Cli::Show) {
        app.activate();
    }
    let Some(window) = app.windows().into_iter().next() else {
        return;
    };
    let Ok(window) = window.downcast::<adw::ApplicationWindow>() else {
        return;
    };
    let act = |name: &str, target: Option<glib::Variant>| {
        gtk::gio::prelude::ActionGroupExt::activate_action(&window, name, target.as_ref());
    };
    match cli {
        Cli::Show => {}
        Cli::Resume(row) => act("open-app-now", Some((row as i32).to_variant())),
        Cli::Open(row) => act("open-plain", Some((row as i32).to_variant())),
        Cli::Search(words) if words.trim().is_empty() => {
            window.present();
            act("search", None);
        }
        Cli::Search(words) => {
            window.present();
            act("search-for", Some(words.to_variant()));
        }
    }
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
