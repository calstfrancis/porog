//! The Suite window (menu → The suite): each app's installed version and which of the things
//! Porog asks of it it answers, found by asking when the window opens and never again while it
//! is open (`PLAN.md` P4). A thing an app does not answer says what to do about it.

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::suite::{self, Report, Standing};
use porog_core::APPS;

use crate::welcome::section;

pub fn show(parent: &adw::ApplicationWindow) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
    body.set_margin_start(24);
    body.set_margin_end(24);
    body.set_margin_top(16);
    body.set_margin_bottom(20);
    body.append(
        &gtk::Label::builder()
            .label("Asked each app when this window opened.")
            .xalign(0.0)
            .css_classes(["caption", "fond-quiet"])
            .build(),
    );

    // One block per app, in the fixed app order, each filled when its answers arrive.
    let blocks: Vec<gtk::Box> = APPS
        .iter()
        .map(|app| {
            let block = gtk::Box::new(gtk::Orientation::Vertical, 2);
            block.append(&section(app.name));
            block.append(
                &gtk::Label::builder()
                    .label("Asking…")
                    .xalign(0.0)
                    .css_classes(["fond-row-detail"])
                    .build(),
            );
            body.append(&block);
            block
        })
        .collect();

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(480);
    clamp.set_child(Some(&body));
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_child(Some(&clamp));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroll));
    let window = adw::Window::builder()
        .title("The suite")
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(560)
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

    for (app, block) in APPS.iter().zip(blocks) {
        glib::spawn_future_local(async move {
            let report = gio::spawn_blocking(move || suite::probe(app)).await;
            fill(&block, app.name, report.ok());
        });
    }
}

/// Replace a block's "Asking…" with what the app said.
fn fill(block: &gtk::Box, name: &str, report: Option<Report>) {
    while let Some(last) = block.last_child() {
        if block.first_child().as_ref() == Some(&last) {
            break;
        }
        block.remove(&last);
    }
    let line = |text: &str| {
        gtk::Label::builder()
            .label(text)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["fond-row-detail"])
            .build()
    };
    let Some(report) = report else {
        block.append(&line(&format!("{name} did not answer.")));
        return;
    };
    if !report.installed {
        block.append(&line("Not installed."));
        return;
    }
    block.append(&line(&match &report.version {
        Some(version) => format!("Version {version}"),
        None => "Installed".to_string(),
    }));
    for (thing, standing) in &report.things {
        block.append(&line(&match standing {
            Standing::Answers => format!("{thing}: answers"),
            Standing::NotYet => format!("{thing}: none yet. {name} writes it once you use it."),
            Standing::Missing(hint) => format!("{thing}: does not answer. {hint}"),
        }));
    }
}
