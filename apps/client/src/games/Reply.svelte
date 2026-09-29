<script lang="ts">
  import ChoiceList from "../components/ChoiceList.svelte";
  import Glossed from "../components/Glossed.svelte";
  import Photo from "../components/Photo.svelte";
  import SpeakerButton from "../components/SpeakerButton.svelte";
  import type { Feedback } from "../lib/api";
  import type { SpeakerStatus } from "../lib/speaker.svelte";

  interface Props {
    itemId: string;
    image: boolean;
    korean: string;
    options: string[];
    feedback: Feedback | null;
    chosen: number | null;
    cursor: number;
    audio: SpeakerStatus;
    onchoose: (index: number) => void;
    onreplay: () => void;
  }
  let { itemId, image, korean, options, feedback, chosen, cursor, audio, onchoose, onreplay }: Props =
    $props();
</script>

<section class="game">
  {#if image}<Photo {itemId} />{/if}
  <div class="line">
    <SpeakerButton status={audio} onclick={onreplay} />
    <p class="bubble" lang="ko"><Glossed text={korean} glosses={feedback?.glosses ?? null} /></p>
    {#if feedback}<p class="line-english">{feedback.english}</p>{/if}
  </div>
  <p class="question">How do you answer?</p>

  <ChoiceList {options} {feedback} {chosen} {cursor} korean {onchoose} />

  <div class="reveal" class:visible={feedback !== null}>
    {#if feedback}
      <p class="verdict" class:good={feedback.correct} class:bad={!feedback.correct}>
        {feedback.correct ? "Natural answer" : "That doesn't fit"}
      </p>
      {#if feedback.note}<p class="note">{feedback.note}</p>{/if}
    {/if}
  </div>
</section>

<style>
  .line {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .bubble {
    font-family: var(--font-ko);
    font-size: 34px;
    font-weight: 500;
    margin: 0;
    padding: 10px 22px;
    border-radius: 18px 18px 18px 4px;
    background: color-mix(in srgb, var(--accent) 12%, var(--panel));
    border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--line));
    word-break: keep-all;
    text-align: center;
  }

  .line-english {
    margin: 6px 0 0;
    font-size: 13px;
    color: var(--fg-dim);
  }
</style>
