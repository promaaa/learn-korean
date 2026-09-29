<script lang="ts">
  import Photo from "../components/Photo.svelte";
  import SpeakerButton from "../components/SpeakerButton.svelte";
  import type { Feedback } from "../lib/api";
  import type { SpeakerStatus } from "../lib/speaker.svelte";

  interface Props {
    itemId: string;
    image: boolean;
    english: string;
    chunks: string[];
    /** Indices of `chunks`, in the order placed. */
    picked: number[];
    feedback: Feedback | null;
    cursor: number;
    audio: SpeakerStatus;
    onpick: (index: number) => void;
    onreplay: () => void;
  }
  let {
    itemId,
    image,
    english,
    chunks,
    picked,
    feedback,
    cursor,
    audio,
    onpick,
    onreplay,
  }: Props = $props();

  const complete = $derived(picked.length === chunks.length);
</script>

<section class="game">
  {#if image}<Photo {itemId} />{/if}
  <p class="english">{english}</p>
  <p class="question">Build it in Korean</p>

  <div
    class="answer"
    class:right={feedback?.correct === true}
    class:wrong={feedback?.correct === false}
    lang="ko"
  >
    {#each picked as index, position (position)}
      <span class="placed">{chunks[index]}</span>
    {/each}
    {#if !feedback}<span class="caret" class:done={complete}></span>{/if}
  </div>

  {#if feedback}
    <div class="reveal visible">
      <p class="verdict" class:good={feedback.correct} class:bad={!feedback.correct}>
        {feedback.correct ? "Correct" : "The sentence is"}
      </p>
      <div class="solution">
        <SpeakerButton status={audio} onclick={onreplay} />
        {#if !feedback.correct}<p class="korean" lang="ko">{feedback.korean}</p>{/if}
      </div>
      {#if feedback.note}<p class="note">{feedback.note}</p>{/if}
    </div>
  {:else}
    <ol class="bank">
      {#each chunks as chunk, index (index)}
        <li>
          <button
            type="button"
            class="chunk"
            class:used={picked.includes(index)}
            class:cursor={index === cursor}
            disabled={picked.includes(index)}
            onclick={() => onpick(index)}
          >
            <kbd>{index + 1}</kbd><span lang="ko">{chunk}</span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .english {
    font-size: 26px;
    font-weight: 500;
    text-align: center;
    margin: 8px 0 0;
  }

  .answer {
    width: 100%;
    min-height: 64px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px dashed var(--line);
    background: var(--panel);
    font-family: var(--font-ko);
    font-size: 24px;
  }

  .answer.right {
    border: 1px solid var(--good);
  }

  .answer.wrong {
    border: 1px solid var(--bad);
    animation: shake 260ms ease;
  }

  .placed {
    animation: pop 120ms ease;
  }

  .caret {
    width: 2px;
    height: 28px;
    background: var(--accent);
    animation: blink 1s steps(1) infinite;
  }

  .caret.done {
    background: var(--good);
  }

  .bank {
    list-style: none;
    margin: 18px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px;
  }

  .chunk {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--fg);
    font-family: var(--font-ko);
    font-size: 20px;
    cursor: pointer;
    transition:
      opacity 120ms ease,
      transform 120ms ease;
  }

  .chunk kbd {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent);
  }

  .chunk.cursor {
    border-color: var(--accent);
  }

  .chunk.used {
    opacity: 0.25;
    transform: scale(0.95);
    cursor: default;
  }

  .solution {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 8px;
  }

  .solution .korean {
    font-size: 32px;
  }

  @keyframes pop {
    from {
      transform: scale(0.85);
      opacity: 0;
    }
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
      transform: translateX(-5px);
    }
    75% {
      transform: translateX(5px);
    }
  }
</style>
