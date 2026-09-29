<script lang="ts">
  import FatalError from "./components/FatalError.svelte";
  import KeyHints from "./components/KeyHints.svelte";
  import XpBar from "./components/XpBar.svelte";
  import Session from "./routes/Session.svelte";
  import TypingGym from "./routes/TypingGym.svelte";
  import { appStatus, profile as loadProfile, type AppStatus, type Profile } from "./lib/api";
  import { actionFor } from "./lib/keys";
  import { hideWindow, onShown } from "./lib/shell";

  let status = $state<AppStatus | null>(null);
  let session = $state<Session | null>(null);
  let profile = $state<Profile | null>(null);
  let gym = $state<TypingGym | null>(null);
  /** Review session or Typing Gym; `Tab` switches. */
  let mode = $state<"review" | "typing">("review");

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
    if (action?.type === "hide") {
      event.preventDefault();
      void hideWindow();
    } else if (action?.type === "mode") {
      event.preventDefault();
      mode = mode === "review" ? "typing" : "review";
    } else if (mode === "typing") {
      gym?.key(event);
    } else if (action) {
      event.preventDefault();
      session?.handle(action);
    }
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
      <!-- Both stay mounted so switching keeps the session where it was. -->
      <div class="screen" class:hidden={mode !== "review"}>
        <Session bind:this={session} onprofile={(p) => (profile = p)} />
      </div>
      {#if mode === "typing"}
        <div class="screen"><TypingGym bind:this={gym} /></div>
      {/if}
    {/if}
  </main>

  <KeyHints
    hints={(mode === "typing" ? gym?.hints() : session?.hints()) ?? [{ keys: "Esc", label: "hide" }]}
  />
</div>

<style>
  .screen {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }

  .hidden {
    display: none;
  }
</style>
