import { describe, expect, test } from "vitest";
import { computeInverseEdits, type ModelChange } from "../lib/inverseEdits";

/** Apply Monaco-style changes (pre-edit offsets) to a string: descending
 *  offset order keeps earlier offsets valid. Mirrors what Monaco did to the
 *  model before our handler runs. */
function applyChanges(shadow: string, changes: ModelChange[]): string {
  const desc = [...changes].sort((a, b) => b.rangeOffset - a.rangeOffset);
  let s = shadow;
  for (const c of desc) {
    s =
      s.slice(0, c.rangeOffset) +
      c.text +
      s.slice(c.rangeOffset + c.rangeLength);
  }
  return s;
}

/** Apply inverse edits (post-edit offsets) to the post-edit string. */
function applyInverse(
  current: string,
  edits: ReturnType<typeof computeInverseEdits>,
): string {
  const desc = [...edits].sort((a, b) => b.start - a.start);
  let s = current;
  for (const e of desc) {
    s = s.slice(0, e.start) + e.text + s.slice(e.end);
  }
  return s;
}

function roundTrips(shadow: string, changes: ModelChange[]) {
  const after = applyChanges(shadow, changes);
  const reverted = applyInverse(after, computeInverseEdits(shadow, changes));
  expect(reverted).toBe(shadow);
}

describe("computeInverseEdits", () => {
  test("single character insert", () => {
    const shadow = "fn main() {}";
    const changes = [{ rangeOffset: 11, rangeLength: 0, text: "x" }];
    expect(computeInverseEdits(shadow, changes)).toEqual([
      { start: 11, end: 12, text: "" },
    ]);
    roundTrips(shadow, changes);
  });

  test("single character delete (backspace)", () => {
    const shadow = "hello";
    const changes = [{ rangeOffset: 4, rangeLength: 1, text: "" }];
    expect(computeInverseEdits(shadow, changes)).toEqual([
      { start: 4, end: 4, text: "o" },
    ]);
    roundTrips(shadow, changes);
  });

  test("selection replaced by typed character", () => {
    const shadow = "let value = 1;";
    const changes = [{ rangeOffset: 4, rangeLength: 5, text: "x" }];
    expect(computeInverseEdits(shadow, changes)).toEqual([
      { start: 4, end: 5, text: "value" },
    ]);
    roundTrips(shadow, changes);
  });

  test("newline insert", () => {
    roundTrips("ab", [{ rangeOffset: 1, rangeLength: 0, text: "\n" }]);
  });

  test("multi-cursor: two simultaneous inserts, unsorted input order", () => {
    const shadow = "aa bb cc";
    // Monaco may report changes in descending order; the function must not care.
    const changes = [
      { rangeOffset: 6, rangeLength: 0, text: "X" },
      { rangeOffset: 0, rangeLength: 0, text: "X" },
    ];
    expect(computeInverseEdits(shadow, changes)).toEqual([
      { start: 0, end: 1, text: "" },
      { start: 7, end: 8, text: "" },
    ]);
    roundTrips(shadow, changes);
  });

  test("multi-cursor: two replaces of different lengths", () => {
    const shadow = "one two three";
    const changes = [
      { rangeOffset: 0, rangeLength: 3, text: "11111" },
      { rangeOffset: 8, rangeLength: 5, text: "3" },
    ];
    roundTrips(shadow, changes);
  });

  test("paste over selection spanning newlines", () => {
    const shadow = "line1\nline2\nline3";
    const changes = [{ rangeOffset: 3, rangeLength: 9, text: "PASTED" }];
    roundTrips(shadow, changes);
  });

  test("empty change list yields no edits", () => {
    expect(computeInverseEdits("abc", [])).toEqual([]);
  });
});
