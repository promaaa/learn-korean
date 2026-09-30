<script lang="ts">
  interface Props {
    label: string;
    /** Right-aligned progress, e.g. `3 / 8`. */
    count?: string;
  }
  let { label, count }: Props = $props();
</script>

<header class="title">
  <span>{label}</span>
  {#if count}<span class="count">{count}</span>{/if}
</header>

<style>
  .title {
    position: absolute;
    top: 52px;
    left: 24px;
    right: 24px;
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }
</style>
