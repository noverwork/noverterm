<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";

  import PortForwardView from "$lib/components/port-forward-view.svelte";
  import type { SavedPortForwardConfig } from "$lib/app-data-types.js";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();

  onMount(() => {
    void app.portForwardStore.list().catch(() => undefined);
  });

  async function handleNewForward() {
    await goto("/forwards/new");
  }

  async function handleEditForward(forward: SavedPortForwardConfig) {
    await goto(`/forwards/${forward.id}/edit`);
  }
</script>

<PortForwardView
  connections={app.connections}
  savedForwards={app.savedPortForwards}
  forwards={app.portForwardStore.getPortForwards()}
  onNew={handleNewForward}
  onEdit={handleEditForward}
  onForward={app.startSavedPortForward}
  onStop={app.stopPortForward}
  onDeleteSaved={app.deleteSavedPortForward}
  onDeleteRuntime={app.deleteRuntimePortForward}
/>
