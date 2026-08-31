import { describe, expect, it } from "vitest";

import type { PortForwardRecord } from "$lib/api/types.js";
import type { ConnectionConfig } from "$lib/app-data-types.js";
import { groupState, runtimeForwardsFor } from "$lib/port-forward-group.js";
import type { PortForward } from "$lib/stores/port-forward.svelte.js";

const connection: ConnectionConfig = {
  id: "host-1",
  name: "prod",
  host: "prod.example.test",
  port: 22,
  username: "deploy",
  groupId: null,
  sshKeyId: null,
  hasPassword: true,
  auth: null,
};

const preset: PortForwardRecord = {
  id: "pf-1",
  name: "stack",
  host_id: "host-1",
  host_name: "prod",
  mappings: [
    { bind_host: "127.0.0.1", bind_port: 8080, target_host: "127.0.0.1", target_port: 80 },
    { bind_host: "127.0.0.1", bind_port: 5432, target_host: "127.0.0.1", target_port: 5432 },
  ],
};

function runtime(
  bindPort: number,
  targetPort: number,
  state: PortForward["state"],
): PortForward {
  return {
    id: `runtime-${bindPort}`,
    name: preset.name,
    host: connection.host,
    port: connection.port,
    username: connection.username,
    bind_host: "127.0.0.1",
    bind_port: bindPort,
    target_host: "127.0.0.1",
    target_port: targetPort,
    state,
    error: state === "error" ? "bind failed" : null,
  };
}

describe("runtimeForwardsFor", () => {
  it("lines runtime forwards up with the mappings they came from", () => {
    const runtimes = runtimeForwardsFor(preset, connection, [
      runtime(5432, 5432, "listening"),
      runtime(8080, 80, "connecting"),
    ]);

    expect(runtimes.map((forward) => forward?.bind_port)).toEqual([8080, 5432]);
  });

  it("reports nothing running when the connection is gone", () => {
    expect(runtimeForwardsFor(preset, null, [runtime(8080, 80, "listening")])).toEqual([
      null,
      null,
    ]);
  });
});

describe("groupState", () => {
  it("is listening only once every mapping is up", () => {
    expect(
      groupState([runtime(8080, 80, "listening"), runtime(5432, 5432, "listening")]),
    ).toBe("listening");
    expect(groupState([runtime(8080, 80, "listening"), null])).toBe("stopped");
  });

  it("takes the worst state in the group", () => {
    expect(
      groupState([runtime(8080, 80, "listening"), runtime(5432, 5432, "error")]),
    ).toBe("error");
    expect(
      groupState([runtime(8080, 80, "connecting"), runtime(5432, 5432, "listening")]),
    ).toBe("connecting");
  });

  it("has no state at all when nothing is running", () => {
    expect(groupState([null, null])).toBeNull();
  });
});
