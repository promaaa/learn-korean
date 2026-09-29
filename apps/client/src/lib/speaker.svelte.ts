import { speak } from "./api";

export type SpeakerStatus = "idle" | "loading" | "error";

/** Audio status for the UI. Synthesis and playback happen in Rust. */
export class Speaker {
  status = $state<SpeakerStatus>("idle");
  #request = 0;

  async say(text: string): Promise<void> {
    const request = ++this.#request;
    this.status = "loading";
    try {
      await speak(text);
      if (request === this.#request) this.status = "idle";
    } catch (err) {
      if (request === this.#request) this.status = "error";
      console.warn(`speech unavailable: ${String(err)}`);
    }
  }
}
