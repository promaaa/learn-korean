<script lang="ts">
  import { fly } from "svelte/transition";
  import Listening from "../games/Listening.svelte";
  import Reply from "../games/Reply.svelte";
  import type { Action } from "../lib/keys";
  import { SessionController } from "../lib/session.svelte";
  import { Speaker } from "../lib/speaker.svelte";

  const session = new SessionController();
  const speaker = new Speaker();
  let cursor = $state(0);
  let spokenKey = "";

  $effect(() => {
    void session.start();
  });

  // Every new card is spoken once when it appears.
  $effect(() => {
    if (session.phase !== "exercise" || !session.exercise) return;
    const key = `${session.exercise.card.itemId}:${session.progress.done}`;
    if (key === spokenKey) return;
    spokenKey = key;
    void speaker.say(session.exercise.prompt.korean);
  });

  function replay(): void {
    if (session.exercise) void speaker.say(session.exercise.prompt.korean);
  }

  const optionCount = $derived(session.exercise?.prompt.options.length ?? 0);
  const Game = $derived(session.exercise?.prompt.type === "response" ? Reply : Listening);

  export function hints(): { keys: string; label: string }[] {
    switch (session.phase) {
      case "exercise":
        return [
          { keys: "1-4", label: "answer" },
          { keys: "J K", label: "move" },
          { keys: "Enter", label: "choose" },
          { keys: "R", label: "replay" },
          { keys: "Esc", label: "hide" },
        ];
      case "feedback":
        return [
          { keys: "Space", label: "next" },
          { keys: "R", label: "replay" },
          { keys: "Esc", label: "hide" },
        ];
      default:
        return [
          { keys: "Enter", label: "new session" },
          { keys: "Esc", label: "hide" },
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
    if (index >= optionCount) return;
    await session.answer({ type: "choice", index });
    // In the reply game, hear the natural answer spoken back.
    const prompt = session.exercise?.prompt;
    if (session.feedback && prompt?.type === "response") {
      const answer = prompt.options[session.feedback.correctIndex];
      if (answer) void speaker.say(answer);
    }
  }

  export function handle(action: Action): void {
    if (action.type === "replay") {
      replay();
      return;
    }
    switch (session.phase) {
      case "exercise":
        if (action.type === "choose") choose(action.index);
        else if (action.type === "move" && (action.direction === "down" || action.direction === "up")) {
          const step = action.direction === "down" ? 1 : -1;
          cursor = (cursor + step + optionCount) % optionCount;
        } else if (action.type === "confirm") choose(cursor);
        break;
      case "feedback":
        if (action.type === "continue" || action.type === "confirm") {
          cursor = 0;
          void session.next();
        }
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

  {#if (session.phase === "exercise" || session.phase === "feedback") && session.exercise}
    {#key `${session.exercise.card.itemId}:${session.progress.done}`}
      <div class="card" in:fly={{ y: 14, duration: 160 }}>
        <Game
          itemId={session.exercise.card.itemId}
          image={session.exercise.image}
          korean={session.exercise.prompt.korean}
          options={session.exercise.prompt.options}
          feedback={session.feedback}
          chosen={session.chosen}
          {cursor}
          audio={speaker.status}
          onchoose={choose}
          onreplay={replay}
        />
      </div>
    {/key}
  {:else if session.phase === "done"}
    <div class="summary" in:fly={{ y: 14, duration: 160 }}>
      <h2>Session complete</h2>
      <dl>
        <div><dt>answered</dt><dd>{session.progress.done}</dd></div>
        <div><dt>accuracy</dt><dd>{accuracy}%</dd></div>
        <div><dt>best streak</dt><dd>×{session.progress.bestStreak}</dd></div>
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
