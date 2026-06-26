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
) {
  return (event: KeyboardEvent): boolean => {
    const terminal = getTerminal();
    if (!terminal || event.type !== "keydown") return true;

    // Let modifier keys pass through so xterm.js tracks internal modifier state
    // correctly. Without this, the first Shift/Ctrl/Alt/Meta press may be lost.
    if (
      event.key === "Shift" ||
      event.key === "Control" ||
      event.key === "Alt" ||
      event.key === "Meta" ||
      event.key === "CapsLock"
    ) {
      return true;
    }

    if (isShiftPrintableSymbol(event)) {
      event.preventDefault();
      event.stopPropagation();
      actions.sendInput(event.key);
      return false;
    }

    const key = event.key.toLowerCase();
    if (key === "c") {
      const isCopyShortcut = event.metaKey || event.ctrlKey;
      if (!isCopyShortcut) return true;
      if (!terminal.hasSelection()) return true;

      const selection = terminal.getSelection();
      if (!selection) return true;

      event.preventDefault();
      event.stopPropagation();
      actions.writeClipboard(selection);
      return false;
    }

    if (key === "f" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      event.stopPropagation();
      actions.openSearchPrompt();
      return false;
    }

    if (key === "w" && event.metaKey) {
      event.preventDefault();
      event.stopPropagation();
      actions.closeTerminal();
      return false;
    }

    if (!event.metaKey) return true;

    if (key === "g") {
      event.preventDefault();
      event.stopPropagation();
      actions.repeatSearch(event.shiftKey);
      return false;
    }

    return true;
  };
}
