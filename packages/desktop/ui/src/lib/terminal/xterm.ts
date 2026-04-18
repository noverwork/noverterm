import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { TerminalConfig } from "$lib/stores/bootstrap.svelte.js";

import type { SessionType } from "$lib/stores/session.svelte.js";

interface TerminalOptions {
  sessionId: string;
  sessionType: SessionType;
  config: TerminalConfig;
  onOutput?: (data: string) => void;
  onClose?: () => void;
}

export interface TerminalController {
  readonly terminal: Terminal | null;
  init(container: HTMLElement): void;
  copySelection(): string | null;
  paste(text: string): void;
  clear(): void;
  focus(): void;
  fit(): void;
  updateConfig(config: TerminalConfig): void;
  onSelectionChange(callback: () => void): void;
  dispose(): void;
}

function getTheme(theme: TerminalConfig["theme"]) {
  if (theme === "dark") {
    return {
      background: "#0a0a0a",
      foreground: "#e5e5e5",
      cursor: "#e5e5e5",
      selectionBackground: "#ffffff20",
      black: "#0a0a0a",
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

  return {
    background: "#ffffff",
    foreground: "#111827",
    cursor: "#111827",
    selectionBackground: "#11182720",
    black: "#1f2937",
    red: "#dc2626",
    green: "#16a34a",
    yellow: "#ca8a04",
    blue: "#2563eb",
    magenta: "#9333ea",
    cyan: "#0891b2",
    white: "#f9fafb",
    brightBlack: "#6b7280",
    brightRed: "#ef4444",
    brightGreen: "#22c55e",
    brightYellow: "#eab308",
    brightBlue: "#3b82f6",
    brightMagenta: "#a855f7",
    brightCyan: "#06b6d4",
    brightWhite: "#ffffff",
  };
}

export function createTerminal(options: TerminalOptions): TerminalController {
  const { sessionId, sessionType } = options;
  let currentConfig = options.config;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let outputUnlisten: UnlistenFn | null = null;
  let disposed = false;
  let selectionCallback: (() => void) | null = null;
  let initialSizeSynced = false;

  const writeCmd = sessionType === "local" ? "local_write" : "ssh_write";
  const resizeCmd = sessionType === "local" ? "local_resize" : "ssh_resize";
  const outputEvent = sessionType === "local" ? "local_output" : "ssh_output";

  function syncInitialSize() {
    if (!terminal || !fitAddon || initialSizeSynced || disposed) return;

    fitAddon.fit();
    initialSizeSynced = true;
    console.info("[xterm:initial-size]", {
      sessionId,
      cols: terminal.cols,
      rows: terminal.rows,
    });
    invoke(resizeCmd, { sessionId, cols: terminal.cols, rows: terminal.rows }).catch(() => void 0);
  }

  function init(container: HTMLElement) {
    if (terminal || disposed) return;

    console.info("[xterm:init]", { sessionId, hasContainer: Boolean(container) });

    terminal = new Terminal({
      theme: getTheme(currentConfig.theme),
      fontSize: currentConfig.fontSize,
      fontFamily: currentConfig.fontFamily,
      cursorStyle: currentConfig.cursorStyle,
      cursorBlink: currentConfig.cursorBlink,
      scrollback: currentConfig.scrollback,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

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
      options.onOutput?.(data);
      invoke(writeCmd, { sessionId, data }).catch(() => void 0);
    });

    terminal.onSelectionChange(() => {
      selectionCallback?.();
    });

    listen(
      outputEvent,
      (event: { payload: { session_id: string; output: string; closed: boolean } }) => {
        if (event.payload.session_id === sessionId && terminal) {
          console.info("[xterm:output]", {
            sessionId,
            closed: event.payload.closed,
            bytes: event.payload.output.length,
          });
          if (event.payload.closed) {
            options.onClose?.();
          } else {
            terminal.write(event.payload.output);
          }
        }
      }
    ).then((unlisten) => {
      outputUnlisten = unlisten;
    }).catch(() => void 0);

  }

  function fit() {
    fitAddon?.fit();
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

    terminal.options.theme = getTheme(config.theme);
    terminal.options.fontSize = config.fontSize;
    terminal.options.fontFamily = config.fontFamily;
    terminal.options.cursorStyle = config.cursorStyle;
    terminal.options.cursorBlink = config.cursorBlink;
    terminal.options.scrollback = config.scrollback;

    fit();
  }

  function dispose() {
    disposed = true;
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
    copySelection,
    paste,
    clear,
    focus,
    fit,
    updateConfig,
    onSelectionChange,
    dispose,
  } satisfies TerminalController;
}
