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
  <SpeakerButton status={audio} onclick={onreplay} />
  <p class="korean" lang="ko"><Glossed text={korean} glosses={feedback?.glosses ?? null} /></p>
  <p class="question">What does this mean?</p>

  <ChoiceList {options} {feedback} {chosen} {cursor} {onchoose} />

  <div class="reveal" class:visible={feedback !== null}>
    {#if feedback}
      <p class="verdict" class:good={feedback.correct} class:bad={!feedback.correct}>
        {feedback.correct ? "Correct" : "Not quite"}: <span>{feedback.english}</span>
      </p>
      {#if feedback.note}<p class="note">{feedback.note}</p>{/if}
    {/if}
  </div>
</section>
