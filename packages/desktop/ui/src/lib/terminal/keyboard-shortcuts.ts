import type { KittyKeyboardProtocol } from "./kitty-keyboard.js";

export interface TerminalKeyboardTarget {
  hasSelection(): boolean;
  getSelection(): string;
}

export interface TerminalKeyboardActions {
  sendInput(data: string): void;
  writeClipboard(selection: string): void;
  openSearchPrompt(): void;
  repeatSearch(backwards: boolean): void;
  closeTerminal(): void;
}

function isShiftPrintableSymbol(event: KeyboardEvent) {
  return (
    event.shiftKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.altKey &&
    event.key.length === 1 &&
    event.key.trim().length > 0 &&
    !/^[a-zA-Z0-9]$/.test(event.key)
  );
}

export function createTerminalKeyHandler(
  getTerminal: () => TerminalKeyboardTarget | null,
  actions: TerminalKeyboardActions,
  getProtocol: () => KittyKeyboardProtocol | null = () => null,
) {
  const shortcutKeys = new Set<string>();
  return (event: KeyboardEvent): boolean => {
    const terminal = getTerminal();
    if (!terminal || event.isComposing || event.keyCode === 229) return true;
    const keyId = event.code || event.key.toLowerCase();
    if (event.type === "keyup" && shortcutKeys.delete(keyId)) return false;
    const protocol = getProtocol();
    if (event.type !== "keydown")
      return protocol?.handleKeyEvent(event) ?? true;
    if (!event.repeat) shortcutKeys.delete(keyId);

    const key = event.key.toLowerCase();
    const command = event.metaKey && !event.ctrlKey && !event.altKey;
    const control = event.ctrlKey && !event.metaKey && !event.altKey;
    let action: (() => void) | undefined;
    if (
      key === "c" &&
      !event.shiftKey &&
      (command || control) &&
      terminal.hasSelection()
    ) {
      const selection = terminal.getSelection();
      if (selection)
        action = () => {
          actions.writeClipboard(selection);
        };
    } else if (key === "f" && !event.shiftKey && (command || control)) {
      action = () => {
        actions.openSearchPrompt();
      };
    } else if (key === "w" && !event.shiftKey && command) {
      action = () => {
        actions.closeTerminal();
      };
    } else if (key === "g" && command) {
      action = () => {
        actions.repeatSearch(event.shiftKey);
      };
    }
    if (action) {
      event.preventDefault();
      event.stopPropagation();
      shortcutKeys.add(keyId);
      if (!event.repeat) action();
      return false;
    }
    // Native paste remains a paste event, never a Kitty key/text event.
    if (key === "v" && (command || (control && event.shiftKey))) return true;
    if (
      event.key === "Insert" &&
      event.shiftKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey
    )
      return true;
    if (protocol?.handleKeyEvent(event) === false) return false;

    // xterm encodes Shift+Enter as CR; retain compatibility outside negotiation.
    if (
      !protocol?.flags &&
      event.key === "Enter" &&
      event.shiftKey &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey
    ) {
      event.preventDefault();
      event.stopPropagation();
      actions.sendInput("\x1b[13;2u");
      return false;
    }
    // Negotiated key reporting gets first refusal, including shifted symbols.
    if (!(protocol && protocol.flags & 8) && isShiftPrintableSymbol(event)) {
      event.preventDefault();
      event.stopPropagation();
      actions.sendInput(event.key);
      return false;
    }
    return true;
  };
}
