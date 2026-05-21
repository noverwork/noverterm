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
    openSearchPrompt: vi.fn(),
    repeatSearch: vi.fn(),
    closeTerminal: vi.fn(),
  };
}

describe("terminal keyboard shortcuts", () => {
  it("uses Ctrl+F for terminal search and passes Ctrl+G through", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget(), actions);
    const ctrlF = createKeyEvent("f", { ctrlKey: true });
    const ctrlG = createKeyEvent("g", { ctrlKey: true });

    expect(handler(ctrlF)).toBe(false);
    expect(handler(ctrlG)).toBe(true);
    expect(actions.openSearchPrompt).toHaveBeenCalledTimes(1);
    expect(actions.repeatSearch).not.toHaveBeenCalled();
    expect(ctrlF.preventDefault).toHaveBeenCalledTimes(1);
    expect(ctrlG.preventDefault).not.toHaveBeenCalled();
  });

  it("uses Cmd+F and Cmd+G for terminal search", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget(), actions);
    const metaF = createKeyEvent("f", { metaKey: true });
    const metaG = createKeyEvent("g", { metaKey: true });
    const shiftMetaG = createKeyEvent("g", { metaKey: true, shiftKey: true });

    expect(handler(metaF)).toBe(false);
    expect(handler(metaG)).toBe(false);
    expect(handler(shiftMetaG)).toBe(false);
    expect(actions.openSearchPrompt).toHaveBeenCalledTimes(1);
    expect(actions.repeatSearch).toHaveBeenNthCalledWith(1, false);
    expect(actions.repeatSearch).toHaveBeenNthCalledWith(2, true);
    expect(metaF.preventDefault).toHaveBeenCalledTimes(1);
    expect(metaG.preventDefault).toHaveBeenCalledTimes(1);
    expect(shiftMetaG.preventDefault).toHaveBeenCalledTimes(1);
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

  it("closes the terminal on Cmd+W and passes Ctrl+W through", () => {
    const actions = createActions();
    const handler = createTerminalKeyHandler(() => createTarget(), actions);
    const metaW = createKeyEvent("w", { metaKey: true });
    const ctrlW = createKeyEvent("w", { ctrlKey: true });

    expect(handler(metaW)).toBe(false);
    expect(handler(ctrlW)).toBe(true);
    expect(actions.closeTerminal).toHaveBeenCalledTimes(1);
    expect(metaW.preventDefault).toHaveBeenCalledTimes(1);
    expect(metaW.stopPropagation).toHaveBeenCalledTimes(1);
    expect(ctrlW.preventDefault).not.toHaveBeenCalled();
  });
});
