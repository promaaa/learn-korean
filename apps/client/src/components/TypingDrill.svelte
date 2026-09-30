<script lang="ts">
  import HangulKeyboard from "./HangulKeyboard.svelte";
  import type { DrillView, KeyCap } from "../lib/api";

  interface Props {
    view: DrillView;
    layout: KeyCap[];
    /** Code of the last wrong key, flashed red. */
    wrong: string | null;
    /** Changes on every wrong key, replaying the shake. */
    shake: number;
  }
  let { view, layout, wrong, shake }: Props = $props();

  const chars = $derived([...view.snapshot.target]);
</script>

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
<p class="typed" lang="ko">
  {view.snapshot.typed}<span class="caret"></span>
</p>

<div class="stats">
  <span><strong>{Math.round(view.snapshot.wpm)}</strong> wpm</span>
  <span><strong>{Math.round(view.snapshot.cpm)}</strong> keys/min</span>
  <span><strong>{Math.round(view.snapshot.accuracy * 100)}%</strong> accuracy</span>
  <span class:hot={view.snapshot.streak >= 10}><strong>×{view.snapshot.streak}</strong> streak</span>
</div>

<HangulKeyboard {layout} next={view.snapshot.next} {wrong} />

<style>
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
