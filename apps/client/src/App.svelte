<script lang="ts">
  import FatalError from "./components/FatalError.svelte";
  import KeyHints from "./components/KeyHints.svelte";
  import { appStatus, type AppStatus } from "./lib/api";
  import { actionFor } from "./lib/keys";
  import { hideWindow } from "./lib/shell";

  let status = $state<AppStatus | null>(null);

  $effect(() => {
    appStatus().then((s) => (status = s));
  });

  function onKeydown(event: KeyboardEvent) {
    const action = actionFor(event);
    if (!action) return;
    event.preventDefault();
    if (action.type === "hide") void hideWindow();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="frame">
  <header class="topbar">
    <span class="brand">KOR</span>
    {#if status}<span class="version">v{status.version}</span>{/if}
  </header>

  <main class="stage">
    {#if status?.error}
      <FatalError message={status.error} />
    {:else}
      <p class="hangul">안녕하세요</p>
    {/if}
  </main>

  <KeyHints hints={[{ keys: "Esc", label: "hide" }]} />
</div>

<style>
  .version {
    font-size: 12px;
    color: var(--muted);
  }
</style>
