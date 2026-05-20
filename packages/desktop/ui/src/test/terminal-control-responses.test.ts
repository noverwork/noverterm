import { describe, expect, it } from "vitest";

import { stripTerminalControlResponses } from "$lib/terminal/terminal-control-responses.js";

describe("terminal control responses", () => {
  it("strips OSC color query responses", () => {
    const responses =
      "\x1b]4;1;rgb:aaaa/bbbb/cccc\x1b\\" +
      "\x1b]10;rgb:e5e5/e5e5/e5e5\x07" +
      "\x1b]11;rgb:0808/0c0c/1313\x07" +
      "\x1b]12;rgb:ffff/ffff/ffff\x1b\\";

    expect(stripTerminalControlResponses(responses)).toBe("");
  });

  it("strips terminal report responses that leak into the shell prompt", () => {
    const responses =
      "\x1b[?1;2R" +
      "\x1b[>0;276;0c" +
      "\x1b[=0;276;0c" +
      "\x1b[?2026;2$y" +
      "\x1b[?1;0;256S" +
      "\x1b[8;24;80t";

    expect(stripTerminalControlResponses(responses)).toBe("");
  });

  it("strips DCS query responses", () => {
    const responses =
      "\x1bP1$r0m\x1b\\" +
      "\x1bP1$r2 q\x1b\\" +
      "\x1bP0$r\x1b\\" +
      "\x1bP1+r636F3D3136\x1b\\";

    expect(stripTerminalControlResponses(responses)).toBe("");
  });

  it("strips the combined response shape seen in polluted prompts", () => {
    const responses =
      "\x1b[?1;2R" +
      "\x1b[?1;2R" +
      "\x1b[>0;276;0c" +
      "\x1b]10;rgb:e5e5/e5e5/e5e5\x07" +
      "\x1b]11;rgb:0808/0c0c/1313\x07" +
      "\x1b[?2026;2$y";

    expect(stripTerminalControlResponses(responses)).toBe("");
  });

  it("preserves normal keyboard input and paste text", () => {
    const input = "ls -la\n\x1b[A\u0003pasted text";

    expect(stripTerminalControlResponses(input)).toBe(input);
  });

  it("keeps user input around stripped responses", () => {
    const input = "a\x1b]10;rgb:e5e5/e5e5/e5e5\x07b";

    expect(stripTerminalControlResponses(input)).toBe("ab");
  });
});
