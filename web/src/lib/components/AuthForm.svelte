<!--
  The form half of sign-in and sign-up. The two screens differ only in words,
  so they share this and pass their own.
-->
<script lang="ts">
  import { enhance } from '$app/forms';
  import { resolve } from '$app/paths';
  import AuthPanel from './AuthPanel.svelte';

  type Props = {
    heading: string;
    blurb: string;
    submit: string;
    switchPrompt: string;
    switchLabel: string;
    switchHref: '/sign-in' | '/sign-up';
    /** Set once a link has been sent, so the form can say so. */
    sent?: string | null;
    /** Something to tell the person, if anything went wrong. */
    problem?: string | null;
  };

  let { heading, blurb, submit, switchPrompt, switchLabel, switchHref, sent, problem }: Props =
    $props();

  let working = $state(false);
</script>

<div class="split">
  <main>
    <div class="column">
      <div class="brand">
        <span class="mark" aria-hidden="true"></span>
        <span class="wordmark">Croncave</span>
      </div>

      <div class="middle">
        <div class="form">
          <div>
            <h1>{heading}</h1>
            <p class="blurb">{blurb}</p>
          </div>

          {#if sent}
            <p class="sent" role="status">
              Check <b>{sent}</b> for your sign-in link. It works once, and expires in 15 minutes.
            </p>
          {:else}
            <form
              method="POST"
              use:enhance={() => {
                working = true;
                return async ({ update }) => {
                  await update();
                  working = false;
                };
              }}
            >
              <label class="cc-field">
                <span class="cc-label">Email</span>
                <input
                  class="cc-input"
                  type="email"
                  name="email"
                  autocomplete="email"
                  placeholder="you@company.com"
                  required
                  aria-describedby={problem ? 'problem' : 'note'}
                  aria-invalid={problem ? 'true' : undefined}
                />
              </label>

              <button class="cc-btn cc-btn--primary" type="submit" disabled={working}>
                {working ? 'Sending…' : submit}
              </button>

              {#if problem}
                <p class="problem" id="problem" role="alert">{problem}</p>
              {:else}
                <p class="cc-help" id="note">
                  We'll email you a sign-in link. No password to remember.
                </p>
              {/if}
            </form>
          {/if}

          <p class="switch">
            {switchPrompt} <a href={resolve(switchHref)}>{switchLabel}</a>
          </p>
        </div>
      </div>

      <!--
      The design links Terms and Privacy Policy. Neither page exists yet, and
      a link to a 404 is worse than none, so they are plain words until the
      pages are written. They must be real links before the alpha opens.
    -->
      <p class="terms">
        By continuing you agree to the Terms and Privacy Policy. Available in the US.
      </p>
    </div>
  </main>

  <AuthPanel />
</div>

<style>
  .split {
    display: flex;
    min-height: 100vh;
    background: var(--bg);
  }

  /*
   * The panel is capped and the form side takes everything else, so a wider
   * screen gives its extra room to the thing people came for. At the canvas's
   * own 1280px this lands on the design's 560/720 split exactly; past that,
   * only the form side grows.
   */
  main {
    flex: 1 1 auto;
    min-width: 0;
    box-sizing: border-box;
    padding: var(--space-8) var(--space-12);
    display: flex;
    flex-direction: column;
  }

  /* Keeps the logo, the form and the terms line as one column rather than
     letting them drift to opposite edges of a wide screen. */
  .column {
    flex: 1;
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 420px;
    margin: 0 auto;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
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

  .middle {
    flex: 1;
    display: flex;
    align-items: center;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    width: 100%;
  }

  h1 {
    margin: 0;
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .blurb {
    margin: var(--space-2) 0 0;
    color: var(--ink-muted);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  button {
    height: 40px;
    justify-content: center;
  }

  .sent {
    margin: 0;
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .problem {
    margin: 0;
    font-size: 13px;
    color: var(--status-failed);
  }

  .switch,
  .terms {
    margin: 0;
    font-size: 13px;
    color: var(--ink-muted);
  }

  .terms {
    font-size: 12px;
  }

  @media (max-width: 900px) {
    main {
      padding: var(--space-6);
    }
  }
</style>
