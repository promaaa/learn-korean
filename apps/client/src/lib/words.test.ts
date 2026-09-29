import { describe, expect, it } from "vitest";
import { nextWord, type Box } from "./words";

const word = (left: number, top: number, width = 40, height = 20): Box => ({ left, top, width, height });

// A big prompt line (2 words) above two option rows; the second option wraps onto two rows.
const card: Box[] = [
  word(100, 0, 120, 50),
  word(240, 0, 120, 50),
  word(20, 100),
  word(80, 100),
  word(20, 150),
  word(20, 175),
  word(200, 175),
];

describe("nextWord", () => {
  it("starts at the first word going forward and at the last going back", () => {
    expect(nextWord(card, null, "right")).toBe(0);
    expect(nextWord(card, null, "down")).toBe(0);
    expect(nextWord(card, null, "left")).toBe(6);
    expect(nextWord(card, null, "up")).toBe(6);
    expect(nextWord([], null, "right")).toBeNull();
  });

  it("steps through words in reading order across lines, wrapping", () => {
    expect(nextWord(card, 1, "right")).toBe(2);
    expect(nextWord(card, 2, "left")).toBe(1);
    expect(nextWord(card, 6, "right")).toBe(0);
    expect(nextWord(card, 0, "left")).toBe(6);
  });

  it("moves to the horizontally nearest word of the adjacent visual row", () => {
    // Under the prompt's second word (middle x 300), the nearest word of the first option is at 80.
    expect(nextWord(card, 1, "down")).toBe(3);
    expect(nextWord(card, 3, "down")).toBe(4);
    // The wrapped part of an option is its own row.
    expect(nextWord(card, 4, "down")).toBe(5);
    expect(nextWord(card, 6, "up")).toBe(4);
    expect(nextWord(card, 2, "up")).toBe(0);
  });

  it("wraps from the last row to the first and back", () => {
    expect(nextWord(card, 6, "down")).toBe(0);
    expect(nextWord(card, 0, "up")).toBe(6);
  });
});
