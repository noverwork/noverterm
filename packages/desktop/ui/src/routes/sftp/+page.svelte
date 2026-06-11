<script lang="ts">
  import SftpView from "$lib/components/sftp-view.svelte";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";

  async function handleConnect(options: {
    host: string;
    port: number;
    username: string;
    password?: string;
    privateKeyPath?: string;
    passphrase?: string;
  }) {
    await sftpStore.connectDirect(options);
  }

  async function handleDisconnect() {
    await sftpStore.disconnect();
  }
</script>

<SftpView
  onConnect={handleConnect}
  onDisconnect={handleDisconnect}
/>
