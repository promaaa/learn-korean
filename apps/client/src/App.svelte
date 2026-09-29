<script lang="ts">
  import FatalError from "./components/FatalError.svelte";
  import KeyHints from "./components/KeyHints.svelte";
  import Session from "./routes/Session.svelte";
  import { appStatus, type AppStatus } from "./lib/api";
  import { actionFor } from "./lib/keys";
  import { hideWindow, onShown } from "./lib/shell";

  let status = $state<AppStatus | null>(null);
  let session = $state<Session | null>(null);

  $effect(() => {
    appStatus().then((s) => (status = s));
    const unlisten = onShown(() => session?.resume());
    return () => void unlisten.then((stop) => stop());
  });

  function onKeydown(event: KeyboardEvent) {
    const action = actionFor(event);
    if (!action) return;
    event.preventDefault();
    if (action.type === "hide") void hideWindow();
    else session?.handle(action);
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
    {:else if status}
      <Session bind:this={session} />
    {/if}
  </main>

  <KeyHints hints={session?.hints() ?? [{ keys: "Esc", label: "hide" }]} />
</div>

<style>
  .version {
    font-size: 12px;
    color: var(--muted);
  }
</style>
