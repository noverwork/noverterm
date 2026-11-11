import { listen } from "@tauri-apps/api/event";
import { SvelteSet } from "svelte/reactivity";

import {
  commands,
  type McpActivity,
  type McpApprovalRequest,
  type McpDecision,
  type McpSessions,
} from "../../bindings.js";

const ACTIVITY_LIMIT = 20;
/** A finished background command sends no event, so poll while any runs. */
const COMMAND_POLL_MS = 5000;

/** Approval requests waiting for the user, oldest first. */
export const approvals = $state<McpApprovalRequest[]>([]);
/** Recent MCP commands, newest first. */
export const activity = $state<McpActivity[]>([]);
/** What MCP clients hold open right now. */
export const live = $state<McpSessions>({ commands: [], forwards: [], grants: [] });
/** Terminal tabs (session ids) an MCP client has read. */
export const readTerminals = new SvelteSet<string>();

let started = false;
let poll: ReturnType<typeof setInterval> | null = null;

/** Subscribes once to the backend's MCP events; safe to call repeatedly. */
export function startMcpEvents() {
  if (started) return;
  started = true;
  void listen<McpApprovalRequest>("mcp_approval_request", (event) => {
    approvals.push(event.payload);
  });
  void listen<McpActivity>("mcp_activity", (event) => {
    activity.unshift(event.payload);
    activity.splice(ACTIVITY_LIMIT);
  });
  void listen<string>("mcp_terminal_read", (event) => {
    readTerminals.add(event.payload);
  });
  void listen("mcp_changed", () => void refreshLive());
  // MCP forwards can fail or be stopped from the Forwards page.
  void listen("port_forward_status", () => void refreshLive());
  void refreshLive();
}

export async function refreshLive() {
  const result = await commands.mcpSessions();
  if (result.status !== "ok") return;
  Object.assign(live, result.data);
  const running = result.data.commands.some((command) => !command.finished);
  if (running && !poll) {
    poll = setInterval(() => void refreshLive(), COMMAND_POLL_MS);
  } else if (!running && poll) {
    clearInterval(poll);
    poll = null;
  }
}

export async function stopCommand(id: string) {
  const result = await commands.mcpStopCommand(id);
  if (result.status === "error") throw new Error(result.error);
}

export async function revokeGrant(key: string) {
  const result = await commands.mcpRevokeGrant(key);
  if (result.status === "error") throw new Error(result.error);
}

export async function stopForward(id: string) {
  const result = await commands.portForwardStop(id);
  if (result.status === "error") throw new Error(result.error);
}

export async function respond(request: McpApprovalRequest, decision: McpDecision) {
  const index = approvals.findIndex((pending) => pending.id === request.id);
  if (index !== -1) approvals.splice(index, 1);
  await commands.mcpRespond(request.id, decision);
}
