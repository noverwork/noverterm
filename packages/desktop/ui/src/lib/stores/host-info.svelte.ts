import { SvelteMap } from "svelte/reactivity";

import { commands as tauriCommands } from "../../bindings.js";
import type {
  HostSystemInfo,
  HostTrustMismatch,
  HostTrustPrompt,
} from "../../bindings.js";
import { createDirectSshConnectInput } from "$lib/services/ssh-connection-input.js";
import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

export type HostInfoStatus =
  | "idle"
  | "loading"
  | "success"
  | "trust_required"
  | "trust_mismatch"
  | "error";

export interface HostInfoEntry {
  connectionId: string;
  status: HostInfoStatus;
  info?: HostSystemInfo;
  error?: string;
  prompt?: HostTrustPrompt;
  mismatch?: HostTrustMismatch;
  checkedAtMs?: number;
}

const MAX_CONCURRENT_PROBES = 3;

interface HostInfoState {
  entries: SvelteMap<string, HostInfoEntry>;
}

const state: HostInfoState = $state({
  entries: new SvelteMap(),
});

function emptyEntry(connectionId: string): HostInfoEntry {
  return { connectionId, status: "idle" };
}

function updateEntry(connectionId: string, entry: HostInfoEntry) {
  state.entries.set(connectionId, entry);
}

export function createHostInfoStore() {
  function getEntry(connectionId: string): HostInfoEntry {
    return state.entries.get(connectionId) ?? emptyEntry(connectionId);
  }

  async function probe(connection: ConnectionConfig): Promise<void> {
    updateEntry(connection.id, {
      connectionId: connection.id,
      status: "loading",
      info: getEntry(connection.id).info,
      checkedAtMs: getEntry(connection.id).checkedAtMs,
    });

    try {
      const result = await tauriCommands.sshProbeHostInfo(
        await createDirectSshConnectInput(connection),
      );

      if (result.status === "error") {
        updateEntry(connection.id, {
          connectionId: connection.id,
          status: "error",
          error: result.error,
          checkedAtMs: Date.now(),
        });
        return;
      }

      switch (result.data.status) {
        case "success":
          updateEntry(connection.id, {
            connectionId: connection.id,
            status: "success",
            info: result.data.info,
            checkedAtMs: Date.now(),
          });
          return;
        case "trust_required":
          updateEntry(connection.id, {
            connectionId: connection.id,
            status: "trust_required",
            prompt: result.data.prompt,
            checkedAtMs: Date.now(),
          });
          return;
        case "trust_mismatch":
          updateEntry(connection.id, {
            connectionId: connection.id,
            status: "trust_mismatch",
            mismatch: result.data.mismatch,
            error: "Host trust mismatch",
            checkedAtMs: Date.now(),
          });
          return;
      }
    } catch (error) {
      updateEntry(connection.id, {
        connectionId: connection.id,
        status: "error",
        error: error instanceof Error ? error.message : String(error),
        checkedAtMs: Date.now(),
      });
    }
  }

  async function confirmTrustAndProbe(
    connection: ConnectionConfig,
  ): Promise<void> {
    const prompt = getEntry(connection.id).prompt;
    if (!prompt) {
      return;
    }

    const result = await tauriCommands.sshConfirmHostTrust({
      host: prompt.host,
      port: prompt.port,
      algorithm: prompt.algorithm,
      fingerprint: prompt.fingerprint,
    });

    if (result.status === "error") {
      updateEntry(connection.id, {
        connectionId: connection.id,
        status: "error",
        error: result.error,
        checkedAtMs: Date.now(),
      });
      return;
    }

    await probe(connection);
  }

  async function probeMany(connections: ConnectionConfig[]): Promise<void> {
    const queue = connections.slice();
    const workers = Array.from(
      { length: Math.min(MAX_CONCURRENT_PROBES, queue.length) },
      async () => {
        while (queue.length > 0) {
          const connection = queue.shift();
          if (connection) {
            await probe(connection);
          }
        }
      },
    );

    await Promise.all(workers);
  }

  function reset(connectionId: string) {
    state.entries.delete(connectionId);
  }

  return {
    get entries() {
      return state.entries;
    },
    getEntry,
    probe,
    probeMany,
    confirmTrustAndProbe,
    reset,
  };
}

export type HostInfoStore = ReturnType<typeof createHostInfoStore>;
