import { describe, expect, it } from "vitest";

import type { SshHostRecord } from "$lib/api/types.js";
import { mapHostsToConnections } from "$lib/app-data-mappers.js";

function host(id: string, jumpHostId: string | null): SshHostRecord {
  return {
    id,
    name: id,
    host: `${id}.example.test`,
    port: 22,
    username: "deploy",
    ssh_key_id: null,
    group_id: null,
    jump_host_id: jumpHostId,
    auth: { kind: "password", password: `${id}-secret` },
    ai_access: "ask",
  };
}

describe("mapHostsToConnections jump chains", () => {
  it("orders hops from the outermost bastion to the nearest one", () => {
    const connections = mapHostsToConnections([
      host("target", "inner"),
      host("inner", "outer"),
      host("outer", null),
    ]);

    const target = connections.find((connection) => connection.id === "target");
    expect(target?.jumps.map((hop) => hop.host)).toEqual([
      "outer.example.test",
      "inner.example.test",
    ]);
    expect(target?.jumps[0]).toMatchObject({
      username: "deploy",
      password: "outer-secret",
      private_key: null,
    });
    expect(connections.find((connection) => connection.id === "outer")?.jumps).toEqual([]);
  });

  it("stops at a missing jump host and caps a looping chain", () => {
    const [orphan] = mapHostsToConnections([host("orphan", "gone")]);
    expect(orphan.jumps).toEqual([]);

    const [looped] = mapHostsToConnections([host("a", "b"), host("b", "a")]);
    expect(looped.jumps).toHaveLength(5);
  });
});
