import { listen } from "@tauri-apps/api/event";

import { commands, type McpActivity, type McpApprovalRequest, type McpDecision } from "../../bindings.js";

const ACTIVITY_LIMIT = 20;

/** Approval requests waiting for the user, oldest first. */
export const approvals = $state<McpApprovalRequest[]>([]);
/** Recent MCP commands, newest first. */
export const activity = $state<McpActivity[]>([]);

let started = false;

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
}

export async function respond(request: McpApprovalRequest, decision: McpDecision) {
  const index = approvals.findIndex((pending) => pending.id === request.id);
  if (index !== -1) approvals.splice(index, 1);
  await commands.mcpRespond(request.id, decision);
}
