/** Keyboard vocabulary shared by every screen. Screens receive actions, never raw key events. */
export type Direction = "left" | "down" | "up" | "right";

export type Action =
  | { type: "choose"; index: number }
  | { type: "confirm" }
  | { type: "continue" }
  | { type: "move"; direction: Direction }
  | { type: "replay" }
  | { type: "erase" }
  | { type: "hide" };

export interface KeyLike {
  key: string;
  ctrlKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
}

const MOVES: Record<string, Direction> = {
  h: "left",
  j: "down",
  k: "up",
  l: "right",
  ArrowLeft: "left",
  ArrowDown: "down",
  ArrowUp: "up",
  ArrowRight: "right",
};

export function actionFor(event: KeyLike): Action | null {
  if (event.ctrlKey || event.altKey || event.metaKey) return null;
  const { key } = event;
  if (key >= "1" && key <= "9" && key.length === 1) {
    return { type: "choose", index: Number(key) - 1 };
  }
  const direction = MOVES[key] ?? MOVES[key.toLowerCase()];
  if (direction) return { type: "move", direction };
  switch (key) {
    case "Enter":
      return { type: "confirm" };
    case " ":
      return { type: "continue" };
    case "r":
    case "R":
      return { type: "replay" };
    case "Backspace":
      return { type: "erase" };
    case "Escape":
      return { type: "hide" };
    default:
      return null;
  }
}
