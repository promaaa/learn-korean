<script lang="ts">
  import { onMount } from "svelte";
  import LessonList from "../components/LessonList.svelte";
  import RoundSummary from "../components/RoundSummary.svelte";
  import ScreenTitle from "../components/ScreenTitle.svelte";
  import TypingDrill from "../components/TypingDrill.svelte";
  import {
    hangulCurrent,
    hangulLessons,
    hangulNext,
    hangulPress,
    hangulStart,
    typingLayout,
    type HangulLesson,
    type KeyCap,
    type LessonResult,
    type NewJamo,
  } from "../lib/api";
  import { LiveDrill, claimKey } from "../lib/drill.svelte";
  import { actionFor } from "../lib/keys";
  import { Speaker } from "../lib/speaker.svelte";

  type Hint = { keys: string; label: string };
  /** Lesson list → intro card of the new jamo → typing drill → summary. */
  type Stage = "list" | "intro" | "drill" | "summary";

  const drill = new LiveDrill(hangulPress);
  const speaker = new Speaker();
  let stage = $state<Stage>("list");
  let lessons = $state<HangulLesson[]>([]);
  /** The highlighted lesson in the list, then the lesson being taught. */
  let selected = $state(0);
  /** Index of the jamo on the intro card. */
  let shown = $state(0);
  let result = $state<LessonResult | null>(null);
  let layout = $state<KeyCap[]>([]);
  let error = $state<string | null>(null);
  /** A command changing the stage is in flight; keys meanwhile are ignored. */
  let busy = false;

  const lesson = $derived(lessons[selected]);
  const upcoming = $derived(result?.passed ? lessons[selected + 1] : undefined);

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

  function sayJamo(jamo: NewJamo | undefined): void {
    if (jamo) void speaker.say(`${jamo.name}. ${jamo.example.korean}`);
  }

  function showLine(): void {
    if (drill.view) void speaker.say(drill.view.target.korean);
  }

  function openIntro(index: number): void {
    const opened = lessons[index];
    if (!opened?.unlocked) return;
    selected = index;
    shown = 0;
    result = null;
    stage = "intro";
    sayJamo(opened.jamo[0]);
  }

  function showJamo(index: number): void {
    const jamo = lesson?.jamo[index];
    if (!jamo || index === shown) return;
    shown = index;
    sayJamo(jamo);
  }

  const startDrill = () =>
    guarded(async () => {
      if (layout.length === 0) layout = await typingLayout();
      drill.view = await hangulStart(selected);
      stage = "drill";
      showLine();
    });

  const nextLine = () =>
    guarded(async () => {
      const next = await hangulNext();
      if (next.type === "line") {
        drill.view = next.view;
        showLine();
        return;
      }
      result = next.result;
      lessons = await hangulLessons();
      stage = "summary";
    });

  const backToList = () =>
    guarded(async () => {
      lessons = await hangulLessons();
      result = null;
      stage = "list";
    });

  // Mounted only while on screen; a round left mid-way (Tab) resumes where it was.
  onMount(
    () =>
      void guarded(async () => {
        const [list, current] = await Promise.all([hangulLessons(), hangulCurrent()]);
        lessons = list;
        if (current) {
          layout = await typingLayout();
          selected = current.lesson;
          drill.view = current.view;
          stage = "drill";
          showLine();
          return;
        }
        selected = Math.max(
          0,
          list.findIndex((item) => item.unlocked && !item.passed),
        );
      }),
  );

  export function hints(): Hint[] {
    const common = [
      { keys: "Tab", label: "stats" },
      { keys: "Esc", label: "hide" },
    ];
    switch (stage) {
      case "list":
        return [{ keys: "↑ ↓", label: "choose" }, { keys: "Enter", label: "start" }, ...common];
      case "intro":
        return [
          { keys: "← →", label: "letters" },
          { keys: "R", label: "replay" },
          { keys: "Enter", label: "type" },
          { keys: "⌫", label: "lessons" },
          ...common,
        ];
      case "drill":
        return drill.view?.snapshot.finished
          ? [{ keys: "Enter", label: "next line" }, ...common]
          : [{ keys: "⌨", label: "type the line" }, ...common];
      case "summary":
        return [
          { keys: "Enter", label: upcoming ? "next lesson" : "retry" },
          { keys: "⌫", label: "lessons" },
          ...common,
        ];
    }
  }

  /** Raw key events while the primer is on screen (Tab and Esc are handled by the app). */
  export function key(event: KeyboardEvent): void {
    if (stage === "drill") {
      // Every letter is a typing key here: no shortcuts until the round is over.
      if (!claimKey(event) || !drill.view) return;
      if (drill.view.snapshot.finished) {
        if (event.key === "Enter") void nextLine();
        return;
      }
      drill.type(event).catch((err: unknown) => (error = String(err)));
      return;
    }
    const action = actionFor(event);
    if (!action) return;
    event.preventDefault();
    if (busy || (event.repeat && action.type !== "move")) return;
    if (stage === "list") {
      if (action.type === "move" && (action.direction === "up" || action.direction === "down")) {
        const step = action.direction === "up" ? -1 : 1;
        selected = Math.min(Math.max(selected + step, 0), lessons.length - 1);
      } else if (action.type === "confirm") {
        openIntro(selected);
      }
    } else if (stage === "intro") {
      if (action.type === "move" && (action.direction === "left" || action.direction === "right")) {
        showJamo(shown + (action.direction === "left" ? -1 : 1));
      } else if (action.type === "replay") {
        sayJamo(lesson?.jamo[shown]);
      } else if (action.type === "confirm") {
        void startDrill();
      } else if (action.type === "erase") {
        void backToList();
      }
    } else if (action.type === "confirm") {
      if (upcoming) openIntro(selected + 1);
      else void startDrill();
    } else if (action.type === "erase") {
      void backToList();
    }
  }

  const passedCount = $derived(lessons.filter((item) => item.passed).length);
  const count = $derived.by(() => {
    if (stage === "list") return lessons.length ? `${passedCount} / ${lessons.length} passed` : undefined;
    if (stage === "intro") return lesson ? `${shown + 1} / ${lesson.jamo.length}` : undefined;
    if (stage === "drill" && drill.view) return `${drill.view.position} / ${drill.view.total}`;
    return undefined;
  });
</script>

<div class="primer">
  <ScreenTitle label={stage === "list" || !lesson ? "Hangul" : `Hangul · ${lesson.title}`} {count} />

  {#if stage === "list"}
    <LessonList
      lessons={lessons.map((item) => ({ ...item, letters: item.jamo.map((j) => j.jamo).join(" ") }))}
      {selected}
    />
  {:else if stage === "intro" && lesson}
    {@const current = lesson.jamo[shown]}
    <div class="strip" lang="ko">
      {#each lesson.jamo as jamo, index (jamo.jamo)}
        <span class:current={index === shown}>{jamo.jamo}</span>
      {/each}
    </div>
    {#if current}
      <div class="card">
        <p class="big" lang="ko">{current.jamo}</p>
        <p class="jamo-name" lang="ko">{current.name}</p>
        <p class="example">
          <span class="example-korean" lang="ko">{current.example.korean}</span>
          <span>{current.example.english}</span>
        </p>
      </div>
    {/if}
  {:else if stage === "drill" && drill.view}
    <TypingDrill view={drill.view} {layout} wrong={drill.wrong} shake={drill.shake} />
  {:else if stage === "summary" && result}
    <RoundSummary
      title={result.passed ? "Lesson passed" : "Not passed yet"}
      stats={[
        { label: "accuracy", value: `${Math.round(result.accuracy * 100)}%` },
        { label: "keys", value: result.keystrokes },
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
  {/if}

  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .primer {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .strip {
    display: flex;
    gap: 10px;
    font-family: var(--font-ko);
    font-size: 20px;
    color: var(--muted);
  }

  .strip .current {
    color: var(--accent);
    box-shadow: 0 2px 0 var(--accent);
  }

  .card {
    min-width: 320px;
    margin-top: 18px;
    padding: 18px 36px 22px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--panel);
    text-align: center;
  }

  .card p {
    margin: 0;
  }

  .big {
    font-family: var(--font-ko);
    font-size: 120px;
    line-height: 1.1;
    color: var(--fg);
  }

  .jamo-name {
    font-family: var(--font-ko);
    font-size: 26px;
    color: var(--accent);
  }

  .card .example {
    margin-top: 16px;
    display: flex;
    gap: 12px;
    align-items: baseline;
    justify-content: center;
    color: var(--fg-dim);
  }

  .example-korean {
    font-family: var(--font-ko);
    font-size: 24px;
    color: var(--fg);
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
