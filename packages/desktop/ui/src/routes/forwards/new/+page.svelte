<script lang="ts">
  import { goto } from "$app/navigation";

  import PortForwardForm from "$lib/components/port-forward-form.svelte";
  import type { PortForwardWriteRequest } from "$lib/api/types.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();

  async function handleSave(input: { id?: string } & PortForwardWriteRequest) {
    await app.savePortForward(input);
    await goto("/forwards");
  }
</script>

<PortForwardForm
  connections={app.connections}
  forward={null}
  onSave={handleSave}
  onCancel={() => goto("/forwards")}
/>
