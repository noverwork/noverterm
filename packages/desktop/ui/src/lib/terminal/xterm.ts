import { Terminal } from "@xterm/xterm";
import { ClipboardAddon } from "@xterm/addon-clipboard";
import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { TerminalConfig } from "$lib/app-data-types.js";
import { createTerminalKeyHandler } from "./keyboard-shortcuts.js";

import type {
  SessionType,
  TerminalOutputCallback,
} from "$lib/stores/session.svelte.js";

interface TerminalOptions {
  sessionId: string;
  sessionType: SessionType;
  config: TerminalConfig;
  onOutput?: (data: string) => void;
  onClose?: () => void;
  subscribeOutput?: (callback: TerminalOutputCallback) => () => void;
}

export interface TerminalController {
  readonly terminal: Terminal | null;
  init(container: HTMLElement): void;
  copySelection(): string | null;
  paste(text: string): void;
  clear(): void;
  findNext(term: string): boolean;
  findPrevious(term: string): boolean;
  clearSearch(): void;
  focus(): void;
  fit(): void;
  refresh(): void;
  reveal(): void;
  updateConfig(config: TerminalConfig): void;
  onSelectionChange(callback: () => void): void;
  dispose(): void;
}

function getTheme() {
  return {
    background: "#080c13",
    foreground: "#e5e5e5",
    cursor: "#e5e5e5",
    selectionBackground: "#ffffff20",
    black: "#1f2937",
    red: "#ef4444",
    green: "#22c55e",
    yellow: "#eab308",
    blue: "#3b82f6",
    magenta: "#a855f7",
    cyan: "#06b6d4",
    white: "#e5e5e5",
    brightBlack: "#404040",
    brightRed: "#f87171",
    brightGreen: "#4ade80",
    brightYellow: "#facc15",
    brightBlue: "#60a5fa",
    brightMagenta: "#c084fc",
    brightCyan: "#22d3ee",
    brightWhite: "#ffffff",
  };
}

export function createTerminal(options: TerminalOptions): TerminalController {
  const { sessionId, sessionType } = options;
  let currentConfig = options.config;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let outputUnlisten: (() => void) | null = null;
  let disposed = false;
  let selectionCallback: (() => void) | null = null;
  let initialSizeSynced = false;
  let revealFrame: number | null = null;
  let lastSearchTerm = "";

  const writeCmd = sessionType === "local" ? "local_write" : "ssh_write";
  const resizeCmd = sessionType === "local" ? "local_resize" : "ssh_resize";

  function sendInput(data: string) {
    options.onOutput?.(data);
    invoke(writeCmd, { sessionId, data }).catch(() => void 0);
  }

  async function openExternalUrl(uri: string) {
    try {
      await openUrl(uri);
    } catch {
      window.open(uri, "_blank", "noopener,noreferrer");
    }
  }

  function searchPrompt() {
    const term = window.prompt("Search terminal buffer", lastSearchTerm);
    if (term === null) return;

    lastSearchTerm = term;
    if (term.length === 0) {
      searchAddon?.clearDecorations();
      return;
    }

    searchAddon?.findNext(term);
  }

  function repeatSearch(backwards: boolean) {
    if (!lastSearchTerm) {
      searchPrompt();
      return;
    }

    if (backwards) {
      searchAddon?.findPrevious(lastSearchTerm);
    } else {
      searchAddon?.findNext(lastSearchTerm);
    }
  }

  const handleTerminalKey = createTerminalKeyHandler(
    () => terminal,
    {
      writeClipboard(selection) {
        void navigator.clipboard.writeText(selection).catch(() => undefined);
      },
      openSearchPrompt: searchPrompt,
      repeatSearch,
    },
  );

  function syncInitialSize() {
    if (!terminal || !fitAddon || initialSizeSynced || disposed) return;

    fitAddon.fit();
    initialSizeSynced = true;
    console.info("[xterm:initial-size]", {
      sessionId,
      cols: terminal.cols,
      rows: terminal.rows,
    });
    invoke(resizeCmd, {
      sessionId,
      cols: terminal.cols,
      rows: terminal.rows,
    }).catch(() => void 0);
  }

  function init(container: HTMLElement) {
    if (terminal || disposed) return;

    console.info("[xterm:init]", {
      sessionId,
      hasContainer: Boolean(container),
    });

    terminal = new Terminal({
      theme: getTheme(),
      fontSize: currentConfig.fontSize,
      fontFamily: currentConfig.fontFamily,
      cursorStyle: currentConfig.cursorStyle,
      cursorBlink: currentConfig.cursorBlink,
      scrollback: currentConfig.scrollback,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    searchAddon = new SearchAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(searchAddon);
    terminal.loadAddon(new ClipboardAddon());
    terminal.loadAddon(new WebglAddon());
    terminal.loadAddon(
      new WebLinksAddon((event, uri) => {
        event.preventDefault();
        void openExternalUrl(uri);
      }),
    );

    terminal.attachCustomKeyEventHandler(handleTerminalKey);

    terminal.onResize(({ cols, rows }) => {
      console.info("[xterm:resize]", { sessionId, cols, rows });
      invoke(resizeCmd, { sessionId, cols, rows }).catch(() => void 0);
    });

    terminal.open(container);
    requestAnimationFrame(() => {
      syncInitialSize();
      if (!terminal) return;
      console.info("[xterm:opened]", {
        sessionId,
        cols: terminal.cols,
        rows: terminal.rows,
      });
      terminal.focus();
    });

    terminal.onData((data) => {
      console.info("[xterm:input]", { sessionId, bytes: data.length });
      sendInput(data);
    });

    terminal.onSelectionChange(() => {
      selectionCallback?.();
    });

    outputUnlisten =
      options.subscribeOutput?.((payload) => {
        if (!terminal) return;
        console.info("[xterm:output]", {
          sessionId,
          closed: payload.closed,
          bytes: payload.output.length,
        });
        if (payload.closed) {
          options.onClose?.();
        } else {
          terminal.write(new Uint8Array(payload.output));
        }
      }) ?? null;
  }

  function fit() {
    fitAddon?.fit();
  }

  function refresh() {
    if (!terminal) return;

    if (terminal.rows > 0) {
      terminal.refresh(0, terminal.rows - 1);
    }
  }

  function reveal() {
    if (!terminal) return;

    fitAddon?.fit();

    if (revealFrame !== null) {
      cancelAnimationFrame(revealFrame);
    }

    revealFrame = requestAnimationFrame(() => {
      revealFrame = null;
      if (!terminal || disposed) return;

      if (terminal.buffer.active.type !== "alternate") {
        terminal.scrollToBottom();
      }

      refresh();
      terminal.focus();
    });
  }

  function copySelection() {
    return terminal?.getSelection() || null;
  }

  function paste(text: string) {
    terminal?.paste(text);
  }

  function clear() {
    terminal?.clear();
  }

  function findNext(term: string) {
    lastSearchTerm = term;
    return searchAddon?.findNext(term) ?? false;
  }

  function findPrevious(term: string) {
    lastSearchTerm = term;
    return searchAddon?.findPrevious(term) ?? false;
  }

  function clearSearch() {
    lastSearchTerm = "";
    searchAddon?.clearDecorations();
  }

  function focus() {
    terminal?.focus();
  }

  function onSelectionChange(callback: () => void) {
    selectionCallback = callback;
  }

  function updateConfig(config: TerminalConfig) {
    currentConfig = config;

    if (!terminal) return;

    terminal.options.theme = getTheme();
    terminal.options.fontSize = config.fontSize;
    terminal.options.fontFamily = config.fontFamily;
    terminal.options.cursorStyle = config.cursorStyle;
    terminal.options.cursorBlink = config.cursorBlink;
    terminal.options.scrollback = config.scrollback;

    if (terminal.rows > 0) {
      terminal.refresh(0, terminal.rows - 1);
    }

    fit();
  }

  function dispose() {
    disposed = true;
    if (revealFrame !== null) {
      cancelAnimationFrame(revealFrame);
      revealFrame = null;
    }
    outputUnlisten?.();
    outputUnlisten = null;
    terminal?.dispose();
    terminal = null;
    fitAddon = null;
    searchAddon = null;
  }

  return {
    get terminal() {
      return terminal;
    },
    init,
    copySelection,
    paste,
    clear,
    findNext,
    findPrevious,
    clearSearch,
    focus,
    fit,
    refresh,
    reveal,
    updateConfig,
    onSelectionChange,
    dispose,
  } satisfies TerminalController;
}
