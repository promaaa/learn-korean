<script lang="ts">
  import { fly } from "svelte/transition";
  import Build from "../games/Build.svelte";
  import Listening from "../games/Listening.svelte";
  import Reply from "../games/Reply.svelte";
  import { formatInterval } from "../lib/format";
  import type { Focus, Profile } from "../lib/api";
  import type { Action, Direction } from "../lib/keys";
  import { SessionController } from "../lib/session.svelte";
  import { Speaker } from "../lib/speaker.svelte";
  import { ACTIVE_ATTRIBUTE, nextWord, WORD_SELECTOR } from "../lib/words";

  type Hint = { keys: string; label: string };

  const FOCUS_LABELS: Record<Focus, string> = {
    words: "words only",
    guided: "words, then sentences",
    all: "everything",
  };

  let { onprofile }: { onprofile: (profile: Profile) => void } = $props();

  const session = new SessionController();
  const speaker = new Speaker();
  /** Highlighted option (choice games) or chunk (build game). */
  let cursor = $state(0);
  /** Build game: indices of the chunks placed so far. */
  let picked = $state<number[]>([]);
  let spokenKey = 0;
  /** The card on screen, remounted per exercise. */
  let card = $state<HTMLElement>();
  /** Word whose gloss the arrow keys show after answering (stale once its card is gone). */
  let activeWord: HTMLElement | null = null;

  $effect(() => {
    void session.start();
  });

  $effect(() => {
    if (session.profile) onprofile(session.profile);
  });

  const prompt = $derived(session.exercise?.prompt ?? null);
  const exerciseKey = $derived(session.exercise ? session.serial : 0);
  const choices = $derived(
    prompt && prompt.type !== "build" ? prompt.options.length : (prompt?.chunks.length ?? 0),
  );

  // A new card: reset the controls and speak it (the build game would give the answer away,
  // so it speaks only after answering).
  $effect(() => {
    if (session.phase !== "exercise" || !prompt || exerciseKey === spokenKey) return;
    spokenKey = exerciseKey;
    cursor = 0;
    picked = [];
    if (prompt.type !== "build") void speaker.say(prompt.korean);
  });

  function replay(): void {
    if (!prompt) return;
    if (prompt.type !== "build") void speaker.say(prompt.korean);
    else if (session.feedback) void speaker.say(session.feedback.korean);
  }

  export function hints(): Hint[] {
    const hide = { keys: "Esc", label: "hide" };
    const focus = { keys: "F", label: "focus" };
    if (session.phase === "exercise" && prompt?.type === "build") {
      return [
        { keys: "1-9", label: "place" },
        { keys: "H L", label: "move" },
        { keys: "Space", label: "place" },
        { keys: "⌫", label: "undo" },
        { keys: "Enter", label: "check" },
        hide,
      ];
    }
    switch (session.phase) {
      case "exercise":
        return [
          { keys: "1-4", label: "answer" },
          { keys: "J K", label: "move" },
          { keys: "Enter", label: "choose" },
          { keys: "R", label: "replay" },
          focus,
          hide,
        ];
      case "feedback": {
        const words = Object.keys(session.feedback?.glosses ?? {}).length > 0;
        return [
          { keys: "Space", label: "next" },
          ...(words ? [{ keys: "← ↑ ↓ →", label: "words" }] : []),
          { keys: "R", label: "replay" },
          hide,
        ];
      }
      default:
        return [
          { keys: "Enter", label: "new session" },
          focus,
          { keys: "Tab", label: "typing gym" },
          hide,
        ];
    }
  }

  /** Called when the window is summoned again. */
  export function resume(): void {
    if (session.phase === "done" || session.phase === "empty") void session.start();
    else {
      session.resumeTimer();
      if (session.phase === "exercise") replay();
    }
  }

  async function choose(index: number): Promise<void> {
    if (index >= choices) return;
    await session.answer({ type: "choice", index });
    // In the reply game, hear the natural answer spoken back.
    const correct = session.feedback?.correctIndex;
    if (prompt?.type === "response" && correct != null) {
      const answer = prompt.options[correct];
      if (answer) void speaker.say(answer);
    }
  }

  function place(index: number): void {
    if (index >= choices || picked.includes(index)) return;
    picked = [...picked, index];
    const next = [...Array(choices).keys()].find((i) => !picked.includes(i));
    if (next !== undefined) cursor = next;
  }

  async function check(): Promise<void> {
    if (picked.length !== choices) return;
    await session.answer({ type: "order", order: picked });
    if (session.feedback) void speaker.say(session.feedback.korean);
  }

  function buildAction(action: Action): void {
    switch (action.type) {
      case "choose":
        place(action.index);
        break;
      case "move":
        if (action.direction === "left" || action.direction === "right") {
          const step = action.direction === "right" ? 1 : -1;
          cursor = (cursor + step + choices) % choices;
        }
        break;
      case "continue":
        place(cursor);
        break;
      case "erase": {
        const last = picked.at(-1);
        if (last === undefined) break;
        picked = picked.slice(0, -1);
        cursor = last;
        break;
      }
      case "confirm":
        void check();
        break;
    }
  }

  function choiceAction(action: Action): void {
    if (action.type === "choose") void choose(action.index);
    else if (action.type === "move" && (action.direction === "down" || action.direction === "up")) {
      const step = action.direction === "down" ? 1 : -1;
      cursor = (cursor + step + choices) % choices;
    } else if (action.type === "confirm") void choose(cursor);
  }

  /** Shows the gloss of the next word in `direction`, as if it were hovered. */
  function browseWords(direction: Direction): void {
    const words = [...(card?.querySelectorAll<HTMLElement>(WORD_SELECTOR) ?? [])];
    const current = activeWord ? words.indexOf(activeWord) : -1;
    const boxes = words.map((word) => word.getBoundingClientRect());
    const next = nextWord(boxes, current < 0 ? null : current, direction);
    activeWord?.removeAttribute(ACTIVE_ATTRIBUTE);
    activeWord = next === null ? null : (words[next] ?? null);
    activeWord?.setAttribute(ACTIVE_ATTRIBUTE, "");
  }

  export function handle(action: Action): void {
    if (action.type === "replay") {
      replay();
      return;
    }
    if (action.type === "focus") {
      void session.cycleFocus();
      return;
    }
    switch (session.phase) {
      case "exercise":
        if (prompt?.type === "build") buildAction(action);
        else choiceAction(action);
        break;
      case "feedback":
        if (action.type === "continue" || action.type === "confirm") void session.next();
        else if (action.type === "move") browseWords(action.direction);
        break;
      case "done":
      case "empty":
      case "error":
        if (action.type === "confirm" || action.type === "continue") void session.start();
        break;
    }
  }

  const accuracy = $derived(
    session.progress.done === 0
      ? 0
      : Math.round((100 * session.progress.correct) / session.progress.done),
  );
</script>

<div class="session">
  <div class="progress" aria-label="session progress">
    <div
      class="bar"
      style:width="{(100 * session.progress.done) /
        Math.max(1, session.progress.done + session.progress.remaining)}%"
    ></div>
  </div>

  {#if session.focus}
    <p class="focus">focus <strong>{FOCUS_LABELS[session.focus]}</strong></p>
  {/if}

  {#if (session.phase === "exercise" || session.phase === "feedback") && session.exercise}
    {#key session.serial}
      <div class="card" bind:this={card} in:fly={{ y: 14, duration: 160 }}>
        {#if prompt?.type === "build"}
          <Build
            itemId={session.exercise.card.itemId}
            image={session.exercise.image}
            english={prompt.english}
            chunks={prompt.chunks}
            {picked}
            feedback={session.feedback}
            {cursor}
            audio={speaker.status}
            onpick={place}
            onreplay={replay}
          />
        {:else if prompt}
          {@const Game = prompt.type === "response" ? Reply : Listening}
          <Game
            itemId={session.exercise.card.itemId}
            image={session.exercise.image}
            korean={prompt.korean}
            options={prompt.options}
            feedback={session.feedback}
            chosen={session.chosen}
            {cursor}
            audio={speaker.status}
            onchoose={choose}
            onreplay={replay}
          />
        {/if}
      </div>
    {/key}
  {:else if session.phase === "done"}
    <div class="summary" in:fly={{ y: 14, duration: 160 }}>
      <h2>Session complete</h2>
      <dl>
        <div><dt>answered</dt><dd>{session.progress.done}</dd></div>
        <div><dt>accuracy</dt><dd>{accuracy}%</dd></div>
        <div><dt>best streak</dt><dd>×{session.progress.bestStreak}</dd></div>
        <div><dt>earned</dt><dd>{session.progress.xp} XP</dd></div>
      </dl>
    </div>
  {:else if session.phase === "empty"}
    <div class="summary">
      <h2>All caught up</h2>
      <p>Nothing is due and there is nothing new to learn right now.</p>
    </div>
  {:else if session.phase === "error"}
    <div class="summary">
      <h2 class="bad">Something went wrong</h2>
      <p class="mono">{session.error}</p>
    </div>
  {/if}

  {#if session.levelUp}
    <div class="level-up" in:fly={{ y: -16, duration: 220 }}>
      <strong>Level {session.levelUp.level}</strong>
      {#each session.levelUp.unlocked as pack (pack)}<span>{pack} unlocked</span>{/each}
    </div>
  {/if}

  {#if session.phase === "feedback" && session.feedback}
    {#key session.progress.done}
      <p class="gain" in:fly={{ y: 8, duration: 200 }}>+{session.feedback.xp} XP</p>
    {/key}
    <p class="due">
      {session.feedback.retry ? "again later in this session" : "next review"}
      {session.feedback.retry ? "" : formatInterval(session.feedback.dueInMs)}
    </p>
  {/if}

  <div class="streak" class:on={session.progress.streak >= 2}>
    streak <strong>×{session.progress.streak}</strong>
  </div>
</div>

<style>
  .session {
    position: relative;
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }

  .progress {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }

  .bar {
    height: 100%;
    background: var(--accent);
    transition: width 240ms ease;
  }

  .focus {
    position: absolute;
    top: 12px;
    right: 0;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .focus strong {
    color: var(--fg-dim);
    font-weight: 600;
  }

  .card {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .summary {
    text-align: center;
  }

  .summary h2 {
    font-weight: 600;
    margin: 0 0 18px;
  }

  .summary p {
    color: var(--fg-dim);
  }

  dl {
    display: flex;
    gap: 36px;
    margin: 0;
  }

  dt {
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  dd {
    margin: 4px 0 0;
    font-size: 28px;
    font-weight: 600;
  }

  .streak {
    position: absolute;
    bottom: 0;
    font-size: 13px;
    color: var(--muted);
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 160ms ease,
      transform 160ms ease;
  }

  .streak.on {
    opacity: 1;
    transform: none;
  }

  .gain {
    position: absolute;
    bottom: 0;
    left: 0;
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--good);
  }

  .level-up {
    position: absolute;
    top: 14px;
    display: flex;
    gap: 12px;
    align-items: baseline;
    padding: 8px 18px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--warn) 18%, var(--panel));
    border: 1px solid var(--warn);
    font-size: 14px;
    z-index: 2;
  }

  .level-up strong {
    color: var(--warn);
  }

  .due {
    position: absolute;
    bottom: 0;
    right: 0;
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  .streak strong {
    color: var(--warn);
  }

  .bad {
    color: var(--bad);
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 13px;
  }
</style>
