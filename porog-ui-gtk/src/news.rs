//! The What's New window for one app, opened from the icon on its row. The words are the app's
//! own, asked for each time (`porog_core::whats_new`); Porog only lays them out. The icon never
//! changes and carries no unread mark (`PLAN.md` P5).

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::whats_new::{self, NewsView};
use porog_core::App;

use crate::welcome::{bullet, section, style};

pub fn show(parent: &gtk::Window, app: &'static App) {
    let heading = section(&format!("What's new in {}", app.name));
    heading.set_margin_top(0);
    let message = gtk::Label::new(Some("Looking…"));
    style(&message);
    message.add_css_class("dim-label");
    let items = gtk::Box::new(gtk::Orientation::Vertical, 12);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_start(24);
    body.set_margin_end(24);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    body.append(&heading);
    body.append(&message);
    body.append(&items);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(450);
    clamp.set_child(Some(&body));
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroll.set_child(Some(&clamp));

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroll));
    let window = adw::Window::builder()
        .title(format!("What's new in {}", app.name))
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .default_height(480)
        .content(&toolbar)
        .build();
    crate::welcome::close_on_escape(&window);
    window.present();

    glib::spawn_future_local(async move {
        let view = gio::spawn_blocking(move || whats_new::fetch(app))
            .await
            .unwrap_or_else(|_| NewsView::Message(format!("{} did not answer.", app.name)));
        match view {
            NewsView::Notes {
                heading: text,
                items: list,
            } => {
                heading.set_label(&text);
                if list.is_empty() {
                    message.set_label("Nothing is listed for this release.");
                } else {
                    message.set_visible(false);
                }
                for (headline, detail) in &list {
                    items.append(&bullet(headline, detail));
                }
            }
            NewsView::Message(text) => message.set_label(&text),
        }
    });
}
