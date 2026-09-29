# IconButton

Page-level bell with unread badge, and the ghost row icon.

The icon button is an action shown only as an icon, like the notifications bell or a table row's quick-stats icon.

- **Classes:** `cc-icon-btn` (32px, outlined, for page-level actions such as the bell at the top right of the content area) or `cc-icon-btn cc-icon-btn--ghost` (24px, no outline, for actions inside rows). Add `cc-icon-btn__badge` inside for an unread dot in `status-needs`.
- **Consumer provides:** one 16px Lucide icon and an `aria-label` that says what it does, including state ("Notifications, 3 unread").
- Only for actions everyone recognises from the icon. Anything else gets a text button.

Preview: [IconButton.html](IconButton.html) (open in a browser; shows light and dark).
