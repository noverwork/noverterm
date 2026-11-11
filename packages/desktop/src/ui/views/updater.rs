//! Checks GitHub for a newer release and offers it.
// ponytail: opens the release page instead of installing in place; add a real self-updater when packaging is settled

use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{h_flex, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::theme;

const LATEST_RELEASE: &str = "https://api.github.com/repos/noverwork/noverterm/releases/latest";

#[derive(Clone, serde::Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
}

/// Whether version `latest` is newer than `current`; a leading `v` is ignored.
// ponytail: compares the numeric parts only, so 1.2.0-beta equals 1.2.0;
// use the semver crate if pre-releases are ever published as "latest".
fn is_newer(latest: &str, current: &str) -> bool {
    let parts = |version: &str| -> Vec<u64> {
        version
            .trim_start_matches('v')
            .split('.')
            .map(|part| {
                let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    };
    parts(latest) > parts(current)
}

async fn newer_release() -> Result<Option<Release>, String> {
    let release: Release = reqwest::Client::new()
        .get(LATEST_RELEASE)
        .header(reqwest::header::USER_AGENT, "noverterm")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    Ok(is_newer(&release.tag_name, env!("CARGO_PKG_VERSION")).then_some(release))
}

/// Looks for a newer release and offers it in a dialog; `true` when there is one.
pub fn check(window: &mut Window, cx: &mut App) -> Task<Result<bool, String>> {
    window.spawn(cx, async move |cx| {
        let Some(release) = backend(newer_release()).await? else {
            return Ok(false);
        };
        cx.update(|window, cx| show(release, window, cx))
            .map_err(|error| error.to_string())?;
        Ok(true)
    })
}

pub fn show(release: Release, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, _| {
        let version = release.tag_name.trim_start_matches('v');
        let notes = release
            .body
            .as_deref()
            .map(str::trim)
            .filter(|notes| !notes.is_empty());
        let url = release.html_url.clone();
        dialog
            .title(
                h_flex()
                    .gap_3()
                    .child(
                        lucide("download")
                            .size_5()
                            .flex_shrink_0()
                            .text_color(theme::GREEN_TEXT),
                    )
                    .child(tf(
                        "Noverterm {version} is available. Install it now?",
                        &[("version", version)],
                    )),
            )
            .when_some(notes, |dialog, notes| {
                dialog.child(
                    div()
                        .id("release-notes")
                        .max_h(px(192.))
                        .overflow_y_scroll()
                        .text_sm()
                        .text_color(theme::STENCIL_DIM)
                        .child(notes.to_string()),
                )
            })
            .footer(
                h_flex()
                    .justify_end()
                    .gap_3()
                    .child(
                        Button::new("update-later")
                            .outline()
                            .label(t("Later"))
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("update-install")
                            .success()
                            .label(t("Install"))
                            .on_click(move |_, window, cx| {
                                cx.open_url(&url);
                                window.close_dialog(cx);
                            }),
                    ),
            )
    });
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("v0.10.0", "0.9.3"));
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("v1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
    }
}
