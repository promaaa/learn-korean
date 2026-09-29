<script lang="ts">
  import FatalError from "./components/FatalError.svelte";
  import KeyHints from "./components/KeyHints.svelte";
  import XpBar from "./components/XpBar.svelte";
  import Session from "./routes/Session.svelte";
  import { appStatus, profile as loadProfile, type AppStatus, type Profile } from "./lib/api";
  import { actionFor } from "./lib/keys";
  import { hideWindow, onShown } from "./lib/shell";

  let status = $state<AppStatus | null>(null);
  let session = $state<Session | null>(null);
  let profile = $state<Profile | null>(null);

  function refreshProfile(): void {
    loadProfile()
      .then((p) => (profile = p))
      .catch((err: unknown) => console.warn(`profile unavailable: ${String(err)}`));
  }

  $effect(() => {
    appStatus().then((s) => {
      status = s;
      if (!s.error) refreshProfile();
    });
    const unlisten = onShown(() => {
      session?.resume();
      refreshProfile();
    });
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
    {#if profile}<XpBar {profile} />{/if}
  </header>

  <main class="stage">
    {#if status?.error}
      <FatalError message={status.error} />
    {:else if status}
      <Session bind:this={session} onprofile={(p) => (profile = p)} />
    {/if}
  </main>

  <KeyHints hints={session?.hints() ?? [{ keys: "Esc", label: "hide" }]} />
</div>
