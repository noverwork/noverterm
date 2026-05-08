import { Terminal } from "@xterm/xterm";
import { ClipboardAddon } from "@xterm/addon-clipboard";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import type { TerminalConfig } from "$lib/stores/bootstrap.svelte.js";

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
  hasSelection(): boolean;
  copySelection(): string | null;
  paste(text: string): void;
  clear(): void;
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
  let outputUnlisten: (() => void) | null = null;
  let disposed = false;
  let selectionCallback: (() => void) | null = null;
  let initialSizeSynced = false;
  let revealFrame: number | null = null;

  const writeCmd = sessionType === "local" ? "local_write" : "ssh_write";
  const resizeCmd = sessionType === "local" ? "local_resize" : "ssh_resize";

  function sendInput(data: string) {
    options.onOutput?.(data);
    invoke(writeCmd, { sessionId, data }).catch(() => void 0);
  }

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
      macOptionClickForcesSelection: true,
      rightClickSelectsWord: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new ClipboardAddon());

    terminal.attachCustomKeyEventHandler((event) => {
      const isCopyShortcut = (event.metaKey || event.ctrlKey) && event.key === "c";
      if (!terminal || event.type !== "keydown" || !isCopyShortcut) return true;
      if (!terminal.hasSelection()) return true;

      const selection = terminal.getSelection();
      if (!selection) return true;

      event.preventDefault();
      event.stopPropagation();
      void navigator.clipboard.writeText(selection).catch(() => undefined);
      return false;
    });

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

      refresh();
      terminal.focus();
    });
  }

  function hasSelection() {
    return terminal?.hasSelection() ?? false;
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
  }

  return {
    get terminal() {
      return terminal;
    },
    init,
    hasSelection,
    copySelection,
    paste,
    clear,
    focus,
    fit,
    refresh,
    reveal,
    updateConfig,
    onSelectionChange,
    dispose,
  } satisfies TerminalController;
}
