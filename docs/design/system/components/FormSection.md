# FormSection

Numbered sections, fields, segmented control, choice cards and the summary panel.

Manual setup forms are a column of numbered, foldable sections beside a summary panel. The sections follow the five setup answers (what, when, results, memory, limits) for tasks, and basics, Claude, computer, secrets, limits and sharing for workspaces.

- **Section:** `cc-section` with a `cc-section__head` button (`aria-expanded`), a mono `cc-section__num` ("01"), the title, and when folded a `cc-section__sum` one-line summary of its answers ("$0.50 a run · 10 min · retry twice") and a right chevron. Open sections show a down chevron and a `cc-section__body`. Open the first two by default; fold the rest with sensible defaults already filled in.
- **Fields:** `cc-field` > `cc-field__label` + `cc-input` (add `cc-input--mono` for commands, paths, repos and times) + optional `cc-help`. Put short related fields side by side in a grid.
- **Segmented control:** `cc-seg` with 2 to 4 buttons, `aria-pressed="true"` on the chosen one. For modes that change the fields below ("Ask Claude / Run a command / Watch a page").
- **Choice cards:** `cc-choice` (`role="radio"`, `aria-checked`) with `cc-choice__radio`, `cc-choice__title` and `cc-choice__text`, when each option needs a sentence (Claude subscription vs API key).
- **Summary panel:** `cc-summary`: a `cc-label` "In plain words", a `cc-summary__text` sentence that restates the setup as it will happen, `cc-fact` rows, then the actions: the one primary ("Create task"), an optional secondary ("Test run first") and a ghost Cancel. Never show a cost we can't know yet; say when it will be shown.
- Page header: mono breadcrumb, `title`, one line of help, and a secondary "Describe it instead" that goes back to the Home composer.

Preview: [FormSection.html](FormSection.html) (open in a browser; shows light and dark).
