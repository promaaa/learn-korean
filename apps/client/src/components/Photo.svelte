<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { itemImage, type ItemImage } from "../lib/api";

  let { itemId }: { itemId: string } = $props();

  let credit = $state<ItemImage | null>(null);
  let status = $state<"loading" | "ready" | "missing">("loading");

  $effect(() => {
    const id = itemId;
    status = "loading";
    credit = null;
    itemImage(id)
      .then((found) => {
        if (id !== itemId) return;
        credit = found;
        status = found ? "ready" : "missing";
      })
      .catch((err: unknown) => {
        console.warn(`image unavailable for ${id}: ${String(err)}`);
        if (id === itemId) status = "missing";
      });
  });
</script>

{#if status !== "missing"}
  <figure class="photo">
    <div class="frame" class:loading={status === "loading"}>
      {#if status === "ready"}
        <img src={convertFileSrc(itemId, "kimg")} alt="" onerror={() => (status = "missing")} />
      {/if}
    </div>
    {#if credit}<figcaption>{credit.attribution}</figcaption>{/if}
  </figure>
{/if}

<style>
  .photo {
    margin: 0 0 12px;
    width: 100%;
    max-width: 360px;
  }

  .frame {
    position: relative;
    height: 150px;
    border-radius: 12px;
    overflow: hidden;
    background: var(--panel);
    border: 1px solid var(--line);
  }

  .frame.loading {
    animation: shimmer 1.1s ease-in-out infinite;
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: appear 200ms ease;
  }

  figcaption {
    margin-top: 4px;
    font-size: 10px;
    color: var(--muted);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @keyframes shimmer {
    50% {
      opacity: 0.55;
    }
  }

  @keyframes appear {
    from {
      opacity: 0;
    }
  }
</style>
