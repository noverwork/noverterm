import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { t } from "$lib/i18n/index.svelte.js";

function updateNotes(version: string, body?: string): string {
  const details = body?.trim();
  if (!details) {
    return t("Noverterm {version} is available. Install it now?", { version });
  }

  return `${t("Noverterm {version} is available. Install it now?", { version })}\n\n${details}`;
}

export async function checkForAppUpdate(): Promise<void> {
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
}
