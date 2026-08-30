import { invoke } from "@tauri-apps/api/core";

import type { SnippetRecord, SnippetWriteRequest } from "./types.js";

export async function listSnippets(): Promise<SnippetRecord[]> {
  return invoke<SnippetRecord[]>("snippet_list");
}

export async function getSnippet(id: string): Promise<SnippetRecord> {
  return invoke<SnippetRecord>("snippet_get", { id });
}

export async function createSnippet(
  snippet: SnippetWriteRequest,
): Promise<SnippetRecord> {
  return invoke<SnippetRecord>("snippet_create", { snippet });
}

export async function updateSnippet(
  id: string,
  snippet: SnippetWriteRequest,
): Promise<SnippetRecord> {
  return invoke<SnippetRecord>("snippet_update", { id, snippet });
}

export async function deleteSnippet(id: string): Promise<void> {
  await invoke("snippet_delete", { id });
}
