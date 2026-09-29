<!--
  Light, dark or follow the system.

  Built on the design system's `cc-seg`, with one addition: a thumb that
  slides to the chosen option rather than the background jumping between
  buttons. The component's own pressed style is turned off so the thumb is
  the only thing marking the choice.

  Not on the canvas yet. It sits where the design says page-level buttons go —
  the top right of the content area — beside where the notifications bell
  will be.
-->
<script lang="ts">
  import { chooseTheme, storedChoice } from '$lib/theme/apply';
  import { Monitor, Moon, Sun } from '@lucide/svelte';
  import type { ThemeChoice } from '$lib/theme/theme';

  const OPTIONS = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'Match the system', icon: Monitor }
  ] as const satisfies readonly { value: ThemeChoice; label: string; icon: unknown }[];

  // The server cannot know which theme this browser is showing, so this stays
  // at the default until the client says otherwise. The inline boot script has
  // already painted the right theme by then.
  let choice = $state<ThemeChoice>('system');
  let ready = $state(false);

  $effect(() => {
    choice = storedChoice();
    ready = true;
  });

  const index = $derived(OPTIONS.findIndex((option) => option.value === choice));

  function pick(next: ThemeChoice) {
    choice = next;
    chooseTheme(next);
  }
</script>

<div class="cc-seg switcher" role="group" aria-label="Theme">
  <span class="thumb" style="--index: {index}" class:ready aria-hidden="true"></span>

  {#each OPTIONS as option (option.value)}
    <button
      type="button"
      aria-label={option.label}
      aria-pressed={choice === option.value}
      title={option.label}
      onclick={() => pick(option.value)}
    >
      <option.icon size={14} aria-hidden="true" />
    </button>
  {/each}
</div>

<style>
  .switcher {
    position: relative;
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    height: 28px;
    /*
     * cc-seg is 2px padding with 2px between buttons, so three equal buttons
     * are each (track - 4px of padding - 4px of gaps) / 3. A percentage in
     * translateX is of the thumb's own width, so one step is its width plus
     * one gap.
     */
    width: calc((100% - 8px) / 3);
    border-radius: var(--radius-md);
    background: var(--surface);
    box-shadow: 0 0 0 1px var(--border-strong);
    transform: translateX(calc(var(--index) * (100% + 2px)));
  }

  /* Only animate once the client has read the stored choice, so the thumb
     does not slide in from "light" on every page load. */
  .thumb.ready {
    transition: transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  @media (prefers-reduced-motion: reduce) {
    .thumb.ready {
      transition: none;
    }
  }

  .switcher button {
    position: relative;
    z-index: 1;
    flex: 1;
    justify-content: center;
    padding: 0 var(--space-3);
  }

  /* The thumb marks the choice now, so the component's own pressed style
     would draw a second marker on top of it. */
  .switcher button[aria-pressed='true'] {
    background: transparent;
    box-shadow: none;
    color: var(--ink);
  }
</style>
