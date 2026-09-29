<script lang="ts">
  import { onMount } from "svelte";
  import HangulKeyboard from "../components/HangulKeyboard.svelte";
  import {
    typingLayout,
    typingNext,
    typingPress,
    typingStart,
    type DrillView,
    type KeyCap,
  } from "../lib/api";

  type Hint = { keys: string; label: string };

  let layout = $state<KeyCap[]>([]);
  let view = $state<DrillView | null>(null);
  let roundOver = $state(false);
  let error = $state<string | null>(null);
  let wrong = $state<string | null>(null);
  let shake = $state(0);
  /** Stats of finished lines in this round, for the summary. */
  let finished = $state<{ wpm: number; accuracy: number }[]>([]);
  let wrongTimer: ReturnType<typeof setTimeout> | undefined;
  /** A round or line change is in flight; Enter presses meanwhile are ignored. */
  let busy = false;

  const MODIFIERS = new Set([
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "MetaLeft",
    "MetaRight",
    "CapsLock",
  ]);

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

  const start = () =>
    guarded(async () => {
      if (layout.length === 0) layout = await typingLayout();
      view = await typingStart();
      roundOver = false;
      finished = [];
    });

  const next = () =>
    guarded(async () => {
      const upcoming = await typingNext();
      if (upcoming) view = upcoming;
      else roundOver = true;
    });

  // Not an $effect: `start` reads `layout`, which would make the effect start a second round.
  onMount(() => void start());

  export function hints(): Hint[] {
    const common = [
      { keys: "Tab", label: "review" },
      { keys: "Esc", label: "hide" },
    ];
    if (roundOver) return [{ keys: "Enter", label: "new round" }, ...common];
    if (view?.snapshot.finished) return [{ keys: "Enter", label: "next line" }, ...common];
    return [{ keys: "⌨", label: "type the line" }, ...common];
  }

  /** Raw key events while the gym is on screen (Tab and Esc are handled by the app). */
  export function key(event: KeyboardEvent): void {
    if (event.ctrlKey || event.altKey || event.metaKey) return;
    event.preventDefault();
    if (event.repeat || MODIFIERS.has(event.code)) return;
    if (roundOver) {
      if (event.key === "Enter") void start();
      return;
    }
    if (!view) return;
    if (view.snapshot.finished) {
      if (event.key === "Enter") void next();
      return;
    }
    const at = Math.round(performance.now());
    typingPress(event.code, event.shiftKey, at)
      .then((result) => {
        view = result.view;
        if (result.outcome.ignored) return;
        if (!result.outcome.correct) {
          wrong = event.code;
          shake += 1;
          clearTimeout(wrongTimer);
          wrongTimer = setTimeout(() => (wrong = null), 280);
        }
        if (result.outcome.finished) {
          finished = [
            ...finished,
            { wpm: result.view.snapshot.wpm, accuracy: result.view.snapshot.accuracy },
          ];
        }
      })
      .catch((err: unknown) => (error = String(err)));
  }

  const chars = $derived(view ? [...view.snapshot.target] : []);
  const average = $derived.by(() => {
    if (finished.length === 0) return { wpm: 0, accuracy: 0 };
    const sum = finished.reduce((a, f) => ({ wpm: a.wpm + f.wpm, accuracy: a.accuracy + f.accuracy }), {
      wpm: 0,
      accuracy: 0,
    });
    return { wpm: sum.wpm / finished.length, accuracy: sum.accuracy / finished.length };
  });
</script>

<div class="gym">
  <header class="title">
    <span>Typing Gym</span>
    {#if view && !roundOver}<span class="count">{view.position} / {view.total}</span>{/if}
  </header>

  {#if error}
    <p class="error">{error}</p>
  {:else if roundOver}
    <div class="summary">
      <h2>Round complete</h2>
      <dl>
        <div><dt>lines</dt><dd>{finished.length}</dd></div>
        <div><dt>speed</dt><dd>{Math.round(average.wpm)} <small>wpm</small></dd></div>
        <div><dt>accuracy</dt><dd>{Math.round(average.accuracy * 100)}%</dd></div>
      </dl>
    </div>
  {:else if view}
    <p class="english">{view.target.english}</p>
    {#key shake}
      <p class="target" class:shake={shake > 0} lang="ko">
        {#each chars as char, index (index)}
          <span
            class:done={index < view.snapshot.doneChars}
            class:current={index === view.snapshot.doneChars && !view.snapshot.finished}
            >{char === " " ? "\u00a0" : char}</span
          >
        {/each}
      </p>
    {/key}
    <p class="typed" class:complete={view.snapshot.finished} lang="ko">
      {view.snapshot.typed}<span class="caret"></span>
    </p>

    <div class="stats">
      <span><strong>{Math.round(view.snapshot.wpm)}</strong> wpm</span>
      <span><strong>{Math.round(view.snapshot.cpm)}</strong> keys/min</span>
      <span><strong>{Math.round(view.snapshot.accuracy * 100)}%</strong> accuracy</span>
      <span class:hot={view.snapshot.streak >= 10}><strong>×{view.snapshot.streak}</strong> streak</span>
    </div>

    <HangulKeyboard {layout} next={view.snapshot.next} {wrong} />
  {/if}
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

  .title {
    position: absolute;
    top: 52px;
    left: 24px;
    right: 24px;
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .english {
    margin: 0;
    color: var(--fg-dim);
    font-size: 15px;
  }

  .target {
    margin: 4px 0 0;
    font-family: var(--font-ko);
    font-size: 38px;
    letter-spacing: 0.02em;
    color: var(--muted);
  }

  .target .done {
    color: var(--fg);
  }

  .target .current {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 8px;
    text-decoration-thickness: 2px;
  }

  .target.shake {
    animation: shake 220ms ease;
  }

  .typed {
    min-height: 34px;
    margin: 0 0 8px;
    font-family: var(--font-ko);
    font-size: 24px;
    color: var(--good);
  }

  .typed.complete {
    color: var(--good);
  }

  .caret {
    display: inline-block;
    width: 2px;
    height: 24px;
    margin-left: 2px;
    vertical-align: middle;
    background: var(--accent);
    animation: blink 1s steps(1) infinite;
  }

  .stats {
    display: flex;
    gap: 22px;
    margin-bottom: 14px;
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .stats strong {
    font-size: 16px;
    color: var(--fg);
  }

  .stats .hot strong {
    color: var(--warn);
  }

  .summary {
    text-align: center;
  }

  .summary h2 {
    font-weight: 600;
    margin: 0 0 18px;
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

  dd small {
    font-size: 13px;
    color: var(--muted);
  }

  .error {
    color: var(--bad);
    font-family: var(--font-mono);
    font-size: 13px;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }

  @keyframes shake {
    0%,
    100% {
      transform: translateX(0);
    }
    25% {
      transform: translateX(-4px);
    }
    75% {
      transform: translateX(4px);
    }
  }
</style>
