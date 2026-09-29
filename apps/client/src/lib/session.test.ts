import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import type { Answered, Current, ExerciseView } from "./api";
import { SessionController } from "./session.svelte";

const exercise: ExerciseView = {
  card: { itemId: "starter/what-is-this", skill: "listening" },
  kind: "sentence",
  image: false,
  prompt: { type: "listening", korean: "이거 뭐예요?", options: ["a", "b", "c", "d"] },
};
const progress = { done: 0, remaining: 1, correct: 0, streak: 0, bestStreak: 0 };

interface Backend {
  total?: number;
  queue?: (ExerciseView | null)[];
  fail?: string;
}

function backend({ total = 1, queue = [exercise, null], fail }: Backend = {}) {
  const calls: { cmd: string; args: unknown }[] = [];
  let latest = progress;
  mockIPC(async (cmd, args) => {
    calls.push({ cmd, args });
    if (fail === cmd) throw new Error("boom");
    switch (cmd) {
      case "session_start":
        return { total, due: 0, new: total };
      case "session_current":
        return { exercise: queue.shift() ?? null, progress: latest } satisfies Current;
      case "session_answer":
        latest = { ...progress, done: 1, remaining: 0, correct: 1, streak: 1, bestStreak: 1 };
        return {
          feedback: {
            correct: true,
            correctIndex: 2,
            rating: "good",
            korean: "이거 뭐예요?",
            english: "What is this?",
            note: null,
            streak: 1,
            retry: false,
          },
          progress: latest,
        } satisfies Answered;
    }
  });
  return calls;
}

function clock(start = 1_000) {
  const c = { t: start, now: () => c.t };
  return c;
}

afterEach(() => clearMocks());

describe("SessionController", () => {
  it("plays a card: exercise, answer with measured time, feedback, then done", async () => {
    const calls = backend();
    const time = clock();
    const s = new SessionController(time.now);
    await s.start();
    expect(s.phase).toBe("exercise");
    expect(s.exercise?.prompt.korean).toBe("이거 뭐예요?");

    time.t += 4_200;
    await s.answer({ type: "choice", index: 2 });
    expect(s.phase).toBe("feedback");
    expect(s.chosen).toBe(2);
    expect(s.feedback?.correctIndex).toBe(2);
    expect(calls.find((c) => c.cmd === "session_answer")?.args).toEqual({
      answer: { type: "choice", index: 2 },
      elapsedMs: 4_200,
    });

    await s.next();
    expect(s.phase).toBe("done");
    expect(s.progress.bestStreak).toBe(1);
  });

  it("does not count time spent hidden", async () => {
    const calls = backend();
    const time = clock();
    const s = new SessionController(time.now);
    await s.start();
    time.t += 60_000; // window hidden for a minute
    s.resumeTimer();
    time.t += 1_500;
    await s.answer({ type: "choice", index: 0 });
    const args = calls.find((c) => c.cmd === "session_answer")?.args as { elapsedMs: number };
    expect(args.elapsedMs).toBe(1_500);
  });

  it("drops a second answer while the first is in flight", async () => {
    const calls = backend();
    const s = new SessionController(clock().now);
    await s.start();
    await Promise.all([
      s.answer({ type: "choice", index: 0 }),
      s.answer({ type: "choice", index: 1 }),
    ]);
    expect(calls.filter((c) => c.cmd === "session_answer")).toHaveLength(1);
    expect(s.chosen).toBe(0);
  });

  it("ignores answers outside the exercise phase", async () => {
    const calls = backend();
    const s = new SessionController(clock().now);
    await s.start();
    await s.answer({ type: "choice", index: 0 });
    await s.answer({ type: "choice", index: 1 });
    expect(calls.filter((c) => c.cmd === "session_answer")).toHaveLength(1);
  });

  it("reports an empty session and backend errors", async () => {
    backend({ total: 0 });
    const empty = new SessionController(clock().now);
    await empty.start();
    expect(empty.phase).toBe("empty");

    backend({ fail: "session_start" });
    const broken = new SessionController(clock().now);
    await broken.start();
    expect(broken.phase).toBe("error");
    expect(broken.error).toContain("boom");
  });
});
