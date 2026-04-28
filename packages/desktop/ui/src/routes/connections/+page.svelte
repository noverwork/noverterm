<script lang="ts">
  import { goto } from "$app/navigation";

  import ConnectionsView from "$lib/components/connections-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

  const app = getAppShellContext();

  async function handleSelectConnection(connection: ConnectionConfig) {
    await goto("/");
    await app.connectSavedConnection(connection);
  }

  async function handleEditConnection(connection: ConnectionConfig) {
    app.resetConnectionFormError();
    await goto(`/connections/${connection.id}/edit`);
  }

  async function handleNewConnection() {
    app.resetConnectionFormError();
    await goto("/connections/new");
  }
</script>

<ConnectionsView
  connections={app.connections}
  onSelect={handleSelectConnection}
  onEdit={handleEditConnection}
  onNew={handleNewConnection}
  onDelete={app.deleteConnection}
/>
