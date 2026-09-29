<script lang="ts">
  let { message }: { message: string } = $props();
</script>

<section class="fatal">
  <h1>Cannot open your progress database</h1>
  <p class="message">{message}</p>
  <p class="help">
    Your data has not been modified. Backups are kept next to the database in <code>backups/</code>.
  </p>
</section>

<style>
  .fatal {
    max-width: 560px;
    text-align: center;
  }
  h1 {
    font-size: 20px;
    color: var(--bad);
  }
  .message {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--fg-dim);
    word-break: break-word;
  }
  .help {
    color: var(--muted);
    font-size: 13px;
  }
</style>
