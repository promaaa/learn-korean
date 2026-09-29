/** Keyboard vocabulary shared by every screen. Screens receive actions, never raw key events. */
export type Direction = "left" | "down" | "up" | "right";

export type Action =
  | { type: "choose"; index: number }
  | { type: "confirm" }
  | { type: "continue" }
  | { type: "move"; direction: Direction }
  | { type: "replay" }
  | { type: "erase" }
  | { type: "focus" }
  | { type: "mode" }
  | { type: "hide" };

export interface KeyLike {
  key: string;
  /** Physical key (`KeyboardEvent.code`); keeps shortcuts on the same keys on AZERTY, QWERTZ… */
  code?: string;
  ctrlKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
}

const MOVES: Record<string, Direction> = {
  KeyH: "left",
  KeyJ: "down",
  KeyK: "up",
  KeyL: "right",
  ArrowLeft: "left",
  ArrowDown: "down",
  ArrowUp: "up",
  ArrowRight: "right",
};

/** Physical key of a Latin letter, for events without a `code`. */
function letterCode(key: string): string | undefined {
  return /^[a-z]$/i.test(key) ? `Key${key.toUpperCase()}` : undefined;
}

export function actionFor(event: KeyLike): Action | null {
  if (event.ctrlKey || event.altKey || event.metaKey) return null;
  const code = event.code || letterCode(event.key) || event.key;

  // Digit row by position: on AZERTY the unshifted digit row types & é " ' … not 1 2 3 4.
  const digit = /^(?:Digit|Numpad)([1-9])$/.exec(code)?.[1] ?? (/^[1-9]$/.test(event.key) ? event.key : undefined);
  if (digit) return { type: "choose", index: Number(digit) - 1 };

  const direction = MOVES[code] ?? MOVES[event.key];
  if (direction) return { type: "move", direction };

  if (code === "KeyR") return { type: "replay" };
  if (code === "KeyF") return { type: "focus" };
  switch (event.key) {
    case "Enter":
      return { type: "confirm" };
    case " ":
      return { type: "continue" };
    case "Backspace":
      return { type: "erase" };
    case "Escape":
      return { type: "hide" };
    case "Tab":
      return { type: "mode" };
    default:
      return null;
  }
}
