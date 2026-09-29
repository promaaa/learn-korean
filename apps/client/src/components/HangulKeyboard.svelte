<script lang="ts">
  import type { Finger, KeyCap, NextKey } from "../lib/api";

  interface Props {
    layout: KeyCap[];
    next: NextKey | null;
    /** Code of the last wrong key, flashed red. */
    wrong: string | null;
  }
  let { layout, next, wrong }: Props = $props();

  // Row stagger of an ANSI keyboard, in key widths.
  const OFFSETS = [0, 0.5, 0.75, 1.25, 3.5];

  const rows = $derived(
    [0, 1, 2, 3, 4].map((row) => layout.filter((key) => key.row === row)).filter((r) => r.length),
  );

  const leftShift = $derived(next?.shift === true && next.shiftFinger === "leftPinky");
  const rightShift = $derived(next?.shift === true && next.shiftFinger === "rightPinky");

  const FINGER_NAMES: Record<Finger, string> = {
    leftPinky: "left pinky",
    leftRing: "left ring finger",
    leftMiddle: "left middle finger",
    leftIndex: "left index finger",
    rightIndex: "right index finger",
    rightMiddle: "right middle finger",
    rightRing: "right ring finger",
    rightPinky: "right pinky",
    thumb: "thumb",
  };
</script>

<div class="keyboard" aria-hidden="true">
  {#each rows as keys (keys[0]?.row)}
    {@const row = keys[0]?.row ?? 0}
    <div class="row" style:padding-left="{OFFSETS[row] ?? 0}em">
      {#if row === 3}
        <span class="key wide shift" class:next={leftShift}>⇧</span>
      {/if}
      {#each keys as key (key.code)}
        <span
          class="key finger-{key.finger}"
          class:space={key.code === "Space"}
          class:next={next?.code === key.code}
          class:wrong={wrong === key.code}
        >
          {#if key.shifted && key.shifted !== key.base}<small>{key.shifted}</small>{/if}
          <span class="base">{key.code === "Space" ? "" : key.base}</span>
        </span>
      {/each}
      {#if row === 3}
        <span class="key wide shift" class:next={rightShift}>⇧</span>
      {/if}
    </div>
  {/each}
</div>

<p class="finger">
  {#if next}
    {next.code === "Space" ? "space" : next.jamo} · {FINGER_NAMES[next.finger]}{#if next.shiftFinger}
      &nbsp;+ shift with {FINGER_NAMES[next.shiftFinger]}{/if}
  {/if}
</p>

<style>
  .keyboard {
    font-size: 15px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-family: var(--font-ko);
  }

  .row {
    display: flex;
    gap: 5px;
  }

  .key {
    --tint: var(--line);
    position: relative;
    width: 2.6em;
    height: 2.6em;
    display: grid;
    place-items: center;
    border-radius: 7px;
    background: var(--panel);
    border: 1px solid color-mix(in srgb, var(--tint) 45%, var(--line));
    border-bottom-width: 3px;
    color: var(--fg-dim);
    transition:
      background 90ms ease,
      transform 90ms ease,
      border-color 90ms ease;
  }

  .key small {
    position: absolute;
    top: 2px;
    right: 5px;
    font-size: 10px;
    color: var(--muted);
  }

  .base {
    font-size: 1.05em;
  }

  .wide {
    width: 4.1em;
    font-family: var(--font-ui);
  }

  .space {
    width: 15em;
  }

  .finger-leftPinky,
  .finger-rightPinky {
    --tint: #bb9af7;
  }
  .finger-leftRing,
  .finger-rightRing {
    --tint: #7dcfff;
  }
  .finger-leftMiddle,
  .finger-rightMiddle {
    --tint: #9ece6a;
  }
  .finger-leftIndex,
  .finger-rightIndex {
    --tint: #e0af68;
  }
  .finger-thumb {
    --tint: #565f89;
  }

  .key.next {
    background: color-mix(in srgb, var(--accent) 30%, var(--panel));
    border-color: var(--accent);
    color: var(--fg);
    transform: translateY(-2px);
    box-shadow: 0 0 14px color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .key.wrong {
    background: color-mix(in srgb, var(--bad) 35%, var(--panel));
    border-color: var(--bad);
  }

  .finger {
    min-height: 18px;
    margin: 10px 0 0;
    text-align: center;
    font-size: 12px;
    color: var(--muted);
  }
</style>
