//! The main window: the Now area, the five fixed app rows and the status bar.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::config::Config;
use porog_core::{App, APPS};

use crate::now_area::NowArea;
use crate::row::AppRow;
use crate::tray::{self, Line, Tray, TrayCmd};
use crate::{changelog, menu, search, welcome, VERSION};

/// Start `app` off the main thread, and say so plainly if it does not start.
pub fn launch(app: &'static App, args: Vec<String>, toasts: &adw::ToastOverlay) {
    launch_then(app, args, toasts, || {});
}

/// Like [`launch`], and calls `done` on the main thread once the app is clearly running or has
/// failed (within [`porog_core::LAUNCH_GRACE`]). A button uses it to show "Starting…" meanwhile.
pub fn launch_then(
    app: &'static App,
    args: Vec<String>,
    toasts: &adw::ToastOverlay,
    done: impl FnOnce() + 'static,
) {
    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let started = gio::spawn_blocking(move || porog_core::launch(app, &args)).await;
        done();
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
    let Some(app) = porog_core::app_for_file(&path) else {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        toasts.add_toast(adw::Toast::new(&format!(
            "Porog cannot open {name}. It handles PDF, EPUB and .typ files."
        )));
        return;
    };
    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        // In the sandbox the path is the portal's; the app needs the real one.
        let sandboxed = porog_core::in_flatpak_sandbox();
        let real = gio::spawn_blocking(move || {
            if sandboxed {
                crate::portal::host_path(&path)
            } else {
                Some(path)
            }
        })
        .await
        .ok()
        .flatten();
        match real {
            Some(path) => launch(app, vec![path.to_string_lossy().into_owned()], &toasts),
            None => toasts.add_toast(adw::Toast::new(
                "Porog cannot pass this file on. Open it from the app instead.",
            )),
        }
    });
}

/// While the tray is on and the window is not being looked at, re-read the rows when an app
/// writes its status file, so the tray menu is never a stale answer. Never while the window is
/// active: nothing changes under the user's eyes (`PLAN.md` P4).
fn watch_status_dir(window: &adw::ApplicationWindow, rows: &Rc<Vec<AppRow>>, tray: &Rc<Tray>) {
    let Some(dir) = porog_core::status::status_dir() else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let Ok(monitor) = gio::File::for_path(&dir)
        .monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
    else {
        return;
    };
    let pending = Rc::new(Cell::new(false));
    monitor.connect_changed({
        let window = window.clone();
        let rows = rows.clone();
        let tray = tray.clone();
        move |_, _, _, _| {
            if !tray.is_on() || window.is_active() || pending.replace(true) {
                return;
            }
            let rows = rows.clone();
            let pending = pending.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                pending.set(false);
                rows.iter().for_each(AppRow::refresh);
            });
        }
    });
    // The monitor lives as long as the window.
    window.connect_destroy(move |_| {
        let _ = &monitor;
    });
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

    let tray = Tray::new(app.upcast_ref());
    let lines: Rc<dyn Fn() -> Vec<Line>> = {
        let rows = rows.clone();
        Rc::new(move || rows.iter().map(AppRow::tray_line).collect())
    };
    // The tray menu follows the rows: whenever one is re-read, the menu is rewritten.
    for row in rows.iter() {
        row.on_change({
            let tray = tray.clone();
            let lines = lines.clone();
            move || tray.update(lines())
        });
    }

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
        .default_width(620)
        .default_height(680)
        .content(&view)
        .build();
    menu_button.set_popover(Some(&menu::build(
        &window,
        &toasts,
        config.clone(),
        config_path.clone(),
        tray.clone(),
        lines.clone(),
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
    let open_fresh = row_action("open-app-now", AppRow::activate_fresh);
    row_action("app-news", AppRow::show_news);
    list.connect_row_activated(move |_, row| open_app.activate(Some(&row.index().to_variant())));

    let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
    shortcuts_action.connect_activate({
        let window = window.clone();
        move |_, _| crate::shortcuts::show(&window)
    });
    window.add_action(&shortcuts_action);
    // N and W act on the row that has the focus: its note, its What's New.
    let row_keys = gtk::EventControllerKey::new();
    row_keys.connect_key_pressed({
        let rows = rows.clone();
        let list = list.clone();
        move |_, key, _, state| {
            let modified = gtk::gdk::ModifierType::CONTROL_MASK
                | gtk::gdk::ModifierType::ALT_MASK
                | gtk::gdk::ModifierType::SUPER_MASK;
            if state.intersects(modified) {
                return glib::Propagation::Proceed;
            }
            let Some(row) = list
                .focus_child()
                .and_downcast::<gtk::ListBoxRow>()
                .and_then(|row| rows.get(usize::try_from(row.index()).ok()?))
            else {
                return glib::Propagation::Proceed;
            };
            match key {
                gtk::gdk::Key::n | gtk::gdk::Key::N => row.edit_note(),
                gtk::gdk::Key::w | gtk::gdk::Key::W => row.show_news(),
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        }
    });
    list.add_controller(row_keys);
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
        move |_, _| search::show(&window, &toasts, "")
    });
    window.add_action(&search);
    // Search with the words already typed, for `porog --search=WORDS`.
    let search_for = gio::SimpleAction::new("search-for", Some(glib::VariantTy::STRING));
    search_for.connect_activate({
        let window = window.clone();
        let toasts = toasts.clone();
        move |_, words| {
            let words = words.and_then(|w| w.get::<String>()).unwrap_or_default();
            search::show(&window, &toasts, &words)
        }
    });
    window.add_action(&search_for);
    app.set_accels_for_action("win.search", &["<Control>k"]);
    let suite_action = gio::SimpleAction::new("suite", None);
    suite_action.connect_activate({
        let window = window.clone();
        move |_, _| crate::suite::show(&window)
    });
    window.add_action(&suite_action);
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
    window.connect_is_active_notify({
        let rows = rows.clone();
        move |window| {
            if window.is_active() {
                now_area.refresh();
                rows.iter().for_each(AppRow::refresh);
            }
        }
    });
    // The tray: its menu clicks come back here, the icon is made when the choice is on, and
    // while it is on closing the window only hides it.
    tray::set_handler({
        let window = window.clone();
        let app = app.clone();
        move |cmd| match cmd {
            TrayCmd::Show => window.present(),
            TrayCmd::Row(index) => open_fresh.activate(Some(&(index as i32).to_variant())),
            TrayCmd::Search => {
                window.present();
                gio::prelude::ActionGroupExt::activate_action(&window, "search", None);
            }
            TrayCmd::Quit => app.quit(),
        }
    });
    window.connect_close_request({
        let tray = tray.clone();
        move |window| {
            if tray.is_on() {
                window.set_visible(false);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    if config.borrow().tray {
        menu::set_tray(true, &tray, &*lines, &config, &config_path, &toasts);
    }
    watch_status_dir(&window, &rows, &tray);

    // Start on the first row, so Enter and the arrow keys work at once.
    if let Some(first) = list.row_at_index(0) {
        GtkWindowExt::set_focus(&window, Some(&first));
    }
    window.present();
    welcome::maybe_show(&window, &config, &config_path, &toasts);
}
