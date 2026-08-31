import type { PortForwardRecord } from "$lib/api/types.js";
import type { ConnectionConfig } from "$lib/app-data-types.js";
import type { PortForward } from "$lib/stores/port-forward.svelte.js";

/// Runtime forwards carry no preset id, so each mapping is matched back to its
/// running tunnel by the fields the backend echoes. Result is aligned with
/// `preset.mappings`; a null means that mapping is not running.
export function runtimeForwardsFor(
  preset: PortForwardRecord,
  connection: ConnectionConfig | null,
  forwards: PortForward[],
): (PortForward | null)[] {
  if (!connection) {
    return preset.mappings.map(() => null);
  }

  return preset.mappings.map(
    (mapping) =>
      forwards.find(
        (forward) =>
          forward.name === preset.name &&
          forward.host === connection.host &&
          forward.port === connection.port &&
          forward.username === connection.username &&
          forward.bind_host === mapping.bind_host &&
          forward.bind_port === mapping.bind_port &&
          forward.target_host === mapping.target_host &&
          forward.target_port === mapping.target_port,
      ) ?? null,
  );
}

/// The group is only as healthy as its worst mapping. Returns null when nothing
/// in the group is running at all.
export function groupState(
  runtimes: (PortForward | null)[],
): PortForward["state"] | null {
  const running = runtimes.filter((forward) => forward !== null);
  if (running.length === 0) {
    return null;
  }

  if (running.some((forward) => forward.state === "error")) {
    return "error";
  }
  if (running.some((forward) => forward.state === "connecting")) {
    return "connecting";
  }
  if (
    running.length === runtimes.length &&
    running.every((forward) => forward.state === "listening")
  ) {
    return "listening";
  }

  return "stopped";
}
