import type { DrillView, Pressed } from "./api";

const MODIFIERS: Record<string, true> = {
  ShiftLeft: true,
  ShiftRight: true,
  ControlLeft: true,
  ControlRight: true,
  AltLeft: true,
  AltRight: true,
  MetaLeft: true,
  MetaRight: true,
  CapsLock: true,
};

/**
 * Claims a raw keydown for a typing screen. Shortcuts (Ctrl/Alt/Meta) are left to the system;
 * every other key is kept from the WebView, and only non-repeated, non-modifier keys count.
 */
export function claimKey(event: KeyboardEvent): boolean {
  if (event.ctrlKey || event.altKey || event.metaKey) return false;
  event.preventDefault();
  return !event.repeat && !MODIFIERS[event.code];
}

/** The line being typed and the wrong-key feedback, for screens rendering `TypingDrill`. */
export class LiveDrill {
  view = $state<DrillView | null>(null);
  /** Code of the last wrong key, flashed on the keyboard. */
  wrong = $state<string | null>(null);
  /** Bumped on every wrong key to replay the shake animation. */
  shake = $state(0);
  #press: (code: string, shift: boolean, atMs: number) => Promise<Pressed>;
  #timer: number | undefined;

  constructor(press: (code: string, shift: boolean, atMs: number) => Promise<Pressed>) {
    this.#press = press;
  }

  /** Sends one key press to Rust and shows the answer. */
  async type(event: KeyboardEvent): Promise<Pressed> {
    const result = await this.#press(event.code, event.shiftKey, Math.round(performance.now()));
    this.view = result.view;
    if (!result.outcome.ignored && !result.outcome.correct) {
      this.wrong = event.code;
      this.shake += 1;
      window.clearTimeout(this.#timer);
      this.#timer = window.setTimeout(() => (this.wrong = null), 280);
    }
    return result;
  }
}
