# Ground lane: roads, banks, trees and fields that look like country

A separate session works this lane in parallel with the map lane. The physical ground is in: road kinds with speeds, rounded centrelines, square road ends, rivers with beds and bridges, one forest rule, forest and street bodies, tree lines, grass heights. What is drawn over it is still flat colour. This lane makes the ground look like a place.

## The contract

Open country and town streets on every map read as a real landscape at the tactical camera and close up: roads with a surface, shoulders and wear; streets with pavement, curbs and markings; river banks; several tree species mixed into forests with closed canopies; grass, crops and fields with variation; street furniture and forest-floor bodies as models. A generated map and the village both pass the composition gate, inside the frame budget.

The design and settled decisions are in [ground look](ground-look.md) and the README's budgets. Each slice file is its own contract. Where a slice names a spike or gate (GG, SG1, SG3, SG4), answer that question on the way with the smallest experiment that settles it and record the verdict in the slice; nobody else will run it. Reference images are under `assets/reference/` (the Broken Arrow shots are the target for how a mixed map reads from above).

## Ownership

| This lane owns | Others own (stay out) |
|---|---|
| `packages/battle-renderer/src/terrain/` and `scenery/`, the grass and water passes | `packages/battle-renderer/src/models/` and building passes (the [buildings lane](buildings-lane.md)) |
| Tree, street-prop and forest-body scripts in `packages/scene-assets/blender/`; their sources and bundles under `assets/` | Building scripts, the template catalogue |
| `fixtures/biomes/`; the presentation rows of `fixtures/game.json` for ground, roads, water and vegetation | `fixtures/game.json` rules; `crates/`; `fixtures/maps/`; the menu and preparation code |
| This file's Status section | `README.md`'s Next Agent Prompt and TODO |

The physical shape of a road, river or forest is a shared contract in `crates/contract` that the simulation reads: this lane draws it and does not change it. If the look needs a physical change (a road width, a bank profile), say so in Status and make it a small named commit. The frame function that orders every pass is shared with the buildings lane: add a pass there in a small commit of its own. No repo-wide renames.

## Work, in order

1. **The rig:** [C62 ground evidence rig](slices/C62-ground-evidence-rig.md), so every later picture is taken the same way.
2. **Country roads:** [C66 road core](slices/C66-road-core.md) → [C67 shoulder](slices/C67-road-shoulder.md) → [C68 ruts and centre strip](slices/C68-ruts-and-centre-strip.md), with the per-kind look left open by C64 and the bend left open by C65.
3. **Town streets:** [C28 pavement](slices/C28-pavement.md) → [C29 curbs](slices/C29-curbs.md) · [C30 markings](slices/C30-markings.md).
4. **Water's edge:** [C70 bank bands](slices/C70-river-bank-bands.md) → [C71 bank roundness](slices/C71-river-bank-roundness.md).
5. **Trees and forest:** [C73 tree skeleton](slices/C73-tree-skeleton.md) → [C74 species](slices/C74-tree-species.md) → [C75 mix and colour](slices/C75-forest-mix-and-colour.md) → [C76 canopy closure](slices/C76-canopy-closure.md); [C78 forest body models](slices/C78-forest-body-models.md) → [C79 dressing](slices/C79-forest-dressing.md); the drawn half of [C86 tree lines](slices/C86-tree-lines.md).
6. **Grass and fields:** [C81 wild grass](slices/C81-wild-grass.md) → [C82 within-field variation](slices/C82-within-field-variation.md) → [C83 crops](slices/C83-crops.md) → [C84 field palette](slices/C84-field-palette.md) → [C85 field texture](slices/C85-field-texture.md). Generated maps have bare open ground today; fields belong on it.
7. **Street furniture:** [C45 street models](slices/C45-street-models.md) for the bodies the simulation already has (C44).
8. **Whole frame:** [C31 city biome](slices/C31-city-biome.md) → [C87 ground composition gate](slices/C87-ground-composition-gate.md).

Steps 2 to 6 are independent of each other after the rig; take them in this order unless a picture says otherwise.

## How to work

Read [`AGENTS.md`](../../AGENTS.md). Load the `renderer` skill before renderer work. Art is programmatic: shaders, Blender scripts and code in the repo, reproducible from a clean checkout; nothing hand-edited.

The ground is judged by looking. For every look change, render it in the real lab on the village and on a generated map (`/battle?type=mixed&size=medium&seed=2`) at the tactical camera and close up, get an unprimed `screenshot-critique` (ask "could any dark region read as shadow, or shadow as fog?"), compare against the references with `compare-screenshots`, and show the user with `preview-shots`. The user gives feedback on pictures directly in this session; a checkpoint never blocks, so decide on the evidence if they are silent and record it.

Test in proportion: the narrow tests for what changed and the one or two scenes it can move. No full gate. No frozen-record tests. Measure frame cost with the feature toggled on and off, interleaved on one machine. Branch from main, merge main often, and push small green passes to main. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Scratch renders go in gitignored `throwaway/`.

**One GPU, shared.** Several sessions are working at once. Run every scene, render and asset sheet through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs (a full Rust test run, a long sweep) to one at a time.

## Status

**2026-10-01.** The rig is in ([C62](slices/C62-ground-evidence-rig.md#outcome)); nothing drawn has changed yet.

- **Landed:** named stations on the village, the river lab and a generated map; the `ground-classes` frame view (the terrain's own class mask); `suppressTrees`; three rig checks in the `ground` scene. `STATIONS=village,river,generated bun run --cwd web scene -- ground` writes each station's shot, bare ground, mask and a sheet into `throwaway/evidence/ground/`.
- **The ground now:** the baseline, `throwaway/evidence/ground/village-stations.png` (regenerate with the command above). Roads are one flat pale band on every kind; the forest floor is blocky; crowns are smooth blobs; the bank is a thin brown line.
- **In flight:** C73 tree skeleton (with SG1's budget).
- **Next:** country roads C66 → C67 → C68; wild grass C81; then in the lane's order.
- **For the other lanes:** `BattleFrame` gained `setTreesShown` and the `ground-classes` view; `post.encode` takes a mode (`look`, `ungraded`, `raw`) in place of two booleans. No pass was added to the frame.
