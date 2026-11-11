import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

export interface DownloadProgress {
  downloaded: number;
  /** `null` when the server sends no Content-Length. */
  total: number | null;
  /** The download is done and the installer is running. */
  finished: boolean;
}

/**
 * The update waiting for the user's answer in `UpdateDialog`, and its download once accepted.
 * `window.confirm` cannot ask: WKWebView on macOS answers it with "cancel" without showing anything.
 */
export const updater = $state<{ available: Update | null; progress: DownloadProgress | null }>({
  available: null,
  progress: null,
});

/** Checks for an update and offers it in `UpdateDialog`; `true` when one is available. */
export async function checkForAppUpdate(): Promise<boolean> {
  const update = await check();
  if (update) {
    updater.available = update;
  }
  return update !== null;
}

export async function installUpdate(update: Update): Promise<void> {
  updater.progress = { downloaded: 0, total: null, finished: false };
  try {
    await update.downloadAndInstall((event) => {
      const progress = updater.progress!;
      switch (event.event) {
        case "Started":
          progress.total = event.data.contentLength ?? null;
          break;
        case "Progress":
          progress.downloaded += event.data.chunkLength;
          break;
        case "Finished":
          progress.finished = true;
          break;
      }
    });
  } catch (cause) {
    updater.progress = null;
    throw cause;
  }
  await relaunch();
}
