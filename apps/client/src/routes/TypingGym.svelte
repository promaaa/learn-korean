<script lang="ts">
  import { onMount } from "svelte";
  import LessonList, { type ListedLesson } from "../components/LessonList.svelte";
  import RoundSummary from "../components/RoundSummary.svelte";
  import ScreenTitle from "../components/ScreenTitle.svelte";
  import TypingDrill from "../components/TypingDrill.svelte";
  import {
    typingCurrent,
    typingLayout,
    typingLessons,
    typingNext,
    typingPress,
    typingStart,
    type KeyCap,
    type KeyLesson,
    type LessonResult,
  } from "../lib/api";
  import { LiveDrill, claimKey } from "../lib/drill.svelte";
  import { actionFor } from "../lib/keys";

  type Hint = { keys: string; label: string };
  /** Lesson list → typing drill → summary. */
  type Stage = "list" | "drill" | "summary";

  const drill = new LiveDrill(typingPress);
  let stage = $state<Stage>("list");
  let lessons = $state<KeyLesson[]>([]);
  /** The highlighted row: a course lesson, or the words round after the last one. */
  let selected = $state(0);
  /** The lesson of the round on screen; `null` for a round of words. */
  let playing = $state<number | null>(null);
  let result = $state<LessonResult | null>(null);
  let layout = $state<KeyCap[]>([]);
  let error = $state<string | null>(null);
  /** Stats of finished lines in this round, for the summary. */
  let finished = $state<{ wpm: number; accuracy: number }[]>([]);
  /** A command changing the stage is in flight; keys meanwhile are ignored. */
  let busy = false;

  const WORDS: ListedLesson = {
    id: "words",
    title: "Free practice · words from your packs",
    letters: "",
    unlocked: true,
    passed: false,
  };
  const rows = $derived<ListedLesson[]>([
    ...lessons.map((lesson) => ({ ...lesson, letters: lesson.keys.join(" ") })),
    WORDS,
  ]);
  const lesson = $derived(playing === null ? undefined : lessons[playing]);
  const upcoming = $derived(result?.passed && playing !== null ? lessons[playing + 1] : undefined);

  async function guarded(action: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await action();
      error = null;
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  /** Starts course lesson `index`, or a round of words for `null`. */
  const start = (index: number | null) =>
    guarded(async () => {
      if (layout.length === 0) layout = await typingLayout();
      drill.view = await typingStart(index);
      playing = index;
      result = null;
      finished = [];
      stage = "drill";
    });

  const nextLine = () =>
    guarded(async () => {
      const next = await typingNext();
      if (next.type === "line") {
        drill.view = next.view;
        return;
      }
      result = next.result;
      if (result) lessons = await typingLessons();
      stage = "summary";
    });

  const backToList = () =>
    guarded(async () => {
      lessons = await typingLessons();
      if (playing !== null) selected = playing;
      result = null;
      stage = "list";
    });

  function firstOpenLesson(list: KeyLesson[]): number {
    const open = list.findIndex((item) => item.unlocked && !item.passed);
    return open === -1 ? list.length : open;
  }

  // Mounted only while on screen; a round left mid-way (Tab) resumes where it was.
  onMount(
    () =>
      void guarded(async () => {
        const [list, current] = await Promise.all([typingLessons(), typingCurrent()]);
        lessons = list;
        selected = firstOpenLesson(list);
        if (current) {
          layout = await typingLayout();
          playing = current.lesson;
          drill.view = current.view;
          stage = "drill";
        }
      }),
  );

  export function hints(): Hint[] {
    const common = [
      { keys: "Tab", label: "hangul" },
      { keys: "Esc", label: "hide" },
    ];
    switch (stage) {
      case "list":
        return [{ keys: "↑ ↓", label: "choose" }, { keys: "Enter", label: "start" }, ...common];
      case "drill":
        return drill.view?.snapshot.finished
          ? [{ keys: "Enter", label: "next line" }, ...common]
          : [{ keys: "⌨", label: "type the line" }, ...common];
      case "summary":
        return [
          { keys: "Enter", label: upcoming ? "next lesson" : playing === null ? "new round" : "retry" },
          { keys: "⌫", label: "lessons" },
          ...common,
        ];
    }
  }

  /** Raw key events while the gym is on screen (Tab and Esc are handled by the app). */
  export function key(event: KeyboardEvent): void {
    if (stage === "drill") {
      // Every letter is a typing key here: no shortcuts until the round is over.
      if (!claimKey(event) || !drill.view) return;
      if (drill.view.snapshot.finished) {
        if (event.key === "Enter") void nextLine();
        return;
      }
      drill
        .type(event)
        .then(({ outcome, view }) => {
          if (!outcome.ignored && outcome.finished) {
            finished = [...finished, { wpm: view.snapshot.wpm, accuracy: view.snapshot.accuracy }];
          }
        })
        .catch((err: unknown) => (error = String(err)));
      return;
    }
    const action = actionFor(event);
    if (!action) return;
    event.preventDefault();
    if (busy || (event.repeat && action.type !== "move")) return;
    if (stage === "list") {
      if (action.type === "move" && (action.direction === "up" || action.direction === "down")) {
        const step = action.direction === "up" ? -1 : 1;
        selected = Math.min(Math.max(selected + step, 0), rows.length - 1);
      } else if (action.type === "confirm" && rows[selected]?.unlocked) {
        void start(selected < lessons.length ? selected : null);
      }
    } else if (action.type === "confirm") {
      void start(upcoming && playing !== null ? playing + 1 : playing);
    } else if (action.type === "erase") {
      void backToList();
    }
  }

  const average = $derived.by(() => {
    if (finished.length === 0) return { wpm: 0, accuracy: 0 };
    const sum = finished.reduce((a, f) => ({ wpm: a.wpm + f.wpm, accuracy: a.accuracy + f.accuracy }), {
      wpm: 0,
      accuracy: 0,
    });
    return { wpm: sum.wpm / finished.length, accuracy: sum.accuracy / finished.length };
  });

  const passedCount = $derived(lessons.filter((item) => item.passed).length);
  const count = $derived.by(() => {
    if (stage === "list") return lessons.length ? `${passedCount} / ${lessons.length} passed` : undefined;
    if (stage === "drill" && drill.view) return `${drill.view.position} / ${drill.view.total}`;
    return undefined;
  });
</script>

<div class="gym">
  <ScreenTitle label={stage === "list" || !lesson ? "Typing Gym" : `Typing Gym · ${lesson.title}`} {count} />

  {#if stage === "list"}
    <LessonList lessons={rows} {selected} />
  {:else if stage === "drill" && drill.view}
    <TypingDrill view={drill.view} {layout} wrong={drill.wrong} shake={drill.shake} />
  {:else if stage === "summary" && result}
    <RoundSummary
      title={result.passed ? "Lesson passed" : "Not passed yet"}
      stats={[
        { label: "speed", value: Math.round(average.wpm), unit: "wpm" },
        { label: "accuracy", value: `${Math.round(result.accuracy * 100)}%` },
        { label: "mistakes", value: result.errors },
      ]}
    >
      <p class="verdict" class:good={result.passed}>
        {#if !result.passed}
          {result.passPercent}% accuracy passes the lesson.
        {:else if upcoming}
          Next: {upcoming.title}
        {:else}
          Every lesson is passed.
        {/if}
      </p>
    </RoundSummary>
  {:else if stage === "summary"}
    <RoundSummary
      title="Round complete"
      stats={[
        { label: "lines", value: finished.length },
        { label: "speed", value: Math.round(average.wpm), unit: "wpm" },
        { label: "accuracy", value: `${Math.round(average.accuracy * 100)}%` },
      ]}
    />
  {/if}

  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .gym {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .verdict {
    margin: 20px 0 0;
    color: var(--fg-dim);
  }

  .verdict.good {
    color: var(--good);
  }

  .error {
    color: var(--bad);
    font-family: var(--font-mono);
    font-size: 13px;
  }
</style>
