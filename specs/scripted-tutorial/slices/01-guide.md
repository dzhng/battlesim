# 01 Guide

**Unlocks:** "tutorial" means one thing (the battle). The page is the Guide, in
the Esc menu.

**Contract (C12, first half):** `PauseMenu`'s item and modal are "Guide". The
main menu stays as it is in this slice: its Tutorial tile keeps opening the old
page until slice 11 swaps it to the battle in one commit, so the menu is never
without a tutorial.

**Seam:**

- `apps/battle-lab/src/Tutorial.tsx` → `Guide.tsx`, component `Guide`. Its
  header comment and prose drop the "main menu's Tutorial page" framing.
- `PauseMenu.tsx`: the item, the dialog's label and title, its close button,
  and the state name (`guideOpen`). Reuse the existing `hud-tutorial-modal`
  look; rename its classes to `hud-guide-*`.
- `web/tutorial-shots.mjs` → `web/guide-shots.mjs`, and the runtime media folder
  `tutorial-media` → `guide-media` (a pure `git mv`, nothing re-encoded). The
  harness re-approval moves from the menu page to the pause-menu modal.
- The battle-lab README paragraph on the main menu and the Guide.

**Playable:** any `/battle` → Esc → Guide opens the centred modal over the paused
battle.

**Gates:**

- `web/tests/pauseMenu.test.tsx`: open and close the Guide by role and order,
  not by a pinned label (AGENTS.md: presentation isn't logic).
- `web/tests/menuRoutes.test.tsx` stays green.
- Typecheck.
- One shot of the modal over a paused battle. Use compare-screenshots against
  today's "Tutorial" modal shot (variable: the modal's placement and
  readability only); run screenshot-critique last. Show it inline
  (non-blocking).

**Delegated:** class names; whether the guide media rename needs a cache-busting
change in the build (check the build config).

**Must stay green:** `pauseMenu`, `menuRoutes`, and the `ui` scene's menu shots
(re-bless after comparing them yourself if only the label changed).
