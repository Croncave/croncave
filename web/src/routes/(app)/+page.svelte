<!--
  Home, before anything has happened.

  The designed first-visit Home also carries a composer and a row of template
  cards. Neither can do anything until steps 4 and 8, and the design system's
  rule is not to put a control on screen that cannot do what it offers, so
  this is the greeting and the one path that works.
-->
<script lang="ts">
  import { resolve } from '$app/paths';
  import { Box } from '@lucide/svelte';

  let { data } = $props();

  const greeting = $derived(data.me.user.email.split('@')[0]);
</script>

<svelte:head>
  <title>Home · Croncave</title>
</svelte:head>

<header>
  <h1>Welcome, {greeting}</h1>
  <p class="lede">
    {#if data.workspaces.length === 0}
      Your computer in the cloud is ready. Make a workspace to put work in.
    {:else}
      Nothing has run yet. Your workspaces are listed on the left.
    {/if}
  </p>
</header>

{#if data.workspaces.length === 0}
  <section class="start">
    <a class="cc-btn cc-btn--primary" href={resolve('/(app)/workspaces/new')}>New workspace</a>
    <p class="cc-help">
      A workspace is a computer of its own for a project. It sleeps when there is nothing to do.
    </p>
  </section>
{:else}
  <section class="list">
    <span class="cc-label">Workspaces</span>
    <ul>
      {#each data.workspaces as workspace (workspace.id)}
        <li>
          <Box size={16} aria-hidden="true" />
          <span class="name">{workspace.name}</span>
          <span class="cc-pill cc-pill--asleep">
            <span class="cc-dot cc-dot--asleep"></span>
            {workspace.state}
          </span>
          <time datetime={workspace.created_at}>
            {new Date(workspace.created_at).toLocaleDateString()}
          </time>
        </li>
      {/each}
    </ul>
    <p class="cc-help">
      They have no computer yet, and nothing to run. Both arrive in later steps.
    </p>
  </section>
{/if}

<style>
  h1 {
    margin: 0;
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .lede {
    margin: var(--space-2) 0 0;
    color: var(--ink-muted);
  }

  .start,
  .list {
    margin-top: var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    align-items: flex-start;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    width: 100%;
    max-width: 640px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  .name {
    flex: 1;
    font-weight: 500;
  }

  time {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--ink-subtle);
  }
</style>
