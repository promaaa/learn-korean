<script lang="ts">
  import type { Profile } from "../lib/api";

  let { profile }: { profile: Profile } = $props();

  const fraction = $derived(
    Math.min(1, profile.level.xpInLevel / Math.max(1, profile.level.xpForNext)),
  );
</script>

<div class="xp" title="{profile.level.xpInLevel} / {profile.level.xpForNext} XP to level {profile.level.level + 1}">
  {#if profile.dayStreak > 0}
    <span class="days">{profile.dayStreak}-day streak</span>
  {/if}
  <span class="level">LV {profile.level.level}</span>
  <span class="bar"><span class="fill" style:width="{fraction * 100}%"></span></span>
  <span class="total">{profile.level.totalXp.toLocaleString("en-US")} XP</span>
</div>

<style>
  .xp {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .days {
    color: var(--warn);
    margin-right: 6px;
  }

  .level {
    font-weight: 700;
    color: var(--fg);
    letter-spacing: 0.06em;
  }

  .bar {
    width: 96px;
    height: 7px;
    border-radius: 4px;
    background: var(--line);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--good));
    transition: width 360ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .total {
    color: var(--fg-dim);
    min-width: 7ch;
    text-align: right;
  }
</style>
