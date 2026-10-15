# 19 — Closeout

1. [x] **Air lab scenes.** Every air scene's header names the sim test behind its claim, and no gap lines remain that this spec should have closed. Delete `CRUISE` in `_air.mjs`.
   *Done:* every `web/scenes/air-*.mjs` header names its tests by file and function (renamed files and the circled-goal test corrected; crash, hover, audio and roster name theirs for the first time). Air-high's "not shown" line is gone, since the jet now holds cruise and circles its goal. Air-defense's interception gap names the test that covers it. `CRUISE` was already gone; only `HIGH_AIR_CRUISE` remains.
2. **Every jet plays (D44).** Extend `every_loadout_flies_a_pass` so every fixed-wing row also arrives and withdraws on low fuel. Record three AI-vs-AI skirmishes, one per faction pair, as GIFs in the scratch folder. Add no new scene or test file.
3. **The full test and scene run,** once. It includes the native client's own tests (`native/client`), which the sim run doesn't reach. A capture fixture re-blessed without its manifest went unseen there for three slices.
4. **The balance report and the price pass.** Prices only, not rules. Tuning target (user): jets punish a side that skipped AA, doing heavy damage before it recovers, while one AA unit is enough insurance to blunt them. Judge it in sample runs, never as a test.
5. [x] **Visual findings carried from earlier slices.** Judge each with `screenshot-critique` at the game camera and fix what still reads wrong:
   - **Slice 01 critique:** a jet at 250 m doesn't read as high up in a close shot. Its shadow falls far off along the sun, there is no drop line by design, and the model looks large against the field plots.
   - **Out of this spec's scope, noted for the user:** fog hatching shimmer and a hard band at the horizon; label leader lines ending on the fuselage; un-backed "Heard engine" lines.

   *Judged (2026-10-15), nothing shipped:* an F-16 at cruise over the air-high field was shot at the game camera, at zooms 65 m to 1,000 m, framed both on the ground beneath it and on the jet. An unprimed critique found that it still does not read as high up.
   - **Below about 330 m of zoom the jet is above the camera.** It is never on screen there, and neither is its card. The player sees only its shadow.
   - **Framed on the jet,** it reads as a large aircraft lying on the fields. Its shadow is off screen, or a faint speck far below it at 1,000 m.
   - **The worst cue is a fog hole beneath the jet.** The observation publishes a high-air unit's ground eye with the F-16's authored `ground_m: 1`, so the drawn fog clears a 1 m disc under it. The sim's own ground fog skips high air. Read as a hay bale or marker on the field, it makes the jet look grounded. An A/B with that eye removed (the sim's observation sending no ground eyes for `HighAir`) was judged clearly better.
   - **The fix is open:** it belongs in the sim's observation, which is outside this item.
   - **Shadow data is not the lever.** Halving `blur_per_m` barely changes the frame and does not change the verdict, so the data stays as it is.

   The out-of-scope findings all still occur:
   - The hatching is screen-locked and runs hard up to the tree line against the sky, with a darker band beneath it. Shimmer in motion was not judged from stills.
   - The leader line still ends on the fuselage, at a 2 m anchor.
   - "Heard engine" captions are still bare text on a halo. A cruising jet also "hears" idling engines on the ground.
6. [x] **Stale native tools.** The scene runner builds the mechanics validator and map-workbench tool only when they are missing, not when the rules or catalog schema changed. During this spec that caused three false scene failures. Make the runner rebuild them when stale, or check their freshness.
   *Done:* the runner builds them on every run and cargo skips a fresh one (about 0.1 s). The WASM still needs `build:wasm` after a schema change.
7. [x] **High air is an elevation layer (user, 2026-10-14).** High air is the one layer that bends physics: real jets fly around 1,000 m up, lowered here to about 250 m for play. Every treatment that follows from that is an elevation treatment keyed on the HighAir layer, never on "jet". With `refactor-clean`, rename what is really about the layer: the `air.jet` rules block, `air/jet.rs`, `step_jet`, `JetAir`, `JetState` and `JetLimits`, the test kit's names, and any remaining `jet`-keyed presentation checks. Keep "jet" only where something is truly specific to fixed-wing flight, and say why. A future high-air unit (a blimp) must need no new logic.
   *Done:* layer heights live in `air.low_air`/`air.high_air` and fixed-wing flight in `air.fixed_wing`, `air/fixed_wing.rs` and `step_fixed_wing`; layer treatments read `Unit::layer`/`high_air()`, and "jet" stays only on the `test_jet` row and scenarios that fly one.
8. **Everything but drones is enabled (user, 2026-10-15).** The test "only drones are planned" passes; the Gepard was the last non-drone row.
9. **Whole-spec review** (`review`), then consolidate `choices.md` and close with `close-spec`.
