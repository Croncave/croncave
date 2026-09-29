<!--
  The always-dark panel beside the sign-in and sign-up forms.

  Everything in the feed is sample data from the design, shown as a picture of
  what Croncave does. It is deliberately static: inventing rows that looked
  like someone's real overnight work would be a lie the product can't back up.
-->
<aside data-theme="dark" class="cc-dotgrid">
  <div class="pitch">
    <span class="cc-label overnight">Overnight</span>
    <p class="headline">Your computer in the cloud, still working after you close the lid.</p>
    <p class="blurb">
      Run coding sessions overnight, schedule jobs and watch pages for changes. Workspaces sleep
      when idle, so you only pay while they're awake.
    </p>
  </div>

  <div class="feed" aria-hidden="true">
    <div class="feed-head">
      <span class="cc-label">While you slept</span>
      <span class="total">$3.63 total</span>
    </div>
    {#each [{ at: '01:02', dot: 'working', name: 'Real Estate App', what: 'session started · 42 tasks', needs: false }, { at: '04:31', dot: 'live', name: 'Stock Watcher', what: 'NVDA crossed $140 · texted you', needs: false }, { at: '06:00', dot: 'live', name: 'Competitor Prices', what: '3 prices changed · sheet updated', needs: false }, { at: '06:12', dot: 'needs', name: 'Real Estate App', what: 'preview ready · needs you', needs: true }, { at: '08:00', dot: 'asleep', name: 'Daily News Digest', what: 'emailed 5 stories · asleep', needs: false }] as row (row.at)}
      <div class="row">
        <span class="at">{row.at}</span>
        <span class="cc-dot cc-dot--{row.dot}"></span>
        <b>{row.name}</b>
        <span class="what" class:needs={row.needs}>{row.what}</span>
      </div>
    {/each}
  </div>
</aside>

<style>
  aside {
    /* Capped, so the form side gets the rest. 720px is what the canvas gives
       it at 1280px wide. */
    flex: 0 1 720px;
    margin: 12px 12px 12px 0;
    box-sizing: border-box;
    padding: var(--space-12);
    overflow: hidden;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    color: var(--ink);
    /* No `background` here: .cc-dotgrid sets background-color and
       background-image together, and the shorthand would wipe the dots. */
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: var(--space-8);
  }

  .pitch {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 460px;
  }

  /* Accent is used rarely and deliberately; this is one of the places the
     design spends it. */
  .overnight {
    color: var(--accent-ink);
  }

  .headline {
    margin: 0;
    font-size: 30px;
    line-height: 38px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .blurb {
    margin: 0;
    color: var(--ink-muted);
  }

  .feed {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--space-4) var(--space-6);
    background: var(--surface);
  }

  .feed-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: var(--space-3);
  }

  .total,
  .at,
  .what {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--ink-muted);
  }

  .row {
    display: grid;
    grid-template-columns: 3.5rem 10px 1fr 1.4fr;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) 0;
    border-top: 1px solid var(--border);
  }

  .row b {
    font-weight: 500;
    font-size: 13px;
  }

  .what.needs {
    color: var(--status-needs);
  }

  /* The panel is decoration beside the form; below the split it would push
     the thing people came for off the screen. */
  @media (max-width: 900px) {
    aside {
      display: none;
    }
  }
</style>
