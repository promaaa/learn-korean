import { describe, expect, it } from "vitest";
import { formatInterval } from "./format";

const MIN = 60_000;
const DAY = 24 * 60 * MIN;

describe("formatInterval", () => {
  it("uses the largest unit that reads naturally", () => {
    expect(formatInterval(20_000)).toBe("in a moment");
    expect(formatInterval(10 * MIN)).toBe("in 10 min");
    expect(formatInterval(90 * MIN)).toBe("in 2 h");
    expect(formatInterval(DAY)).toBe("tomorrow");
    expect(formatInterval(3 * DAY)).toBe("in 3 days");
    expect(formatInterval(80 * DAY)).toBe("in 3 mo");
    expect(formatInterval(800 * DAY)).toBe("in 2 years");
  });

  it("rounds at unit boundaries instead of showing 60 min or 24 h", () => {
    expect(formatInterval(59.6 * MIN)).toBe("in 1 h");
    expect(formatInterval(23.6 * 60 * MIN)).toBe("tomorrow");
  });
});
