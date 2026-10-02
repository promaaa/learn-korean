<script lang="ts">
  import Glossed from "../components/Glossed.svelte";
  import Photo from "../components/Photo.svelte";
  import SpeakerButton from "../components/SpeakerButton.svelte";
  import type { ExerciseView } from "../lib/api";
  import type { SpeakerStatus } from "../lib/speaker.svelte";

  interface Props {
    itemId: string;
    image: boolean;
    kind: ExerciseView["kind"];
    korean: string;
    english: string;
    note: string | null;
    glosses: Record<string, string>;
    audio: SpeakerStatus;
    onreplay: () => void;
  }
  let { itemId, image, kind, korean, english, note, glosses, audio, onreplay }: Props = $props();
</script>

<section class="game">
  <p class="question new">New {kind}</p>
  {#if image}<Photo {itemId} />{/if}
  <SpeakerButton status={audio} onclick={onreplay} />
  <p class="korean" lang="ko"><Glossed text={korean} {glosses} /></p>
  <p class="meaning">{english}</p>
  {#if note}<p class="note">{note}</p>{/if}
</section>

<style>
  .new {
    align-self: center;
    margin-top: 0;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 12px;
    color: var(--accent);
  }

  .meaning {
    margin: 8px 0 0;
    font-size: 22px;
    color: var(--fg);
    text-align: center;
  }
</style>
