<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";

  import ConnectionForm from "$lib/components/connection-form.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
import type { SaveConnectionInput } from "$lib/app-data-types.js";

  const app = getAppShellContext();
  const connection = $derived(app.connections.find((candidate) => candidate.id === $page.params.id) ?? null);

  onMount(() => {
    app.resetConnectionFormError();
  });

  async function handleSave(input: SaveConnectionInput) {
    const saved = await app.saveConnection(input);
    if (saved) {
      await goto("/connections");
    }
  }
</script>

{#if connection}
  <ConnectionForm
    {connection}
    keys={app.keys}
    error={app.connectionFormError}
    isSaving={app.connectionSaving}
    onSave={handleSave}
    onCancel={() => goto("/connections")}
  />
{:else}
  <div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
    <section class="flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-stencil">
      <div class="mx-auto grid size-14 place-items-center rounded-[2px] border border-seal/60 text-red-300">
        <AlertCircle class="size-7" />
      </div>
      <h1 class="mt-5 text-xl font-semibold">Connection not found</h1>
      <p class="mt-2 max-w-md text-sm leading-6 text-stencil-mute">This saved connection may have been deleted.</p>
      <Button class="mt-6 rounded-[2px] bg-stencil text-port hover:bg-white" onclick={() => goto("/connections")}>
        Back to connections
      </Button>
    </section>
  </div>
{/if}
