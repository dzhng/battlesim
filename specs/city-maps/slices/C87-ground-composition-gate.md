# C87: ground composition gate

**Depends on:** every retained ground slice. **Kind:** slice.

## Question
Do the accepted ground contracts compose on real routes at ≥30 FPS?

## Contract it unlocks
An evidence gate, not a module. A tour of the village, `/lab/river`, the farmland lab and a C31 city park edge.

## API seam
The frame and map loader.

## What the human can run or see
The tour's shots and benchmark.

## Verification
- Whole-frame comparison is allowed here, because every component variable has already passed.
- The full `village_report` once.
- Serial frame benchmarks; native/wasm replay; memory and reset.
- **The lane budget: at most +3 ms GPU p50 on the village benchmark** (trees ≤1.5, dressing ≤1, grass ≤0.5).
- A failure returns to the owning slice; don't fix composition by changing several variables at once.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**whole-frame composition**) against **the accepted per-slice shots and `../assets/reference/ground/ours-vs-refs-board.jpg`**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** None.


## Delegated to the implementer
None. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every accepted slice.

## Feedback that would change this slice
Whole-frame confusion between vegetation, cover, shadow and fog reopens the responsible per-variable ground slice.

## Outcome

**The ground contracts compose and the lane is inside its GPU budget. The gate is not clean: forest floor bodies went back off, two scenes fail for reasons this lane did not cause or could not attribute, and a fresh eye still reads open country as a map.** The farmland lab the slice names does not exist, so the tour is the village, the river lab and a generated map (mixed, medium, seed 2) with its town edge. Generated maps do carry tree lines; no station stands on one, so C86's drawn half is held by tests alone and has had no picture or critique.

**Frame budget.** The short village benchmark, alternated four times between the lane's starting commit (`ca7bb972`) and the final tree (Apple Metal, 1920 × 1080, production build, load average 5 to 20):

| | Start | Final |
|---|---|---|
| GPU frame total, mean | 5.95, 5.53, 6.35, 5.70 ms | 6.32, 5.66, 5.98, 5.75 ms |
| Average FPS | 51.1, 55.7, 48.2, 54.5 | 44.8, 47.5, 46.0, 48.8 |
| Frames over 33 ms | 32, 9, 27, 16 | 107, 52, 88, 33 |
| Main-thread heap at the end | 277 to 297 MiB | 426 to 454 MiB |
| GPU buffers | 182.8 MB | 223.3 MB |

- GPU: the pairs differ by +0.37, +0.13, −0.37 and +0.05 ms, median +0.09 ms, against the lane's +3 ms. Met.
- The 30 FPS floor holds, but the final tree is about 5.6 FPS lower, paces frames at 25 ms where the start paced at 16.7 ms in several camera phases, and holds about 150 MiB more heap, with CPU per frame up from 1.2 to 1.4 ms. **Not attributed:** the final tree carries every lane's work since the start, and no build of main without the ground lane exists. One probe: on the running village, the heap is the same with every ground visual switched off as on (440 to 460 MiB). Whoever owns scale should bisect it; the benchmark logs are scratch.
- The passes' own paired rows (each in its slice): trees +0.50 ms, grass inside +0.4 ms, undergrowth +0.25 ms, field texture +0.15 to +0.45 ms, road wear and streets under +0.2 ms. Most were taken under load and did not resolve finer than about half a millisecond.

**Scenes on the final tree.** Village replay, geometry, sensors, consequences, river, fog-look, workbench and ground pass. Three do not:
- `movement`: "the rifle group's destinations are admitted and its routes use the 5 m gap" (routes come back empty). Bisected over 8 builds: the first bad commit is `82564693`, "Merge main and retain move admission and bounded routing contracts", and both its parents pass. It is that merge's, not this lane's.
- `village`: the scripted fight no longer yields a last sighting, a firing report and a suppressed squad; two unit-panel layout checks fail; one wait times out. The fight check bisects to the same first bad commit as `movement`, `82564693`, with both parents passing: that merge's, not this lane's. The panel checks were not bisected.
- `generated`: the overview counts 43,533 of 43,534 trees at the far tier. Not explained.

**Village report** (full, ten seeds, once): completes; road push 6 of 10 captured, flank 7 of 10, ambush and crossfire 0 of 10; 13,651 G instructions. With forest floor bodies on, the flank script's order was refused on seed 34 and the report panicked, which the three-seed quick report had not shown.

**Forest floor bodies are off again** (`b0d5b19a` reverts the one rule). C78's models stay and draw wherever a map's own rules turn the bodies on (generated battles do). Turning them on for the village needs its scripts and labs to stand cover appearing in their woods: the simulation's and the map's work.

**What a fresh eye saw in the tour** (one unprimed critique, fifteen frames):
- *Works:* town streets up close ("read immediately and convincingly"); the farm track, wheat tramlines and ploughed furrows close up; town, fields and woods told apart at once from high up.
- *Worst:* (1) open country is a flat, outlined carpet: half the fields are one muddy olive, field boundaries are uniform outlines, and nothing is vertical, no hedge, wall or ditch; (2) roads and the river are lines laid on top: constant width, ruler straight, ending at nothing, with a smudged gravel-to-asphalt join; (3) woods look stamped: rectangles on the village, lump crowns on sticks, a bare floor, a crop sliver running under one wood's edge.
- *Shadow or fog:* medium confidence that the darkest green fields beside pale crops, and the darker green under the whole town, read as shadow with nothing to cast it (SG4's constraint on the yard's green); low to medium for distance haze and a pale band beside the track reading as an overlay.
- *Fixed from it:* the bank read as an orange outline round the river; its earth is a pale grey-brown now.
- *Returned to their owners, not patched here:* everything else. The lane's Status lists them by slice; the hedgerows, wood shapes, street stubs and "where a town is" need the map.

**Checks restated at the gate**, each recorded in `choices.md` ("Ground lane integration" and "C87 ground composition gate"): a road against its neighbour is "lighter, or within 15% and apart in hue"; the walk beside a road has one rule, no dark trough; the generated scene counts boulders with the static world; the town's edge needs some drilled field in frame, not 15%.
