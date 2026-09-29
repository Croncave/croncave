<script lang="ts">
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';

  let { data, children } = $props();
</script>

<div class="shell">
  <Sidebar me={data.me} workspaces={data.workspaces} />
  <main class="cc-dotgrid">
    <!--
      Page-level controls sit at the top right of the content area, never in
      the sidebar. The notifications bell joins this row when it is built.
      Positioned rather than in the flow, so a page's own heading still starts
      at the top left beside it, as the designed screens show.
    -->
    <div class="page-tools">
      <ThemeSwitcher />
    </div>

    {@render children()}
  </main>
</div>

<style>
  .shell {
    display: flex;
    min-height: 100vh;
  }

  main {
    position: relative;
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    padding: 40px 48px 32px;
    color: var(--ink);
  }

  .page-tools {
    position: absolute;
    top: 40px;
    right: 48px;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  @media (max-width: 900px) {
    .shell {
      flex-direction: column;
    }

    main {
      padding: var(--space-6);
    }

    /* No room to float it beside a heading at this width. */
    .page-tools {
      position: static;
      justify-content: flex-end;
      margin-bottom: var(--space-4);
    }
  }
</style>
