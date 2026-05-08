export interface TerminalKeyboardTarget {
  hasSelection(): boolean;
  getSelection(): string;
}

export interface TerminalKeyboardActions {
  writeClipboard(selection: string): void;
  openSearchPrompt(): void;
  repeatSearch(backwards: boolean): void;
}

export function createTerminalKeyHandler(
  getTerminal: () => TerminalKeyboardTarget | null,
  actions: TerminalKeyboardActions,
) {
  return (event: KeyboardEvent): boolean => {
    const terminal = getTerminal();
    if (!terminal || event.type !== "keydown") return true;

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

    if (!event.metaKey) return true;

    if (key === "f") {
      event.preventDefault();
      event.stopPropagation();
      actions.openSearchPrompt();
      return false;
    }

    if (key === "g") {
      event.preventDefault();
      event.stopPropagation();
      actions.repeatSearch(event.shiftKey);
      return false;
    }

    return true;
  };
}
