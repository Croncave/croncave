Croncave is a private computer in the cloud that keeps working after you close your laptop. The interface should feel like a precise, calm instrument: modern, technical and quiet, where the only loud thing on screen is the one item that needs you.

## Content fundamentals

- **Plain words first.** Say "asleep," "awake," "needs you," "hours awake," never "instance," "vCPU" or "container" outside the expert layer.
- **Sentence case everywhere** except mono labels, which are uppercase (`WORKSPACES`, `RUNNING`).
- **Address the user as "you"; Croncave speaks as "we"** only for platform notices ("We'll pause work at your cap").
- **Timelines report facts in past tense, shortest form:** "Ran at 8:00, found 3 new listings, emailed you."
- **Numbers are exact and set in mono:** `$18.40`, `06:04`, `1,204 / 5,000`.
- **No emoji** in the product UI.

## Visual foundations

### Color
- Neutral-first. `bg` behind content, `surface` for cards and the main column, `surface-sunken` for the sidebar, `surface-raised` for menus and hovered items.
- Text is `ink`, secondary text `ink-muted`, placeholders and mono labels `ink-subtle`. All three hold 4.5:1 on every surface in both themes.
- `accent` is ink (near-black) in light mode and lime (`#99d52a`) in dark mode: quiet in light, bright in dark. It is rare on purpose: the primary button, the active nav marker, progress bars and the logo mark. Where lime appears as text in light mode (Live, links), it is the deep lime `accent-ink`. Text on an accent fill is `on-accent`; accent as text is `accent-ink`.
- Status always pairs a color with a word: Live (`status-live`, lime, the accent family), Working (`status-working`, blue), Needs you (`status-needs`, orange), Asleep (`status-asleep`, gray), Failed (`status-failed`, red). Every state carries its word, so nothing depends on telling red from green.
- Light is the default theme; dark is a full equal. Never hard-code a hex in product code; use the tokens.

### Type
- **Inter** (`sans`) for all UI and reading text. **JetBrains Mono** (`mono`) for labels, code, commands, times, costs and IDs.
- Scale: `display` for the home greeting only, `title` for page and workspace names, `heading` for card and section heads, `body` by default, `body-strong` for names and nav labels, `small` for metadata.
- Mono styles: `label` (11px, uppercase, 0.08em tracking) for section labels; `code` for commands and logs; `numeric` wherever digits should line up.
- Load both from Google Fonts: Inter 400/500/600 and JetBrains Mono 400/500.

### Space, shape and depth
- 4px grid: `space-1` through `space-12`. Cards pad `space-4`; sections are `space-6` apart; page gutters `space-8`.
- `radius-md` for buttons, inputs and nav items; `radius-lg` for cards; `radius-sm` for pills; `radius-full` only for status dots.
- **Borders, not shadows.** Cards and panels are outlined in `border`. Only popovers and menus get `shadow-popover`.
- Controls that must be seen (inputs, checkboxes) use `border-strong`.

### States
- Hover: move one surface step up (`surface-sunken` → `surface-raised`), no color shift in text.
- Active nav item: `surface-raised` fill, `ink` text, a 2px `accent` marker on the left edge of the item.
- Focus: a solid 2px `focus` ring offset 2px on every interactive element, visible on every surface in both themes.
- Disabled: 40% opacity, no pointer.

### Motion
- Short and functional: 120ms for hover and press, 180ms for panels. Status dots for Working pulse gently; nothing else animates on its own.

### Layout
- App shell: a 248px sidebar on `surface-sunken`, content on `bg`, a 1px `border` between them.
- **Dot grid.** Every content area outside the sidebar sits on `bg` with a 1px `border`-colored dot every 16px (`cc-dotgrid`). It is quiet texture, never content: cards, tables and inputs sit on it as opaque `surface`. The sidebar stays plain `surface-sunken`.
- Content max width 1200px; tables and timelines may run full width.
- **Sidebar:** logo, the primary "New workspace" button, search, then Home, Templates and Activity. Below them two sections, **Workspaces** and **Tasks**, each showing its two most urgent items and a "Show N more" row. The spend card at the bottom links to Usage.
- **Page header:** page-level actions such as the notifications bell (`cc-icon-btn` with a badge) sit at the top right of the content area, never in the sidebar.
- **Forms** are numbered foldable sections beside an "In plain words" summary panel (see FormSection).

## Iconography

- Use **Lucide** icons (open source): 1.5px stroke, 16px in nav and buttons, 20px in empty states, colored with the text color they sit beside.
- Icons never carry meaning alone: pair them with a label, or give icon-only buttons an accessible name.
- **Kind icons are fixed:** Lucide `box` always means a workspace and `zap` always means a task, in `ink-subtle` before the name in lists and tables.
- No logo exists yet: set "Croncave" in Inter 600 until one is designed.
