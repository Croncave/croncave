# Croncave design

The UI designs for R1, exported from the design canvas and design system on claude.ai. Build the web app from these files.

- Canvas (all screens, interactive in Play mode): https://claude.ai/artifact/8mea9BG9nnLJ7w1Jh1moYL
- Design system: https://claude.ai/artifact/3x5sdj95Rg5Dw5a2s2MWy2

Both links are private to the account owner and can't be fetched from here. This folder is the snapshot. If it and the canvas disagree, the canvas wins; ask the user to re-export.

## What's here and what wins

| Path | What it is | Use it for |
| --- | --- | --- |
| `system/tokens.json` | Every design token: colors (light and dark), type styles, spacing, radius, shadow, with a usage note on each | **Source of truth** for values |
| `system/tokens.css` | The same tokens as CSS variables. Light on `:root` / `[data-theme="light"]`, dark on `[data-theme="dark"]` | Import once, globally |
| `system/components.css` | Styles for every shared component (`cc-*` classes) | Import once, globally, or port class by class into Svelte components |
| `system/README.md` | The brand book: voice, color roles, type, spacing, states, motion, layout, icons | Read before any UI work |
| `system/components/*.md` + `*.html` | One note per component (when to use it, classes, rules) and a preview page showing it in light and dark | Building each component |
| `screens/*.png` | Every screen at 1280×820 (sidebar 248×820), light and dark | What it should look like |
| `screens/*.html` | The same screens as standalone pages (open in a browser) with the exact markup and styles | Exact spacing, sizes and structure |

Order of authority: **tokens and component notes > screen HTML > screenshots.** The screen HTML uses inline styles and a few canvas-only helper classes; when building, replace those with the shared `cc-*` components:

| In the screen HTML | Build it as |
| --- | --- |
| `hc-chip` (Home composer controls) | `cc-chip` inside `cc-composer` |
| `nf-sec`, `nf-field`, `nf-input`, `nf-seg`, `nf-choice` (setup forms) | `cc-section`, `cc-field`, `cc-input`, `cc-seg`, `cc-choice` |
| inline "In plain words" panel | `cc-summary` + `cc-fact` |
| inline hover card on Home | `cc-peek`, `cc-stat`, `cc-spark` |
| `au-row` (sign-in panel feed) | a small read-only list; no shared component needed |

Everything in the screens (names, times, costs, counts) is **sample data**. Prices and rates are not decided yet (see `docs/pricing.md`); never hard-code a cost the product can't know.

## Rules that apply to every screen

- **Tokens only.** No hex values or new colors in product code. Light is the default theme; dark is a full equal (`data-theme` on the root).
- **Accent:** ink (near-black) in light mode, lime `#99d52a` in dark mode. Used rarely: the one primary button per view, the current nav marker, progress bars, the logo mark.
- **Statuses** always pair a color with a word: Live and Done (lime), Working (blue, pulsing dot), Needs you (orange), Asleep (gray, hollow dot), Failed (red). Use exactly these words.
- **Kind icons:** Lucide `box` always means a workspace, `zap` always means a task.
- **Fonts:** Inter for UI and reading text; JetBrains Mono for labels, times, costs, counts, commands and IDs.
- **Content area** (everything right of the sidebar) sits on the dot grid (`cc-dotgrid`); cards on it are solid `surface` with a 1px `border`. Borders, not shadows; only popovers and the hover card get `shadow-popover`.
- **Page-level buttons** (the notifications bell) go at the top right of the content area, never in the sidebar.
- **Plain words:** "asleep", "awake", "needs you". No "instance", "vCPU" or "container" outside expert details.
- **Accessibility:** real `<button>`, `<a href>` and form elements; `aria-label` on icon-only buttons; visible 2px `focus` ring; text meets 4.5:1 in both themes (already checked for every token pair).

## Screens

### Sidebar (`sidebar-*`)
The app shell's left column, 248px on `surface-sunken`. Top to bottom: logo, primary **New workspace** (`N`), **Search or run a command** (`⌘K`), nav (Home with a "needs you" count, Templates, Activity), then two sections, **Workspaces** and **Tasks**. Each shows its two most urgent items (status dot + name) and a "Show N more" row, with a `+` to create one. Hovering an item shows its status in words. At the bottom, the **This month** spend card (spend, cap, hours awake, progress bar), which links to Usage, then the account row (avatar, name, plan, settings). The current page gets `surface-raised` and a 2px accent marker.

### Home (`home-*`)
- Greeting ("Good morning, Treasure") with a one-line summary ("Three things need you. Everything else is on track.") and the bell (`cc-icon-btn` with an unread badge) at top right.
- **Composer** (`cc-composer`): a big box where people describe a job in plain words. Under it: attach, the **In** / **When** / **Limit** chips (what Claude will assume; each opens a picker), a "Set up manually" link to the forms below, and **Start** (`⌘↵`). Always open.
- **Since your last visit** (`ActivityTable`): what workspaces and tasks did since the user was last here, with the window start beside the title ("from 23:40 last night").
  - Filters: All, Needs you (includes Failed), Done, with counts.
  - Columns: time, name (kind icon + name, which links to its detail page), what happened (one line, truncated), status, chart icon.
  - Sort by Time, Name or Status (click again to reverse). Default is Status: Needs you, Failed, Working, Done, newest first within each.
  - No cost or action columns: actions live on the detail pages.

### Home, hover card (`home-hover-card-*`)
Quick stats for one row (`HoverCard`). Opens after hovering a row for about 400ms, or at once when hovering or focusing the row's chart icon. It sits right-aligned by the icon column, below the row for the top four rows and above it for lower ones, and closes when the pointer leaves the table. Contents: kind icon, name and status; three stats that fit the kind (a finished task: Took / Cost / Runs; a workspace: Awake / Cost / its own count; a watcher: Checks / Alerts / Cost); a 14-day bar chart of cost per run (neutral bars, latest in `accent-ink`, failed runs red at half height, a title on each bar, an `aria-label` summary); a footer line ("Next run tomorrow 08:00") and a **Details** link.

### Sign in and sign up (`sign-in-*`, `sign-up-*`)
Every user creates a Croncave account; there is no Google or GitHub sign-in. Left: logo, heading, email field, primary button ("Send sign-in link" / "Create account"), a note that we email a sign-in link (no password), a link to switch between sign in and sign up, and the terms line with "Available in the US". Right: a panel that is always dark (`data-theme="dark"`, `cc-dotgrid`) with the product line "Your computer in the cloud, still working after you close the lid." and a sample "While you slept" feed.

### New task, manual (`new-task-*`)
Header: breadcrumb, title, one line of help, and **Describe it instead** (back to the Home composer). Numbered sections (`FormSection`), following the five answers every task has:
1. **What:** name, Runs in (its own workspace or an existing one), kind (Ask Claude / Run a command / Watch a page) and its fields (here a command).
2. **When:** On a schedule / When something happens / Just once. A schedule is set with plain pickers (repeat, time, time zone); the cron expression and next run show underneath in mono.
3. **Results**, 4. **Memory**, 5. **Limits:** folded, each with a one-line summary of its defaults.

Right: the **In plain words** summary, facts (runs a month, asleep between runs, per-run limits, cost "shown after a test run"), then **Create task**, **Test run first** and Cancel.

### New workspace, manual (`new-workspace-*`)
Same layout. Sections:
1. **Basics:** name; start from Empty / A template / A GitHub repo, plus the repository picker.
2. **Claude:** **Your Claude subscription** (sign in once inside the workspace, through Anthropic's own flow) or **An Anthropic API key** (kept in our gateway, never stored in the workspace).
3. **Computer**, 4. **Secrets**, 5. **Limits**, 6. **Sharing:** folded with summaries.

The summary panel ends with **Create workspace**.

### Task detail (`task-detail-*`, example: Competitor Prices)
- Header: kind icon, name, status, schedule and next run in mono, with **Pause** and **Run now**.
- Four stat tiles: last run, cost this month, runs succeeded, changes found.
- **Runs** table with filters (All / With changes / Failed): when, what happened (runs that found something in `ink`), took, cost, status, chevron to the run.
- Right column:
  - **Latest result**: the actual output in short form, plus a link to where results went.
  - **Cost per run** chart.
  - **Setup**: the five answers, one line each, with **Edit** to the task form.

### Workspace detail (`workspace-detail-*`, example: Real Estate App)
- Header: kind icon, name, status, repo, branch and state in mono, with **Settings** and **New session**.
- When something needs the user, an orange banner (`status-needs-soft`) says what and offers the actions: **Open preview** and **Review changes** (the primary).
- Tabs: Overview, Sessions, Tasks, Files, Activity.
- Overview:
  - **Last session**: a timeline (mono time, step icon, one line each, the step that needs the user in orange), with links to the full log and all sessions.
  - Right column: **Computer** (state, size, what wakes it, storage bar), **This month** (spend against the workspace cap), **Tasks here**.
- The sidebar marks the current workspace or task as the current page.

## Not designed yet

- The **Review changes** screen (the end of an overnight coding session), the preview viewer, and a run or log detail page
- The notifications panel behind the bell, and Search / command palette (`⌘K`)
- The other workspace tabs, Templates, Activity, Usage, Settings and account pages
- Folded form sections opened (Results, Memory, Limits, Computer, Secrets, Sharing) and the "When something happens" trigger setup
- First-run and empty states (for example the composer's "Try" suggestions, shown only to new users), loading and error states
- Mobile and narrow layouts

Ask the user before inventing any of these; they are designed on the canvas first.

## History

Earlier directional mockups from planning (with the old working name "Nightshift") are superseded by this folder: https://claude.ai/artifact/UiRePeq7vzwoqQfAosvUG5
