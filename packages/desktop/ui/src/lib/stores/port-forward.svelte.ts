import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteMap } from "svelte/reactivity";

import { commands as tauriCommands } from "../../bindings.js";
import { decryptSecret } from "$lib/crypto/vault.js";
import type { ConnectionConfig, SavedPortForwardConfig } from "$lib/app-data-types.js";

export interface PortForward {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  bind_host: string;
  bind_port: number;
  target_host: string;
  target_port: number;
  state: "connecting" | "listening" | "stopped" | "error";
  error: string | null;
}

export interface PortForwardCreateInput {
  name: string;
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
  bind_host: string;
  bind_port: number;
  target_host: string;
  target_port: number;
}

export interface PortForwardConnectionCreateInput {
  connection: ConnectionConfig;
  name: string;
  bind_host: string;
  bind_port: number;
  target_host: string;
  target_port: number;
}

export interface PortForwardPresetStartInput {
  preset: SavedPortForwardConfig;
  connection: ConnectionConfig;
}

interface PortForwardStatusEvent {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  bind_host: string;
  bind_port: number;
  target_host: string;
  target_port: number;
  state: "connecting" | "listening" | "stopped" | "error";
  error: string | null;
}

interface PortForwardState {
  forwards: SvelteMap<string, PortForward>;
}

const state: PortForwardState = $state({
  forwards: new SvelteMap(),
});

let eventUnlisten: UnlistenFn | null = null;
let initPromise: Promise<void> | null = null;

function updateForward(status: PortForwardStatusEvent) {
  state.forwards.set(status.id, {
    id: status.id,
    name: status.name,
    host: status.host,
    port: status.port,
    username: status.username,
    bind_host: status.bind_host,
    bind_port: status.bind_port,
    target_host: status.target_host,
    target_port: status.target_port,
    state: status.state,
    error: status.error,
  });
}

async function connectionAuthInput(connection: ConnectionConfig): Promise<{
  password: string | null;
  privateKey: string | null;
  passphrase: string | null;
}> {
  switch (connection.auth?.kind) {
    case "password":
      return {
        password: await decryptSecret(connection.auth.password),
        privateKey: null,
        passphrase: null,
      };
    case "public_key":
      return {
        password: null,
        privateKey: await decryptSecret(connection.auth.private_key),
        passphrase: await decryptSecret(connection.auth.passphrase),
      };
    case "public_key_and_password":
      return {
        password: await decryptSecret(connection.auth.password),
        privateKey: await decryptSecret(connection.auth.private_key),
        passphrase: await decryptSecret(connection.auth.passphrase),
      };
    default:
      throw new Error("host has no connectable authentication material");
  }
}

export function createPortForwardStore() {
  async function init() {
    if (eventUnlisten) return;
    if (initPromise) return initPromise;

    initPromise = (async () => {
      if (!eventUnlisten) {
        eventUnlisten = await listen(
          "port_forward_status",
          (event: { payload: PortForwardStatusEvent }) => {
            updateForward(event.payload);
          },
        );
      }
    })();

    try {
      await initPromise;
    } finally {
      initPromise = null;
    }
  }

  async function start(input: PortForwardCreateInput): Promise<PortForward> {
    await init();

    const result = await tauriCommands.portForwardStart({
      name: input.name,
      host: input.host,
      port: input.port,
      username: input.username,
      password: input.password?.trim() || null,
      private_key: input.privateKey?.trim() || null,
      passphrase: input.passphrase?.trim() || null,
      bind_host: input.bind_host,
      bind_port: input.bind_port,
      target_host: input.target_host,
      target_port: input.target_port,
    });

    if (result.status === "error") {
      throw new Error(result.error);
    }

    updateForward(result.data);
    return {
      id: result.data.id,
      name: result.data.name,
      host: result.data.host,
      port: result.data.port,
      username: result.data.username,
      bind_host: result.data.bind_host,
      bind_port: result.data.bind_port,
      target_host: result.data.target_host,
      target_port: result.data.target_port,
      state: result.data.state,
      error: result.data.error,
    };
  }

  async function startFromConnection(input: PortForwardConnectionCreateInput): Promise<PortForward> {
    const auth = await connectionAuthInput(input.connection);

    return start({
      name: input.name,
      host: input.connection.host,
      port: input.connection.port,
      username: input.connection.username,
      password: auth.password ?? undefined,
      privateKey: auth.privateKey ?? undefined,
      passphrase: auth.passphrase ?? undefined,
      bind_host: input.bind_host,
      bind_port: input.bind_port,
      target_host: input.target_host,
      target_port: input.target_port,
    });
  }

  async function startSavedForward(input: PortForwardPresetStartInput): Promise<PortForward> {
    return startFromConnection({
      connection: input.connection,
      name: input.preset.name,
      bind_host: input.preset.bind_host,
      bind_port: input.preset.bind_port,
      target_host: input.preset.target_host,
      target_port: input.preset.target_port,
    });
  }

  async function stop(forwardId: string): Promise<PortForward> {
    const result = await tauriCommands.portForwardStop(forwardId);

    if (result.status === "error") {
      throw new Error(result.error);
    }

    updateForward(result.data);
    return {
      id: result.data.id,
      name: result.data.name,
      host: result.data.host,
      port: result.data.port,
      username: result.data.username,
      bind_host: result.data.bind_host,
      bind_port: result.data.bind_port,
      target_host: result.data.target_host,
      target_port: result.data.target_port,
      state: result.data.state,
      error: result.data.error,
    };
  }

  async function list(): Promise<PortForward[]> {
    await init();

    const result = await tauriCommands.portForwardList();

    if (result.status === "error") {
      throw new Error(result.error);
    }

    for (const status of result.data) {
      updateForward(status);
    }

    return getPortForwards();
  }

  function getPortForwards(): PortForward[] {
    return Array.from(state.forwards.values());
  }

  function remove(forwardId: string) {
    state.forwards.delete(forwardId);
  }

  function cleanup() {
    if (eventUnlisten) {
      eventUnlisten();
      eventUnlisten = null;
    }
    initPromise = null;
    state.forwards.clear();
  }

  return {
    get forwards() {
      return state.forwards;
    },
    init,
    start,
    startFromConnection,
    startSavedForward,
    stop,
    list,
    getPortForwards,
    remove,
    cleanup,
  };
}
