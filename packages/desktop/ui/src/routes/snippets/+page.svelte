<script lang="ts">
  import { goto } from "$app/navigation";
  import { createQuery, createMutation } from "@tanstack/svelte-query";

  import SnippetsView from "$lib/components/snippets-view.svelte";
  import { snippetListQueryOptions } from "$lib/queries/snippet-queries.js";
  import { deleteSnippet } from "$lib/api/snippets-api.js";
  import { mutationKeys } from "$lib/queries/query-keys.js";
  import type { SnippetRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();

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

  async function handleRun(connection: ConnectionConfig, command: string) {
    const success = await app.runSnippet(connection, command);
    if (success) {
      await goto("/terminal");
    }
    return success;
  }
</script>

<SnippetsView
  snippets={snippets}
  connections={app.connections}
  onNew={() => goto("/snippets/new")}
  onRun={handleRun}
  onEdit={(snippet) => goto(`/snippets/${snippet.id}/edit`)}
  onDelete={handleDelete}
/>
