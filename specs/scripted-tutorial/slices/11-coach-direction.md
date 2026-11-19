# 11 Coach direction and the Tutorial tile

**Unlocks:** the coach can show, not only tell. The main menu's Tutorial tile
opens the battle.

There is one visual variable per step, each with its own shot and verdict:

1. **Camera tour** (variable: framing and pacing). A step's `camera` is a capture
   `Reel` flown by the existing `ViewportPilot` (`sampleReel`). Control then
   returns to the player's camera. Chapter 1's tour visits the three flags and
   `entry`. There is no second tour format.
2. **World marks** (variable: is a zone or unit mark legible at play distance).
   The battle overlay owner (`useBattleOverlay`, ground markers) draws them from
   coach data. The coach draws nothing in world space. A zone mark is paint on
   the ground in an existing colour role (game-ui: marks belong to the world;
   one colour, one meaning).
3. **HUD highlights** (variable: does a highlighted control read as "press
   this"). HUD components register typed anchors (`HudAnchor`: the shop button,
   a shop card, Start battle, a command button, an army card, the wallet, the
   score bar). The coach rings the anchor's rectangle, with no class-name
   lookups.

**Tile cutover (C12, second half):**

- `MENU_PAGES` loses the tutorial page, and `MenuVisit`'s `CONTENTS` loses
  `tutorial`.
- Home's Tutorial tile opens `/battle?scenario=ferryman-valley/operation-ferryman`.
  How `MenuPageRow` expresses "a tile that opens a battle" is delegated, but it
  must be one general row shape, not a special case for this tile.
- The Guide is now reachable only from Esc.

**Playable:** main menu → Tutorial → the briefing tour over the valley, chapters
1–2 with marks and highlights.

**Gates:**

- `menuRoutes.test.tsx`: the tile leads to a scenario battle address, and no
  menu page renders the Guide.
- `tutorial.mjs` extended: the tour releases the camera, and each mark and
  highlight appears on its step.
- Per variable: a shot, compare-screenshots against 10's shot of the same step,
  then screenshot-critique last.
- Human checkpoint (non-blocking, inline): main menu → Tutorial → the briefing.

**Delegated:** reel timings, ring style within game-ui, anchor naming.

**Must stay green:** capture reel tests (`validateReel`), ground marker and
overlay scenes, menu tests.
