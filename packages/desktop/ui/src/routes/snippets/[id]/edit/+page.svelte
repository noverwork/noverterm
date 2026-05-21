<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { createMutation, createQuery } from "@tanstack/svelte-query";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";
  import Loader2 from "@lucide/svelte/icons/loader-2";

  import { updateSnippet } from "$lib/api/snippets-api.js";
  import type { SnippetRecord } from "$lib/api/types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import SnippetForm from "$lib/components/snippet-form.svelte";
  import { mutationKeys } from "$lib/queries/query-keys.js";
  import { snippetListQueryOptions } from "$lib/queries/snippet-queries.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();
  const snippetId = $derived($page.params.id ?? "");
  const snippetListQuery = createQuery(() => snippetListQueryOptions());
  const snippet = $derived(
    snippetListQuery.data?.find((candidate) => candidate.id === snippetId) ?? null,
  );

  const updateSnippetMutation = createMutation(() => ({
    mutationKey: mutationKeys.updateSnippet,
    mutationFn: ({ id, input }: { id: string; input: { host_id: string; title: string; body: string } }) =>
      updateSnippet(id, input),
    onSuccess: () => {
      snippetListQuery.refetch();
      void goto("/snippets");
    },
  }));

  async function handleSave(hostId: string, title: string, body: string) {
    await updateSnippetMutation.mutateAsync({
      id: snippetId,
      input: { host_id: hostId, title, body },
    });
  }
</script>

{#if snippet}
  <SnippetForm
    hosts={app.connections}
    snippet={snippet as SnippetRecord}
    onSave={handleSave}
    onCancel={() => goto("/snippets")}
  />
{:else if snippetListQuery.isPending}
  <div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
    <section class="ide-panel flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-white">
      <Loader2 class="size-7 animate-spin text-cyan-200" />
      <p class="mt-4 text-sm text-slate-400">Loading snippet…</p>
    </section>
  </div>
{:else}
  <div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
    <section class="ide-panel flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-white">
      <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300">
        <AlertCircle class="size-7" />
      </div>
      <h1 class="mt-5 text-xl font-semibold">Snippet not found</h1>
      <p class="mt-2 max-w-md text-sm leading-6 text-slate-400">This snippet may have been deleted.</p>
      <Button class="mt-6 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" onclick={() => goto("/snippets")}>
        Back to snippets
      </Button>
    </section>
  </div>
{/if}
