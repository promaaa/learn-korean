import { describe, expect, it } from "vitest";
import { actionFor } from "./keys";

describe("actionFor", () => {
  it("maps digits to zero-based choices", () => {
    expect(actionFor({ key: "1" })).toEqual({ type: "choose", index: 0 });
    expect(actionFor({ key: "4" })).toEqual({ type: "choose", index: 3 });
  });

  it("maps vim keys and arrows to the same moves, case-insensitively", () => {
    expect(actionFor({ key: "h" })).toEqual({ type: "move", direction: "left" });
    expect(actionFor({ key: "L" })).toEqual({ type: "move", direction: "right" });
    expect(actionFor({ key: "ArrowDown" })).toEqual({ type: "move", direction: "down" });
  });

  it("maps session keys", () => {
    expect(actionFor({ key: "Enter" })).toEqual({ type: "confirm" });
    expect(actionFor({ key: " " })).toEqual({ type: "continue" });
    expect(actionFor({ key: "R" })).toEqual({ type: "replay" });
    expect(actionFor({ key: "Escape" })).toEqual({ type: "hide" });
  });

  it("ignores modified keys so system shortcuts keep working", () => {
    expect(actionFor({ key: "r", ctrlKey: true })).toBeNull();
    expect(actionFor({ key: "1", metaKey: true })).toBeNull();
  });

  it("ignores 0 and unrelated keys", () => {
    expect(actionFor({ key: "0" })).toBeNull();
    expect(actionFor({ key: "x" })).toBeNull();
  });
});
