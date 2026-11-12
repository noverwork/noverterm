//! English is the source text; other locales look it up in a table. Text
//! missing from the table shows in English rather than breaking.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use gpui_kit::SharedString;

/// Settings key the chosen locale is saved under.
pub const LOCALE_SETTING: &str = "noverterm.locale";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    ZhTw,
}

impl Locale {
    pub const ALL: [Locale; 2] = [Locale::En, Locale::ZhTw];

    pub fn code(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::ZhTw => "zh-TW",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Locale::En => "English",
            Locale::ZhTw => "繁體中文",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|locale| locale.code() == code)
    }

    /// The system language, for a first launch with nothing saved.
    pub fn system() -> Self {
        // ponytail: reads LANG, plus AppleLanguages on macOS where GUI apps
        // get no LANG. Windows falls back to English; use GetUserDefaultLocaleName
        // if that matters.
        let mut language = std::env::var("LANG").unwrap_or_default();
        if cfg!(target_os = "macos") {
            if let Ok(output) = std::process::Command::new("defaults")
                .args(["read", "-g", "AppleLanguages"])
                .output()
            {
                language = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .nth(1)
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }
        }
        if language.to_lowercase().starts_with("zh") {
            Locale::ZhTw
        } else {
            Locale::En
        }
    }
}

static ZH_TW: AtomicBool = AtomicBool::new(false);

pub fn locale() -> Locale {
    if ZH_TW.load(Ordering::Relaxed) {
        Locale::ZhTw
    } else {
        Locale::En
    }
}

pub fn set_locale(locale: Locale) {
    ZH_TW.store(locale == Locale::ZhTw, Ordering::Relaxed);
}

fn zh_tw() -> &'static HashMap<String, &'static str> {
    static TABLE: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let table: HashMap<String, String> =
            serde_json::from_str(include_str!("i18n/zh-tw.json")).expect("zh-tw.json is valid");
        table
            .into_iter()
            .map(|(english, translated)| (english, &*translated.leak()))
            .collect()
    })
}

fn lookup(text: &'static str) -> &'static str {
    match locale() {
        Locale::En => text,
        Locale::ZhTw => zh_tw().get(text).copied().unwrap_or(text),
    }
}

/// Translates English UI text.
pub fn t(text: &'static str) -> SharedString {
    SharedString::new_static(lookup(text))
}

/// Like [`t`], filling `{name}` placeholders from `params`.
pub fn tf(text: &'static str, params: &[(&str, &str)]) -> SharedString {
    let mut filled = lookup(text).to_string();
    for (name, value) in params {
        filled = filled.replace(&format!("{{{name}}}"), value);
    }
    filled.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_and_fills_placeholders() {
        set_locale(Locale::ZhTw);
        assert_eq!(t("Settings").as_ref(), "設定");
        assert_eq!(t("not in the table").as_ref(), "not in the table");
        assert_eq!(
            tf("Version {version}", &[("version", "1.2")]).as_ref(),
            "版本 1.2"
        );
        set_locale(Locale::En);
        assert_eq!(
            tf("Version {version}", &[("version", "1.2")]).as_ref(),
            "Version 1.2"
        );
    }
}
