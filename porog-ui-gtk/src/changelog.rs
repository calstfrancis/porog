//! The changelog window: `CHANGELOG.md` itself, laid out. The newest release is open; each
//! earlier one is a single line that opens on request (`PLAN.md` P6), so the window starts
//! with one thing to read rather than the whole history.

use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

use porog_core::changelog::{self, inline_markup, Item, Release};

use crate::welcome::style;

const CHANGELOG: &str = include_str!("../../CHANGELOG.md");

pub fn show_changelog(parent: &impl IsA<gtk::Window>) {
    let releases = changelog::parse(CHANGELOG);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    body.set_margin_start(24);
    body.set_margin_end(24);
    body.set_margin_top(20);
    body.set_margin_bottom(28);

    let mut releases = releases.iter();
    if let Some(newest) = releases.next() {
        body.append(&heading(newest, true));
        body.append(&notes(newest));
    }
    let earlier: Vec<&Release> = releases.collect();
    if !earlier.is_empty() {
        let label = gtk::Label::builder()
            .label("Earlier releases")
            .xalign(0.0)
            .margin_top(28)
            .margin_bottom(6)
            .css_classes(["fond-section-title", "dim-label"])
            .build();
        body.append(&label);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        list.add_css_class("porog-earlier");
        for release in earlier {
            let notes = notes(release);
            notes.set_margin_start(26);
            notes.set_margin_bottom(14);
            let expander = gtk::Expander::builder()
                .label_widget(&heading(release, false))
                .child(&notes)
                .css_classes(["porog-release"])
                .build();
            list.append(&expander);
        }
        body.append(&list);
    }

    let clamp = adw::Clamp::builder()
        .maximum_size(620)
        .tightening_threshold(480)
        .child(&body)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&clamp)
        .build();
    let header = adw::HeaderBar::new();
    header.add_css_class("fond-chrome");
    header.set_title_widget(Some(&adw::WindowTitle::new(
        "Changelog",
        &format!("You are on {}", crate::VERSION),
    )));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroll));
    let window = adw::Window::builder()
        .title("Changelog")
        .transient_for(parent)
        .default_width(600)
        .default_height(640)
        .content(&toolbar)
        .build();
    crate::welcome::close_on_escape(&window);
    window.present();
}

/// A release's heading. Open (`full`): the name large, then the version, date and the line
/// that says what it was about. Folded: all of that on one or two quiet lines.
fn heading(release: &Release, full: bool) -> gtk::Box {
    let current = release.version == crate::VERSION;
    let mut meta = vec![release.version.clone()];
    meta.extend(release.date.clone());
    if current {
        meta.push("the version you have".to_string());
    }
    let meta = meta.join(" · ");
    let name = release
        .name
        .clone()
        .or_else(|| release.title.clone())
        .unwrap_or_else(|| release.version.clone());
    let title = release.title.as_ref().filter(|_| release.name.is_some());

    let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let meta_label = gtk::Label::builder()
        .label(&meta)
        .xalign(0.0)
        .css_classes(["caption", "numeric", "dim-label"])
        .build();
    let name_label = gtk::Label::builder()
        .label(&name)
        .xalign(0.0)
        .wrap(true)
        .css_classes([if full { "title-2" } else { "heading" }])
        .build();
    if full {
        column.append(&meta_label);
        column.append(&name_label);
        if let Some(title) = title {
            let line = gtk::Label::new(Some(title));
            style(&line);
            line.add_css_class("dim-label");
            column.append(&line);
        }
    } else {
        // "Steady Threshold  0.5.0 · 17 Nov 2026", then what it was about.
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        name_label.set_wrap(false);
        meta_label.set_valign(gtk::Align::Baseline);
        name_label.set_valign(gtk::Align::Baseline);
        top.append(&name_label);
        top.append(&meta_label);
        column.append(&top);
        if let Some(title) = title {
            let line = gtk::Label::builder()
                .label(title)
                .xalign(0.0)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .css_classes(["fond-row-detail"])
                .build();
            column.append(&line);
        }
        column.set_margin_top(6);
        column.set_margin_bottom(6);
    }
    column
}

/// A release's sections and bullets.
fn notes(release: &Release) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_top(6);
    column.set_margin_bottom(6);
    for section in &release.sections {
        if let Some(heading) = &section.heading {
            column.append(&section_label(heading));
        }
        for item in &section.items {
            column.append(&bullet(item));
        }
    }
    column
}

/// "● ADDED", as every section in the suite is announced.
fn section_label(text: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.add_css_class("fond-section");
    row.set_margin_top(12);
    let dot = gtk::Label::new(Some("●"));
    dot.add_css_class("fond-section-dot");
    dot.add_css_class("porog-ochre");
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .css_classes(["fond-section-title"])
        .build();
    row.append(&dot);
    row.append(&label);
    row
}

/// One change: its headline in bold, and under it the detail, dimmer; or one plain line.
fn bullet(item: &Item) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_start(4);
    let dot = gtk::Label::new(Some("•"));
    dot.add_css_class("dim-label");
    dot.set_valign(gtk::Align::Start);
    row.append(&dot);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let label = |text: &str, classes: &[&str]| {
        let label = gtk::Label::new(None);
        set_markup(&label, text);
        style(&label);
        for class in classes {
            label.add_css_class(class);
        }
        label
    };
    match &item.headline {
        Some(headline) => {
            column.append(&label(headline, &["porog-headline"]));
            if !item.text.is_empty() {
                column.append(&label(&item.text, &["porog-detail"]));
            }
        }
        None => column.append(&label(&item.text, &[])),
    }
    row.append(&column);
    row
}

/// The file's inline Markdown as markup, or as the plain words if that ever fails to parse.
fn set_markup(label: &gtk::Label, text: &str) {
    let markup = inline_markup(text);
    if gtk::pango::parse_markup(&markup, '\0').is_ok() {
        label.set_markup(&markup);
    } else {
        label.set_text(text);
    }
}
