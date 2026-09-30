<script lang="ts">
  import FatalError from "./components/FatalError.svelte";
  import KeyHints from "./components/KeyHints.svelte";
  import XpBar from "./components/XpBar.svelte";
  import Hangul from "./routes/Hangul.svelte";
  import Session from "./routes/Session.svelte";
  import Stats from "./routes/Stats.svelte";
  import TypingGym from "./routes/TypingGym.svelte";
  import { appStatus, profile as loadProfile, type AppStatus, type Profile } from "./lib/api";
  import { actionFor } from "./lib/keys";
  import { hideWindow, onShown } from "./lib/shell";

  /** Screens in `Tab` order; `Shift+Tab` goes backwards. */
  const SCREENS = ["review", "typing", "hangul", "stats"] as const;
  type Screen = (typeof SCREENS)[number];

  let status = $state<AppStatus | null>(null);
  let session = $state<Session | null>(null);
  let profile = $state<Profile | null>(null);
  let gym = $state<TypingGym | null>(null);
  let hangul = $state<Hangul | null>(null);
  let stats = $state<Stats | null>(null);
  let mode = $state<Screen>("review");
  /** Another device's progress was merged; the review session restarts when next on screen. */
  let stale = false;

  function resumeReview(): void {
    if (stale) {
      stale = false;
      session?.restart();
    } else {
      session?.resume();
    }
  }

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
    const unlisten = onShown(({ synced }) => {
      if (synced) stale = true;
      if (mode === "review") resumeReview();
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
      mode = SCREENS[(SCREENS.indexOf(mode) + action.step + SCREENS.length) % SCREENS.length] ?? mode;
      // Time spent on other screens is not thinking time for the waiting card.
      if (mode === "review") resumeReview();
    } else if (mode === "typing") {
      gym?.key(event);
    } else if (mode === "hangul") {
      hangul?.key(event);
    } else if (mode === "review" && action) {
      event.preventDefault();
      session?.handle(action);
    }
  }

  function screenHints() {
    switch (mode) {
      case "typing":
        return gym?.hints();
      case "hangul":
        return hangul?.hints();
      case "stats":
        return stats?.hints();
      default:
        return session?.hints();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="frame">
  <header class="topbar">
    <div class="left">
      <span class="brand">KOR</span>
      <nav class="screens" aria-label="screens">
        {#each SCREENS as screen (screen)}
          <span class:current={screen === mode}>{screen}</span>
        {/each}
      </nav>
    </div>
    {#if profile}<XpBar {profile} />{/if}
  </header>

  <main class="stage">
    {#if status?.error}
      <FatalError message={status.error} />
    {:else if status}
      <!-- The session stays mounted so switching keeps it where it was; other screens mount only
           while shown. -->
      <div class="screen" class:hidden={mode !== "review"}>
        <Session bind:this={session} onprofile={(p) => (profile = p)} />
      </div>
      {#if mode === "typing"}
        <div class="screen"><TypingGym bind:this={gym} /></div>
      {/if}
      {#if mode === "hangul"}
        <div class="screen"><Hangul bind:this={hangul} /></div>
      {/if}
      {#if mode === "stats"}
        <div class="screen"><Stats bind:this={stats} /></div>
      {/if}
    {/if}
  </main>

  <KeyHints hints={screenHints() ?? [{ keys: "Esc", label: "hide" }]} />
</div>

<style>
  .screen {
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    place-items: center;
  }

  .hidden {
    display: none;
  }

  .left {
    display: flex;
    align-items: baseline;
    gap: 18px;
  }

  .screens {
    display: flex;
    gap: 12px;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .screens .current {
    color: var(--fg);
    box-shadow: 0 2px 0 var(--accent);
  }
</style>
