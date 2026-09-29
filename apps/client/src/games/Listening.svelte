<script lang="ts">
  import SpeakerButton from "../components/SpeakerButton.svelte";
  import type { Feedback } from "../lib/api";
  import type { SpeakerStatus } from "../lib/speaker.svelte";

  interface Props {
    korean: string;
    options: string[];
    feedback: Feedback | null;
    chosen: number | null;
    cursor: number;
    audio: SpeakerStatus;
    onchoose: (index: number) => void;
    onreplay: () => void;
  }
  let { korean, options, feedback, chosen, cursor, audio, onchoose, onreplay }: Props = $props();

  function optionState(index: number): "correct" | "wrong" | "dim" | "idle" {
    if (!feedback) return "idle";
    if (index === feedback.correctIndex) return "correct";
    if (index === chosen) return "wrong";
    return "dim";
  }
</script>

<section class="listening">
  <SpeakerButton status={audio} onclick={onreplay} />
  <p class="korean" lang="ko">{korean}</p>
  <p class="question">What does this mean?</p>

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
          <span>{option}</span>
        </button>
      </li>
    {/each}
  </ol>

  <div class="reveal" class:visible={feedback !== null}>
    {#if feedback}
      <p class="verdict" class:good={feedback.correct} class:bad={!feedback.correct}>
        {feedback.correct ? "Correct" : "Not quite"} — <span>{feedback.english}</span>
      </p>
      {#if feedback.note}<p class="note">{feedback.note}</p>{/if}
    {/if}
  </div>
</section>

<style>
  .listening {
    width: 100%;
    max-width: 600px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .korean {
    font-family: var(--font-ko);
    font-size: 44px;
    font-weight: 500;
    line-height: 1.25;
    margin: 0 0 6px;
    text-align: center;
    letter-spacing: 0.01em;
    word-break: keep-all;
  }

  .question {
    align-self: flex-start;
    color: var(--muted);
    font-size: 14px;
    margin: 18px 0 10px;
  }

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
    opacity: 0.4;
  }

  .reveal {
    min-height: 56px;
    margin-top: 14px;
    text-align: center;
    opacity: 0;
    transform: translateY(4px);
    transition:
      opacity 160ms ease,
      transform 160ms ease;
  }

  .reveal.visible {
    opacity: 1;
    transform: none;
  }

  .verdict {
    margin: 0;
    font-weight: 600;
  }

  .verdict span {
    font-weight: 400;
    color: var(--fg);
  }

  .good {
    color: var(--good);
  }

  .bad {
    color: var(--bad);
  }

  .note {
    margin: 6px 0 0;
    font-size: 13px;
    color: var(--fg-dim);
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
