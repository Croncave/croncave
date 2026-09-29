<script lang="ts">
  import AuthForm from '$lib/components/AuthForm.svelte';

  let { data, form } = $props();
</script>

<svelte:head>
  <title>Sign in · Croncave</title>
</svelte:head>

<AuthForm
  heading="Sign in to Croncave"
  blurb="Welcome back. Pick up where your workspaces left off."
  submit="Send sign-in link"
  switchPrompt="New to Croncave?"
  switchLabel="Create an account"
  switchHref="/sign-up"
  sent={form?.sent}
  problem={form?.problem ??
    (data.expired ? 'That link has already been used, or it expired. Here is a fresh one.' : null)}
/>
