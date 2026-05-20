import type { IDisposable, Terminal } from "@xterm/xterm";

const ESCAPE_SEQUENCE = "\\x1b";
const BELL = "\\x07";

const terminalControlResponsePatterns: readonly RegExp[] = [
  new RegExp(
    `${ESCAPE_SEQUENCE}\\]4;\\d+;rgb:[0-9a-fA-F]{1,4}/[0-9a-fA-F]{1,4}/[0-9a-fA-F]{1,4}(?:${BELL}|${ESCAPE_SEQUENCE}\\\\)`,
    "g",
  ),
  new RegExp(
    `${ESCAPE_SEQUENCE}\\](?:10|11|12);rgb:[0-9a-fA-F]{1,4}/[0-9a-fA-F]{1,4}/[0-9a-fA-F]{1,4}(?:${BELL}|${ESCAPE_SEQUENCE}\\\\)`,
    "g",
  ),
  new RegExp(
    `${ESCAPE_SEQUENCE}\\[(?:\\?[\\d;]+|>[\\d;]+|=[\\d;]+|\\d+(?:;\\d+)*)c`,
    "g",
  ),
  new RegExp(`${ESCAPE_SEQUENCE}\\[\\??\\d+(?:;\\d+)*\\$y`, "g"),
  new RegExp(`${ESCAPE_SEQUENCE}\\[\\??\\d+;\\d+R`, "g"),
  new RegExp(`${ESCAPE_SEQUENCE}\\[\\?\\d+(?:;\\d+)*S`, "g"),
  new RegExp(`${ESCAPE_SEQUENCE}\\[(?:4|6|8);\\d+;\\d+t`, "g"),
  new RegExp(
    `${ESCAPE_SEQUENCE}P[01]\\$r[^${ESCAPE_SEQUENCE}${BELL}]*(?:${BELL}|${ESCAPE_SEQUENCE}\\\\)`,
    "g",
  ),
  new RegExp(
    `${ESCAPE_SEQUENCE}P[01]\\+r[0-9a-fA-F;=]*(?:${BELL}|${ESCAPE_SEQUENCE}\\\\)`,
    "g",
  ),
];

function getFirstNumericParam(params: (number | number[])[]) {
  const [first] = params;
  return typeof first === "number" ? first : null;
}

function isWindowReportQuery(params: (number | number[])[]) {
  const first = getFirstNumericParam(params);
  return first === 14 || first === 16 || first === 18;
}

function isIndexedColorQuery(data: string) {
  const slots = data.split(";");

  for (let index = 1; index < slots.length; index += 2) {
    if (slots[index].trim() === "?") {
      return true;
    }
  }

  return false;
}

export function stripTerminalControlResponses(data: string) {
  let stripped = data;

  for (const pattern of terminalControlResponsePatterns) {
    stripped = stripped.replace(pattern, "");
  }

  return stripped;
}

export function registerTerminalControlResponseSuppression(
  terminal: Terminal,
) {
  const disposables: IDisposable[] = [
    terminal.parser.registerCsiHandler({ final: "c" }, () => true),
    terminal.parser.registerCsiHandler({ prefix: ">", final: "c" }, () => true),
    terminal.parser.registerCsiHandler({ prefix: "=", final: "c" }, () => true),
    terminal.parser.registerCsiHandler({ final: "n" }, () => true),
    terminal.parser.registerCsiHandler({ prefix: "?", final: "n" }, () => true),
    terminal.parser.registerCsiHandler({ final: "R" }, () => true),
    terminal.parser.registerCsiHandler({ prefix: "?", final: "S" }, () => true),
    terminal.parser.registerCsiHandler({ final: "t" }, isWindowReportQuery),
    terminal.parser.registerCsiHandler(
      { intermediates: "$", final: "p" },
      () => true,
    ),
    terminal.parser.registerCsiHandler(
      { prefix: "?", intermediates: "$", final: "p" },
      () => true,
    ),
    terminal.parser.registerDcsHandler(
      { intermediates: "$", final: "q" },
      () => true,
    ),
    terminal.parser.registerOscHandler(4, isIndexedColorQuery),
    terminal.parser.registerOscHandler(10, (data) => data.trim() === "?"),
    terminal.parser.registerOscHandler(11, (data) => data.trim() === "?"),
    terminal.parser.registerOscHandler(12, (data) => data.trim() === "?"),
  ];

  return () => {
    for (const disposable of disposables) {
      disposable.dispose();
    }
  };
}
