# C78: forest body models

**Depends on:** C77, C73. **Kind:** slice.

## Question
Do logs and boulders visibly fit their bodies?

## Contract it unlocks
Workbench appearances, fitted to their boxes: the log reuses C73's bark; the boulder comes from a new small rock generator (not `~/dev/game`'s pyramids).

## API seam
`packages/scene-assets/blender/`.

## What the human can run or see
A workbench sheet and `forest-deep-25`.

## Verification
- `asset check`; `fit.*`.
- "Models are good enough": one round of fixes for outright errors.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**body model read only**) against **`../assets/reference/ground/forest-road-rocks.jpg` and C77's box shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Dressing.


## Delegated to the implementer
Form details. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C77's behaviour.

## Feedback that would change this slice
A forest floor body that reads as impassable despite its physical properties reopens model/body fit.

## Outcome (2026-10-02)

**Changed since.** [C87](C87-ground-composition-gate.md) reverted the default activation (`b0d5b19a`): the forest rule lays no log or boulder, so the village's `floor-log-25` and `floor-boulder-25` stations have nothing to stand on. The models draw wherever a map's own rules lay the bodies, as generated battles do.

**Contract as landed.**

- **Two appearances.** `forest_log` (scenery kind `log`, box 4.4 × 0.7 × 0.7 m) and `forest_boulder` (kind `boulder`, box 2 × 1.6 × 1.5 m), built by `packages/scene-assets/blender/forest_floor.py` into `assets/source/forest/`. `asset check` passes with both; each fills its box inside the 0.1 m footprint tolerance and runs a little below the ground (5 cm and 12 cm, its own `ground_m`) so it does not float on a slope.
- **The log** is the tree generator's bole laid down: `trees.tube` with its carried ring frames and furrows, tapering from a flared butt to a thin top, both ends torn to pale wood, three limb stubs, bark mottled and bare to grey wood in patches, mossed on top.
- **The boulder** is the new rock generator: a ball cut by twelve seeded planes whose arrises are rounded by a power mean, squashed underneath so it sits on its widest section, mossed where it faces up and stained by the soil at its foot. The dressing's small rocks are the same generator.
- **The binding.** `fixtures/props/forest/{log,boulder}.json` lose `status: "systems_only"`; a placed log or boulder now draws through the ordinary prop path. That alone moves no digest.
- **Default activation** is its own commit (below).

**What changed from the slice as written.** One script builds the bodies and C79's dressing kinds, because they share the bark, the clumps and the rock. `trees.py` is importable (its kinds build only when it is the script), and its GLBs are byte-identical.

| Measure | Log | Boulder |
|---|---|---|
| Triangles per tier | 406 / 213 / 84 / 20 | 1,280 / 320 / 80 / 20 |
| Bounds against the box | 4.44 × 0.72 × 0.76 m | 2.00 × 1.60 × 1.50 m above the ground |

**Default activation: on, as a named digest change** (`forests.rule`: 5 log and 3 boulder candidates a hectare, C77's test densities, which it measured as 4 logs and 6 boulders on the village). C77's release regression passes: the bodies have accepted bindings and appearances. `village_report -- --quick`, six trials:

| Row | Digest before | Digest after |
|---|---|---|
| flank, seed 1 | `203dd0988c7e8f29` | `84d3461379002d91` |
| flank, seed 2 | `ec12338a4484226b` | `aa8d6fc309ee494d` |
| flank, seed 3 | `8ca931d6142d93ae` | `e167a7bfb7cd3e6a` |
| ambush, seed 1 | `1493585b9df6f1cd` | `fb8cf5192ceed2cd` |
| ambush, seed 2 | `3c1e0233a64cb70d` | `121de0c9dc0bc751` |
| ambush, seed 3 | `57fe46160ba929f6` | `98a1a425f50860cc` |

Every digest moves and no outcome does: results, capture time (565 s), blue cost lost (1,895), tanks lost (2) and rejoined (6) are the same in both arms, at 986 and 987 G instructions. `cargo test -p sim --test sim village::` and `forest::` pass with the bodies on. Not run: every other scene whose map has a forest (the river lab, the sensors and consequences labs, the saved generated map). The bodies now stand in all of them.

**Compare.** Against C77's box and `forest-road-rocks.jpg`: the boulder is a rounded grey mass with moss, where the reference's rock is broken outcrop running into the ground; ours is one clean stone on a smooth floor. Less wrong than a box; not the reference.

**Critique** (unprimed, twice; the models were rebuilt once between). First: the log read as "a cut pole or fence post", the boulder as "a smooth egg resting on the surface". After the rebuild:

- The boulder "agrees with solid cover that blocks vehicles" and reads as a rock at 25 m, still "egg- or potato-like": smooth, no cracks or strata. It does not fill its box's corners (a rounded rock in a box cannot). Left: models are good enough.
- **The log still reads as "a thin pole a tank would drive over and a man would step across".** Its thickness is the simulation's box (0.7 m), not the art's. Open, for the simulation lane: a log that is cover and stops a jeep wants to be nearer a metre through.
- Under closed crowns neither body is seen at all from the play camera. This is C76's closure, the same open question as the soldiers under two crowns.
- Shadow question: the floor's own dark drifts and root lines read as shadows of nothing with the crowns off. The root lines were made fainter in C79; the drifts are C76's.

**GPU.** Shared with C79's holds of the lock.

**Pictures** (scratch): `throwaway/shots/final/floor-village-floor-log-25-open.png`, `floor-village-floor-boulder-25-open.png`; sheets in `throwaway/sheets/forest_log/` and `forest_boulder/`.
