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
const bytes = (s: string) => enc.encode(s);
// DA1 sentinel + DECRQM 2026 + CPR + OSC 11 query, like omp's startup probe.
const PROBE = "\x1b[?u\x1b[c\x1b[?2026$p\x1b[c\x1b[6n\x1b]11;?\x07";

// xterm parses writes asynchronously; a trailing write's callback marks the
// point where every earlier chunk has been parsed and its replies emitted.
async function settle(controller: TerminalController) {
  const parsed = Promise.withResolvers<void>();
  controller.terminal!.write("", () => parsed.resolve());
  await parsed.promise;
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
    term.terminal!.input("typed", true);
    term.paste("\x1b[pasted");
    container.querySelector("textarea")!.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "ArrowDown",
        code: "ArrowDown",
        keyCode: 40,
        bubbles: true,
        cancelable: true,
      }),
    );

    await settle(term);
    expect(sent).toEqual(["typed", "\x1b[pasted", "\x1b[B"]);
    sent.length = 0;

    live!({ session_id: "s", output: bytes(PROBE), closed: false });
    await settle(term);
    const replies = sent.join("");
    expect(replies).toContain("\x1b[?1;2c"); // DA1
    expect(replies).toContain("\x1b[?2026;2$y"); // DECRPM: 2026 recognised
    expect(replies).toContain("\x1b[2;9R"); // CPR: row 2, after "prompt$ "
    expect(replies).toContain("\x1b]11;rgb:"); // OSC 11 background colour
    term.dispose();
  });

  it("distinguishes Shift+Enter from Enter in a TUI", async () => {
    const sent: string[] = [];
    const term = createTerminal({
      sessionId: "keyboard",
      sessionType: "local",
      config: {
        fontSize: 12,
        fontFamily: "monospace",
        cursorStyle: "block",
        cursorBlink: false,
        scrollback: 100,
      },
      onOutput: (data) => sent.push(data),
    });
    const container = document.createElement("div");
    document.body.appendChild(container);
    try {
      term.init(container);
      term.terminal!.write("\x1b[?1049h");
      await settle(term);
      const textarea = container.querySelector("textarea")!;
      for (const shiftKey of [true, false]) {
        textarea.dispatchEvent(
          new KeyboardEvent("keydown", {
            key: "Enter",
            code: "Enter",
            keyCode: 13,
            shiftKey,
            bubbles: true,
            cancelable: true,
          }),
        );
      }
      expect(sent).toEqual(["\x1b[13;2u", "\r"]);
    } finally {
      term.dispose();
      container.remove();
    }
  });

  it.each(["local", "ssh"] as const)(
    "sends %s input immediately",
    (sessionType) => {
      const sent: string[] = [];
      const term = createTerminal({
        sessionId: "input",
        sessionType,
        config: {
          fontSize: 12,
          fontFamily: "monospace",
          cursorStyle: "block",
          cursorBlink: false,
          scrollback: 100,
        },
        onOutput: (data) => sent.push(data),
      });
      const container = document.createElement("div");
      document.body.appendChild(container);
      try {
        term.init(container);
        term.terminal!.input("a", true);
        term.terminal!.input("b", true);
        expect(sent).toEqual(["a", "b"]);
      } finally {
        term.dispose();
        container.remove();
      }
    },
  );

  it("acknowledges parsed output and preserves final bytes before closing", async () => {
    let live: TerminalOutputCallback | undefined;
    let parsed = false;
    let finalLine = "";
    const term = createTerminal({
      sessionId: "final-output",
      sessionType: "local",
      config: {
        fontSize: 12,
        fontFamily: "monospace",
        cursorStyle: "block",
        cursorBlink: false,
        scrollback: 100,
      },
      subscribeOutput(callback) {
        live = callback;
        return () => {};
      },
      onClose() {
        finalLine = term
          .terminal!.buffer.active.getLine(0)!
          .translateToString(true);
      },
    });
    const container = document.createElement("div");
    document.body.appendChild(container);
    try {
      term.init(container);
      live!(
        {
          session_id: "final-output",
          output: bytes("last bytes"),
          closed: true,
        },
        () => {
          parsed = true;
        },
      );
      expect(parsed).toBe(false);
      await settle(term);
      expect(parsed).toBe(true);
      expect(finalLine).toBe("last bytes");
    } finally {
      term.dispose();
      container.remove();
    }
  });
});
