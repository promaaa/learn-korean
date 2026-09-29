<script lang="ts">
  interface Hint {
    keys: string;
    label: string;
  }
  let { hints }: { hints: Hint[] } = $props();
</script>

<footer class="hints">
  {#each hints as hint (hint.keys)}
    <span><kbd>{hint.keys}</kbd>{hint.label}</span>
  {/each}
</footer>

<style>
  .hints {
    display: flex;
    gap: 18px;
    justify-content: center;
    font-size: 12px;
    color: var(--muted);
  }
  kbd {
    font-family: var(--font-mono);
    color: var(--fg-dim);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 1px 5px;
    margin-right: 6px;
  }
</style>
