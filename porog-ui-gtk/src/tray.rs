//! The system tray icon (StatusNotifierItem), off unless the user turns it on in the menu.
//!
//! The icon is Porog's own and never changes (`PLAN.md` P5). Its menu is Show Porog, one line
//! per app in row order saying what the row's button says (P3), Search…, and Quit. The tray
//! runs on its own thread; its menu clicks come back to the GTK thread through
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
    /// Do what row N's button says: Resume or Open.
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
    pub text: String,
    pub enabled: bool,
}

struct PorogTray {
    lines: Vec<Line>,
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
        for (index, line) in self.lines.iter().enumerate() {
            menu.push(
                StandardItem {
                    label: line.text.clone(),
                    enabled: line.enabled,
                    activate: Box::new(move |_: &mut Self| send(TrayCmd::Row(index))),
                    ..Default::default()
                }
                .into(),
            );
        }
        menu.push(MenuItem::Separator);
        menu.push(item("Search…", true, || TrayCmd::Search));
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
                PorogTray { lines }.assume_sni_available(true).spawn()
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
