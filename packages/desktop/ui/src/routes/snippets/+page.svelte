<script lang="ts">
  import { goto } from "$app/navigation";
  import { getQueryClient } from "@tanstack/svelte-query";

  import SnippetsView from "$lib/components/snippets-view.svelte";
  import {
    useSnippetListQuery,
    useDeleteSnippetMutation,
  } from "$lib/queries/snippet-queries.js";
  import type { SnippetRecord } from "$lib/api/types.js";

  const queryClient = getQueryClient();
  const snippetListQuery = useSnippetListQuery(queryClient);
  const deleteSnippetMutation = useDeleteSnippetMutation(queryClient);

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
