<!--
  New workspace, by hand.

  The designed form has six numbered sections. Five of them — Claude, the
  computer, secrets, limits and sharing — configure things that do not exist
  until later steps, so they are left out rather than shown folded and dead.
  The "In plain words" panel is kept, saying only what the product can
  actually promise today.
-->
<script lang="ts">
  import { enhance } from '$app/forms';
  import { resolve } from '$app/paths';

  let { form } = $props();

  let name = $state('');
  let working = $state(false);
</script>

<svelte:head>
  <title>New workspace · Croncave</title>
</svelte:head>

<nav class="crumbs" aria-label="Breadcrumb">
  <a href={resolve('/(app)')}>Workspaces</a>
  <span aria-hidden="true">/</span>
  <span aria-current="page">New</span>
</nav>

<h1>New workspace</h1>
<p class="lede">
  A computer of its own for a project. Add tasks and sessions to it once it exists.
</p>

<div class="columns">
  <!-- The button lives in the summary panel, as the design has it; `form`
       attaches it to this form across the DOM without any script. -->
  <form
    id="new-workspace"
    method="POST"
    use:enhance={() => {
      working = true;
      return async ({ update }) => {
        await update();
        working = false;
      };
    }}
  >
    <section class="cc-section">
      <div class="head">
        <span class="step">01</span>
        <h2>Basics</h2>
      </div>

      <label class="cc-field">
        <span class="cc-label">Name</span>
        <input
          class="cc-input"
          name="name"
          bind:value={name}
          placeholder="Real Estate App"
          maxlength="100"
          required
          autocomplete="off"
          aria-describedby={form?.problem ? 'problem' : undefined}
          aria-invalid={form?.problem ? 'true' : undefined}
        />
      </label>

      {#if form?.problem}
        <p class="problem" id="problem" role="alert">{form.problem}</p>
      {/if}
    </section>

    <p class="cc-help later">
      Claude, the computer, secrets, limits and sharing are set up in later steps.
    </p>
  </form>

  <aside class="summary">
    <span class="cc-label">In plain words</span>
    <p>
      Croncave will keep a private workspace called <b>{name.trim() || 'your workspace'}</b>. It has
      nothing to run yet, and stays asleep until it does.
    </p>

    <dl class="cc-fact">
      <dt>Awake</dt>
      <dd>only while working</dd>
      <dt>Who can see it</dt>
      <dd>only you</dd>
    </dl>

    <div class="actions">
      <button class="cc-btn cc-btn--primary" type="submit" form="new-workspace" disabled={working}>
        {working ? 'Creating…' : 'Create workspace'}
      </button>
      <a class="cc-btn cc-btn--ghost" href={resolve('/(app)')}>Cancel</a>
    </div>
  </aside>
</div>

<style>
  .crumbs {
    display: flex;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--ink-subtle);
    margin-bottom: var(--space-3);
  }

  h1 {
    margin: 0;
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .lede {
    margin: var(--space-2) 0 var(--space-8);
    color: var(--ink-muted);
  }

  .columns {
    display: flex;
    gap: var(--space-8);
    align-items: flex-start;
  }

  form {
    flex: 1;
    min-width: 0;
    max-width: 560px;
  }

  .cc-section {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    padding: var(--space-6);
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }

  .step {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--ink-subtle);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }

  .problem {
    margin: var(--space-3) 0 0;
    font-size: 13px;
    color: var(--status-failed);
  }

  .later {
    margin-top: var(--space-4);
  }

  .summary {
    width: 320px;
    flex: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .summary p {
    margin: 0;
    color: var(--ink-muted);
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-2) var(--space-4);
    margin: 0;
    font-size: 13px;
  }

  dt {
    color: var(--ink-subtle);
  }

  dd {
    margin: 0;
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  @media (max-width: 1100px) {
    .columns {
      flex-direction: column;
    }

    .summary {
      width: 100%;
      box-sizing: border-box;
    }
  }
</style>
