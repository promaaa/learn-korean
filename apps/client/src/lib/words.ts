import type { Direction } from "./keys";

/** Glossed words on screen (rendered by `Glossed.svelte`). */
export const WORD_SELECTOR = "[data-word]";
/** Set on the word whose gloss the keyboard shows, as if it were hovered. */
export const ACTIVE_ATTRIBUTE = "data-active";

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Visual row of each box; `boxes` are in reading order (top to bottom, left to right). */
function rows(boxes: Box[]): number[] {
  let row = -1;
  let bottom = -Infinity;
  return boxes.map((box) => {
    // A word whose middle is below the current row starts the next one.
    if (box.top + box.height / 2 > bottom) {
      row += 1;
      bottom = box.top + box.height;
    }
    return row;
  });
}

const middleX = (box: Box) => box.left + box.width / 2;

/**
 * Word to show after moving from `current` (`null` when none is shown yet): left/right step
 * through the words in reading order, up/down go to the nearest word on the row above/below.
 * Both wrap around, like the option cursor.
 */
export function nextWord(boxes: Box[], current: number | null, direction: Direction): number | null {
  const count = boxes.length;
  if (count === 0) return null;
  const backwards = direction === "left" || direction === "up";
  const from = current === null ? undefined : boxes[current];
  if (current === null || !from) return backwards ? count - 1 : 0;
  if (direction === "left" || direction === "right") {
    return (current + (backwards ? -1 : 1) + count) % count;
  }
  const row = rows(boxes);
  const last = row[count - 1] ?? 0;
  const target = ((row[current] ?? 0) + (backwards ? -1 : 1) + last + 1) % (last + 1);
  let best = current;
  let distance = Infinity;
  boxes.forEach((box, index) => {
    const d = Math.abs(middleX(box) - middleX(from));
    if (row[index] === target && d < distance) {
      best = index;
      distance = d;
    }
  });
  return best;
}
