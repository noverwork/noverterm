<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";

  import ConnectionForm from "$lib/components/connection-form.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import type { SaveConnectionInput } from "$lib/stores/bootstrap.svelte.js";

  const app = getAppShellContext();

  onMount(() => {
    app.resetConnectionFormError();
  });

  async function handleSave(connection: SaveConnectionInput) {
    const saved = await app.saveConnection(connection);
    if (saved) {
      await goto("/connections");
    }
  }
</script>

<ConnectionForm
  connection={null}
  keys={app.keys}
  error={app.connectionFormError}
  isSaving={app.connectionSaving}
  onSave={handleSave}
  onCancel={() => goto("/connections")}
/>
