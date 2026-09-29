import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { DEFAULT_THEME, isThemeChoice, resolveTheme, THEMES } from './theme';

describe('resolveTheme', () => {
  it('shows what the person chose', () => {
    expect(resolveTheme('dark', false)).toBe('dark');
    expect(resolveTheme('light', true)).toBe('light');
  });

  it('follows the system when nothing is chosen', () => {
    expect(resolveTheme('system', true)).toBe('dark');
    expect(resolveTheme('system', false)).toBe('light');
  });

  it('falls back rather than throwing on a value it does not know', () => {
    // A stale value in someone's browser must not stop the page rendering.
    expect(resolveTheme('sepia', false)).toBe(DEFAULT_THEME);
    expect(resolveTheme(null, false)).toBe(DEFAULT_THEME);
    expect(resolveTheme(undefined, true)).toBe('dark');
  });

  it('defaults to light, as the design system says', () => {
    expect(DEFAULT_THEME).toBe('light');
  });
});

describe('isThemeChoice', () => {
  it('accepts only what we can show', () => {
    expect(isThemeChoice('light')).toBe(true);
    expect(isThemeChoice('dark')).toBe(true);
    expect(isThemeChoice('system')).toBe(true);
    expect(isThemeChoice('sepia')).toBe(false);
    expect(isThemeChoice(null)).toBe(false);
  });
});

/**
 * The design system calls dark "a full equal". These read the snapshot
 * itself, so a re-export that drops a dark value fails here rather than
 * showing someone an unstyled corner of the app.
 */
describe('the design tokens', () => {
  const css = readFileSync(
    new URL('../../../../docs/design/system/tokens.css', import.meta.url),
    'utf8'
  );

  /** The custom properties declared inside one selector's block. */
  function tokensIn(selector: string): string[] {
    const start = css.indexOf(selector);
    expect(start, `${selector} should exist in tokens.css`).toBeGreaterThanOrEqual(0);
    const block = css.slice(css.indexOf('{', start) + 1, css.indexOf('}', start));
    return [...block.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]).sort();
  }

  it('defines every light token in dark too', () => {
    const light = tokensIn(':root, [data-theme="light"]');
    const dark = tokensIn('[data-theme="dark"]');

    expect(light.length).toBeGreaterThan(10);
    expect(dark).toEqual(light);
  });

  it('carries the theme-independent tokens once', () => {
    const shared = css.slice(css.lastIndexOf(':root {'));

    for (const token of ['--font-sans', '--font-mono', '--radius-lg', '--space-4']) {
      expect(shared, `${token} should be declared once, outside both themes`).toContain(token);
    }
  });

  it('knows exactly two themes', () => {
    expect([...THEMES]).toEqual(['light', 'dark']);
  });
});
