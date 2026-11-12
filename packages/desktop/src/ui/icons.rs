//! Icons by their Lucide file name, plus the hand-drawn shortcut marks.

use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::*;

/// A Lucide icon by its kebab-case name, e.g. `folder-open`.
pub fn lucide(name: &str) -> Icon {
    Icon::default().path(format!("icons/{name}.svg"))
}

/// Icons a snippet can be pinned to the sidebar with.
pub const PIN_ICON_NAMES: [&str; 27] = [
    // Hand-drawn marks for tools that have no Lucide equivalent.
    "k9s",
    "claude",
    "opencode",
    "omp",
    "terminal",
    "square-terminal",
    "server",
    "database",
    "container",
    "cloud",
    "bot",
    "sparkles",
    "code",
    "git-branch",
    "rocket",
    "activity",
    "gauge",
    "cpu",
    "hard-drive",
    "file-text",
    "folder",
    "globe",
    "shield",
    "wrench",
    "zap",
    "refresh-cw",
    "layout-grid",
];

/// The mark for a pinned snippet; unknown names fall back to a document.
pub fn pin_icon(name: &str) -> AnyElement {
    match name {
        "omp" => div()
            .text_base()
            .font_weight(FontWeight::SEMIBOLD)
            .line_height(relative(1.))
            .child("π")
            .into_any_element(),
        "k9s" | "claude" | "opencode" => Icon::default()
            .path(format!("icons/pin-{name}.svg"))
            .small()
            .into_any_element(),
        name if PIN_ICON_NAMES.contains(&name) => lucide(name).small().into_any_element(),
        _ => lucide("file-text").small().into_any_element(),
    }
}
