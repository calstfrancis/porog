//! The system tray icon (StatusNotifierItem), off unless the user turns it on in the menu.
//!
//! The icon is Porog's own and never changes (`PLAN.md` P5). A left click drops the menu down
//! (`item_is_menu`):
//!
//! ```text
//! Show Porog
//! ──────────
//! Sputnik
//! Pereplyot
//!     Resume “An Example Reader”
//! Zerkalo
//! ──────────
//! Search the suite…
//! Quit Porog
//! ```
//!
//! Each installed app in row order: its name opens or switches to it, and the line under it
//! resumes what it was last doing (P3). Nothing in the menu is bold, coloured or counted. The
//! tray runs on its own thread; its menu clicks come back to the GTK thread through
//! [`set_handler`].

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use ksni::MenuItem;

/// What a click in the tray menu asks for.
pub enum TrayCmd {
    Show,
    /// Open app N, or switch to it if it is already running.
    Open(usize),
    /// Resume app N's newest item, as its row's button does.
    Row(usize),
    Search,
    Quit,
}

type Handler = Rc<dyn Fn(TrayCmd)>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
}

/// Where tray clicks go. Set once, on the GTK thread.
pub fn set_handler(f: impl Fn(TrayCmd) + 'static) {
    HANDLER.with(|h| *h.borrow_mut() = Some(Rc::new(f)));
}

/// Called from the tray's thread: hop to the GTK main thread and run the handler there.
fn send(cmd: TrayCmd) {
    glib::MainContext::default().invoke(move || {
        let handler = HANDLER.with(|h| h.borrow().clone());
        if let Some(handler) = handler {
            handler(cmd);
        }
    });
}

/// One app's line in the tray menu.
#[derive(Clone)]
pub struct Line {
    /// The app's row number, which is what a click sends back.
    pub index: usize,
    pub text: String,
    /// "Resume “title”", when the app has something to resume.
    pub resume: Option<String>,
}

struct PorogTray {
    lines: Vec<Line>,
}

/// `text` as a menu label shows it. The menu protocol reads a lone `_` as "underline the next
/// letter", so a title like `essay_2.typ` would lose its underscore; doubling it keeps it.
fn label(text: &str) -> String {
    text.replace('_', "__")
}

impl ksni::Tray for PorogTray {
    fn id(&self) -> String {
        "io.github.calstfrancis.Porog".into()
    }

    fn title(&self) -> String {
        "Porog".into()
    }

    fn icon_name(&self) -> String {
        "io.github.calstfrancis.Porog".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Porog".into(),
            description: "The way back into the Fond suite".into(),
            ..Default::default()
        }
    }

    /// A left click drops the menu down instead of calling `activate` (SNI `ItemIsMenu`).
    const MENU_ON_ACTIVATE: bool = true;

    fn activate(&mut self, _x: i32, _y: i32) {
        send(TrayCmd::Show);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let item = |label: &str, enabled: bool, cmd: fn() -> TrayCmd| {
            StandardItem {
                label: label.to_string(),
                enabled,
                activate: Box::new(move |_: &mut Self| send(cmd())),
                ..Default::default()
            }
            .into()
        };
        let mut menu: Vec<MenuItem<Self>> = vec![item("Show Porog", true, || TrayCmd::Show)];
        menu.push(MenuItem::Separator);
        for line in &self.lines {
            let index = line.index;
            menu.push(
                StandardItem {
                    label: label(&line.text),
                    activate: Box::new(move |_: &mut Self| send(TrayCmd::Open(index))),
                    ..Default::default()
                }
                .into(),
            );
            if let Some(resume) = &line.resume {
                menu.push(
                    StandardItem {
                        label: format!("    {}", label(resume)),
                        activate: Box::new(move |_: &mut Self| send(TrayCmd::Row(index))),
                        ..Default::default()
                    }
                    .into(),
                );
            }
        }
        if self.lines.is_empty() {
            menu.push(
                StandardItem {
                    label: "No Fond apps are installed".into(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }
        menu.push(MenuItem::Separator);
        menu.push(item("Search the suite…", true, || TrayCmd::Search));
        menu.push(item("Quit Porog", true, || TrayCmd::Quit));
        menu
    }
}

/// The tray icon and, while it is there, the hold that keeps Porog running with no window.
pub struct Tray {
    app: gtk::Application,
    handle: Rc<RefCell<Option<Handle<PorogTray>>>>,
    hold: RefCell<Option<gio::ApplicationHoldGuard>>,
    /// True from the moment the icon is asked for until it is removed.
    on: std::cell::Cell<bool>,
}

impl Tray {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        Rc::new(Tray {
            app: app.clone(),
            handle: Rc::default(),
            hold: RefCell::new(None),
            on: std::cell::Cell::new(false),
        })
    }

    /// Whether closing the window should hide it instead of quitting.
    pub fn is_on(&self) -> bool {
        self.on.get()
    }

    /// Put the icon in the tray with `lines` as the app lines. `failed` runs on the GTK thread
    /// if that cannot be done. With no tray host on the bus the icon waits for one, so turning
    /// this on before installing the extension works once the extension is there.
    pub fn start(self: &Rc<Self>, lines: Vec<Line>, failed: impl FnOnce() + 'static) {
        if self.on.replace(true) {
            return;
        }
        *self.hold.borrow_mut() = Some(self.app.hold());
        let tray = self.clone();
        glib::spawn_future_local(async move {
            let spawned = gio::spawn_blocking(move || {
                PorogTray { lines }
                    .disable_dbus_name(true)
                    .assume_sni_available(true)
                    .spawn()
            })
            .await;
            match spawned {
                Ok(Ok(handle)) if tray.on.get() => *tray.handle.borrow_mut() = Some(handle),
                Ok(Ok(handle)) => {
                    // Turned off while it was starting.
                    let _ = gio::spawn_blocking(move || handle.shutdown().wait()).await;
                }
                _ => {
                    tray.on.set(false);
                    tray.hold.borrow_mut().take();
                    failed();
                }
            }
        });
    }

    /// Take the icon away; Porog quits with its last window again.
    pub fn stop(&self) {
        self.on.set(false);
        self.hold.borrow_mut().take();
        if let Some(handle) = self.handle.borrow_mut().take() {
            glib::spawn_future_local(async move {
                let _ = gio::spawn_blocking(move || handle.shutdown().wait()).await;
            });
        }
    }

    /// Replace the app lines, off the GTK thread.
    pub fn update(&self, lines: Vec<Line>) {
        let Some(handle) = self.handle.borrow().clone() else {
            return;
        };
        glib::spawn_future_local(async move {
            let _ = gio::spawn_blocking(move || handle.update(|t| t.lines = lines)).await;
        });
    }
}

/// Whether anything on the session bus is watching for tray icons. Blocks on D-Bus, so call it
/// from a worker thread. GNOME has no watcher unless the AppIndicator extension is installed.
pub fn host_present() -> bool {
    let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return false;
    };
    bus.call_sync(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        Some(&("org.kde.StatusNotifierWatcher",).to_variant()),
        Some(glib::VariantTy::new("(b)").unwrap()),
        gio::DBusCallFlags::NONE,
        1000,
        gio::Cancellable::NONE,
    )
    .ok()
    .and_then(|reply| reply.get::<(bool,)>())
    .is_some_and(|(present,)| present)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn underscores_in_titles_survive_the_menu() {
        assert_eq!(label("Resume “essay_2.typ”"), "Resume “essay__2.typ”");
        assert_eq!(label("Zerkalo"), "Zerkalo");
    }
}
