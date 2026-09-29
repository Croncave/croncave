<!--
  The app's left column, from the designed sidebar.

  Much of that design describes things that do not exist yet — the command
  palette, Templates, Activity, Tasks, and the spend card — and the design
  system's own rule is never to show a number the product cannot know. So
  this is the part of the sidebar that is real in step 1: what you own, and
  how to make another.
-->
<script lang="ts">
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
  import type { Me } from '$lib/server/session';
  import { House, Plus } from '@lucide/svelte';

  type Workspace = { id: string; name: string; state: string };

  let { me, workspaces }: { me: Me; workspaces: Workspace[] } = $props();

  /** The design pairs every status colour with a word. */
  const WORDS: Record<string, string> = {
    asleep: 'asleep',
    waking: 'waking',
    awake: 'awake',
    stopping: 'going to sleep'
  };
</script>

<nav aria-label="Main">
  <div class="brand">
    <span class="mark" aria-hidden="true"></span>
    <span class="wordmark">Croncave</span>
  </div>

  <a class="cc-btn cc-btn--primary new" href={resolve('/(app)/workspaces/new')}>
    <Plus size={16} aria-hidden="true" />
    <span>New workspace</span>
  </a>

  <div class="nav">
    <a
      class="cc-nav-item"
      href={resolve('/(app)')}
      aria-current={page.url.pathname === '/' ? 'page' : undefined}
    >
      <House size={16} aria-hidden="true" />
      Home
    </a>
  </div>

  <div class="section">
    <div class="section-head">
      <span class="cc-label">Workspaces</span>
      <a
        class="cc-icon-btn cc-icon-btn--ghost"
        href={resolve('/(app)/workspaces/new')}
        aria-label="New workspace"
      >
        <Plus size={14} aria-hidden="true" />
      </a>
    </div>

    {#if workspaces.length === 0}
      <p class="cc-help empty">Nothing here yet.</p>
    {:else}
      {#each workspaces as workspace (workspace.id)}
        <span
          class="cc-nav-item"
          title="{workspace.name}: {WORDS[workspace.state] ?? workspace.state}"
        >
          <span class="cc-dot cc-dot--{workspace.state === 'awake' ? 'live' : 'asleep'}"></span>
          <span class="name">{workspace.name}</span>
        </span>
      {/each}
    {/if}
  </div>

  <div class="spacer"></div>

  <div class="account">
    <span class="avatar" aria-hidden="true">{me.user.email.slice(0, 1).toUpperCase()}</span>
    <div class="who">
      <span class="email">{me.user.email}</span>
      <span class="plan">Personal</span>
    </div>
    <form method="POST" action="/sign-out">
      <button class="cc-btn cc-btn--ghost" type="submit">Sign out</button>
    </form>
  </div>
</nav>

<style>
  nav {
    width: 248px;
    flex: none;
    box-sizing: border-box;
    padding: var(--space-4);
    background: var(--surface-sunken);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
  }

  .mark {
    width: 20px;
    height: 20px;
    border-radius: var(--radius-md);
    background: var(--accent);
  }

  .wordmark {
    font-weight: 600;
  }

  .new {
    justify-content: flex-start;
    gap: var(--space-2);
  }

  .nav,
  .section {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-2) var(--space-1);
  }

  .empty {
    padding: 0 var(--space-2);
    margin: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spacer {
    flex: 1;
  }

  .account {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border);
  }

  .avatar {
    width: 28px;
    height: 28px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: var(--radius-full);
    background: var(--surface-raised);
    border: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
  }

  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .email {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .plan {
    font-size: 11px;
    color: var(--ink-subtle);
  }

  @media (max-width: 900px) {
    nav {
      width: 100%;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }

    .spacer {
      display: none;
    }
  }
</style>
