use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

const CHANGELOG: &str = include_str!("../../CHANGELOG.md");

/// A small window rendering `CHANGELOG.md` verbatim, so the in-app view can never drift
/// from the file itself. Plain text, the same as Pereplyot's.
pub fn show_changelog(parent: &impl IsA<gtk::Window>) {
    let win = adw::Window::new();
    win.set_title(Some("Changelog"));
    win.set_default_width(560);
    win.set_default_height(480);
    win.set_transient_for(Some(parent));

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("Changelog", &format!("You're on v{}", crate::VERSION));
    header.set_title_widget(Some(&title));

    let text_view = gtk::TextView::new();
    text_view.set_editable(false);
    text_view.set_cursor_visible(false);
    text_view.set_wrap_mode(gtk::WrapMode::Word);
    text_view.set_margin_start(16);
    text_view.set_margin_end(16);
    text_view.set_margin_top(12);
    text_view.set_margin_bottom(12);
    text_view.buffer().set_text(CHANGELOG);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_child(Some(&text_view));

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    win.set_content(Some(&toolbar));

    win.present();
}
