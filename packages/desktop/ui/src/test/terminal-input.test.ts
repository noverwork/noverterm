import { afterEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(
    async (_command: string, _args: Record<string, unknown>): Promise<void> =>
      undefined,
  ),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { writeTerminalInput } from "$lib/terminal/input.js";

afterEach(() => {
  vi.resetAllMocks();
});

describe("terminal input delivery", () => {
  it("starts immediately, preserves write order, and reports the completed write", async () => {
    const firstWrite = Promise.withResolvers<void>();
    invoke.mockImplementationOnce(() => firstWrite.promise);
    const first = writeTerminalInput("ordered", "local", "a");
    const second = writeTerminalInput("ordered", "local", "b");
    expect(invoke.mock.calls).toEqual([
      ["local_write", { sessionId: "ordered", data: "a" }],
    ]);
    firstWrite.resolve();
    await Promise.all([first, second]);
    expect(invoke.mock.calls).toEqual([
      ["local_write", { sessionId: "ordered", data: "a" }],
      ["local_write", { sessionId: "ordered", data: "b" }],
    ]);
  });

  it("does not send later input after a failed write", async () => {
    const firstWrite = Promise.withResolvers<void>();
    invoke.mockImplementationOnce(() => firstWrite.promise);
    const first = writeTerminalInput("failed", "ssh", "a");
    const second = writeTerminalInput("failed", "ssh", "b");
    const failed = Promise.allSettled([first, second]);
    firstWrite.reject("SSH disconnected");
    expect(await failed).toEqual([
      { status: "rejected", reason: new Error("SSH disconnected") },
      { status: "rejected", reason: new Error("SSH disconnected") },
    ]);
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("bounds pending UTF-8 bytes without blocking another session", async () => {
    const firstWrite = Promise.withResolvers<void>();
    invoke.mockImplementationOnce(() => firstWrite.promise);
    const first = writeTerminalInput("full", "local", "é".repeat(512 * 1024));
    await expect(writeTerminalInput("full", "local", "a")).rejects.toThrow(
      "Terminal input queue is full",
    );
    await writeTerminalInput("other", "ssh", "b");
    firstWrite.resolve();
    await first;
    await writeTerminalInput("full", "local", "c");
    expect(invoke.mock.calls.map(([, args]) => args.sessionId)).toEqual([
      "full",
      "other",
      "full",
    ]);
  });
});
