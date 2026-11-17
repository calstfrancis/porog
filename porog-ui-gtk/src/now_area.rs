//! The Now area above the rows: Sputnik's Now card.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::now::{self, LaterItem, NowView};
use porog_core::App;

use crate::window::launch_then;

/// The Now area above the rows: Sputnik's Now card in Sputnik's words (`PLAN.md` P2). It
/// keeps its height whatever it shows, so the rows below never move (P1).
#[derive(Clone)]
pub(crate) struct NowArea {
    title: gtk::Label,
    detail: gtk::Label,
    footer: gtk::Label,
    action: gtk::Button,
    /// What the action button starts; set with each answer from Sputnik.
    target: ActionTarget,
    /// What is due next, in Sputnik's words; shown only with `details` on.
    later: gtk::Box,
    details: Rc<Cell<bool>>,
}

type ActionTarget = Rc<RefCell<Option<(&'static App, Vec<String>)>>>;

impl NowArea {
    pub(crate) fn new(toasts: &adw::ToastOverlay, details: Rc<Cell<bool>>) -> (gtk::Box, Self) {
        let eyebrow = gtk::Label::builder()
            .label("NOW  (from Sputnik)")
            .xalign(0.0)
            .css_classes(["caption-heading", "fond-quiet"])
            .build();
        let title = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["title-4"])
            .build();
        let detail = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["fond-row-detail"])
            .build();
        let footer = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["fond-row-detail"])
            .build();
        let text = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let later = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .visible(false)
            .build();
        for label in [&eyebrow, &title, &detail, &footer] {
            text.append(label);
        }
        text.append(&later);
        let open = gtk::Button::builder()
            .label("Open Sputnik")
            .valign(gtk::Align::Center)
            .tooltip_text("Open Sputnik")
            .action_name("win.open-plain")
            .action_target(&0i32.to_variant())
            .build();
        let action = gtk::Button::builder()
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        let target = ActionTarget::default();
        action.connect_clicked({
            let target = target.clone();
            let toasts = toasts.clone();
            move |button| {
                let Some((app, args)) = target.borrow().clone() else {
                    return;
                };
                let label = button.label().unwrap_or_default();
                button.set_label("Starting…");
                button.set_sensitive(false);
                let button = button.clone();
                launch_then(app, args, &toasts, move || {
                    button.set_label(&label);
                    button.set_sensitive(true);
                });
            }
        });
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        line.append(&text);
        line.append(&action);
        line.append(&open);
        let card = gtk::Box::builder()
            .height_request(104)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        card.append(&line);
        line.set_hexpand(true);
        line.set_margin_start(12);
        line.set_margin_end(12);
        line.set_margin_top(8);
        line.set_margin_bottom(8);
        card.add_css_class("card");
        let area = Self {
            title,
            detail,
            footer,
            action,
            target,
            later,
            details,
        };
        area.show(&NowView::Message("Looking…".to_string()));
        (card, area)
    }

    fn show(&self, view: &NowView) {
        let (title, detail, footer, action, later) = match view {
            NowView::Card {
                title,
                detail,
                footer,
                action,
                later,
            } => (
                title.as_str(),
                detail.as_str(),
                footer.as_deref(),
                action.as_ref(),
                later.as_slice(),
            ),
            NowView::Message(message) => (message.as_str(), "", None, None, &[][..]),
        };
        self.show_later(later);
        self.action.set_visible(action.is_some());
        if let Some(action) = action {
            self.action.set_label(&action.label);
            self.action
                .set_tooltip_text(Some(&format!("{} in {}", action.label, action.app.name)));
        }
        *self.target.borrow_mut() = action.map(|a| (a.app, a.args.clone()));
        self.title.set_label(title);
        self.detail.set_label(detail);
        self.detail.set_visible(!detail.is_empty());
        self.footer.set_label(footer.unwrap_or(""));
        self.footer.set_visible(footer.is_some());
    }

    /// Replace the "what is due next" lines, one label each.
    fn show_later(&self, later: &[LaterItem]) {
        while let Some(child) = self.later.first_child() {
            self.later.remove(&child);
        }
        for item in later {
            let text = if item.detail.is_empty() {
                item.title.clone()
            } else {
                format!("{} · {}", item.title, item.detail)
            };
            self.later.append(
                &gtk::Label::builder()
                    .label(&text)
                    .tooltip_text(&text)
                    .xalign(0.0)
                    .wrap(true)
                    .css_classes(["fond-row-detail"])
                    .build(),
            );
        }
        self.show_details();
    }

    /// Show or hide the "due next" lines to match the `details` toggle.
    pub(crate) fn show_details(&self) {
        self.later
            .set_visible(self.details.get() && self.later.first_child().is_some());
    }

    /// Ask Sputnik again, off the main thread.
    pub(crate) fn refresh(&self) {
        let area = self.clone();
        glib::spawn_future_local(async move {
            let view = gio::spawn_blocking(now::fetch)
                .await
                .unwrap_or_else(|_| NowView::Message(now::NO_ANSWER.to_string()));
            area.show(&view);
        });
    }
}
