<script lang="ts">
  let { data } = $props();
</script>

<svelte:head>
  <title>Croncave</title>
</svelte:head>

<main>
  <h1>Croncave</h1>
  <p>A private computer in the cloud that keeps working after you close your laptop.</p>

  <dl>
    <dt>Service</dt>
    <dd>{data.service}</dd>
    <dt>Version</dt>
    <dd>{data.version}</dd>
    <dt>Environment</dt>
    <dd>{data.environment}</dd>
  </dl>

  <p class="note">Build step 0. Nothing here yet but proof the app runs.</p>
</main>

<style>
  main {
    max-width: 34rem;
    margin: 4rem auto;
    padding: 0 1.5rem;
    font-family: system-ui, sans-serif;
    line-height: 1.5;
  }

  h1 {
    margin-bottom: 0.25rem;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.25rem 1rem;
    margin: 2rem 0;
  }

  dt {
    font-weight: 600;
  }

  dd {
    margin: 0;
    font-family: ui-monospace, monospace;
  }

  .note {
    color: #666;
    font-size: 0.9rem;
  }
</style>
