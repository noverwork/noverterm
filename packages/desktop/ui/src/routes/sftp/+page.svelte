<script lang="ts">
  import SftpView from "$lib/components/sftp-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import { createDirectSshConnectInput } from "$lib/services/ssh-connection-input.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";

  const app = getAppShellContext();

  let openingActiveSftp = $state(false);
  let attemptedActiveSshSessionId = $state<string | null>(null);

  async function openActiveSessionSftp(sessionId: string): Promise<void> {
    if (openingActiveSftp) {
      return;
    }

    openingActiveSftp = true;
    try {
      if (sftpStore.sftpSessionId && sftpStore.sshSessionId !== sessionId) {
        await sftpStore.closeSftp();
      }

      if (!sftpStore.sftpSessionId || sftpStore.sshSessionId !== sessionId) {
        console.info("[SFTP][Route] opening SFTP for active SSH session", {
          sessionId,
        });
        await sftpStore.openSftp(sessionId);
      }
    } finally {
      openingActiveSftp = false;
    }
  }

  $effect(() => {
    const session = app.activeSession;
    if (session?.type !== "ssh" || session.status !== "connected") {
      attemptedActiveSshSessionId = null;
      return;
    }

    if (sftpStore.isDirectConnection) {
      return;
    }

    if (sftpStore.sftpSessionId && sftpStore.sshSessionId === session.id) {
      attemptedActiveSshSessionId = null;
      return;
    }

    if (openingActiveSftp || attemptedActiveSshSessionId === session.id) {
      return;
    }

    attemptedActiveSshSessionId = session.id;
    void openActiveSessionSftp(session.id);
  });

  async function handleConnect(connection: ConnectionConfig) {
    attemptedActiveSshSessionId = null;
    if (sftpStore.sftpSessionId) {
      await sftpStore.closeSftp();
    }

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
