<script lang="ts">
  import { onMount } from "svelte";
  import { stats as loadStats, type Skill, type Stats } from "../lib/api";
  import { onShown } from "../lib/shell";

  type Hint = { keys: string; label: string };

  const DAY_MS = 86_400_000;
  const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const SKILL_NAMES: Record<Skill, string> = { listening: "listening", response: "reply", build: "build" };

  let data = $state<Stats | null>(null);
  let error = $state<string | null>(null);

  function load(): void {
    loadStats()
      .then((s) => {
        data = s;
        error = null;
      })
      .catch((err: unknown) => (error = String(err)));
  }

  // Mounted only while on screen: every visit reloads, and so does every summon of the window.
  onMount(() => {
    load();
    const unlisten = onShown(load);
    return () => void unlisten.then((stop) => stop());
  });

  export function hints(): Hint[] {
    return [
      { keys: "Tab", label: "review" },
      { keys: "Esc", label: "hide" },
    ];
  }

  /** "Sep 3" for a local day number (days since the epoch in local time). */
  function dayLabel(day: number): string {
    const date = new Date(day * DAY_MS);
    return `${MONTHS[date.getUTCMonth()]} ${date.getUTCDate()}`;
  }

  function weekday(day: number): string {
    // The epoch, day 0, was a Thursday.
    return WEEKDAYS[(((day + 3) % 7) + 7) % 7] ?? "";
  }

  function duration(ms: number): string {
    const minutes = Math.round(ms / 60_000);
    if (minutes < 60) return `${minutes} min`;
    return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
  }

  function percent(rate: number | null): string {
    return rate === null ? "—" : `${Math.round(rate * 100)}%`;
  }

  const count = (n: number) => n.toLocaleString("en-US");

  const heatmap = $derived.by(() => {
    if (!data) return null;
    const { firstDay, firstWeekday, days } = data.heatmap;
    const max = Math.max(1, ...days.map((d) => d.answers));
    const cells = days.map((d, i) => ({
      ...d,
      day: firstDay + i,
      row: ((firstWeekday + i) % 7) + 1,
      column: Math.floor((firstWeekday + i) / 7) + 1,
      level: d.answers === 0 ? 0 : Math.min(4, Math.ceil((d.answers / max) * 4)),
    }));
    // A month is labelled above the week holding its 1st; the first week gets its own month when
    // there is room before the next label.
    const months = cells
      .filter((c) => new Date(c.day * DAY_MS).getUTCDate() === 1)
      .map((c) => ({ column: c.column, label: MONTHS[new Date(c.day * DAY_MS).getUTCMonth()] }));
    if (cells.length > 0 && (months[0]?.column ?? Infinity) > 3) {
      months.unshift({ column: 1, label: MONTHS[new Date(firstDay * DAY_MS).getUTCMonth()] });
    }
    return { cells, months, columns: Math.ceil((firstWeekday + days.length) / 7) };
  });

  const accuracy = $derived(data && data.totals.answers > 0 ? data.totals.correct / data.totals.answers : null);

  const words = $derived.by(() => {
    if (!data) return null;
    const w = data.words;
    const share = (n: number, of: number) => (of > 0 ? (n / of) * 100 : 0);
    return {
      ...w,
      unseen: Math.max(0, w.total - w.known - w.learning),
      knownWidth: share(w.known, w.total),
      learningWidth: share(w.learning, w.total),
      readyWidth: share(w.sentencesReady, w.sentences),
    };
  });

  const forecast = $derived.by(() => {
    if (!data) return null;
    const today = data.today;
    const max = Math.max(1, ...data.forecast);
    return {
      total: data.forecast.reduce((a, n) => a + n, 0),
      bars: data.forecast.map((cards, i) => ({
        cards,
        height: (cards / max) * 100,
        label: weekday(today + i).slice(0, 2),
        title: `${i === 0 ? "today (with overdue)" : `${weekday(today + i)} ${dayLabel(today + i)}`}: ${cards} due`,
      })),
    };
  });

  const time = $derived.by(() => {
    if (!data) return null;
    const firstDay = data.today - (data.timePerDay.length - 1);
    const max = Math.max(60_000, ...data.timePerDay);
    const total = data.timePerDay.reduce((a, ms) => a + ms, 0);
    return {
      firstDay,
      total,
      average: total / Math.max(1, data.timePerDay.length),
      bars: data.timePerDay.map((ms, i) => ({
        height: (ms / max) * 100,
        title: `${dayLabel(firstDay + i)}: ${duration(ms)}`,
      })),
    };
  });
</script>

<div class="stats">
  {#if error}
    <p class="error">{error}</p>
  {:else if data && heatmap && words && forecast && time}
    <section class="panel activity">
      <div class="heat">
        <header><h2>Activity</h2><span>26 weeks</span></header>
        <div class="calendar">
          <div class="months" style:grid-template-columns="repeat({heatmap.columns}, var(--cell))">
            {#each heatmap.months as month (month.column)}
              <span style:grid-column={month.column}>{month.label}</span>
            {/each}
          </div>
          <div class="weekdays">
            <span style:grid-row="1">Mon</span>
            <span style:grid-row="3">Wed</span>
            <span style:grid-row="5">Fri</span>
          </div>
          <div class="cells" style:grid-template-columns="repeat({heatmap.columns}, var(--cell))">
            {#each heatmap.cells as cell (cell.day)}
              <span
                class="cell level-{cell.level}"
                class:today={cell.day === data.today}
                style:grid-row={cell.row}
                style:grid-column={cell.column}
                title="{weekday(cell.day)} {dayLabel(cell.day)}: {cell.answers} answers, {cell.correct} correct"
              ></span>
            {/each}
          </div>
        </div>
        <div class="legend">
          less
          {#each [0, 1, 2, 3, 4] as level (level)}<span class="cell level-{level}"></span>{/each}
          more
        </div>
      </div>

      <dl class="totals">
        <div><dt>answers</dt><dd>{count(data.totals.answers)}</dd></div>
        <div><dt>accuracy</dt><dd>{percent(accuracy)}</dd></div>
        <div><dt>time</dt><dd>{duration(data.totals.timeMs)}</dd></div>
        <div><dt>practice days</dt><dd>{count(data.totals.practiceDays)}</dd></div>
        <div>
          <dt>streak</dt>
          <dd class:hot={data.totals.dayStreak > 0}>
            {data.totals.dayStreak} <small>{data.totals.dayStreak === 1 ? "day" : "days"}</small>
          </dd>
        </div>
      </dl>
    </section>

    <section class="panel">
      <header><h2>Retention</h2><span>after a day or more</span></header>
      <div class="retention">
        <span></span><span class="column">30 days</span><span class="column">all time</span>
        {#each data.retention as row (row.skill)}
          <span class="skill">{SKILL_NAMES[row.skill]}</span>
          {#each [row.recent, row.allTime] as r, index (index)}
            <div class="rate" title="{r.correct} of {r.reviews} reviews remembered">
              <strong class:empty={r.rate === null}>{percent(r.rate)}</strong>
              <span class="track"><span class="fill" style:width="{(r.rate ?? 0) * 100}%"></span></span>
              <small>{count(r.reviews)}</small>
            </div>
          {/each}
        {/each}
      </div>
    </section>

    <section class="panel">
      <header><h2>Words</h2><span>{count(words.total)} in all packs</span></header>
      <p class="figure"><strong>{count(words.known)}</strong> known</p>
      <span class="track stacked">
        <span class="fill" style:width="{words.knownWidth}%"></span>
        <span class="fill learning" style:width="{words.learningWidth}%"></span>
      </span>
      <p class="key">
        <span><i class="dot"></i>known {count(words.known)}</span>
        <span><i class="dot learning"></i>learning {count(words.learning)}</span>
        <span><i class="dot unseen"></i>unseen {count(words.unseen)}</span>
      </p>
      <p class="figure small">
        <strong>{count(words.sentencesReady)}</strong> / {count(words.sentences)} sentences ready
      </p>
      <span class="track"><span class="fill ready" style:width="{words.readyWidth}%"></span></span>
    </section>

    <section class="panel">
      <header><h2>Due</h2><span>{count(forecast.total)} cards in 14 days</span></header>
      <div class="chart">
        {#each forecast.bars as bar, index (index)}
          <div class="column" title={bar.title}>
            <div class="plot">
              <span class="bar" class:today={index === 0} style:height="{bar.height}%"></span>
              <span class="value" style:bottom="{bar.height}%">{bar.cards > 0 ? bar.cards : ""}</span>
            </div>
            <span class="label" class:today={index === 0}>{bar.label}</span>
          </div>
        {/each}
      </div>
    </section>

    <section class="panel">
      <header>
        <h2>Time</h2>
        <span>{duration(time.total)} in 30 days · {duration(time.average)}/day</span>
      </header>
      <div class="chart dense">
        {#each time.bars as bar, index (index)}
          <div class="column" title={bar.title}>
            <div class="plot"><span class="bar" style:height="{bar.height}%"></span></div>
          </div>
        {/each}
      </div>
      <div class="axis"><span>{dayLabel(time.firstDay)}</span><span>today</span></div>
    </section>
  {/if}
</div>

<style>
  .stats {
    --cell: 13px;
    --gap: 3px;
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: auto 1fr 1fr;
    gap: 10px;
    padding: 12px 0 10px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .error {
    grid-column: 1 / -1;
    align-self: center;
    justify-self: center;
    color: var(--bad);
  }

  .panel {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel);
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
    color: var(--muted);
  }

  h2 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--fg-dim);
  }

  /* Activity: heatmap and totals side by side. */
  .activity {
    grid-column: 1 / -1;
    flex-direction: row;
    justify-content: space-between;
    gap: 24px;
  }

  .calendar {
    display: grid;
    grid-template-columns: auto auto;
    grid-template-areas:
      ". months"
      "weekdays cells";
    column-gap: 6px;
    row-gap: 4px;
  }

  .months,
  .cells {
    display: grid;
    gap: var(--gap);
  }

  .months {
    grid-area: months;
    height: 12px;
    font-size: 10px;
    color: var(--muted);
  }

  .months span {
    grid-row: 1;
    white-space: nowrap;
  }

  .weekdays {
    grid-area: weekdays;
    display: grid;
    grid-template-rows: repeat(7, var(--cell));
    gap: var(--gap);
    font-size: 9px;
    line-height: var(--cell);
    color: var(--muted);
  }

  .cells {
    grid-area: cells;
    grid-template-rows: repeat(7, var(--cell));
  }

  .cell {
    display: inline-block;
    width: var(--cell);
    height: var(--cell);
    border-radius: 3px;
    background: var(--accent);
  }

  .cell.level-0 {
    background: var(--line);
  }

  .cell.level-1 {
    opacity: 0.35;
  }

  .cell.level-2 {
    opacity: 0.5;
  }

  .cell.level-3 {
    opacity: 0.75;
  }

  .cell.today {
    outline: 1px solid var(--fg-dim);
    outline-offset: 1px;
  }

  .legend {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 3px;
    margin-top: 8px;
    font-size: 10px;
    color: var(--muted);
  }

  .legend .cell {
    width: 10px;
    height: 10px;
  }

  .totals {
    margin: 0;
    min-width: 170px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .totals div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    padding: 3px 0;
    border-bottom: 1px solid var(--line);
  }

  .totals div:last-child {
    border-bottom: none;
  }

  dt {
    color: var(--muted);
  }

  dd {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--fg);
  }

  dd small {
    font-size: 11px;
    font-weight: 400;
    color: var(--fg-dim);
  }

  dd.hot {
    color: var(--warn);
  }

  /* Bars shared by retention and words. */
  .track {
    position: relative;
    display: flex;
    height: 6px;
    border-radius: 3px;
    background: var(--line);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 360ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .retention {
    flex: 1;
    display: grid;
    grid-template-columns: auto 1fr 1fr;
    align-items: center;
    column-gap: 16px;
    row-gap: 8px;
  }

  .retention .column {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .skill {
    color: var(--fg-dim);
  }

  .rate {
    display: grid;
    grid-template-columns: 38px 1fr auto;
    align-items: center;
    gap: 8px;
  }

  .rate strong {
    font-size: 15px;
    color: var(--fg);
  }

  .rate strong.empty {
    color: var(--muted);
  }

  .rate small {
    min-width: 3ch;
    text-align: right;
    color: var(--muted);
  }

  .figure {
    margin: 0 0 8px;
    color: var(--fg-dim);
  }

  .figure strong {
    font-size: 22px;
    color: var(--fg);
  }

  .figure.small {
    margin-top: auto;
  }

  .figure.small strong {
    font-size: 15px;
  }

  .stacked {
    height: 8px;
  }

  .fill.learning,
  .dot.learning {
    background: var(--warn);
  }

  .fill.ready {
    background: var(--good);
  }

  .key {
    display: flex;
    gap: 14px;
    margin: 8px 0 10px;
    color: var(--muted);
  }

  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    margin-right: 5px;
    border-radius: 50%;
    background: var(--accent);
  }

  .dot.unseen {
    background: var(--line);
  }

  /* Bar charts. */
  .chart {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    gap: 4px;
  }

  .chart.dense {
    gap: 2px;
  }

  .chart .column {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }

  /* The bar grows from the bottom; the margin leaves room for the value above the tallest bar. */
  .plot {
    position: relative;
    flex: 1;
    margin-top: 14px;
    border-bottom: 1px solid var(--line);
  }

  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: 2px 2px 0 0;
    background: var(--accent);
    opacity: 0.55;
    transition: height 360ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .chart.dense .bar {
    opacity: 0.8;
  }

  .bar.today {
    opacity: 1;
  }

  .value {
    position: absolute;
    left: 0;
    right: 0;
    margin-bottom: 2px;
    text-align: center;
    font-size: 10px;
    color: var(--fg-dim);
  }

  .label.today {
    color: var(--accent);
    font-weight: 600;
  }

  .label {
    margin-top: 4px;
    text-align: center;
    font-size: 9px;
    color: var(--muted);
    white-space: nowrap;
  }

  .axis {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    font-size: 9px;
    color: var(--muted);
  }
</style>
