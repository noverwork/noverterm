import { invoke } from "@tauri-apps/api/core";

import type { PortForwardRecord, PortForwardWriteRequest } from "./types.js";

export async function listPortForwards(): Promise<PortForwardRecord[]> {
  return invoke<PortForwardRecord[]>("port_forward_preset_list");
}

export async function createPortForward(
  forward: PortForwardWriteRequest,
): Promise<PortForwardRecord> {
  return invoke<PortForwardRecord>("port_forward_preset_create", { forward });
}

export async function updatePortForward(
  id: string,
  forward: PortForwardWriteRequest,
): Promise<PortForwardRecord> {
  return invoke<PortForwardRecord>("port_forward_preset_update", {
    id,
    forward,
  });
}

export async function deletePortForward(id: string): Promise<void> {
  await invoke("port_forward_preset_delete", { id });
}
