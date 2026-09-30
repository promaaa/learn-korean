<script lang="ts">
  import { onMount } from "svelte";
  import RoundSummary from "../components/RoundSummary.svelte";
  import ScreenTitle from "../components/ScreenTitle.svelte";
  import TypingDrill from "../components/TypingDrill.svelte";
  import { typingLayout, typingNext, typingPress, typingStart, type KeyCap } from "../lib/api";
  import { LiveDrill, claimKey } from "../lib/drill.svelte";

  type Hint = { keys: string; label: string };

  const drill = new LiveDrill(typingPress);
  let layout = $state<KeyCap[]>([]);
  let roundOver = $state(false);
  let error = $state<string | null>(null);
  /** Stats of finished lines in this round, for the summary. */
  let finished = $state<{ wpm: number; accuracy: number }[]>([]);
  /** A round or line change is in flight; Enter presses meanwhile are ignored. */
  let busy = false;

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
      drill.view = await typingStart();
      roundOver = false;
      finished = [];
    });

  const next = () =>
    guarded(async () => {
      const upcoming = await typingNext();
      if (upcoming) drill.view = upcoming;
      else roundOver = true;
    });

  // Not an $effect: `start` reads `layout`, which would make the effect start a second round.
  onMount(() => void start());

  export function hints(): Hint[] {
    const common = [
      { keys: "Tab", label: "hangul" },
      { keys: "Esc", label: "hide" },
    ];
    if (roundOver) return [{ keys: "Enter", label: "new round" }, ...common];
    if (drill.view?.snapshot.finished) return [{ keys: "Enter", label: "next line" }, ...common];
    return [{ keys: "⌨", label: "type the line" }, ...common];
  }

  /** Raw key events while the gym is on screen (Tab and Esc are handled by the app). */
  export function key(event: KeyboardEvent): void {
    if (!claimKey(event)) return;
    if (roundOver) {
      if (event.key === "Enter") void start();
      return;
    }
    if (!drill.view) return;
    if (drill.view.snapshot.finished) {
      if (event.key === "Enter") void next();
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
  }

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
  <ScreenTitle
    label="Typing Gym"
    count={drill.view && !roundOver ? `${drill.view.position} / ${drill.view.total}` : undefined}
  />

  {#if error}
    <p class="error">{error}</p>
  {:else if roundOver}
    <RoundSummary
      title="Round complete"
      stats={[
        { label: "lines", value: finished.length },
        { label: "speed", value: Math.round(average.wpm), unit: "wpm" },
        { label: "accuracy", value: `${Math.round(average.accuracy * 100)}%` },
      ]}
    />
  {:else if drill.view}
    <TypingDrill view={drill.view} {layout} wrong={drill.wrong} shake={drill.shake} />
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

  .error {
    color: var(--bad);
    font-family: var(--font-mono);
    font-size: 13px;
  }
</style>
