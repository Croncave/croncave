<!--
  One workspace.

  The designed screen also carries tabs, the last session, a spend card and
  the tasks in this workspace. All of those describe things that arrive in
  steps 3 to 7, so what is here is the header and the Computer card — the
  parts that are real now.
-->
<script lang="ts">
  import { enhance } from '$app/forms';
  import { resolve } from '$app/paths';
  import { Box } from '@lucide/svelte';

  let { data, form } = $props();

  let working = $state(false);
  let running = $state(false);

  const lifecycle = $derived(data.workspace.state);
  const awake = $derived(lifecycle === 'awake');

  /** The design pairs every status colour with a word, never colour alone. */
  const DOT: Record<string, string> = {
    awake: 'live',
    waking: 'working',
    stopping: 'working',
    asleep: 'asleep'
  };
</script>

<svelte:head>
  <title>{data.workspace.name} · Croncave</title>
</svelte:head>

<nav class="crumbs" aria-label="Breadcrumb">
  <a href={resolve('/(app)')}>Workspaces</a>
  <span aria-hidden="true">/</span>
  <span aria-current="page">{data.workspace.name}</span>
</nav>

<header>
  <div class="title">
    <Box size={20} aria-hidden="true" />
    <h1>{data.workspace.name}</h1>
    <span class="cc-pill cc-pill--{DOT[lifecycle] ?? 'asleep'}">
      <span class="cc-dot cc-dot--{DOT[lifecycle] ?? 'asleep'}"></span>
      {lifecycle}
    </span>
  </div>
</header>

<div class="columns">
  <section class="card">
    <span class="cc-label">Computer</span>
    <dl>
      <dt>State</dt>
      <dd>{lifecycle}</dd>
      <dt>Connected</dt>
      <dd>
        {#if data.workspace.connected}
          <span class="cc-dot cc-dot--live"></span> yes
        {:else}
          <span class="cc-dot cc-dot--asleep"></span> no
        {/if}
      </dd>
      <dt>Runs on</dt>
      <dd>a computer of its own</dd>
    </dl>

    <p class="cc-help">
      The workspace opens the connection itself. Nothing reaches in: it has no address and nothing
      inside it listens.
    </p>

    <form
      method="POST"
      action={awake ? '?/stop' : '?/start'}
      use:enhance={() => {
        working = true;
        return async ({ update }) => {
          await update();
          working = false;
        };
      }}
    >
      <button class="cc-btn cc-btn--primary" type="submit" disabled={working}>
        {#if working}
          {awake ? 'Putting it to sleep…' : 'Waking it…'}
        {:else}
          {awake ? 'Put it to sleep' : 'Wake it'}
        {/if}
      </button>
    </form>

    {#if form?.problem}
      <p class="problem" role="alert">{form.problem}</p>
    {/if}
  </section>

  {#if data.workspace.connected}
    <!--
      A stand-in so the connection can be tried by hand. Sessions and
      scheduled runs replace it in steps 4 and 6, with records behind them.
    -->
    <section class="card run">
      <span class="cc-label">Run a command</span>

      <form
        method="POST"
        action="?/run"
        use:enhance={() => {
          running = true;
          return async ({ update }) => {
            await update({ reset: false });
            running = false;
          };
        }}
      >
        <input
          class="cc-input cc-input--mono"
          name="command"
          value={form?.command ?? 'echo hello'}
          placeholder="echo hello"
          autocomplete="off"
          spellcheck="false"
        />
        <button class="cc-btn cc-btn--secondary" type="submit" disabled={running}>
          {running ? 'Running…' : 'Run'}
        </button>
      </form>

      {#if form?.ran}
        <div class="result">
          <span class="cc-pill cc-pill--{form.ran.succeeded ? 'live' : 'failed'}">
            <span class="cc-dot cc-dot--{form.ran.succeeded ? 'live' : 'failed'}"></span>
            {form.ran.outcome}
          </span>
          {#if form.ran.output.trim()}
            <pre>{form.ran.output}</pre>
          {:else}
            <p class="cc-help">It printed nothing.</p>
          {/if}
        </div>
      {/if}
    </section>
  {/if}

  <section class="card">
    <span class="cc-label">Made</span>
    <p class="made">
      <time datetime={data.workspace.created_at}>
        {new Date(data.workspace.created_at).toLocaleString()}
      </time>
    </p>
  </section>
</div>

<style>
  .crumbs {
    display: flex;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--ink-subtle);
    margin-bottom: var(--space-3);
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h1 {
    margin: 0;
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .columns {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-6);
    margin-top: var(--space-8);
    align-items: flex-start;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 280px;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-2) var(--space-6);
    margin: 0;
    font-size: 13px;
  }

  dt {
    color: var(--ink-subtle);
  }

  dd {
    margin: 0;
    font-family: var(--font-mono);
  }

  .made,
  .problem {
    margin: 0;
  }

  .run {
    min-width: 420px;
    flex: 1;
  }

  .run form {
    display: flex;
    gap: var(--space-2);
  }

  .run input {
    flex: 1;
    min-width: 0;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  pre {
    margin: 0;
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    border: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 18px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 18rem;
    overflow: auto;
  }

  .made {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--ink-muted);
  }

  .problem {
    font-size: 13px;
    color: var(--status-failed);
  }
</style>
