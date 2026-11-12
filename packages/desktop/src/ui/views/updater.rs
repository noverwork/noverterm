//! Checks for a newer release and installs it. Releases publish a signed
//! `latest.json` in the format the old Tauri updater read, with the same key,
//! so Tauri builds and this app update from the same files.
// ponytail: Linux deb installs and source builds still open the release page.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{h_flex, v_flex, Disableable as _, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::ui::bridge::backend;
use crate::ui::i18n::{t, tf};
use crate::ui::icons::lucide;
use crate::ui::theme;

const LATEST_MANIFEST: &str =
    "https://github.com/noverwork/noverterm/releases/latest/download/latest.json";
/// The updater key's public half, as the Tauri app embedded it.
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDE4MEMzRjE2MjBBMjcyNDEKUldSQmNxSWdGajhNR0V1ay92YWgzcFlMRG4rMENNeDFEQWZoMEMweXg0Um5PUGYySEpSaURLWjAK";

#[derive(Clone, serde::Deserialize)]
pub struct Release {
    pub version: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub platforms: std::collections::HashMap<String, Download>,
}

#[derive(Clone, serde::Deserialize)]
pub struct Download {
    signature: String,
    url: String,
}

impl Release {
    fn page(&self) -> String {
        format!(
            "https://github.com/noverwork/noverterm/releases/tag/v{}",
            self.version
        )
    }
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
        .get(LATEST_MANIFEST)
        .header(reqwest::header::USER_AGENT, "noverterm")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    Ok(is_newer(&release.version, env!("CARGO_PKG_VERSION")).then_some(release))
}

/// Checks `data` against a Tauri-style signature: base64 of a minisign signature file.
fn verify(public_key: &str, data: &[u8], signature: &str) -> Result<(), String> {
    use base64::Engine as _;
    let decode = |text: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text.trim())
            .map_err(|error| error.to_string())?;
        String::from_utf8(bytes).map_err(|error| error.to_string())
    };
    let key = decode(public_key)?;
    let key_line = key.lines().nth(1).ok_or("malformed public key")?;
    let key = minisign_verify::PublicKey::from_base64(key_line).map_err(|e| e.to_string())?;
    let signature =
        minisign_verify::Signature::decode(&decode(signature)?).map_err(|e| e.to_string())?;
    key.verify(data, &signature, false)
        .map_err(|_| "The update's signature does not match; it was not installed".to_string())
}

/// How this copy of the app gets replaced.
#[derive(Clone)]
enum Target {
    /// The macOS `.app` bundle it runs from.
    Bundle(PathBuf),
    /// The Linux AppImage it runs from.
    AppImage(PathBuf),
    /// Windows: the NSIS installer takes over once the app quits.
    Installer,
}

/// What this copy can update itself as, with its `latest.json` platform key.
/// Linux packages from apt and copies run from source get `None`.
fn current_target() -> Option<(&'static str, Target)> {
    if cfg!(target_os = "macos") {
        let exe = std::env::current_exe().ok()?;
        let bundle = exe.ancestors().nth(3)?;
        (bundle.extension()? == "app").then(|| ("darwin-aarch64", Target::Bundle(bundle.into())))
    } else if cfg!(target_os = "linux") {
        let appimage = std::env::var_os("APPIMAGE")?;
        Some(("linux-x86_64", Target::AppImage(appimage.into())))
    } else if cfg!(target_os = "windows") {
        Some(("windows-x86_64", Target::Installer))
    } else {
        None
    }
}

/// Swaps `bundle` for `new`, keeping the old one until the new one is in place.
fn replace_bundle(bundle: &Path, new: &Path) -> std::io::Result<()> {
    let backup = bundle.with_extension("app.old");
    let _ = std::fs::remove_dir_all(&backup);
    std::fs::rename(bundle, &backup)?;
    if let Err(error) = std::fs::rename(new, bundle) {
        let _ = std::fs::rename(&backup, bundle);
        return Err(error);
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(())
}

/// Downloads and verifies the release, then installs it for `target`.
async fn install(download: Download, target: Target) -> Result<(), String> {
    let data = reqwest::Client::new()
        .get(&download.url)
        .header(reqwest::header::USER_AGENT, "noverterm")
        .timeout(Duration::from_secs(600))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?
        .bytes()
        .await
        .map_err(|error| error.to_string())?;
    verify(PUBLIC_KEY, &data, &download.signature)?;
    match target {
        Target::Bundle(bundle) => install_bundle(&data, &bundle),
        Target::AppImage(appimage) => install_appimage(&data, &appimage),
        Target::Installer => {
            let installer = std::env::temp_dir().join("noverterm-setup.exe");
            std::fs::write(&installer, &data).map_err(|error| error.to_string())?;
            std::process::Command::new(&installer)
                .spawn()
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }
}

fn install_bundle(data: &[u8], bundle: &Path) -> Result<(), String> {
    // Unpack next to the bundle so the swap is a rename on one volume.
    let parent = bundle.parent().ok_or("The app has no parent folder")?;
    let staging = parent.join(".noverterm-update");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir(&staging)
        .map_err(|error| format!("Cannot write to {}: {error}", parent.display()))?;
    let result = (|| {
        let archive = staging.join("update.tar.gz");
        std::fs::write(&archive, data).map_err(|error| error.to_string())?;
        let unpacked = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("-C")
            .arg(&staging)
            .status()
            .map_err(|error| error.to_string())?;
        let new = staging.join("noverterm.app");
        if !unpacked.success() || !new.is_dir() {
            return Err("The update archive has no noverterm.app".to_string());
        }
        replace_bundle(bundle, &new).map_err(|error| error.to_string())?;
        let _ = std::process::Command::new("xattr")
            .args(["-dr", "com.apple.quarantine"])
            .arg(bundle)
            .status();
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

/// Writes the new AppImage beside the old one, then renames it over it; a
/// running AppImage keeps its open file, so this is safe while it runs.
fn install_appimage(data: &[u8], appimage: &Path) -> Result<(), String> {
    let new = appimage.with_extension("update");
    let written = (|| {
        std::fs::write(&new, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&new, appimage)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&new);
    }
    written.map_err(|error| format!("Cannot replace {}: {error}", appimage.display()))
}

/// Starts the updated app once this process has exited, so the two never run
/// together. The Windows installer starts it itself.
fn relaunch(target: &Target) {
    let (opener, path) = match target {
        Target::Bundle(bundle) => ("open \"$0\"", bundle),
        Target::AppImage(appimage) => ("exec \"$0\"", appimage),
        Target::Installer => return,
    };
    let script = format!(
        "while kill -0 {} 2>/dev/null; do sleep 0.2; done; {opener}",
        std::process::id()
    );
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg(path)
        .spawn();
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

#[derive(Default)]
struct Progress {
    installing: bool,
    error: Option<SharedString>,
}

pub fn show(release: Release, window: &mut Window, cx: &mut App) {
    let progress = cx.new(|_| Progress::default());
    window.open_dialog(cx, move |dialog, _, cx| {
        let version = release.version.clone();
        let notes = release
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|notes| !notes.is_empty())
            .map(str::to_string);
        let state = progress.read(cx);
        let (installing, error) = (state.installing, state.error.clone());
        let release = release.clone();
        let progress = progress.clone();
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
                        &[("version", &version)],
                    )),
            )
            .child(
                v_flex()
                    .gap_3()
                    .when_some(notes, |body, notes| {
                        body.child(
                            div()
                                .id("release-notes")
                                .max_h(px(192.))
                                .overflow_y_scroll()
                                .text_sm()
                                .text_color(theme::STENCIL_DIM)
                                .child(notes),
                        )
                    })
                    .when_some(error, |body, error| {
                        body.child(div().text_sm().text_color(theme::RED_TEXT).child(error))
                    }),
            )
            .footer(
                h_flex()
                    .justify_end()
                    .gap_3()
                    .child(
                        Button::new("update-later")
                            .outline()
                            .label(t("Later"))
                            .disabled(installing)
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("update-install")
                            .success()
                            .label(if installing {
                                t("Installing…")
                            } else {
                                t("Install")
                            })
                            .loading(installing)
                            .disabled(installing)
                            .on_click(move |_, window, cx| {
                                start_install(&release, &progress, window, cx)
                            }),
                    ),
            )
    });
}

fn start_install(
    release: &Release,
    progress: &Entity<Progress>,
    window: &mut Window,
    cx: &mut App,
) {
    let found = current_target()
        .and_then(|(platform, target)| Some((release.platforms.get(platform)?.clone(), target)));
    let Some((download, target)) = found else {
        cx.open_url(&release.page());
        window.close_dialog(cx);
        return;
    };
    progress.update(cx, |progress, cx| {
        progress.installing = true;
        progress.error = None;
        cx.notify();
    });
    let progress = progress.clone();
    window
        .spawn(cx, async move |cx| {
            let installed = backend(install(download, target.clone())).await;
            let _ = cx.update(|_, cx| match installed {
                Ok(()) => {
                    relaunch(&target);
                    cx.quit();
                }
                Err(error) => progress.update(cx, |progress, cx| {
                    progress.installing = false;
                    progress.error = Some(error.into());
                    cx.notify();
                }),
            });
        })
        .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn compares_versions_numerically() {
        assert!(is_newer("v0.10.0", "0.9.3"));
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(!is_newer("v1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
    }

    // Made with `update-signer` and a throwaway key.
    const TEST_KEY: &str = include_str!("updater_fixtures/test.key.pub");
    const TEST_SIGNATURE: &str = include_str!("updater_fixtures/payload.tar.gz.sig");
    const TEST_PAYLOAD: &[u8] = include_bytes!("updater_fixtures/payload.tar.gz");

    #[::core::prelude::v1::test]
    fn accepts_only_a_matching_signature() {
        assert_eq!(verify(TEST_KEY, TEST_PAYLOAD, TEST_SIGNATURE), Ok(()));
        let mut tampered = TEST_PAYLOAD.to_vec();
        tampered[0] ^= 1;
        assert!(verify(TEST_KEY, &tampered, TEST_SIGNATURE).is_err());
        // The release key did not sign the test payload.
        assert!(verify(PUBLIC_KEY, TEST_PAYLOAD, TEST_SIGNATURE).is_err());
    }

    #[::core::prelude::v1::test]
    fn replaces_the_appimage_with_an_executable_copy() {
        let directory = tempfile::tempdir().unwrap();
        let appimage = directory.path().join("noverterm.AppImage");
        std::fs::write(&appimage, "old").unwrap();

        install_appimage(b"new", &appimage).unwrap();
        assert_eq!(std::fs::read_to_string(&appimage).unwrap(), "new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&appimage).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        assert!(!appimage.with_extension("update").exists());
    }

    #[::core::prelude::v1::test]
    fn replaces_the_bundle_and_drops_the_old_one() {
        let directory = tempfile::tempdir().unwrap();
        let bundle = directory.path().join("noverterm.app");
        let new = directory.path().join("staging/noverterm.app");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("version"), "old").unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("version"), "new").unwrap();

        replace_bundle(&bundle, &new).unwrap();
        assert_eq!(
            std::fs::read_to_string(bundle.join("version")).unwrap(),
            "new"
        );
        assert!(!bundle.with_extension("app.old").exists());

        // A failed swap puts the old bundle back.
        let missing = directory.path().join("staging/missing.app");
        assert!(replace_bundle(&bundle, &missing).is_err());
        assert_eq!(
            std::fs::read_to_string(bundle.join("version")).unwrap(),
            "new"
        );
    }
}
