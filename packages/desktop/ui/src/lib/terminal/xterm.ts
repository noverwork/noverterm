import { Terminal } from "@xterm/xterm";
import { ClipboardAddon } from "@xterm/addon-clipboard";
import { FitAddon } from "@xterm/addon-fit";
import { ImageAddon } from "@xterm/addon-image";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";
import { commands } from "../../bindings.js";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { TerminalConfig } from "$lib/app-data-types.js";
import { createTerminalKeyHandler } from "./keyboard-shortcuts.js";
import { writeTerminalInput } from "./input.js";
import {
  createKittyKeyboardProtocol,
  type KittyKeyboardProtocol,
} from "./kitty-keyboard.js";

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
  onError?: (message: string) => void;
  onRequestClose?: () => void;
  onSearchRequest?: () => void;
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
    selectionBackground: "#2563eb",
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
    scrollbarSliderBackground: "transparent",
    scrollbarSliderHoverBackground: "transparent",
    scrollbarSliderActiveBackground: "transparent",
    overviewRulerBorder: "transparent",
  };
}

function loadImageAddon(terminal: Terminal) {
  try {
    terminal.loadAddon(new ImageAddon({ enableSizeReports: false }));
  } catch (error) {
    console.warn("[xterm:image-addon] failed to load", error);
  }
}

export function createTerminal(options: TerminalOptions): TerminalController {
  const { sessionId, sessionType } = options;
  let currentConfig = options.config;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let outputUnlisten: (() => void) | null = null;
  let disposed = false;
  let inputFailed = false;
  let keyboardProtocol: KittyKeyboardProtocol | null = null;
  let selectionCallback: (() => void) | null = null;
  // Count of transcript chunks still queued in xterm's write buffer. Replies
  // xterm generates while re-parsing recorded queries (DA1, CPR, DECRQM, OSC
  // color) must not reach the pty: the program that asked is long gone and
  // the shell would echo them into the prompt.
  let replayWritesPending = 0;
  let userInput = false;
  let webglAddon: WebglAddon | null = null;
  let initialSizeSynced = false;
  let lastSearchTerm = "";

  const resize =
    sessionType === "local" ? commands.localResize : commands.sshResize;

  function reportError(error: unknown) {
    if (disposed) return;
    const message = error instanceof Error ? error.message : String(error);
    console.error("[terminal:io]", sessionId, message);
    options.onError?.(message);
  }

  function sendInput(data: string) {
    if (disposed || inputFailed || data.length === 0) return;
    options.onOutput?.(data);
    void writeTerminalInput(sessionId, sessionType, data).catch(
      (error: unknown) => {
        inputFailed = true;
        reportError(error);
      },
    );
  }

  function markUserInput() {
    userInput = true;
    queueMicrotask(() => {
      userInput = false;
    });
  }

  async function openExternalUrl(uri: string) {
    try {
      await openUrl(uri);
    } catch {
      window.open(uri, "_blank", "noopener,noreferrer");
    }
  }

  function requestSearch() {
    options.onSearchRequest?.();
  }

  function repeatSearch(backwards: boolean) {
    if (!lastSearchTerm) {
      requestSearch();
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
      sendInput,
      writeClipboard(selection) {
        void navigator.clipboard.writeText(selection).catch(() => undefined);
      },
      openSearchPrompt: requestSearch,
      repeatSearch,
      closeTerminal() {
        options.onRequestClose?.();
      },
    },
    () => keyboardProtocol,
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
    void resize(sessionId, terminal.cols, terminal.rows)
      .then((result) => {
        if (result.status === "error") reportError(result.error);
      })
      .catch(reportError);
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
      overviewRuler: {
        width: 1,
      },
      allowProposedApi: true,
      // TUI apps (herdr, k9s) enable mouse reporting, which swallows link
      // clicks; Alt+click is the macOS escape hatch back to selection/links.
      macOptionClickForcesSelection: true,
    });

    fitAddon = new FitAddon();
    searchAddon = new SearchAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(searchAddon);
    terminal.loadAddon(new ClipboardAddon());
    loadImageAddon(terminal);
    webglAddon = new WebglAddon();
    // WebKit drops WebGL contexts when the window is occluded or too many are
    // live; disposing the addon falls back to the DOM renderer.
    webglAddon.onContextLoss(() => {
      console.warn("[xterm:webgl] context lost", { sessionId });
      webglAddon?.dispose();
      webglAddon = null;
      refresh();
    });
    terminal.loadAddon(webglAddon);
    terminal.loadAddon(
      new WebLinksAddon((event, uri) => {
        event.preventDefault();
        void openExternalUrl(uri);
      }),
    );

    keyboardProtocol = createKittyKeyboardProtocol(
      terminal,
      sendInput,
      () => replayWritesPending === 0 && !disposed,
    );
    terminal.attachCustomKeyEventHandler(handleTerminalKey);

    terminal.onResize(({ cols, rows }) => {
      console.info("[xterm:resize]", { sessionId, cols, rows });
      void resize(sessionId, cols, rows)
        .then((result) => {
          if (result.status === "error") reportError(result.error);
        })
        .catch(reportError);
    });

    terminal.open(container);
    keyboardProtocol.attachInputListeners();
    terminal.onKey(markUserInput);
    terminal.element?.addEventListener("paste", markUserInput, true);
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
      // Parser replies start with ESC. Keyboard/paste events and committed
      // text must still reach the PTY while the transcript is being parsed.
      if (replayWritesPending > 0 && !userInput && data.startsWith("\x1b"))
        return;
      const input = keyboardProtocol?.encodeInput(data) ?? data;
      sendInput(input);
    });

    terminal.onSelectionChange(() => {
      selectionCallback?.();
    });

    let replaying = true;
    outputUnlisten =
      options.subscribeOutput?.((payload, consumed) => {
        if (!terminal) {
          consumed?.();
          return;
        }
        if (replaying) replayWritesPending += 1;
        const isReplay = replaying;
        terminal.write(payload.output, () => {
          if (isReplay) replayWritesPending -= 1;
          consumed?.();
          if (disposed) return;
          if (payload.error) {
            inputFailed = true;
            reportError(payload.error);
          } else if (payload.closed) {
            options.onClose?.();
          }
        });
      }) ?? null;
    // subscribeOutput replays the recorded transcript synchronously before
    // returning; everything after this point is live output.
    replaying = false;
  }

  function fit() {
    fitAddon?.fit();
  }

  function refresh() {
    if (!terminal) return;

    // The GPU may reclaim the glyph atlas while the window is occluded or
    // idle, without reporting a context loss. Rebuild it before redrawing.
    webglAddon?.clearTextureAtlas();

    if (terminal.rows > 0) {
      terminal.refresh(0, terminal.rows - 1);
    }
  }

  function reveal() {
    if (!terminal) return;

    // Everything here runs synchronously so the repaint lands in the same frame
    // the tab becomes visible. The viewport is left where the user had it.
    fitAddon?.fit();
    refresh();
    terminal.focus();
  }

  function copySelection() {
    return terminal?.getSelection() || null;
  }

  function paste(text: string) {
    markUserInput();
    keyboardProtocol?.paste(text);
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
    outputUnlisten?.();
    outputUnlisten = null;
    terminal?.element?.removeEventListener("paste", markUserInput, true);
    keyboardProtocol?.dispose();
    keyboardProtocol = null;
    terminal?.dispose();
    terminal = null;
    webglAddon = null;
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
