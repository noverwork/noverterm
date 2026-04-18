/**
 * Tauri-backed query/mutation wrappers for @tanstack/svelte-query.
 *
 * This module wraps Tauri invoke calls so they can be used with
 * createQuery / createMutation throughout the UI layer.
 */

import { commands as tauriCommands } from "$lib/bindings.js";
import type {
  ConnectionConfig,
  SaveConnectionInput,
  TerminalConfig,
} from "$lib/stores/bootstrap.svelte.js";
import { createBootstrapStore } from "$lib/stores/bootstrap.svelte.js";

// ---------------------------------------------------------------------------
// Query keys – centralised so invalidations stay consistent
// ---------------------------------------------------------------------------

export const queryKeys = {
  bootstrap: ["bootstrap"] as const,
  metadata: ["bootstrap", "metadata"] as const,
  connections: ["bootstrap", "connections"] as const,
  terminalConfig: ["bootstrap", "terminal-config"] as const,
  keys: ["bootstrap", "keys"] as const,
  settings: ["bootstrap", "settings"] as const,
};

// ---------------------------------------------------------------------------
// Bootstrap store instance (shared singleton for mutations)
// ---------------------------------------------------------------------------

const bootstrapStore = createBootstrapStore();

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

export async function fetchMetadata() {
  return bootstrapStore.refreshMetadata();
}

// ---------------------------------------------------------------------------
// Mutation helpers – thin wrappers that return the same shapes the existing
// components already expect, so the rest of the codebase needs minimal changes.
// ---------------------------------------------------------------------------

export async function mutateLogin(username: string, password: string) {
  await bootstrapStore.login(username, password);
  return bootstrapStore;
}

export async function mutateLogout() {
  await bootstrapStore.logout();
}

export async function mutateSaveConnection(input: SaveConnectionInput) {
  return bootstrapStore.saveConnection(input);
}

export async function mutateDeleteConnection(connection: ConnectionConfig) {
  await bootstrapStore.deleteConnection(connection);
}

export async function mutateSaveTerminalConfig(config: TerminalConfig) {
  await bootstrapStore.saveTerminalConfig(config);
}
