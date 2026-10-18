<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { createMutation } from "@tanstack/svelte-query";

  import SnippetForm from "$lib/components/snippet-form.svelte";
  import { createSnippet } from "$lib/api/snippets-api.js";
  import { mutationKeys } from "$lib/queries/query-keys.js";
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

  async function handleSave(
    hostId: string | null,
    title: string,
    body: string,
    pinIcon: string | null,
  ) {
    await createSnippetMutation.mutateAsync({
      host_id: hostId,
      title,
      body,
      pin_icon: pinIcon,
    });
  }
</script>

<SnippetForm
  hosts={hosts}
  defaultShowOnPanel={$page.url.searchParams.has("shortcut")}
  onSave={handleSave}
  onCancel={() => goto("/snippets")}
/>
