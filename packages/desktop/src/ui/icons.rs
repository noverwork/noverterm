//! Icons by their Lucide file name, plus the hand-drawn shortcut marks.

use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::*;

/// A Lucide icon by its kebab-case name, e.g. `folder-open`.
pub fn lucide(name: &str) -> Icon {
    Icon::default().path(format!("icons/{name}.svg"))
}

/// Icons a snippet can be pinned to the sidebar with.
pub const PIN_ICON_NAMES: [&str; 89] = [
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
    "box",
    "boxes",
    "package",
    "network",
    "router",
    "wifi",
    "plug",
    "cable",
    "monitor",
    "laptop",
    "smartphone",
    "key",
    "lock",
    "bug",
    "flask-conical",
    "hammer",
    "settings",
    "sliders-horizontal",
    "play",
    "square",
    "power",
    "clock",
    "calendar",
    "bell",
    "mail",
    "message-square",
    "search",
    "download",
    "upload",
    "archive",
    "flame",
    "star",
    "heart",
    "flag",
    "bookmark",
    "music",
    "camera",
    "image",
    "film",
    "chart-line",
    "chart-bar",
    "list",
    "notebook-pen",
    "coffee",
    "gamepad-2",
    "map",
    "compass",
    "anchor",
    "leaf",
    "sun",
    "moon",
    "github",
    "workflow",
    "webhook",
    "binary",
    "braces",
    "hash",
    "at-sign",
    "dollar-sign",
    "shopping-cart",
    "users",
    "user",
];

/// Colors a pinned snippet's icon can take, as `#rrggbb`.
pub const PIN_COLORS: [&str; 8] = [
    "#f87171", "#fb923c", "#facc15", "#4ade80", "#2dd4bf", "#38bdf8", "#a78bfa", "#f472b6",
];

/// Parses a `#rrggbb` pin color; anything else is `None`.
pub fn pin_color(value: &str) -> Option<Rgba> {
    let hex = value.strip_prefix('#').filter(|hex| hex.len() == 6)?;
    u32::from_str_radix(hex, 16).ok().map(rgb)
}

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

#[cfg(test)]
mod tests {
    use gpui_kit::AssetSource as _;

    #[test]
    fn every_pin_icon_has_an_svg() {
        let missing: Vec<_> = super::PIN_ICON_NAMES
            .iter()
            .filter(|name| **name != "omp")
            .map(|name| match *name {
                "k9s" | "claude" | "opencode" => format!("icons/pin-{name}.svg"),
                name => format!("icons/{name}.svg"),
            })
            .filter(|path| crate::ui::Assets.load(path).ok().flatten().is_none())
            .collect();
        assert!(missing.is_empty(), "missing icons: {missing:?}");
    }
}
