//! The Now area above the rows: Sputnik's Now card.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::now::{self, LaterItem, NowPiece, NowView};
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
    /// What the Now item is made of, each with its own button; shown only with `details` on.
    pieces: gtk::Box,
    toasts: adw::ToastOverlay,
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
        let pieces = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .visible(false)
            .build();
        for label in [&eyebrow, &title, &detail, &footer] {
            text.append(label);
        }
        text.append(&pieces);
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
            pieces,
            toasts: toasts.clone(),
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
                ..
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
        self.show_pieces(match view {
            NowView::Card { pieces, .. } => pieces,
            NowView::Message(_) => &[],
        });
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

    /// Replace the Now item's pieces: one line each, "Draft · THM 501: Essay", with its own
    /// button. Each button starts one app; none starts several (`PLAN.md` D8).
    fn show_pieces(&self, pieces: &[NowPiece]) {
        while let Some(child) = self.pieces.first_child() {
            self.pieces.remove(&child);
        }
        for piece in pieces {
            let text = if piece.label.is_empty() {
                piece.title.clone()
            } else {
                format!("{} · {}", piece.label, piece.title)
            };
            let label = gtk::Label::builder()
                .label(&text)
                .tooltip_text(&text)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .css_classes(["fond-row-detail"])
                .build();
            let hint = format!("Open {text} in {}", piece.app.name);
            let open = gtk::Button::builder()
                .label("Open")
                .valign(gtk::Align::Center)
                .tooltip_text(&hint)
                .css_classes(["flat"])
                .build();
            open.update_property(&[gtk::accessible::Property::Label(&hint)]);
            open.connect_clicked({
                let app = piece.app;
                let args = piece.args.clone();
                let toasts = self.toasts.clone();
                move |button| {
                    button.set_label("Starting…");
                    button.set_sensitive(false);
                    let button = button.clone();
                    launch_then(app, args.clone(), &toasts, move || {
                        button.set_label("Open");
                        button.set_sensitive(true);
                    });
                }
            });
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            line.append(&label);
            line.append(&open);
            self.pieces.append(&line);
        }
        self.show_details();
    }

    /// Show or hide the pieces and the "due next" lines to match the `details` toggle.
    pub(crate) fn show_details(&self) {
        let on = self.details.get();
        self.later
            .set_visible(on && self.later.first_child().is_some());
        self.pieces
            .set_visible(on && self.pieces.first_child().is_some());
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
