<script lang="ts">
  import AuthForm from '$lib/components/AuthForm.svelte';

  let { form } = $props();
</script>

<svelte:head>
  <title>Create your account · Croncave</title>
</svelte:head>

<AuthForm
  heading="Create your account"
  blurb="Start on the free plan and upgrade when you need more."
  submit="Create account"
  switchPrompt="Already have an account?"
  switchLabel="Sign in"
  switchHref="/sign-in"
  sent={form?.sent}
  problem={form?.problem}
/>
