<script lang="ts">
  import KeyHints from "./components/KeyHints.svelte";
  import { actionFor } from "./lib/keys";
  import { hideWindow } from "./lib/shell";

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
  </header>

  <main class="stage">
    <p class="hangul">안녕하세요</p>
  </main>

  <KeyHints hints={[{ keys: "Esc", label: "hide" }]} />
</div>
