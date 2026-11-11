//! The GPUI front end.

pub mod bridge;
pub mod i18n;
pub mod icons;
pub mod shell;
pub mod stencil;
pub mod store;
pub mod terminal;
pub mod theme;
pub mod views;

use std::borrow::Cow;

use gpui_kit::*;

use crate::app::AppHandle;

/// App icons, layered over the full Lucide set.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icon.png" => include_bytes!("../../assets/icon.png"),
            "icons/pin-k9s.svg" => include_bytes!("../../assets/icons/pin-k9s.svg"),
            "icons/pin-claude.svg" => include_bytes!("../../assets/icons/pin-claude.svg"),
            "icons/pin-opencode.svg" => include_bytes!("../../assets/icons/pin-opencode.svg"),
            _ => return gpui_kit::assets::AllAssets.load(path),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::AllAssets.list(path)
    }
}

/// Sets up everything the views need: components, fonts, theme, store, keys.
pub fn init(app: AppHandle, cx: &mut App) {
    gpui_kit::init(cx);
    cx.text_system()
        .add_fonts(vec![Cow::Borrowed(include_bytes!(
            "../../assets/fonts/SarasaTermTC-SemiBold.ttf"
        ))])
        .expect("the bundled terminal font loads");
    theme::apply(cx);
    cx.set_global(bridge::Backend(app.clone()));
    let store = cx.new(|cx| store::Store::new(app, cx));
    cx.set_global(store::Globals { store });
    shell::bind_keys(cx);
}

pub fn open_main_window(options: WindowOptions, cx: &mut App) -> Result<AnyWindowHandle> {
    let (window, _) = gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| shell::Shell::new(window, cx))
    })?;
    Ok(window)
}

pub fn run(app: AppHandle) {
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        init(app, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Maximized(Bounds::centered(
                None,
                size(px(1200.), px(800.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("noverterm".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        open_main_window(options, cx).expect("failed to open the main window");
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
