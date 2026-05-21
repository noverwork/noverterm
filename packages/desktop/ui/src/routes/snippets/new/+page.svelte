<script lang="ts">
  import { goto } from "$app/navigation";
  import { createMutation, createQuery } from "@tanstack/svelte-query";

  import SnippetForm from "$lib/components/snippet-form.svelte";
  import { createSnippet } from "$lib/api/snippets-api.js";
  import { mutationKeys, queryKeys } from "$lib/queries/query-keys.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();
  const hosts = $derived(app.connections as ConnectionConfig[]);

  const createSnippetMutation = createMutation(() => ({
    mutationKey: mutationKeys.createSnippet,
    mutationFn: createSnippet,
    onSuccess: () => {
      void goto("/snippets");
    },
  }));

  async function handleSave(hostId: string, title: string, body: string) {
    await createSnippetMutation.mutateAsync({ host_id: hostId, title, body });
  }
</script>

<SnippetForm
  hosts={hosts}
  onSave={handleSave}
  onCancel={() => goto("/snippets")}
/>
