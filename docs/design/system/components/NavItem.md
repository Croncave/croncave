# NavItem

Default, current, with count, workspace.

The nav item is one link in the sidebar: an icon or status dot, a label, and an optional count.

- **Classes:** `cc-nav-item`, with `aria-current="page"` on the current page; optional `cc-nav-item__count` for a mono count on the right. Group items under a `cc-label` heading.
- **Consumer provides:** an `<a href>`, a 16px Lucide icon (or a `cc-dot` for workspaces), the label, and a count when it helps ("Needs you 2").
- The current page gets `surface-raised`, `ink` text and a 2px `accent` marker; others are `ink-muted` until hovered.
- Keep labels to one line; truncate long workspace names with an ellipsis and show the full name on hover.

Preview: [NavItem.html](NavItem.html) (open in a browser; shows light and dark).
