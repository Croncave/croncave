# ActivityTable

Filters, sortable columns, rows that open their detail page.

The activity table lists what workspaces and tasks did since the user's last visit, most urgent first, with a filter row above it and sortable columns. Each row is a link to that run's detail page, where actions live.

- **Classes:** `cc-table` wraps `cc-table__head` and one `cc-table__row` per event; make each row an `<a href>`. Set the same `grid-template-columns` on head and rows. Column headers that sort are `cc-sort` buttons with `data-active="true"` on the current one and an arrow (↑ or ↓) after the word. Filters above the table are `cc-filter` buttons with `aria-pressed` and a `cc-filter__count`.
- **Columns, in order:** time (`cc-table__num`); name, led by a 16px `ink-subtle` icon that says what it is (Lucide `box` for a workspace, `zap` for a task, each with an accessible label); what happened (one past-tense line, truncated); status (`cc-pill`); a chevron.
- **No status dots, costs or action buttons in rows.** The pill carries the state; cost and actions are on the detail page.
- **Filters:** All, Needs you (Needs you plus Failed, since a failure always needs the user) and Done, each with its count.
- **Default sort** is status (Needs you, Failed, Working, Done), newest first within a status, so anything waiting on the user is on top. Time sorts newest first; Name A to Z.
- Title it "Since your last visit" with the window start in mono beside it ("from 23:40 last night").

Preview: [ActivityTable.html](ActivityTable.html) (open in a browser; shows light and dark).
