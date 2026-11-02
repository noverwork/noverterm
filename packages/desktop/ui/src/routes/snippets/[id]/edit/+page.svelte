<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { createMutation, createQuery } from "@tanstack/svelte-query";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";
  import Loader2 from "@lucide/svelte/icons/loader-2";

  import { updateSnippet } from "$lib/api/snippets-api.js";
  import type { SnippetRecord, SnippetWriteRequest } from "$lib/api/types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import SnippetForm from "$lib/components/snippet-form.svelte";
  import { mutationKeys } from "$lib/queries/query-keys.js";
  import { snippetListQueryOptions } from "$lib/queries/snippet-queries.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import { t } from "$lib/i18n/index.svelte.js";

  const app = getAppShellContext();
  const snippetId = $derived($page.params.id ?? "");
  const snippetListQuery = createQuery(() => snippetListQueryOptions());
  const snippet = $derived(
    snippetListQuery.data?.find((candidate) => candidate.id === snippetId) ?? null,
  );

  const updateSnippetMutation = createMutation(() => ({
    mutationKey: mutationKeys.updateSnippet,
    mutationFn: ({ id, input }: { id: string; input: SnippetWriteRequest }) =>
      updateSnippet(id, input),
    onSuccess: () => {
      snippetListQuery.refetch();
      void goto("/snippets");
    },
  }));

  async function handleSave(
    hostId: string | null,
    title: string,
    body: string,
    pinIcon: string | null,
  ) {
    await updateSnippetMutation.mutateAsync({
      id: snippetId,
      input: { host_id: hostId, title, body, pin_icon: pinIcon },
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
    <section class="flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-stencil">
      <Loader2 class="size-7 animate-spin text-stencil" />
      <p class="mt-4 text-sm text-stencil-mute">{t("Loading snippet…")}</p>
    </section>
  </div>
{:else}
  <div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
    <section class="flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-stencil">
      <div class="mx-auto grid size-14 place-items-center rounded-[2px] border border-seal/60 text-red-300">
        <AlertCircle class="size-7" />
      </div>
      <h1 class="mt-5 text-xl font-semibold">{t("Snippet not found")}</h1>
      <Button class="mt-6 rounded-[2px] bg-stencil text-port hover:bg-white" onclick={() => goto("/snippets")}>
        {t("Back to snippets")}
      </Button>
    </section>
  </div>
{/if}
