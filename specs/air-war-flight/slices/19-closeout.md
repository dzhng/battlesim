# 19 — Closeout

1. **Air lab scenes.** Every air scene's header names the sim test behind its claim, and no gap lines remain that this spec should have closed. Delete `CRUISE` in `_air.mjs`.
2. **A played skirmish:** `/labs/air-skirmish`, or a played game against the skirmish AI. Every jet is bought, flies, attacks and withdraws. Record GIFs.
3. **The full test and scene run,** once.
4. **The balance report and the price pass.** Prices only, not rules. Tuning target (user): jets punish a side that skipped AA, doing heavy damage before it recovers, while one AA unit is enough insurance to blunt them. Judge it in sample runs, never as a test.
5. **Visual findings carried from earlier slices.** Judge each with `screenshot-critique` at the game camera and fix what still reads wrong:
   - **Slice 01 critique:** a jet at 250 m doesn't read as high up in a close shot. Its shadow falls far off along the sun, there is no drop line by design, and the model looks large against the field plots.
   - **Out of this spec's scope, noted for the user:** fog hatching shimmer and a hard band at the horizon; label leader lines ending on the fuselage; un-backed "Heard engine" lines.
6. **Whole-spec review** (`review`), then consolidate `choices.md` and close with `close-spec`.
