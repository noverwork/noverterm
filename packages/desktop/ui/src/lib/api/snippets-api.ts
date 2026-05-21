import {
  requestWithAuth,
  requestNoContentWithAuth,
  withAuthorizedRetry,
} from "./api-client.js";

import type { SnippetRecord, SnippetWriteRequest } from "./types.js";

export async function listSnippets(): Promise<SnippetRecord[]> {
  return withAuthorizedRetry(async (accessToken) => {
    return requestWithAuth<SnippetRecord[]>("/snippets", accessToken);
  });
}

export async function getSnippet(id: string): Promise<SnippetRecord> {
  return withAuthorizedRetry(async (accessToken) => {
    return requestWithAuth<SnippetRecord>(`/snippets/${id}`, accessToken);
  });
}

export async function createSnippet(
  input: SnippetWriteRequest,
): Promise<SnippetRecord> {
  return withAuthorizedRetry(async (accessToken) => {
    return requestWithAuth<SnippetRecord>("/snippets", accessToken, {
      method: "POST",
      body: JSON.stringify(input),
    });
  });
}

export async function updateSnippet(
  id: string,
  input: SnippetWriteRequest,
): Promise<SnippetRecord> {
  return withAuthorizedRetry(async (accessToken) => {
    return requestWithAuth<SnippetRecord>(`/snippets/${id}`, accessToken, {
      method: "PUT",
      body: JSON.stringify(input),
    });
  });
}

export async function deleteSnippet(id: string): Promise<void> {
  await withAuthorizedRetry(async (accessToken) => {
    await requestNoContentWithAuth(`/snippets/${id}`, accessToken, {
      method: "DELETE",
    });
  });
}
