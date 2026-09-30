<script lang="ts" module>
  export interface ListedLesson {
    id: string;
    title: string;
    /** The letters or keys the lesson teaches. */
    letters: string;
    unlocked: boolean;
    passed: boolean;
  }
</script>

<script lang="ts">
  interface Props {
    lessons: ListedLesson[];
    selected: number;
  }
  let { lessons, selected }: Props = $props();

  let rows = $state<HTMLLIElement[]>([]);

  // Long lists scroll: keep the highlighted lesson in view.
  $effect(() => rows[selected]?.scrollIntoView({ block: "nearest" }));
</script>

<ol class="lessons">
  {#each lessons as item, index (item.id)}
    <li bind:this={rows[index]} class:selected={index === selected} class:locked={!item.unlocked}>
      <span class="number">{index + 1}</span>
      <span class="name">{item.title}</span>
      <span class="letters" lang="ko">{item.letters}</span>
      <span class="status" class:passed={item.passed}>
        {item.passed ? "passed" : item.unlocked ? "" : "locked"}
      </span>
    </li>
  {/each}
</ol>

<style>
  .lessons {
    width: 100%;
    max-width: 560px;
    min-height: 0;
    margin: 24px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .lessons li {
    display: grid;
    grid-template-columns: 28px 1fr auto 64px;
    align-items: baseline;
    gap: 14px;
    padding: 8px 14px;
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--fg-dim);
  }

  .lessons li.selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, var(--panel));
    color: var(--fg);
  }

  .lessons li.locked {
    opacity: 0.4;
  }

  .number {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .letters {
    font-family: var(--font-ko);
    color: var(--fg);
    letter-spacing: 0.05em;
  }

  .status {
    font-size: 12px;
    color: var(--muted);
    text-align: right;
  }

  .status.passed {
    color: var(--good);
  }
</style>
