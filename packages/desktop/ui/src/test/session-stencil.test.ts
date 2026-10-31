import { describe, expect, it } from "vitest";

import {
  LINE_COLORS,
  UNGROUPED_LINE,
  groupPrefix,
  lineColorFor,
  sessionStencil,
} from "$lib/session-stencil.js";

const connections = [
  { id: "c-prod", groupId: "g-prod" },
  { id: "c-loose", groupId: null },
];
const hostGroups = [{ id: "g-prod", name: "Production" }];

describe("sessionStencil", () => {
  it("marks local sessions LOC on the ungrouped line", () => {
    expect(
      sessionStencil({ type: "local", connectionId: null }, connections, hostGroups),
    ).toEqual({ prefix: "LOC", line: UNGROUPED_LINE });
  });

  it("uses the group's prefix and line colour", () => {
    const stencil = sessionStencil(
      { type: "ssh", connectionId: "c-prod" },
      connections,
      hostGroups,
    );
    expect(stencil.prefix).toBe("PRO");
    expect(stencil.line).toBe(lineColorFor("g-prod", hostGroups));
  });

  it("falls back to SSH when the connection has no group", () => {
    expect(
      sessionStencil({ type: "ssh", connectionId: "c-loose" }, connections, hostGroups),
    ).toEqual({ prefix: "SSH", line: UNGROUPED_LINE });
  });
});

describe("lineColorFor", () => {
  const groups = [
    { id: "b", name: "beta" },
    { id: "a", name: "alpha" },
    { id: "c", name: "gamma" },
  ];

  it("gives neighbouring groups distinct colours in name order", () => {
    expect(lineColorFor("a", groups)).toBe(LINE_COLORS[0]);
    expect(lineColorFor("b", groups)).toBe(LINE_COLORS[1]);
    expect(lineColorFor("c", groups)).toBe(LINE_COLORS[2]);
  });

  it("falls back to the ungrouped line for unknown groups", () => {
    expect(lineColorFor("missing", groups)).toBe(UNGROUPED_LINE);
  });
});

describe("groupPrefix", () => {
  it("keeps three letters or digits", () => {
    expect(groupPrefix("k8s nodes")).toBe("K8S");
    expect(groupPrefix("—")).toBe("GRP");
  });
});
