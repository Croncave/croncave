/**
 * Light and dark, which the design system calls "a full equal."
 *
 * The chosen theme is stamped on the root element as `data-theme`, which is
 * what every token in `docs/design/system/tokens.css` keys off. A person can
 * pick one; until they do, the operating system decides.
 */

/** A theme someone can be shown. */
export const THEMES = ['light', 'dark'] as const;

export type Theme = (typeof THEMES)[number];

/** What someone can choose: a theme, or "whatever the system says". */
export type ThemeChoice = Theme | 'system';

/** Where the choice is remembered between visits. */
export const STORAGE_KEY = 'croncave-theme';

/** The design system's default. */
export const DEFAULT_THEME: Theme = 'light';

/** Whether a stored value is one we understand. */
export function isThemeChoice(value: unknown): value is ThemeChoice {
  return value === 'system' || THEMES.includes(value as Theme);
}

/**
 * Work out which theme to show.
 *
 * An unrecognised stored value falls back to the system rather than throwing:
 * this is a display preference, and a stale value in someone's browser must
 * not stop the page rendering.
 */
export function resolveTheme(choice: unknown, prefersDark: boolean): Theme {
  if (choice === 'light' || choice === 'dark') {
    return choice;
  }

  return prefersDark ? 'dark' : DEFAULT_THEME;
}

/** The script that runs before the first paint, so no one sees a flash of the
 *  wrong theme. Kept as a string because it has to be inline in the document
 *  head, ahead of everything else. */
export const BOOT_SCRIPT = `(function(){try{
var c=localStorage.getItem(${JSON.stringify(STORAGE_KEY)});
var d=window.matchMedia('(prefers-color-scheme: dark)').matches;
var t=(c==='light'||c==='dark')?c:(d?'dark':${JSON.stringify(DEFAULT_THEME)});
document.documentElement.setAttribute('data-theme',t);
}catch(e){document.documentElement.setAttribute('data-theme',${JSON.stringify(DEFAULT_THEME)});}})();`;
