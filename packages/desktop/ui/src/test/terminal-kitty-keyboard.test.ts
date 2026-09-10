import { Terminal } from "@xterm/xterm";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createKittyKeyboardProtocol } from "$lib/terminal/kitty-keyboard.js";
import { createTerminalKeyHandler } from "$lib/terminal/keyboard-shortcuts.js";

const cleanup: (() => void)[] = [];
afterEach(() => {
  for (const dispose of cleanup.splice(0)) dispose();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function createProtocol(canReply = () => true) {
  const terminal = new Terminal({ allowProposedApi: true });
  const sent: string[] = [];
  const protocol = createKittyKeyboardProtocol(
    terminal,
    (data) => {
      sent.push(data);
    },
    canReply,
  );
  cleanup.push(() => {
    protocol.dispose();
    terminal.dispose();
  });
  return {
    terminal,
    protocol,
    sent,
    write(data: string) {
      return new Promise<void>((resolve) => {
        terminal.write(data, resolve);
      });
    },
    key(key: string, options: KeyboardEventInit = {}, type = "keydown") {
      return protocol.handleKeyEvent(
        new KeyboardEvent(type, { key, cancelable: true, ...options }),
      );
    },
  };
}

describe("Kitty keyboard protocol", () => {
  it("negotiates replace/add/remove, rejects unknown bits and restores nested modes", async () => {
    const { write, sent, protocol } = createProtocol();
    await write("\x1b[?u\x1b[=1u\x1b[=6;2u\x1b[?u\x1b[=2;3u\x1b[?u");
    expect(sent).toEqual(["\x1b[?0u", "\x1b[?7u", "\x1b[?5u"]);
    await write("\x1b[>31u\x1b[>8u\x1b[<u");
    expect(protocol.flags).toBe(31);
    await write("\x1b[<u");
    expect(protocol.flags).toBe(5);
    await write("\x1b[=255u\x1b[?u\x1b[<99u");
    expect(sent.at(-1)).toBe("\x1b[?31u");
    expect(protocol.flags).toBe(0);
  });

  it("evicts oldest entries from bounded stacks without losing recent pushes", async () => {
    const { write, protocol } = createProtocol();
    await write("\x1b[=1u" + "\x1b[>8u".repeat(40) + "\x1b[<32u");
    expect(protocol.flags).toBe(8);
    await write("\x1b[<u");
    expect(protocol.flags).toBe(0);
  });

  it("keeps normal and alternate stacks separate and resets both", async () => {
    const { write, protocol } = createProtocol();
    await write("\x1b[>1u\x1b[?1049h\x1b[>31u\x1b[?1049l");
    expect(protocol.flags).toBe(1);
    await write("\x1b[?1049h");
    expect(protocol.flags).toBe(31);
    await write("\x1b[!p");
    expect(protocol.flags).toBe(0);
    await write("\x1b[?1049l");
    expect(protocol.flags).toBe(0);
    await write("\x1b[>7u\x1bc");
    expect(protocol.flags).toBe(0);
    await write("\x1b[<u");
    expect(protocol.flags).toBe(0);
  });

  it("replays negotiation without answering historical queries", async () => {
    let live = false;
    const { write, sent, protocol } = createProtocol(() => live);
    await write("\x1b[>31u\x1b[?u");
    expect(protocol.flags).toBe(31);
    expect(sent).toEqual([]);
    live = true;
    await write("\x1b[?u");
    expect(sent).toEqual(["\x1b[?31u"]);
    protocol.reset();
    expect(protocol.flags).toBe(0);
    protocol.dispose();
    await write("\x1b[>7u\x1b[?u");
    expect(sent).toEqual(["\x1b[?31u"]);
  });

  it("disambiguates Ctrl+C, Escape, modified Enter, F3 and keypad navigation", async () => {
    const { write, key, sent } = createProtocol();
    await write("\x1b[>1u");
    expect(key("Enter")).toBe(true);
    expect(key("Tab")).toBe(true);
    expect(key("Backspace")).toBe(true);
    key("c", { code: "KeyC", ctrlKey: true });
    key("Escape", { code: "Escape" });
    key("Enter", { code: "Enter", shiftKey: true, ctrlKey: true });
    key("F3", { code: "F3" });
    key("ArrowLeft", { code: "Numpad4", location: 3 });
    expect(sent).toEqual([
      "\x1b[99;5u",
      "\x1b[27u",
      "\x1b[13;6u",
      "\x1b[13~",
      "\x1b[57417u",
    ]);
  });

  it("reports repeat/release for encoded keys but not ordinary text or recovery keys", async () => {
    const { write, key, sent } = createProtocol();
    await write("\x1b[>3u");
    key("c", { code: "KeyC", ctrlKey: true });
    key("c", { code: "KeyC", ctrlKey: true, repeat: true });
    key("c", { code: "KeyC" }, "keyup");
    key("a", { code: "KeyA" });
    key("a", { code: "KeyA" }, "keyup");
    key("Enter", { code: "Enter", shiftKey: true });
    key("Enter", { code: "Enter", shiftKey: true }, "keyup");
    expect(sent).toEqual([
      "\x1b[99;5u",
      "\x1b[99;5:2u",
      "\x1b[99;1:3u",
      "\x1b[13;2u",
    ]);
  });

  it("encodes alternate keys, associated text and modifier transitions", async () => {
    const { write, key, sent } = createProtocol();
    await write("\x1b[>31u");
    key("+", { code: "Equal", shiftKey: true });
    key("+", { code: "Equal", shiftKey: true, repeat: true });
    key("=", { code: "Equal" }, "keyup");
    key("С", { code: "KeyC", shiftKey: true, ctrlKey: true });
    key("Shift", { code: "ShiftRight", location: 2, shiftKey: true });
    key("Shift", { code: "ShiftRight", location: 2 }, "keyup");
    key("Enter", { code: "Enter" });
    expect(sent).toEqual([
      "\x1b[61:43;2;43u",
      "\x1b[61:43;2:2;43u",
      "\x1b[61;1:3u",
      "\x1b[1089:1057:99;6u",
      "\x1b[57447;2u",
      "\x1b[57447;1:3u",
      "\x1b[13u",
    ]);
  });

  it("keeps composition native, encodes committed text, and never rewrites paste or replies", async () => {
    const { terminal, protocol, write, key, sent } = createProtocol();
    vi.stubGlobal("matchMedia", () => ({
      matches: false,
      addListener() {},
      removeListener() {},
      addEventListener() {},
      removeEventListener() {},
    }));
    const container = document.createElement("div");
    document.body.appendChild(container);
    terminal.open(container);
    const element = terminal.element!;
    cleanup.push(() => {
      container.remove();
    });
    protocol.attachInputListeners();
    await write("\x1b[>31u");
    element.dispatchEvent(new CompositionEvent("compositionstart"));
    expect(key("Enter", { isComposing: true })).toBe(true);
    expect(key("a", { code: "KeyA" })).toBe(true);
    element.dispatchEvent(
      new CompositionEvent("compositionend", { data: "你好" }),
    );
    expect(protocol.encodeInput("你好")).toBe("\x1b[0;;20320:22909u");
    expect(protocol.encodeInput("\x1b[1;2R")).toBe("\x1b[1;2R");
    element.dispatchEvent(new Event("paste"));
    expect(protocol.encodeInput("plain paste")).toBe("plain paste");
    await Promise.resolve();
    const input = terminal.onData((data) => {
      sent.push(protocol.encodeInput(data));
    });
    protocol.paste("programmatic paste");
    input.dispose();
    expect(sent).toEqual(["programmatic paste"]);
    await write("\x1b[=8u");
    expect(protocol.encodeInput("你好")).toBe("");
    expect(key("Dead", { code: "Quote" })).toBe(true);
    expect(key("é", { code: "KeyE" })).toBe(true);
  });

  it("orders deferred IME text before the next encoded key", async () => {
    const { terminal, protocol, write, key, sent } = createProtocol();
    await write("\x1b[>24u");
    vi.useFakeTimers();
    const element = document.createElement("div");
    const textarea = document.createElement("textarea");
    element.append(textarea);
    Object.defineProperty(terminal, "element", { value: element });
    // xterm commits textarea composition from its own zero-delay timer.
    textarea.addEventListener("compositionend", () => {
      window.setTimeout(() => {
        sent.push(protocol.encodeInput("é"));
      }, 0);
    });
    protocol.attachInputListeners();
    textarea.dispatchEvent(
      new CompositionEvent("compositionstart", { bubbles: true }),
    );
    textarea.dispatchEvent(
      new CompositionEvent("compositionend", { bubbles: true, data: "é" }),
    );
    key("Enter", { code: "Enter" });
    expect(sent).toEqual([]);
    await vi.runAllTimersAsync();
    expect(sent).toEqual(["\x1b[0;;233u", "\x1b[13u"]);
  });

  it("gives shortcuts priority without swallowing modified keys or protocol Ctrl+C", async () => {
    const { protocol, write, sent } = createProtocol();
    let selection = "selected";
    let copies = 0;
    const handler = createTerminalKeyHandler(
      () => ({
        hasSelection: () => Boolean(selection),
        getSelection: () => selection,
      }),
      {
        sendInput(data) {
          sent.push(data);
        },
        writeClipboard() {
          copies += 1;
        },
        openSearchPrompt() {},
        repeatSearch() {},
        closeTerminal() {},
      },
      () => protocol,
    );
    await write("\x1b[>31u");
    handler(
      new KeyboardEvent("keydown", { key: "c", code: "KeyC", ctrlKey: true }),
    );
    handler(
      new KeyboardEvent("keyup", { key: "c", code: "KeyC", ctrlKey: true }),
    );
    expect(copies).toBe(1);
    expect(sent).toEqual([]);
    selection = "";
    handler(
      new KeyboardEvent("keydown", { key: "c", code: "KeyC", ctrlKey: true }),
    );
    handler(
      new KeyboardEvent("keydown", { key: "+", code: "Equal", shiftKey: true }),
    );
    handler(
      new KeyboardEvent("keydown", {
        key: "f",
        code: "KeyF",
        ctrlKey: true,
        altKey: true,
      }),
    );
    expect(sent).toEqual(["\x1b[99;5u", "\x1b[61:43;2;43u", "\x1b[102;7u"]);
    expect(
      handler(new KeyboardEvent("keydown", { key: "v", metaKey: true })),
    ).toBe(true);
  });
});
