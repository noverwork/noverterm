import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";
import type { SessionStatus } from "$lib/stores/session.svelte.js";

export interface SessionLookup {
  id: string;
  name: string;
  status: SessionStatus;
  connectionId?: string | null;
}

export interface SignupDraft {
  fullName: string;
  email: string;
  team: string;
  useCase: string;
}

export function findConnectionSession(
  sessions: Iterable<SessionLookup>,
  connection: Pick<ConnectionConfig, "id" | "username" | "host" | "port">,
): SessionLookup | undefined {
  const fallbackName = `${connection.username}@${connection.host}:${connection.port}`;
  return Array.from(sessions).find(
    (session) => session.connectionId === connection.id || session.name === fallbackName,
  );
}

export function validateSignupDraft(draft: SignupDraft): Record<string, string> {
  const errors: Record<string, string> = {};
  if (!draft.fullName.trim()) errors.fullName = "Full name is required";
  if (!draft.email.trim()) errors.email = "Work email is required";
  if (draft.email.trim() && !draft.email.includes("@")) errors.email = "Enter a valid email address";
  if (!draft.team.trim()) errors.team = "Workspace or team is required";
  if (!draft.useCase.trim()) errors.useCase = "Tell us what infrastructure you need to access";
  return errors;
}

export function buildSignupSummary(draft: SignupDraft): string {
  return [
    "Noverterm account setup",
    "",
    `Name: ${draft.fullName.trim() || "-"}`,
    `Email: ${draft.email.trim() || "-"}`,
    `Team / Workspace: ${draft.team.trim() || "-"}`,
    `Use case: ${draft.useCase.trim() || "-"}`,
  ].join("\n");
}
