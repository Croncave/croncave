/**
 * Reading and changing the theme in a browser.
 *
 * Every function here is safe to call when storage is unavailable — a private
 * window, or a browser with site data blocked — because a display preference
 * must never be the reason a page fails to render.
 */

import { BOOT_SCRIPT, STORAGE_KEY, isThemeChoice, resolveTheme } from './theme';
import type { Theme, ThemeChoice } from './theme';

export { BOOT_SCRIPT };

/** What this person has chosen, or "system" if they haven't. */
export function storedChoice(): ThemeChoice {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return isThemeChoice(value) ? value : 'system';
  } catch {
    return 'system';
  }
}

/** Whether the operating system asks for dark. */
export function systemPrefersDark(): boolean {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  } catch {
    return false;
  }
}

/** The theme showing right now. */
export function currentTheme(): Theme {
  return resolveTheme(storedChoice(), systemPrefersDark());
}

/** Choose a theme, remember it, and show it at once. */
export function chooseTheme(choice: ThemeChoice): Theme {
  try {
    if (choice === 'system') {
      localStorage.removeItem(STORAGE_KEY);
    } else {
      localStorage.setItem(STORAGE_KEY, choice);
    }
  } catch {
    // Remembering is a convenience; showing it is not.
  }

  const theme = resolveTheme(choice, systemPrefersDark());
  document.documentElement.setAttribute('data-theme', theme);
  return theme;
}
