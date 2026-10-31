import type { ConnectionConfig } from "$lib/app-data-types.js";
import type { HostGroupRecord } from "$lib/api/types.js";
import type { Session } from "$lib/stores/session.svelte.js";

/** A line colour and whether stencil text on it must be dark for contrast. */
export interface LineColor {
  fill: string;
  darkText: boolean;
}

export interface SessionStencil {
  /** Group prefix, e.g. "PRD"; "LOC" for local, "SSH" when ungrouped. */
  prefix: string;
  line: LineColor;
}

// Container line colours, each with >= 4.5:1 contrast for its stencil text.
export const LINE_COLORS: readonly LineColor[] = [
  { fill: "#a33b28", darkText: false },
  { fill: "#2a648f", darkText: false },
  { fill: "#336a41", darkText: false },
  { fill: "#c9952e", darkText: true },
];

export const UNGROUPED_LINE: LineColor = { fill: "#2a2f36", darkText: false };

// Groups take line colours in name order, so neighbours in the Connections
// list never share a colour until there are more groups than colours.
// ponytail: renaming a group can shift colours; store a per-group colour if
// users want them pinned.
export function lineColorFor(
  groupId: string,
  hostGroups: readonly HostGroupRecord[],
): LineColor {
  const index = [...hostGroups]
    .sort((a, b) => a.name.localeCompare(b.name))
    .findIndex((group) => group.id === groupId);
  return index < 0 ? UNGROUPED_LINE : LINE_COLORS[index % LINE_COLORS.length];
}

export function groupPrefix(name: string): string {
  const letters = name.replace(/[^a-z0-9]/gi, "").toUpperCase();
  return letters.slice(0, 3) || "GRP";
}

export function sessionStencil(
  session: Pick<Session, "type" | "connectionId">,
  connections: readonly Pick<ConnectionConfig, "id" | "groupId">[],
  hostGroups: readonly HostGroupRecord[],
): SessionStencil {
  if (session.type === "local") {
    return { prefix: "LOC", line: UNGROUPED_LINE };
  }
  const groupId = connections.find(
    (connection) => connection.id === session.connectionId,
  )?.groupId;
  const group = groupId
    ? hostGroups.find((candidate) => candidate.id === groupId)
    : undefined;
  if (!group) return { prefix: "SSH", line: UNGROUPED_LINE };
  return { prefix: groupPrefix(group.name), line: lineColorFor(group.id, hostGroups) };
}
