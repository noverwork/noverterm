<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";

  import PortForwardForm from "$lib/components/port-forward-form.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import type { SavePortForwardInput } from "$lib/app-data-types.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();
  const forward = $derived(
    app.savedPortForwards.find((candidate) => candidate.id === $page.params.id) ??
      null,
  );

  async function handleSave(input: SavePortForwardInput) {
    await app.savePortForward(input);
    await goto("/forwards");
  }
</script>

{#if forward}
  <PortForwardForm
    connections={app.connections}
    {forward}
    onSave={handleSave}
    onCancel={() => goto("/forwards")}
  />
{:else}
  <div
    class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8"
  >
    <section
      class="ide-panel flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-white"
    >
      <div
        class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300"
      >
        <AlertCircle class="size-7" />
      </div>
      <h1 class="mt-5 text-xl font-semibold">Port forward not found</h1>
      <p class="mt-2 max-w-md text-sm leading-6 text-slate-400">
        This saved port forward may have been deleted.
      </p>
      <Button
        class="mt-6 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
        onclick={() => goto("/forwards")}
      >
        Back to port forwards
      </Button>
    </section>
  </div>
{/if}
