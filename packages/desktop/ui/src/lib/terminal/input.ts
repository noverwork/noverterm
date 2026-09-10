import { commands } from "../../bindings.js";
import type { SessionType } from "$lib/stores/session.svelte.js";

interface PendingInput {
  bytes: number;
  completion: Promise<void>;
}

const pendingInput = new Map<string, PendingInput>();
const encoder = new TextEncoder();
const MAX_PENDING_INPUT_BYTES = 1024 * 1024;

// The first write starts synchronously; subsequent writes preserve PTY ordering.
export function writeTerminalInput(
  sessionId: string,
  sessionType: SessionType,
  data: string,
): Promise<void> {
  if (data.length === 0) return Promise.resolve();
  const current = pendingInput.get(sessionId);
  if (data.length > MAX_PENDING_INPUT_BYTES) {
    return Promise.reject(new Error("Terminal input queue is full"));
  }
  const bytes = encoder.encode(data).byteLength;
  if ((current?.bytes ?? 0) + bytes > MAX_PENDING_INPUT_BYTES) {
    return Promise.reject(new Error("Terminal input queue is full"));
  }
  const write = async () => {
    const result =
      sessionType === "local"
        ? await commands.localWrite(sessionId, data)
        : await commands.sshWrite(sessionId, data);
    if (result.status === "error") throw new Error(result.error);
  };
  const queue = current ?? { bytes: 0, completion: Promise.resolve() };
  queue.bytes += bytes;
  const completion = (
    current ? current.completion.then(write) : write()
  ).finally(() => {
    queue.bytes -= bytes;
    if (queue.completion === completion) pendingInput.delete(sessionId);
  });
  queue.completion = completion;
  pendingInput.set(sessionId, queue);
  return completion;
}
