# Button

Primary, secondary, ghost, disabled.

The button starts an action; primary is ink in light mode and lime in dark mode and appears at most once per view.

- **Classes:** `cc-btn` plus one of `cc-btn--primary`, `cc-btn--secondary`, `cc-btn--ghost`.
- **Consumer provides:** a real `<button>` (or `<a>` for navigation), a text label, and optionally a 16px Lucide icon before it. Icon-only buttons need `aria-label`.
- **Primary** (`accent` fill, `on-accent` text): the one action the view exists for, such as "Create workspace".
- **Secondary** (`surface` fill, `border-strong` outline): alternatives like "Set up manually".
- **Ghost:** low-emphasis actions in toolbars and the sidebar.
- Don't put two primary buttons side by side, and don't use accent for destructive actions.

Preview: [Button.html](Button.html) (open in a browser; shows light and dark).
