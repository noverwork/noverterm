import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

/**
 * The update waiting for the user's answer in `UpdateDialog`.
 * `window.confirm` cannot ask: WKWebView on macOS answers it with "cancel" without showing anything.
 */
export const updater = $state<{ available: Update | null }>({ available: null });

/** Checks for an update and offers it in `UpdateDialog`; `true` when one is available. */
export async function checkForAppUpdate(): Promise<boolean> {
  const update = await check();
  if (update) {
    updater.available = update;
  }
  return update !== null;
}

export async function installUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall();
  await relaunch();
}
