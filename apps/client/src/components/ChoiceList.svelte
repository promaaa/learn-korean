<script lang="ts">
  import type { Feedback } from "../lib/api";
  import Glossed from "./Glossed.svelte";

  interface Props {
    options: string[];
    feedback: Feedback | null;
    chosen: number | null;
    cursor: number;
    /** Options are Korean (reply game): Korean font, translations revealed after answering. */
    korean?: boolean;
    onchoose: (index: number) => void;
  }
  let { options, feedback, chosen, cursor, korean = false, onchoose }: Props = $props();

  function optionState(index: number): "correct" | "wrong" | "dim" | "idle" {
    if (!feedback) return "idle";
    if (index === feedback.correctIndex) return "correct";
    if (index === chosen) return "wrong";
    return "dim";
  }
</script>

<ol class="options">
  {#each options as option, index (index)}
    <li>
      <button
        type="button"
        class="option {optionState(index)}"
        class:cursor={!feedback && index === cursor}
        disabled={feedback !== null}
        onclick={() => onchoose(index)}
      >
        <kbd>{index + 1}</kbd>
        <span class="text">
          <span class:ko={korean} lang={korean ? "ko" : undefined}>
            {#if korean}<Glossed text={option} glosses={feedback?.glosses ?? null} />{:else}{option}{/if}
          </span>
          {#if feedback && feedback.translations[index]}
            <span class="translation">{feedback.translations[index]}</span>
          {/if}
        </span>
      </button>
    </li>
  {/each}
</ol>

<style>
  .options {
    list-style: none;
    padding: 0;
    margin: 0;
    width: 100%;
    display: grid;
    gap: 8px;
  }

  .option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 11px 14px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--fg);
    font: inherit;
    font-size: 16px;
    text-align: left;
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      opacity 160ms ease,
      transform 120ms ease;
  }

  .option:disabled {
    cursor: default;
  }

  .option.cursor,
  .option:not(:disabled):hover {
    border-color: var(--accent);
  }

  .option kbd {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--accent);
    min-width: 1ch;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ko {
    font-family: var(--font-ko);
    font-size: 18px;
  }

  .translation {
    font-size: 12px;
    color: var(--fg-dim);
  }

  .option.correct {
    border-color: var(--good);
    background: color-mix(in srgb, var(--good) 16%, var(--panel));
    transform: scale(1.015);
  }

  .option.correct kbd {
    color: var(--good);
  }

  .option.wrong {
    border-color: var(--bad);
    background: color-mix(in srgb, var(--bad) 14%, var(--panel));
    animation: shake 260ms ease;
  }

  .option.wrong kbd {
    color: var(--bad);
  }

  .option.dim {
    opacity: 0.45;
  }

  /* A dimmed Korean option stays readable while one of its words shows its gloss. */
  .option.dim:hover:has(:global(.word)),
  .option.dim:has(:global([data-active])) {
    opacity: 1;
  }

  @keyframes shake {
    0%,
    100% {
      transform: translateX(0);
    }
    25% {
      transform: translateX(-5px);
    }
    75% {
      transform: translateX(5px);
    }
  }
</style>
