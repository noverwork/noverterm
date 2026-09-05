import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => undefined),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@xterm/addon-webgl", () => ({
  WebglAddon: class {
    onContextLoss() {}
    activate() {}
    dispose() {}
    clearTextureAtlas() {}
  },
}));
vi.mock("@xterm/addon-image", () => ({
  ImageAddon: class {
    activate() {}
    dispose() {}
  },
}));

import type { TerminalController } from "$lib/terminal/xterm.js";
import { createTerminal } from "$lib/terminal/xterm.js";
import type { TerminalOutputCallback } from "$lib/stores/session.svelte.js";

window.matchMedia = () =>
  ({
    matches: false,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
  }) as unknown as MediaQueryList;
// Overview ruler wants a 2D context; a no-op proxy is enough for parsing.
HTMLCanvasElement.prototype.getContext = (() =>
  new Proxy(
    {},
    { get: () => () => undefined },
  )) as unknown as HTMLCanvasElement["getContext"];

const enc = new TextEncoder();
const bytes = (s: string) => Array.from(enc.encode(s));
// DA1 sentinel + DECRQM 2026 + CPR + OSC 11 query, like omp's startup probe.
const PROBE = "\x1b[?u\x1b[c\x1b[?2026$p\x1b[c\x1b[6n\x1b]11;?\x07";

// xterm parses writes asynchronously; a trailing write's callback marks the
// point where every earlier chunk has been parsed and its replies emitted.
// The extra frame lets sendInput's rAF batch flush to onOutput.
async function settle(controller: TerminalController) {
  const parsed = Promise.withResolvers<void>();
  controller.terminal!.write("", () => parsed.resolve());
  await parsed.promise;
  const frame = Promise.withResolvers<void>();
  requestAnimationFrame(() => frame.resolve());
  await frame.promise;
}

describe("terminal query replies", () => {
  it("suppresses replies while replaying the transcript, answers live queries", async () => {
    let live: TerminalOutputCallback | null = null;
    const sent: string[] = [];
    const term = createTerminal({
      sessionId: "s",
      sessionType: "local",
      config: {
        fontSize: 12,
        fontFamily: "monospace",
        cursorStyle: "block",
        cursorBlink: false,
        scrollback: 100,
      },
      onOutput: (d) => sent.push(d),
      subscribeOutput: (cb) => {
        cb({
          session_id: "s",
          output: bytes("old\r\n" + PROBE),
          closed: false,
        });
        cb({ session_id: "s", output: bytes("prompt$ "), closed: false });
        live = cb;
        return () => {};
      },
    });
    const container = document.createElement("div");
    document.body.appendChild(container);
    term.init(container);

    await settle(term);
    expect(sent).toEqual([]);

    live!({ session_id: "s", output: bytes(PROBE), closed: false });
    await settle(term);
    const replies = sent.join("");
    expect(replies).toContain("\x1b[?1;2c"); // DA1
    expect(replies).toContain("\x1b[?2026;2$y"); // DECRPM: 2026 recognised
    expect(replies).toContain("\x1b[2;9R"); // CPR: row 2, after "prompt$ "
    expect(replies).toContain("\x1b]11;rgb:"); // OSC 11 background colour
    term.dispose();
  });
});
