// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
const MACOS_BUNDLE_IDENTIFIER: &str = "com.noverwork.noverterm";

fn main() {
    #[cfg(target_os = "macos")]
    disable_press_and_hold_for_app();

    desktop_lib::run()
}

#[cfg(target_os = "macos")]
fn disable_press_and_hold_for_app() {
    let _ = std::process::Command::new("defaults")
        .args([
            "write",
            MACOS_BUNDLE_IDENTIFIER,
            "ApplePressAndHoldEnabled",
            "-bool",
            "false",
        ])
        .status();
}
