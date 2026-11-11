//! The app's own palette (see DESIGN.md), applied over the component theme.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{px, App, Rgba};

const fn hex(value: u32) -> Rgba {
    Rgba {
        r: ((value >> 16) & 0xff) as f32 / 255.,
        g: ((value >> 8) & 0xff) as f32 / 255.,
        b: (value & 0xff) as f32 / 255.,
        a: 1.,
    }
}

const fn white(alpha: f32) -> Rgba {
    Rgba {
        r: 1.,
        g: 1.,
        b: 1.,
        a: alpha,
    }
}

pub const TERMINAL_INK: Rgba = hex(0x000000);
pub const PORT: Rgba = hex(0x101317);
pub const RAIL: Rgba = hex(0x0b0e11);
pub const RIB: Rgba = hex(0x1a1f25);
pub const STENCIL: Rgba = hex(0xe8e4d8);
pub const STENCIL_DIM: Rgba = hex(0xc1beb5);
pub const STENCIL_MUTE: Rgba = hex(0x9f9d96);
pub const STENCIL_FAINT: Rgba = hex(0x82807b);
pub const SEAL: Rgba = hex(0xa33b28);
pub const SEAL_HOVER: Rgba = hex(0xb4432f);
pub const AMBER: Rgba = hex(0xc9952e);
pub const RED_TEXT: Rgba = hex(0xfca5a5);
pub const AMBER_TEXT: Rgba = hex(0xe6c27a);
pub const GREEN_TEXT: Rgba = hex(0x86efac);

/// The 10% white rule used for most borders.
pub const fn hairline() -> Rgba {
    white(0.1)
}

/// Hover wash over dark surfaces.
pub const fn wash() -> Rgba {
    white(0.05)
}

pub const RADIUS: gpui_kit::Pixels = px(2.);

pub fn apply(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.radius = RADIUS;
        theme.radius_lg = RADIUS;
        theme.font_family = crate::ui::terminal::FONT_FAMILY.into();
        // The component library uses this as the rem size: 16px keeps every
        // rem-based size equal to the old CSS layout.
        theme.font_size = px(16.);
        theme.background = PORT.into();
        theme.foreground = STENCIL.into();
        theme.border = hairline().into();
        theme.input = hairline().into();
        theme.ring = STENCIL.into();
        theme.primary = STENCIL.into();
        theme.primary_hover = STENCIL_DIM.into();
        theme.primary_active = STENCIL_DIM.into();
        theme.primary_foreground = PORT.into();
        theme.secondary = RIB.into();
        theme.secondary_hover = white(0.08).into();
        theme.secondary_active = white(0.12).into();
        theme.secondary_foreground = STENCIL.into();
        theme.muted = RIB.into();
        theme.muted_foreground = STENCIL_MUTE.into();
        theme.accent = white(0.08).into();
        theme.accent_foreground = STENCIL.into();
        theme.popover = RAIL.into();
        theme.popover_foreground = STENCIL.into();
        theme.danger = SEAL.into();
        theme.danger_hover = SEAL_HOVER.into();
        theme.danger_active = SEAL_HOVER.into();
        theme.danger_foreground = STENCIL.into();
        theme.sidebar = RAIL.into();
        theme.sidebar_foreground = STENCIL.into();
        theme.sidebar_border = RIB.into();
        theme.list_hover = wash().into();
        theme.list_active = white(0.08).into();
        theme.scrollbar_thumb = Rgba { a: 0.26, ..STENCIL }.into();
        theme.scrollbar_thumb_hover = Rgba { a: 0.42, ..STENCIL }.into();
        theme.selection = Rgba { a: 0.28, ..STENCIL }.into();
    });
}
