<script lang="ts">
  import SftpView from "$lib/components/sftp-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import { machineKey, sftpStore, type SftpPane } from "$lib/stores/sftp.svelte.js";
  import { createDirectSshConnectInput } from "$lib/services/ssh-connection-input.js";
  import { commands, type HostTrustConfirmation } from "../../bindings.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import type { Session } from "$lib/stores/session.svelte.js";

  const app = getAppShellContext();
  const right = sftpStore.right;

  let openingActiveSftp = $state(false);

  const sshSessions = $derived(
    (app.activeSessions ?? []).filter(
      (session) => session.type === "ssh" && session.status === "connected",
    ),
  );

  /** A choice made on the right pane stops it following the active terminal. */
  function claimPane(pane: SftpPane): void {
    pane.attemptGeneration += 1;
    pane.trustConfirming = false;
    pane.trustError = null;
    if (pane === right) {
      sftpStore.attemptedActiveSshSessionId = app.activeSession?.id ?? null;
    }
  }

  async function openActiveSessionSftp(session: Session): Promise<void> {
    if (openingActiveSftp) {
      return;
    }

    openingActiveSftp = true;
    const generation = right.attemptGeneration;
    const sessionId = session.id;
    try {
      if (right.sftpSessionId && right.sshSessionId !== sessionId) {
        await right.closeSftp();
      }
      if (generation !== right.attemptGeneration) return;

      if (!right.sftpSessionId || right.sshSessionId !== sessionId) {
        console.info("[SFTP][Route] opening SFTP for active SSH session", {
          sessionId,
        });
        await right.openSftp(sessionId, session);
      }
    } finally {
      openingActiveSftp = false;
    }
  }

  $effect(() => {
    const session = app.activeSession;
    if (session?.type !== "ssh" || session.status !== "connected") {
      sftpStore.attemptedActiveSshSessionId = null;
      return;
    }

    if (right.isLocal || right.connectionError || right.isDirectConnection || right.isClosing || right.isConnecting) {
      return;
    }

    if (right.sftpSessionId && right.sshSessionId === session.id) {
      return;
    }

    if (openingActiveSftp || sftpStore.attemptedActiveSshSessionId === session.id) {
      return;
    }

    if (sftpStore.left.machine === machineKey(session)) {
      return;
    }

    sftpStore.attemptedActiveSshSessionId = session.id;
    void openActiveSessionSftp(session);
  });

  $effect(() => () => {
    sftpStore.left.attemptGeneration += 1;
    right.attemptGeneration += 1;
  });

  async function handleConnect(pane: SftpPane, connection: ConnectionConfig): Promise<void> {
    claimPane(pane);
    const generation = pane.attemptGeneration;
    pane.connectionId = connection.id;
    if (pane.sftpSessionId) {
      await pane.closeSftp();
    }
    if (generation !== pane.attemptGeneration) return;

    try {
      const input = createDirectSshConnectInput(connection);
      await pane.connectDirect({
        connectionId: connection.id,
        name: connection.name,
        host: input.host,
        port: input.port,
        username: input.username,
        password: input.password ?? undefined,
        privateKey: input.private_key ?? undefined,
        passphrase: input.passphrase ?? undefined,
      });
    } catch (error: unknown) {
      if (generation !== pane.attemptGeneration) return;
      pane.connectionId = connection.id;
      pane.trustPrompt = null;
      pane.trustMismatch = null;
      pane.connection = {
        name: connection.name,
        host: connection.host,
        port: connection.port,
        username: connection.username,
      };
      pane.isDirectConnection = true;
      pane.connectionError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleOpenSession(pane: SftpPane, session: Session): Promise<void> {
    claimPane(pane);
    const generation = pane.attemptGeneration;
    if (pane.sftpSessionId) {
      await pane.closeSftp();
    }
    if (generation !== pane.attemptGeneration) return;
    await pane.openSftp(session.id, session);
  }

  async function handleUseLocal(pane: SftpPane): Promise<void> {
    claimPane(pane);
    await pane.useLocal();
  }

  async function handleRetry(pane: SftpPane): Promise<void> {
    if (pane.trustConfirming || pane.isConnecting) return;
    if (!pane.isDirectConnection && pane.sshSessionId) {
      pane.attemptGeneration += 1;
      if (pane === right) {
        sftpStore.attemptedActiveSshSessionId = app.activeSession?.id ?? null;
      }
      await pane.openSftp(pane.sshSessionId, pane.connection ?? undefined);
      return;
    }
    const connection = app.connections.find((candidate) => candidate.id === pane.connectionId);
    if (!connection) {
      pane.connectionError = "Saved connection not found. Open Connections and try again.";
      return;
    }
    await handleConnect(pane, connection);
  }

  async function confirmTrustAndRetry(pane: SftpPane, confirmation: HostTrustConfirmation): Promise<void> {
    if (pane.trustConfirming || !pane.connectionId) return;
    if (!app.connections.some((connection) => connection.id === pane.connectionId)) {
      pane.trustError = "Saved connection not found. Open Connections and try again.";
      return;
    }
    const generation = pane.attemptGeneration;
    pane.trustConfirming = true;
    pane.trustError = null;
    try {
      const result = await commands.sshConfirmHostTrust(confirmation);
      if (generation !== pane.attemptGeneration) return;
      if (result.status === "error") throw new Error(result.error);
      pane.trustConfirming = false;
      await handleRetry(pane);
    } catch (error: unknown) {
      if (generation === pane.attemptGeneration) {
        pane.trustError = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (generation === pane.attemptGeneration) {
        pane.trustConfirming = false;
      }
    }
  }

  async function handleTrust(pane: SftpPane): Promise<void> {
    const prompt = pane.trustPrompt;
    if (prompt) {
      await confirmTrustAndRetry(pane, {
        host: prompt.host,
        port: prompt.port,
        algorithm: prompt.algorithm,
        fingerprint: prompt.fingerprint,
      });
    }
  }

  async function handleReplaceTrust(pane: SftpPane): Promise<void> {
    const mismatch = pane.trustMismatch;
    if (mismatch) {
      await confirmTrustAndRetry(pane, {
        host: mismatch.host,
        port: mismatch.port,
        algorithm: mismatch.presented_algorithm,
        fingerprint: mismatch.presented_fingerprint,
      });
    }
  }

  async function handleDisconnect(pane: SftpPane): Promise<void> {
    claimPane(pane);
    await pane.disconnect();
  }
</script>

<SftpView
  connections={app.connections}
  {sshSessions}
  onConnect={handleConnect}
  onOpenSession={handleOpenSession}
  onUseLocal={handleUseLocal}
  onDisconnect={handleDisconnect}
  onRetry={handleRetry}
  onTrust={handleTrust}
  onReplaceTrust={handleReplaceTrust}
/>
