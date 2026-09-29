/**
 * What the built app is allowed to contain.
 *
 * Run after `vite build`. These are properties that unit tests can't see,
 * because they only exist once the design system, the fonts and the app are
 * bundled together.
 */

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ASSETS = '.svelte-kit/output/client/_app/immutable/assets';

function read(dir, ext) {
  let files = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) files = files.concat(read(path, ext));
    else if (entry.endsWith(ext)) files.push(path);
  }
  return files;
}

const css = read(ASSETS, '.css')
  .map((f) => readFileSync(f, 'utf8'))
  .join('\n');
const fonts = read(ASSETS, '.woff2');

const checks = [
  [
    'no third-party font request',
    !css.includes('fonts.googleapis.com') && !css.includes('fonts.gstatic.com'),
    "the design system's components.css @imports Google Fonts; vite.config.ts drops it"
  ],
  ['fonts are self-hosted', fonts.length > 0 && css.includes('@font-face'), 'via @fontsource'],
  ['light tokens', css.includes('--surface-sunken'), 'from docs/design/system/tokens.css'],
  [
    'dark tokens',
    css.includes('[data-theme=dark]') || css.includes('[data-theme="dark"]'),
    'dark is a full equal, not an afterthought'
  ],
  ['component styles', css.includes('.cc-btn'), 'from docs/design/system/components.css']
];

let failed = 0;
for (const [name, ok, why] of checks) {
  if (!ok) failed += 1;
  console.log(`${ok ? '  ok  ' : '  FAIL'} ${name}${ok ? '' : ` — ${why}`}`);
}

if (failed > 0) {
  console.error(`\n${failed} check(s) failed on the built assets.`);
  process.exit(1);
}
