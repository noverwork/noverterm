import type { IDisposable, Terminal } from "@xterm/xterm";

const supportedFlags = 31;
const stackLimit = 32;
const csi = "\x1b[";
const shiftedSymbols = '~!@#$%^&*()_+{}|:"<>?';
const unshiftedSymbols = "`1234567890-=[]\\;',./";
const physicalSymbols: Record<string, string> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Space: " ",
  IntlBackslash: "\\",
};
const functionalKeys: Record<string, [number, string]> = {
  Escape: [27, "u"],
  Enter: [13, "u"],
  Tab: [9, "u"],
  Backspace: [127, "u"],
  Insert: [2, "~"],
  Delete: [3, "~"],
  PageUp: [5, "~"],
  PageDown: [6, "~"],
  ArrowUp: [1, "A"],
  ArrowDown: [1, "B"],
  ArrowRight: [1, "C"],
  ArrowLeft: [1, "D"],
  Home: [1, "H"],
  End: [1, "F"],
  Clear: [1, "E"],
  CapsLock: [57358, "u"],
  ScrollLock: [57359, "u"],
  NumLock: [57360, "u"],
  PrintScreen: [57361, "u"],
  Pause: [57362, "u"],
  ContextMenu: [57363, "u"],
  MediaPlay: [57428, "u"],
  MediaPause: [57429, "u"],
  MediaPlayPause: [57430, "u"],
  MediaReverse: [57431, "u"],
  MediaStop: [57432, "u"],
  MediaFastForward: [57433, "u"],
  MediaRewind: [57434, "u"],
  MediaTrackNext: [57435, "u"],
  MediaTrackPrevious: [57436, "u"],
  MediaRecord: [57437, "u"],
  AudioVolumeDown: [57438, "u"],
  AudioVolumeUp: [57439, "u"],
  AudioVolumeMute: [57440, "u"],
  F1: [1, "P"],
  F2: [1, "Q"],
  F3: [13, "~"],
  F4: [1, "S"],
  F5: [15, "~"],
  F6: [17, "~"],
  F7: [18, "~"],
  F8: [19, "~"],
  F9: [20, "~"],
  F10: [21, "~"],
  F11: [23, "~"],
  F12: [24, "~"],
};
const keypadKeys: Record<string, number> = {
  NumpadDecimal: 57409,
  NumpadDivide: 57410,
  NumpadMultiply: 57411,
  NumpadSubtract: 57412,
  NumpadAdd: 57413,
  NumpadEnter: 57414,
  NumpadEqual: 57415,
  NumpadComma: 57416,
};
const keypadNavigation: Record<string, number> = {
  ArrowLeft: 57417,
  ArrowRight: 57418,
  ArrowUp: 57419,
  ArrowDown: 57420,
  PageUp: 57421,
  PageDown: 57422,
  Home: 57423,
  End: 57424,
  Insert: 57425,
  Delete: 57426,
  Clear: 57427,
};
const modifierKeys: Record<string, number> = {
  Shift: 57441,
  Control: 57442,
  Alt: 57443,
  Meta: 57444,
  Super: 57444,
  Hyper: 57445,
  AltGraph: 57453,
};

function codepoint(text: string): number | undefined {
  const point = text.codePointAt(0);
  return point !== undefined && String.fromCodePoint(point) === text
    ? point
    : undefined;
}

function baseKey(code: string): string | undefined {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  return physicalSymbols[code];
}

function textPoints(text: string): string {
  return Array.from(text, (character) => character.codePointAt(0)!)
    .filter((point) => point >= 32 && (point < 127 || point > 159))
    .join(":");
}

interface KeyIdentity {
  number: number;
  suffix: string;
  printable: boolean;
}

export interface KittyKeyboardProtocol {
  readonly flags: number;
  handleKeyEvent(event: KeyboardEvent): boolean;
  encodeInput(data: string): string;
  attachInputListeners(): void;
  paste(text: string): void;
  reset(): void;
  dispose(): void;
}

/** Parser mutations also run during replay; only query replies are suppressed. */
export function createKittyKeyboardProtocol(
  terminal: Terminal,
  sendInput: (data: string) => void,
  canReply: () => boolean = () => true,
): KittyKeyboardProtocol {
  const states = {
    normal: { flags: 0, stack: [] as number[] },
    alternate: { flags: 0, stack: [] as number[] },
  };
  const disposables: IDisposable[] = [];
  const heldKeys = new Map<string, KeyIdentity>();
  const layout = new Map<string, string>();
  let disposed = false;
  let composing = false;
  let deadKey = false;
  let pasting = false;
  let compositionTimer: number | undefined;
  const afterComposition: string[] = [];
  let inputElement: HTMLElement | undefined;
  const state = () => states[terminal.buffer.active.type];
  const keyboard = (
    navigator as Navigator & {
      keyboard?: {
        getLayoutMap(): Promise<{
          forEach(callback: (value: string, key: string) => void): void;
        }>;
      };
    }
  ).keyboard;
  if (keyboard) {
    void keyboard
      .getLayoutMap()
      .then((map) => {
        if (!disposed)
          map.forEach((value, key) => {
            layout.set(key, value);
          });
      })
      .catch(() => undefined);
  }

  function reset() {
    for (const value of Object.values(states)) {
      value.flags = 0;
      value.stack.length = 0;
    }
    heldKeys.clear();
    composing = false;
    deadKey = false;
    clearTimeout(compositionTimer);
    compositionTimer = undefined;
    afterComposition.length = 0;
  }

  function register(
    prefix: string,
    callback: (params: (number | number[])[]) => void,
  ) {
    disposables.push(
      terminal.parser.registerCsiHandler({ prefix, final: "u" }, (params) => {
        if (!disposed) callback(params);
        return true;
      }),
    );
  }
  register("?", () => {
    if (canReply()) sendInput(`${csi}?${state().flags}u`);
  });
  register("=", (params) => {
    if (params.some(Array.isArray)) return;
    const flags = Number(params[0] ?? 0) & supportedFlags;
    const mode = Number(params[1] ?? 1) || 1;
    if (mode === 1) state().flags = flags;
    else if (mode === 2) state().flags |= flags;
    else if (mode === 3) state().flags &= ~flags;
  });
  register(">", (params) => {
    if (params.some(Array.isArray)) return;
    const current = state();
    if (current.stack.length === stackLimit) current.stack.shift();
    current.stack.push(current.flags);
    current.flags = Number(params[0] ?? 0) & supportedFlags;
  });
  register("<", (params) => {
    if (params.some(Array.isArray)) return;
    const current = state();
    const count = Number(params[0] ?? 1) || 1;
    if (count > current.stack.length) {
      current.flags = 0;
      current.stack.length = 0;
    } else {
      current.flags = current.stack[current.stack.length - count];
      current.stack.length -= count;
    }
  });
  // Fall through so xterm still performs RIS/DECSTR itself.
  disposables.push(
    terminal.parser.registerEscHandler({ final: "c" }, () => {
      reset();
      return false;
    }),
  );
  disposables.push(
    terminal.parser.registerCsiHandler(
      { intermediates: "!", final: "p" },
      () => {
        reset();
        return false;
      },
    ),
  );
  disposables.push(
    terminal.buffer.onBufferChange(() => {
      heldKeys.clear();
    }),
  );

  function identity(
    event: KeyboardEvent,
    disambiguate: boolean,
  ): KeyIdentity | undefined {
    const printable = codepoint(event.key) !== undefined;
    if (disambiguate && event.location === 3) {
      const navigation = keypadNavigation[event.key];
      const number =
        navigation ??
        keypadKeys[event.code] ??
        (/^Numpad[0-9]$/.test(event.code)
          ? 57399 + Number(event.code.slice(6))
          : undefined);
      if (number !== undefined) return { number, suffix: "u", printable };
    }
    const modifier = modifierKeys[event.key];
    if (modifier !== undefined)
      return {
        number:
          modifier + (event.location === 2 && event.key !== "AltGraph" ? 6 : 0),
        suffix: "u",
        printable: false,
      };
    const functional = functionalKeys[event.key];
    if (functional)
      return { number: functional[0], suffix: functional[1], printable: false };
    if (/^F(1[3-9]|2[0-9]|3[0-5])$/.test(event.key)) {
      return {
        number: 57376 + Number(event.key.slice(1)) - 13,
        suffix: "u",
        printable: false,
      };
    }
    if (!printable) return undefined;
    let key = event.key.toLowerCase();
    if (
      !event.shiftKey &&
      !event.altKey &&
      !event.ctrlKey &&
      !event.metaKey &&
      event.code &&
      codepoint(key) !== undefined
    )
      layout.set(event.code, key);
    if (event.shiftKey) {
      const shifted = shiftedSymbols.indexOf(event.key);
      const standard =
        shifted >= 0 &&
        (!event.code || baseKey(event.code) === unshiftedSymbols[shifted]);
      key =
        layout.get(event.code) ?? (standard ? unshiftedSymbols[shifted] : key);
    }
    return {
      number: codepoint(key) ?? codepoint(event.key)!,
      suffix: "u",
      printable: true,
    };
  }

  function handleKeyEvent(event: KeyboardEvent): boolean {
    if (disposed || (event.type !== "keydown" && event.type !== "keyup"))
      return true;
    if (
      event.isComposing ||
      composing ||
      event.keyCode === 229 ||
      event.key === "Process"
    )
      return true;
    if (event.key === "Dead") {
      deadKey = true;
      return true;
    }
    if (deadKey) {
      if (event.type === "keydown" && !modifierKeys[event.key]) deadKey = false;
      return true;
    }
    const flags = state().flags;
    if (!flags) return true;
    const all = Boolean(flags & 8);
    const disambiguate = Boolean(flags & 1) || all;
    const reportEvents = Boolean(flags & 2);
    const release = event.type === "keyup";
    const keyId = event.code || event.key;
    const held = heldKeys.get(keyId);
    if (release) heldKeys.delete(keyId);
    // AltGraph text is committed through xterm's native input/composition path.
    if (event.getModifierState("AltGraph") && !modifierKeys[event.key])
      return true;
    if (
      event.altKey &&
      !event.ctrlKey &&
      !event.metaKey &&
      codepoint(event.key) !== undefined &&
      /Mac|iPhone|iPad/.test(navigator.platform) &&
      !terminal.options.macOptionIsMeta
    )
      return true;
    if (!all && modifierKeys[event.key]) return true;
    if (release && (!reportEvents || !held)) return true;
    const key = held ?? identity(event, disambiguate);
    if (!key) return true;
    const modifiers =
      (event.shiftKey ? 1 : 0) |
      (event.altKey ? 2 : 0) |
      (event.ctrlKey ? 4 : 0) |
      (event.metaKey ? 8 : 0) |
      (event.getModifierState("Hyper") ? 16 : 0) |
      (event.getModifierState("CapsLock") ? 64 : 0) |
      (event.getModifierState("NumLock") ? 128 : 0);
    const shortcutModifiers = modifiers & 63;
    const recoveryKey =
      event.location !== 3 &&
      (event.key === "Enter" ||
        event.key === "Tab" ||
        event.key === "Backspace");
    // Preserve the protocol's recovery keys unless modified or report-all is set.
    if (!all && recoveryKey && !shortcutModifiers) return true;
    if (!release && !all && key.printable && !(shortcutModifiers & ~1))
      return true;
    if (!all && key.printable && !disambiguate) {
      if (!reportEvents) return true;
      if (!release && !event.repeat) {
        heldKeys.set(keyId, key);
        return true;
      }
    }
    if (!all && !disambiguate && !reportEvents) return true;
    if (release && !all && recoveryKey) return true;
    if (!release) heldKeys.set(keyId, key);
    let number = String(key.number);
    if (
      flags & 4 &&
      key.printable &&
      key.suffix === "u" &&
      key.number < 57344
    ) {
      const shifted = event.shiftKey ? codepoint(event.key) : undefined;
      const base = baseKey(event.code)?.codePointAt(0);
      if (shifted !== undefined && shifted !== key.number)
        number += `:${shifted}`;
      else if (base !== undefined && base !== key.number) number += ":";
      if (base !== undefined && base !== key.number) number += `:${base}`;
    }
    const eventType = release ? 3 : event.repeat ? 2 : 1;
    const eventField = reportEvents && eventType !== 1 ? `:${eventType}` : "";
    const text =
      all &&
      flags & 16 &&
      !release &&
      key.printable &&
      !(shortcutModifiers & ~1)
        ? textPoints(event.key)
        : "";
    const modifierField =
      modifiers || eventField || text ? `${modifiers + 1}${eventField}` : "";
    const parameters = modifierField ? `;${modifierField}` : "";
    if (key.number === 1 && key.suffix !== "u" && !parameters) number = "";
    event.preventDefault();
    event.stopPropagation();
    const encoded = `${csi}${number}${parameters}${text ? `;${text}` : ""}${key.suffix}`;
    if (compositionTimer !== undefined) afterComposition.push(encoded);
    else sendInput(encoded);
    return false;
  }

  function onCompositionStart() {
    composing = true;
    heldKeys.clear();
  }
  function onCompositionEnd() {
    composing = false;
    deadKey = false;
    // xterm commits composition in a timer. This bubbling listener runs after
    // xterm's textarea listener, so subsequent keys cannot overtake that text.
    clearTimeout(compositionTimer);
    compositionTimer = window.setTimeout(() => {
      compositionTimer = undefined;
      for (const data of afterComposition) sendInput(data);
      afterComposition.length = 0;
    }, 0);
  }
  function onBlur() {
    composing = false;
    deadKey = false;
    heldKeys.clear();
  }
  function onPaste() {
    pasting = true;
    queueMicrotask(() => {
      pasting = false;
    });
  }

  return {
    get flags() {
      return state().flags;
    },
    handleKeyEvent,
    encodeInput(data) {
      if (disposed || pasting || !(state().flags & 8)) return data;
      for (let index = 0; index < data.length; index += 1) {
        const unit = data.charCodeAt(index);
        if (unit < 32 || (unit >= 127 && unit <= 159)) return data;
      }
      const points = textPoints(data);
      return state().flags & 16 && points ? `${csi}0;;${points}u` : "";
    },
    attachInputListeners() {
      if (disposed || inputElement || !terminal.element) return;
      inputElement = terminal.element;
      inputElement.addEventListener(
        "compositionstart",
        onCompositionStart,
        true,
      );
      inputElement.addEventListener("compositionend", onCompositionEnd);
      inputElement.addEventListener("blur", onBlur, true);
      inputElement.addEventListener("paste", onPaste, true);
    },
    paste(text) {
      pasting = true;
      try {
        terminal.paste(text);
      } finally {
        pasting = false;
      }
    },
    reset,
    dispose() {
      if (disposed) return;
      disposed = true;
      for (const disposable of disposables) disposable.dispose();
      inputElement?.removeEventListener(
        "compositionstart",
        onCompositionStart,
        true,
      );
      inputElement?.removeEventListener("compositionend", onCompositionEnd);
      inputElement?.removeEventListener("blur", onBlur, true);
      inputElement?.removeEventListener("paste", onPaste, true);
      reset();
    },
  };
}
