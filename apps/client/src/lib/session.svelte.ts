import {
  sessionAnswer,
  sessionCurrent,
  sessionStart,
  type Answer,
  type ExerciseView,
  type Feedback,
  type LevelUp,
  type Profile,
  type Progress,
} from "./api";

export type Phase = "loading" | "exercise" | "feedback" | "done" | "empty" | "error";

const EMPTY_PROGRESS: Progress = {
  done: 0,
  remaining: 0,
  correct: 0,
  streak: 0,
  bestStreak: 0,
  xp: 0,
};

/**
 * UI state of a review session. It only sequences calls: which card comes next and whether an
 * answer is right is decided in Rust.
 */
export class SessionController {
  phase = $state<Phase>("loading");
  exercise = $state<ExerciseView | null>(null);
  feedback = $state<Feedback | null>(null);
  chosen = $state<number | null>(null);
  progress = $state<Progress>(EMPTY_PROGRESS);
  error = $state<string | null>(null);
  /** Latest learner profile returned with an answer. */
  profile = $state<Profile | null>(null);
  /** Set when the last answer crossed a level; cleared on the next card. */
  levelUp = $state<LevelUp | null>(null);

  #shownAt = 0;
  #busy = false;

  constructor(private readonly now: () => number = () => performance.now()) {}

  async start(): Promise<void> {
    await this.#guard(async () => {
      this.phase = "loading";
      const started = await sessionStart();
      if (started.total === 0) {
        this.exercise = null;
        this.phase = "empty";
        return;
      }
      await this.#loadCurrent();
    });
  }

  /** The window was hidden for a while: do not count that time as thinking time. */
  resumeTimer(): void {
    if (this.phase === "exercise") this.#shownAt = this.now();
  }

  async answer(answer: Answer): Promise<void> {
    if (this.phase !== "exercise") return;
    await this.#guard(async () => {
      const elapsed = Math.max(0, Math.round(this.now() - this.#shownAt));
      if (answer.type === "choice") this.chosen = answer.index;
      const result = await sessionAnswer(answer, elapsed);
      this.feedback = result.feedback;
      this.progress = result.progress;
      this.profile = result.profile;
      this.levelUp = result.levelUp;
      this.phase = "feedback";
    });
  }

  async next(): Promise<void> {
    if (this.phase !== "feedback") return;
    await this.#guard(() => this.#loadCurrent());
  }

  async #loadCurrent(): Promise<void> {
    const current = await sessionCurrent();
    this.progress = current.progress;
    this.feedback = null;
    this.levelUp = null;
    this.chosen = null;
    this.exercise = current.exercise;
    this.phase = current.exercise ? "exercise" : "done";
    this.#shownAt = this.now();
  }

  /** Serializes actions: key repeats while a call is in flight are dropped. */
  async #guard(action: () => Promise<void>): Promise<void> {
    if (this.#busy) return;
    this.#busy = true;
    try {
      await action();
      this.error = null;
    } catch (err) {
      this.error = String(err);
      this.phase = "error";
    } finally {
      this.#busy = false;
    }
  }
}
