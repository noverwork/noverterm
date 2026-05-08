import { describe, expect, it, vi } from "vitest";

import {
  createTerminalKeyHandler,
  type TerminalKeyboardActions,
  type TerminalKeyboardTarget,
} from "$lib/terminal/keyboard-shortcuts.js";

function createKeyEvent(
  key: string,
  options: Pick<KeyboardEventInit, "ctrlKey" | "metaKey" | "shiftKey"> = {},
) {
  const event = new KeyboardEvent("keydown", { key, ...options });
  vi.spyOn(event, "preventDefault");
  vi.spyOn(event, "stopPropagation");
  return event;
}

function createTarget(selection = ""): TerminalKeyboardTarget {
  return {
    hasSelection: () => selection.length > 0,
    getSelection: () => selection,
  };
}

function createActions(): TerminalKeyboardActions {
  return {
    writeClipboard: vi.fn(),
  };
}

describe("terminal keyboard shortcuts", () => {
  it("passes Ctrl+F and Ctrl+G through for terminal apps like vi", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget(), actions);
    const ctrlF = createKeyEvent("f", { ctrlKey: true });
    const ctrlG = createKeyEvent("g", { ctrlKey: true });

    expect(handler(ctrlF)).toBe(true);
    expect(handler(ctrlG)).toBe(true);
    expect(ctrlF.preventDefault).not.toHaveBeenCalled();
    expect(ctrlG.preventDefault).not.toHaveBeenCalled();
  });

  it("passes Cmd+F and Cmd+G through instead of shadowing terminal apps", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget(), actions);
    const metaF = createKeyEvent("f", { metaKey: true });
    const metaG = createKeyEvent("g", { metaKey: true });
    const shiftMetaG = createKeyEvent("g", { metaKey: true, shiftKey: true });

    expect(handler(metaF)).toBe(true);
    expect(handler(metaG)).toBe(true);
    expect(handler(shiftMetaG)).toBe(true);
    expect(metaF.preventDefault).not.toHaveBeenCalled();
    expect(metaG.preventDefault).not.toHaveBeenCalled();
    expect(shiftMetaG.preventDefault).not.toHaveBeenCalled();
  });

  it("copies selected text on Ctrl+C or Cmd+C", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget("selected"), actions);
    const ctrlC = createKeyEvent("c", { ctrlKey: true });
    const metaC = createKeyEvent("c", { metaKey: true });

    expect(handler(ctrlC)).toBe(false);
    expect(handler(metaC)).toBe(false);
    expect(actions.writeClipboard).toHaveBeenNthCalledWith(1, "selected");
    expect(actions.writeClipboard).toHaveBeenNthCalledWith(2, "selected");
    expect(ctrlC.preventDefault).toHaveBeenCalledTimes(1);
    expect(metaC.preventDefault).toHaveBeenCalledTimes(1);
  });

  it("passes Ctrl+C through when there is no selection", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget(), actions);
    const ctrlC = createKeyEvent("c", { ctrlKey: true });

    expect(handler(ctrlC)).toBe(true);
    expect(actions.writeClipboard).not.toHaveBeenCalled();
    expect(ctrlC.preventDefault).not.toHaveBeenCalled();
  });
});
