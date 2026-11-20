//! Porog — the way back into the Fond suite.
//!
//! One window: a fixed row per Fond app. See `PLAN.md` for what each later phase adds
//! (Resume, the Now card, Ctrl+K search) and for the rules the layout follows.

mod changelog;
mod install;
mod menu;
mod news;
mod now_area;
mod portal;
mod row;
mod search;
mod shortcuts;
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
/* The board: the Now card and the rows, each a hairline card on the recessed ground. */
.porog-board { margin: 14px 14px 18px 14px; }
.porog-card {
  background: @card_bg_color;
  border: 1px solid alpha(@borders, 0.55);
  border-radius: 12px;
}
.porog-now { padding: 12px 16px 12px 16px; }
.porog-board list.porog-rows { background: @card_bg_color; }
.porog-rows > row { border-radius: 0; }
.porog-rows > row:not(:last-child) { border-bottom: 1px solid alpha(@borders, 0.35); }
.porog-rows > row:first-child { border-top-left-radius: 11px; border-top-right-radius: 11px; }
.porog-rows > row:last-child { border-bottom-left-radius: 11px; border-bottom-right-radius: 11px; }
.porog-rows row.fond-row > box { padding: 9px 10px 9px 4px; }
.porog-name { padding-left: 10px; padding-right: 4px; }
.porog-note { font-style: italic; }
.porog-ochre { color: @fond_hl_ochre; }

/* Resume is a soft fill, Open a hairline: told apart by shape, never by colour. */
button.porog-act { min-height: 30px; padding: 0 14px; border-radius: 9999px; }
button.porog-act label { font-weight: 400; }
button.porog-open { background: transparent; box-shadow: inset 0 0 0 1px alpha(@borders, 0.9); }
button.porog-open:hover { background: alpha(@window_fg_color, 0.05); }
button.porog-open:active { background: alpha(@window_fg_color, 0.10); }
button.porog-resume { background: alpha(@window_fg_color, 0.09); box-shadow: none; }
button.porog-resume label { font-weight: 600; }
button.porog-resume:hover { background: alpha(@window_fg_color, 0.13); }
button.porog-resume:active { background: alpha(@window_fg_color, 0.17); }
button.porog-act-quiet label { font-weight: 400; }

/* The row menu's ⋯ is there on every row, but quiet until the row is pointed at. */
.porog-more { opacity: 0.45; }
row:hover .porog-more { opacity: 0.8; }
.porog-more:hover, .porog-more:focus-within, .porog-more:checked { opacity: 1; }
button.porog-menu-item { min-height: 30px; padding: 0 10px; }
button.porog-menu-item label { font-weight: 400; }
popover.porog-row-menu > contents { padding: 6px; }

headerbar button.porog-search-pill { min-height: 30px; padding: 0 16px; }
headerbar button.porog-search-pill label { font-size: 0.92em; }
headerbar button.porog-search-pill image { opacity: 0.6; }

/* The changelog. */
.porog-headline { font-weight: 600; }
.porog-detail { opacity: 0.75; }
.porog-earlier > expander-widget { border-top: 1px solid alpha(@borders, 0.35); }
.porog-earlier > expander-widget:last-child { border-bottom: 1px solid alpha(@borders, 0.35); }
";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_startup(|app| {
        load_css();
        app.set_accels_for_action(
            "win.shortcuts",
            &["<Control>question", "<Control><Shift>slash"],
        );
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
        (
            "resume",
            "Do what an app's row button says: Resume or Open",
            "APP",
        ),
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
