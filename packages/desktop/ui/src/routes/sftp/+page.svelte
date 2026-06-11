<script lang="ts">
  import SftpView from "$lib/components/sftp-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import { createDirectSshConnectInput } from "$lib/services/ssh-connection-input.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";

  const app = getAppShellContext();

  async function handleConnect(connection: ConnectionConfig) {
    const input = await createDirectSshConnectInput(connection);
    await sftpStore.connectDirect({
      host: input.host,
      port: input.port,
      username: input.username,
      password: input.password ?? undefined,
      privateKey: input.private_key ?? undefined,
      passphrase: input.passphrase ?? undefined,
    });
  }

  async function handleDisconnect() {
    await sftpStore.disconnect();
  }
</script>

<SftpView
  connections={app.connections}
  onConnect={handleConnect}
  onDisconnect={handleDisconnect}
/>
