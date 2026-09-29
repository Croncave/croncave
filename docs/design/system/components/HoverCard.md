# HoverCard

Quick stats for a row: numbers, a 14-day cost chart, what happens next.

The hover card gives a quick look at one activity row without leaving Home: key numbers, a 14-day chart and what happens next. Most useful for Done rows.

- **Opens:** hovering a row for about 400ms, or hovering or focusing the row's `cc-icon-btn--ghost` chart icon (at once). Closes when the pointer leaves the table and card. It sits right-aligned beside the icon column, below the row for the top rows and above it for the bottom rows, so it never leaves the screen. The hovered row gets a `bg` tint.
- **Classes:** `cc-peek` (on `surface-raised` with `shadow-popover`, the one place cards get a shadow). Inside: header (kind icon, name in `heading`, `cc-pill`), a 3-column grid of `cc-stat` (`cc-label` + `cc-stat__value`), the chart, and a footer line plus a "Details →" link.
- **Stats fit the kind:** a finished task shows Took / Cost / Runs (14 / 14); a workspace shows Awake / Cost / its own count (files, rows, step); a watcher shows Checks / Alerts / Cost.
- **Chart:** `cc-spark` with 14 `cc-spark__bar`s, oldest to newest: cost per run (or per day for watchers). Bars are neutral; the latest is `cc-spark__bar--latest` (accent-ink), failed runs `cc-spark__bar--failed` with half height. One series, so no legend: the caption above names it ("Cost per run, last 14 days") with the max in mono, and "14d ago" / "today" below. Each bar has a title with its date and amount, and the chart an `aria-label` summary.

Preview: [HoverCard.html](HoverCard.html) (open in a browser; shows light and dark).
