<script lang="ts">
  import { goto } from "$app/navigation";
  import { createQuery, createMutation } from "@tanstack/svelte-query";

  import SnippetsView from "$lib/components/snippets-view.svelte";
  import { snippetListQueryOptions } from "$lib/queries/snippet-queries.js";
  import { deleteSnippet } from "$lib/api/snippets-api.js";
  import { mutationKeys } from "$lib/queries/query-keys.js";
  import type { SnippetRecord } from "$lib/api/types.js";

  const snippetListQuery = createQuery(() => snippetListQueryOptions());

  const deleteSnippetMutation = createMutation(() => ({
    mutationKey: mutationKeys.deleteSnippet,
    mutationFn: (id: string) => deleteSnippet(id),
    onSuccess: () => {
      snippetListQuery.refetch();
    },
  }));

  const snippets = $derived(snippetListQuery.data ?? []);

  async function handleDelete(snippet: SnippetRecord) {
    await deleteSnippetMutation.mutateAsync(snippet.id);
  }
</script>

<SnippetsView
  snippets={snippets}
  onNew={() => goto("/snippets/new")}
  onDelete={handleDelete}
/>
