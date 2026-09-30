<script lang="ts">
  import type { Snippet } from "svelte";

  interface Stat {
    label: string;
    value: string | number;
    unit?: string;
  }
  interface Props {
    title: string;
    stats: Stat[];
    /** Shown under the numbers. */
    children?: Snippet;
  }
  let { title, stats, children }: Props = $props();
</script>

<div class="summary">
  <h2>{title}</h2>
  <dl>
    {#each stats as stat (stat.label)}
      <div>
        <dt>{stat.label}</dt>
        <dd>{stat.value}{#if stat.unit}&nbsp;<small>{stat.unit}</small>{/if}</dd>
      </div>
    {/each}
  </dl>
  {@render children?.()}
</div>

<style>
  .summary {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  h2 {
    font-weight: 600;
    margin: 0 0 18px;
  }

  dl {
    display: flex;
    gap: 36px;
    margin: 0;
  }

  dt {
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  dd {
    margin: 4px 0 0;
    font-size: 28px;
    font-weight: 600;
  }

  dd small {
    font-size: 13px;
    color: var(--muted);
  }
</style>
