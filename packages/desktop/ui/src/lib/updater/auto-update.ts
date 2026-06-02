import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";

function updateNotes(version: string, body?: string): string {
  const details = body?.trim();
  if (!details) {
    return `Noverterm ${version} is available. Install it now?`;
  }

  return `Noverterm ${version} is available. Install it now?\n\n${details}`;
}

export async function checkForAppUpdate(): Promise<void> {
  try {
    const update = await check();
    if (!update?.available) {
      return;
    }

    const shouldInstall = window.confirm(updateNotes(update.version, update.body));
    if (!shouldInstall) {
      return;
    }

    await update.downloadAndInstall();
    await relaunch();
  } catch (error) {
    console.warn("Failed to check for app updates", error);
  }
}
