<script lang="ts">
  import type { SpeakerStatus } from "../lib/speaker.svelte";

  let { status, onclick }: { status: SpeakerStatus; onclick: () => void } = $props();
</script>

<button
  type="button"
  class="speaker {status}"
  title={status === "error" ? "Audio unavailable" : "Replay (R)"}
  {onclick}
>
  <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true">
    <path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" />
    {#if status === "error"}
      <path d="M16 9l5 6M21 9l-5 6" stroke="currentColor" stroke-width="2" fill="none" />
    {:else}
      <path
        d="M16 8.5a5 5 0 0 1 0 7M18.5 6a8.5 8.5 0 0 1 0 12"
        stroke="currentColor"
        stroke-width="2"
        fill="none"
        stroke-linecap="round"
      />
    {/if}
  </svg>
  <kbd>R</kbd>
</button>

<style>
  .speaker {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--accent);
    cursor: pointer;
  }

  .speaker kbd {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--muted);
  }

  .speaker.loading svg {
    animation: pulse 700ms ease-in-out infinite;
  }

  .speaker.error {
    color: var(--muted);
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
