//! "How to install": what a row's button opens for an app that is not installed. Porog cannot
//! install anything itself (it is sandboxed and starts apps through the host), so it says exactly
//! what to run, lets the user copy it, and offers the software centre.

use adw::prelude::*;
use gtk::gio;
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::App;

pub fn show(parent: &gtk::Window, app: &'static App) {
    let commands = porog_core::install_commands(app);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(
        &gtk::Label::builder()
            .label(format!(
                "{} is not installed. Run this in a terminal:",
                app.name
            ))
            .xalign(0.0)
            .wrap(true)
            .build(),
    );
    body.append(
        &gtk::Label::builder()
            .label(&commands)
            .xalign(0.0)
            .selectable(true)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::Char)
            .css_classes(["monospace", "card"])
            .margin_top(2)
            .build(),
    );
    let dialog = adw::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .heading(format!("Install {}", app.name))
        .extra_child(&body)
        .build();
    dialog.set_size_request(520, -1);
    dialog.add_response("close", "Close");
    dialog.add_response("software", "Open in Software");
    dialog.add_response("copy", "Copy commands");
    dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("copy"));
    dialog.set_close_response("close");
    dialog.connect_response(None, move |dialog, response| match response {
        "copy" => {
            dialog.clipboard().set_text(&commands);
            // Stay open: the next thing is to paste it somewhere.
        }
        "software" => {
            let uri = format!("appstream://{}", app.flatpak_id);
            let _ = gio::AppInfo::launch_default_for_uri(&uri, gio::AppLaunchContext::NONE);
        }
        _ => {}
    });
    dialog.present();
}
