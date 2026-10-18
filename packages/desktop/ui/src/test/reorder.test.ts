import { describe, expect, it } from "vitest";

import { moveToInsertionIndex } from "$lib/reorder.js";

describe("moveToInsertionIndex", () => {
  const ids = ["a", "b", "c"];

  it("moves down past the next item", () => {
    expect(moveToInsertionIndex(ids, 0, 2)).toEqual(["b", "a", "c"]);
  });

  it("moves to the end", () => {
    expect(moveToInsertionIndex(ids, 0, 3)).toEqual(["b", "c", "a"]);
  });

  it("moves up to the start", () => {
    expect(moveToInsertionIndex(ids, 2, 0)).toEqual(["c", "a", "b"]);
  });

  it("ignores drops on either side of the dragged item", () => {
    expect(moveToInsertionIndex(ids, 1, 1)).toBeNull();
    expect(moveToInsertionIndex(ids, 1, 2)).toBeNull();
  });
});
