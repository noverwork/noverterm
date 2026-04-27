<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";

  import SshKeyForm from "$lib/components/ssh-key-form.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();
  const keyRecord = $derived(app.keys.find((candidate) => candidate.id === $page.params.id) ?? null);

  async function handleUpdate(keyId: string, name: string, privateKey?: string, passphrase?: string) {
    await app.updateKey(keyId, name, privateKey, passphrase);
    await goto("/keys");
  }
</script>

{#if keyRecord}
  <SshKeyForm
    {keyRecord}
    onSave={app.saveKey}
    onUpdate={handleUpdate}
    onCancel={() => goto("/keys")}
  />
{:else}
  <div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
    <section class="ide-panel flex min-h-0 flex-1 flex-col items-center justify-center p-8 text-center text-white">
      <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300">
        <AlertCircle class="size-7" />
      </div>
      <h1 class="mt-5 text-xl font-semibold">SSH key not found</h1>
      <p class="mt-2 max-w-md text-sm leading-6 text-slate-400">This saved SSH key may have been deleted.</p>
      <Button class="mt-6 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" onclick={() => goto("/keys")}>
        Back to SSH keys
      </Button>
    </section>
  </div>
{/if}
