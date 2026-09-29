import { describe, expect, it } from "vitest";
import { actionFor } from "./keys";

describe("actionFor", () => {
  it("maps digits to zero-based choices", () => {
    expect(actionFor({ key: "1", code: "Digit1" })).toEqual({ type: "choose", index: 0 });
    expect(actionFor({ key: "4", code: "Numpad4" })).toEqual({ type: "choose", index: 3 });
  });

  it("uses physical keys, so AZERTY's unshifted digit row still answers", () => {
    // French layout: the key left of "é" types "&" without Shift.
    expect(actionFor({ key: "&", code: "Digit1" })).toEqual({ type: "choose", index: 0 });
    expect(actionFor({ key: "'", code: "Digit4" })).toEqual({ type: "choose", index: 3 });
    // The key labelled R on every layout replays.
    expect(actionFor({ key: "r", code: "KeyR" })).toEqual({ type: "replay" });
    // Same for F (focus), which types "f" on AZERTY too but "u" on Dvorak.
    expect(actionFor({ key: "u", code: "KeyF" })).toEqual({ type: "focus" });
  });

  it("maps vim keys and arrows to the same moves", () => {
    expect(actionFor({ key: "h", code: "KeyH" })).toEqual({ type: "move", direction: "left" });
    expect(actionFor({ key: "L", code: "KeyL" })).toEqual({ type: "move", direction: "right" });
    expect(actionFor({ key: "ArrowDown", code: "ArrowDown" })).toEqual({
      type: "move",
      direction: "down",
    });
  });

  it("falls back to the key when no code is given", () => {
    expect(actionFor({ key: "j" })).toEqual({ type: "move", direction: "down" });
    expect(actionFor({ key: "R" })).toEqual({ type: "replay" });
    expect(actionFor({ key: "2" })).toEqual({ type: "choose", index: 1 });
  });

  it("maps session keys", () => {
    expect(actionFor({ key: "Enter", code: "Enter" })).toEqual({ type: "confirm" });
    expect(actionFor({ key: " ", code: "Space" })).toEqual({ type: "continue" });
    expect(actionFor({ key: "Backspace", code: "Backspace" })).toEqual({ type: "erase" });
    expect(actionFor({ key: "Escape", code: "Escape" })).toEqual({ type: "hide" });
    expect(actionFor({ key: "Tab", code: "Tab" })).toEqual({ type: "mode" });
  });

  it("ignores modified keys so system shortcuts keep working", () => {
    expect(actionFor({ key: "r", code: "KeyR", ctrlKey: true })).toBeNull();
    expect(actionFor({ key: "1", code: "Digit1", metaKey: true })).toBeNull();
  });

  it("ignores 0 and unrelated keys", () => {
    expect(actionFor({ key: "0", code: "Digit0" })).toBeNull();
    expect(actionFor({ key: "x", code: "KeyX" })).toBeNull();
  });
});
