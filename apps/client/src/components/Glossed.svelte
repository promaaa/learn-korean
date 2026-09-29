<script lang="ts">
  interface Props {
    /** Korean text, words separated by single spaces. */
    text: string;
    /** English per displayed word (`Feedback.glosses`); `null` until answered. */
    glosses: Record<string, string> | null;
  }
  let { text, glosses }: Props = $props();

  const words = $derived(text.split(" "));
</script>

<!-- No whitespace between nodes: it would add or drop spaces between the words. -->
{#each words as word, index (index)}{index > 0 ? " " : ""}{#if glosses?.[word]}<span class="word" data-word
      >{word}<span class="gloss" role="tooltip">{glosses[word]}</span></span
    >{:else}{word}{/if}{/each}

<style>
  .word {
    position: relative;
    text-decoration: underline dotted color-mix(in srgb, currentColor 45%, transparent);
    text-decoration-thickness: 1px;
    text-underline-offset: 0.22em;
    cursor: help;
  }

  .gloss {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 50%;
    z-index: 10;
    translate: -50% 4px;
    padding: 4px 9px;
    border-radius: 7px;
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--fg);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 400;
    line-height: 1.3;
    letter-spacing: normal;
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition:
      opacity 100ms ease,
      translate 100ms ease;
  }

  /* Hovered, or picked with the arrow keys (`data-active`, see lib/words.ts). */
  .word:hover .gloss,
  .word:global([data-active]) .gloss {
    opacity: 1;
    translate: -50% 0;
  }

  .word:hover,
  .word:global([data-active]) {
    text-decoration-color: var(--accent);
  }
</style>
