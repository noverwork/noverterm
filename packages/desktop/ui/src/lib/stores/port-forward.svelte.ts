import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteMap } from "svelte/reactivity";

import { commands as tauriCommands } from "../../bindings.js";
import type { PortForwardRecord } from "$lib/api/types.js";
import type { ConnectionConfig } from "$lib/app-data-types.js";

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
  preset: PortForwardRecord;
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

/// A preset starts one tunnel per mapping, and they live or die together: this
/// maps every runtime id to the others started alongside it, so the first
/// failure can tear the rest down. Backend forwards carry no group of their own.
const groupSiblings = new SvelteMap<string, string[]>();

function forgetGroup(forwardId: string) {
  for (const sibling of groupSiblings.get(forwardId) ?? []) {
    groupSiblings.delete(sibling);
  }
  groupSiblings.delete(forwardId);
}

function updateForward(status: PortForwardStatusEvent) {
  if (status.state === "error") {
    const siblings = groupSiblings.get(status.id) ?? [];
    forgetGroup(status.id);
    for (const sibling of siblings) {
      void stopForward(sibling).catch(() => {
        // The group is already failing; a sibling that refuses to stop adds nothing.
      });
    }
  }

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
        password: connection.auth.password,
        privateKey: null,
        passphrase: null,
      };
    case "public_key":
      return {
        password: null,
        privateKey: connection.auth.private_key,
        passphrase: connection.auth.passphrase,
      };
    case "public_key_and_password":
      return {
        password: connection.auth.password,
        privateKey: connection.auth.private_key,
        passphrase: connection.auth.passphrase,
      };
    default:
      throw new Error("host has no connectable authentication material");
  }
}

async function stopForward(forwardId: string): Promise<PortForward> {
  forgetGroup(forwardId);
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

  /// Starts every mapping in the preset as one group. A mapping that fails
  /// outright takes the whole group down; one that fails later (a bind clash
  /// only surfaces on the status event) is rolled back by `updateForward`.
  async function startSavedForward(
    input: PortForwardPresetStartInput,
  ): Promise<PortForward[]> {
    const started: PortForward[] = [];

    try {
      for (const mapping of input.preset.mappings) {
        started.push(
          await startFromConnection({
            connection: input.connection,
            name: input.preset.name,
            bind_host: mapping.bind_host,
            bind_port: mapping.bind_port,
            target_host: mapping.target_host,
            target_port: mapping.target_port,
          }),
        );
      }
    } catch (cause) {
      await Promise.allSettled(started.map((forward) => stopForward(forward.id)));
      throw cause;
    }

    const ids = started.map((forward) => forward.id);
    if (ids.length > 1) {
      for (const id of ids) {
        groupSiblings.set(
          id,
          ids.filter((sibling) => sibling !== id),
        );
      }
    }

    return started;
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
    forgetGroup(forwardId);
    state.forwards.delete(forwardId);
  }

  function cleanup() {
    if (eventUnlisten) {
      eventUnlisten();
      eventUnlisten = null;
    }
    initPromise = null;
    groupSiblings.clear();
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
    stop: stopForward,
    list,
    getPortForwards,
    remove,
    cleanup,
  };
}
