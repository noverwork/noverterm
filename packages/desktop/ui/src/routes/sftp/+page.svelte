<script lang="ts">
  import SftpView from "$lib/components/sftp-view.svelte";
  import { getAppShellContext } from "$lib/stores/app-shell.svelte.js";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import { createDirectSshConnectInput } from "$lib/services/ssh-connection-input.js";
  import { commands, type HostTrustConfirmation } from "../../bindings.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import type { Session } from "$lib/stores/session.svelte.js";

  const app = getAppShellContext();

  let openingActiveSftp = $state(false);
  let trustConfirming = $state(false);
  let trustError = $state<string | null>(null);
  let attemptGeneration = 0;

  async function openActiveSessionSftp(session: Session): Promise<void> {
    if (openingActiveSftp) {
      return;
    }

    openingActiveSftp = true;
    const generation = attemptGeneration;
    const sessionId = session.id;
    try {
      if (sftpStore.sftpSessionId && sftpStore.sshSessionId !== sessionId) {
        await sftpStore.closeSftp();
      }
      if (generation !== attemptGeneration) return;

      if (!sftpStore.sftpSessionId || sftpStore.sshSessionId !== sessionId) {
        console.info("[SFTP][Route] opening SFTP for active SSH session", {
          sessionId,
        });
        await sftpStore.openSftp(sessionId, session);
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

    if (sftpStore.connectionError || sftpStore.isDirectConnection || sftpStore.isClosing || sftpStore.isConnecting) {
      return;
    }

    if (sftpStore.sftpSessionId && sftpStore.sshSessionId === session.id) {
      return;
    }

    if (openingActiveSftp || sftpStore.attemptedActiveSshSessionId === session.id) {
      return;
    }

    sftpStore.attemptedActiveSshSessionId = session.id;
    void openActiveSessionSftp(session);
  });

  $effect(() => () => {
    attemptGeneration += 1;
  });

  async function handleConnect(connection: ConnectionConfig): Promise<void> {
    const generation = ++attemptGeneration;
    sftpStore.connectionId = connection.id;
    trustError = null;
    sftpStore.attemptedActiveSshSessionId = app.activeSession?.id ?? null;
    if (sftpStore.sftpSessionId) {
      await sftpStore.closeSftp();
    }
    if (generation !== attemptGeneration) return;

    try {
      const input = createDirectSshConnectInput(connection);
      await sftpStore.connectDirect({
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
      if (generation !== attemptGeneration) return;
      sftpStore.connectionId = connection.id;
      sftpStore.trustPrompt = null;
      sftpStore.trustMismatch = null;
      sftpStore.connection = {
        name: connection.name,
        host: connection.host,
        port: connection.port,
        username: connection.username,
      };
      sftpStore.isDirectConnection = true;
      sftpStore.connectionError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleRetry(): Promise<void> {
    if (trustConfirming || sftpStore.isConnecting) return;
    if (!sftpStore.isDirectConnection && sftpStore.sshSessionId) {
      attemptGeneration += 1;
      sftpStore.attemptedActiveSshSessionId = app.activeSession?.id ?? null;
      await sftpStore.openSftp(sftpStore.sshSessionId, sftpStore.connection ?? undefined);
      return;
    }
    const connection = app.connections.find((candidate) => candidate.id === sftpStore.connectionId);
    if (!connection) {
      sftpStore.connectionError = "Saved connection not found. Open Connections and try again.";
      return;
    }
    await handleConnect(connection);
  }

  async function confirmTrustAndRetry(confirmation: HostTrustConfirmation): Promise<void> {
    if (trustConfirming || !sftpStore.connectionId) return;
    if (!app.connections.some((connection) => connection.id === sftpStore.connectionId)) {
      trustError = "Saved connection not found. Open Connections and try again.";
      return;
    }
    const generation = attemptGeneration;
    trustConfirming = true;
    trustError = null;
    try {
      const result = await commands.sshConfirmHostTrust(confirmation);
      if (generation !== attemptGeneration) return;
      if (result.status === "error") throw new Error(result.error);
      trustConfirming = false;
      await handleRetry();
    } catch (error: unknown) {
      if (generation === attemptGeneration) {
        trustError = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (generation === attemptGeneration) {
        trustConfirming = false;
      }
    }
  }

  async function handleTrust(): Promise<void> {
    const prompt = sftpStore.trustPrompt;
    if (prompt) {
      await confirmTrustAndRetry({
        host: prompt.host,
        port: prompt.port,
        algorithm: prompt.algorithm,
        fingerprint: prompt.fingerprint,
      });
    }
  }

  async function handleReplaceTrust(): Promise<void> {
    const mismatch = sftpStore.trustMismatch;
    if (mismatch) {
      await confirmTrustAndRetry({
        host: mismatch.host,
        port: mismatch.port,
        algorithm: mismatch.presented_algorithm,
        fingerprint: mismatch.presented_fingerprint,
      });
    }
  }

  async function handleDisconnect(): Promise<void> {
    attemptGeneration += 1;
    trustConfirming = false;
    trustError = null;
    sftpStore.attemptedActiveSshSessionId = app.activeSession?.id ?? null;
    await sftpStore.disconnect();
  }
</script>

<SftpView
  connections={app.connections}
  onConnect={handleConnect}
  onDisconnect={handleDisconnect}
  onRetry={handleRetry}
  onTrust={handleTrust}
  onReplaceTrust={handleReplaceTrust}
  {trustError}
  {trustConfirming}
  canTrust={sftpStore.connectionId !== null}
/>
