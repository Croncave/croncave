# Composer

Describe a job; chips show what Claude will assume.

The composer is the big box on Home where people describe a job in plain words; Claude turns it into a task or workspace.

- **Classes:** `cc-composer` holds `cc-composer__text` (the input, at least two lines tall) and `cc-composer__bar`. The bar has an attach `cc-chip`, then setting chips, a spacer, a "Set up manually" link and the one `cc-btn--primary` "Start" with a `⌘↵` hint.
- **Setting chips** (`cc-chip`) show what Claude will assume and open a picker when clicked: **In** (which workspace), **When** (schedule or trigger), **Limit** (spending cap). The label is `ink-muted`, the value in `<b>`.
- Always shown open on Home; it is the page's main action.
- Memory and results are not chips: Claude proposes them and the person approves them on the next step.

Preview: [Composer.html](Composer.html) (open in a browser; shows light and dark).
