<script lang="ts">
  import { onMount } from "svelte";

  import PortForwardView from "$lib/components/port-forward-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";

  const app = getAppShellContext();

  onMount(() => {
    void app.portForwardStore.list().catch(() => undefined);
  });
</script>

<PortForwardView
  connections={app.connections}
  savedForwards={app.savedPortForwards}
  forwards={app.portForwardStore.getPortForwards()}
  onSave={app.savePortForward}
  onForward={app.startSavedPortForward}
  onStop={app.stopPortForward}
  onDeleteSaved={app.deleteSavedPortForward}
  onDeleteRuntime={app.deleteRuntimePortForward}
/>
