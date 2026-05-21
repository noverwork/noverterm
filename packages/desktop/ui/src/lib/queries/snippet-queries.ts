import { createMutation, createQuery } from "@tanstack/svelte-query";
import type { QueryClient } from "@tanstack/svelte-query";

import {
  listSnippets,
  createSnippet,
  updateSnippet,
  deleteSnippet,
} from "$lib/api/snippets-api.js";
import type { SnippetWriteRequest } from "$lib/api/types.js";
import { queryKeys, mutationKeys } from "$lib/queries/query-keys.js";

export function snippetListQueryOptions() {
  return {
    queryKey: queryKeys.snippets(),
    queryFn: () => listSnippets(),
  };
}

export function useSnippetListQuery(queryClient: QueryClient) {
  return createQuery(
    () => ({
      ...snippetListQueryOptions(),
    }),
    () => queryClient,
  );
}

export function useCreateSnippetMutation(queryClient: QueryClient) {
  return createMutation(
    () => ({
      mutationKey: mutationKeys.createSnippet,
      mutationFn: (input: SnippetWriteRequest) => createSnippet(input),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: queryKeys.snippets() });
      },
    }),
    () => queryClient,
  );
}

export function useUpdateSnippetMutation(queryClient: QueryClient) {
  return createMutation(
    () => ({
      mutationKey: mutationKeys.updateSnippet,
      mutationFn: ({ id, input }: { id: string; input: SnippetWriteRequest }) =>
        updateSnippet(id, input),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: queryKeys.snippets() });
      },
    }),
    () => queryClient,
  );
}

export function useDeleteSnippetMutation(queryClient: QueryClient) {
  return createMutation(
    () => ({
      mutationKey: mutationKeys.deleteSnippet,
      mutationFn: (id: string) => deleteSnippet(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: queryKeys.snippets() });
      },
    }),
    () => queryClient,
  );
}
