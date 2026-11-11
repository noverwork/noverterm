import { listen } from "@tauri-apps/api/event";

import {
  commands,
  type McpActivity,
  type McpApprovalRequest,
  type McpDecision,
  type McpSessions,
} from "../../bindings.js";
import { t } from "$lib/i18n/index.svelte.js";

const ACTIVITY_LIMIT = 20;
/** A finished background command sends no event, so poll while any runs. */
const COMMAND_POLL_MS = 5000;

export type LoggedActivity = McpActivity & { at: number };

/** Approval requests waiting for the user, oldest first. */
export const approvals = $state<McpApprovalRequest[]>([]);
/** Recent MCP commands, newest first. */
export const activity = $state<LoggedActivity[]>([]);
/** What MCP clients are doing or hold open right now. */
export const live = $state<McpSessions>({
  running: [],
  commands: [],
  forwards: [],
});
/** AI access that is not stored on a host. */
export const aiAccess = $state({ localTerminal: false });

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
    activity.unshift({ ...event.payload, at: Date.now() });
    activity.splice(ACTIVITY_LIMIT);
  });
  void listen("mcp_changed", () => void refreshLive());
  // MCP forwards can fail or be stopped from the Forwards page.
  void listen("port_forward_status", () => void refreshLive());
  void listen("ai_access_changed", () => void refreshLocalAccess());
  void refreshLive();
  void refreshLocalAccess();
}

async function refreshLocalAccess() {
  const result = await commands.mcpStatus();
  if (result.status === "ok") {
    aiAccess.localTerminal = result.data.settings.allow_local_terminal ?? false;
  }
}

async function refreshLive() {
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

export function activityLabel(entry: McpActivity): string {
  switch (entry.status) {
    case "finished":
      return entry.exit_status === null
        ? t("Done")
        : t("exit {code}", { code: entry.exit_status });
    case "started":
      return t("Started");
    case "denied":
      return t("Denied");
    case "stopped":
      return t("Stopped");
    default:
      return t("Failed");
  }
}

export function activityTone(entry: McpActivity): string {
  if (
    entry.status === "started" ||
    (entry.status === "finished" && (entry.exit_status ?? 0) === 0)
  ) {
    return "text-emerald-400";
  }
  return entry.status === "denied" || entry.status === "stopped"
    ? "text-stencil-mute"
    : "text-red-300";
}

export async function stopCommand(id: string) {
  const result = await commands.mcpStopCommand(id);
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
