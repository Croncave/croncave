# Home, first visit (empty) — design reference

The empty-state Home, exported from the design canvas. Treat it as a
**reference mockup, not production code**: the markup and inline styles carry
the design's exact values — colors, font sizes, spacing, radii, shadows,
layout — which an implementation should reproduce in its own components,
rather than copy wholesale.

Unlike the other screens in this folder, this one exports as a Design
Component rather than a standalone page: the artboard is an `<x-dc>` template
with `{{theme}}` substitution, and the sidebar arrives through
`<dc-import name="Main">`. Rendering it needs the runtime here. A faithful
standalone HTML and a 1280×820 PNG, matching the other screens, need either a
re-export from the canvas or a headless render.

## Contents

- `HomeEmpty.dc.html` — the artboard, light by default and dark via its
  `theme` prop. The values to reproduce live in its inline `style="…"`
  attributes and its `<helmet><style>` block.
- `HomeEmptyDark.dc.html` — the same artboard pinned to dark.
- `Main.dc.html` — the sidebar in its empty state, imported by the artboard.
- `support.js`, `vendor/react*.js` — the runtime that renders it in a browser.
  Not part of the design.

Tokens and component CSS are **not** copied here. Both link to
`../../system/`, which is the one copy in this repository.

## Viewing

Serve the repository root and open this folder's `HomeEmpty.dc.html`; some
browsers block the scripts over `file://`.

```
python3 -m http.server
```
