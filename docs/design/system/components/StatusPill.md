# StatusPill

The five workspace states.

The status pill shows a workspace's or run's state as a colored dot plus a word.

- **Classes:** `cc-pill` plus one of `cc-pill--live`, `cc-pill--working`, `cc-pill--needs`, `cc-pill--asleep`, `cc-pill--failed`.
- **Consumer provides:** the state and its word. Use exactly these words: Live, Working, Needs you, Asleep, Failed. A single run that finished well is Done (`cc-pill--done`, same lime as Live). Extra detail goes beside the pill in `small`, not inside it.
- For tight lists (like the sidebar) a bare `cc-dot` may stand in, but only where the state is also spelled out on hover or on the page it links to.
- Never use a pill as a button.

Preview: [StatusPill.html](StatusPill.html) (open in a browser; shows light and dark).
