# Ground lane: roads, banks, trees and fields that look like country

A separate session works this lane in parallel with the map lane. The physical ground is in: road kinds with speeds, rounded centrelines, square road ends, rivers with beds and bridges, one forest rule, forest and street bodies, tree lines, grass heights. When this lane began, what was drawn over it was flat colour. This lane makes the ground look like a place; its Status says where that stands.

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

**2026-10-02. Every slice of the lane is built and on main; the composition gate has run.** What is left is listed under "Open" by who owns it. Each slice's Outcome holds its seam and numbers; `choices.md` holds the lane's consolidated ledger (120 entries from `## C62 ground evidence rig` on).

- **Landed, in the lane's order:**
  - Rig: [C62](slices/C62-ground-evidence-rig.md#outcome). `STATIONS=village,river,generated bun run --cwd web scene -- ground` writes each station's shot, bare ground, class mask and a sheet into `throwaway/evidence/ground/`; `STATIONS=map:station+station` shoots only those.
  - Country roads: [C66](slices/C66-road-core.md#outcome), [C67](slices/C67-road-shoulder.md#outcome), [C68](slices/C68-ruts-and-centre-strip.md#outcome), [SG3](slices/SG3-road-wear-read.md).
  - Town streets: [C28](slices/C28-pavement.md#outcome), [C29](slices/C29-curbs.md#outcome), [C30](slices/C30-markings.md#outcome).
  - Water's edge: [C70](slices/C70-river-bank-bands.md#outcome), [C71](slices/C71-river-bank-roundness.md#outcome).
  - Trees and forest: [C73](slices/C73-tree-skeleton.md#outcome), [SG1](slices/SG1-tree-and-dressing-cost.md), [C74](slices/C74-tree-species.md#outcome), [C75](slices/C75-forest-mix-and-colour.md#outcome), [C76](slices/C76-canopy-closure.md#outcome), [C78](slices/C78-forest-body-models.md#outcome), [C79](slices/C79-forest-dressing.md#outcome), [C86](slices/C86-tree-lines.md) (a hedge under every tree line, on a verge of its own).
  - Grass and fields: [C80](slices/C80-grass-presets.md), [C81](slices/C81-wild-grass.md#outcome), [C82](slices/C82-within-field-variation.md#outcome), [C83](slices/C83-crops.md#outcome), [SG4](slices/SG4-palette-vs-shadow-floor.md), [C84](slices/C84-field-palette.md#outcome), [C85](slices/C85-field-texture.md#outcome).
  - Street furniture: [C45](slices/C45-street-models.md#outcome): a model for every street body; a street tree is drawn by the forest's own species.
  - Whole frame: [C31](slices/C31-city-biome.md#outcome), [C87](slices/C87-ground-composition-gate.md#outcome).
- **The ground now:** shoot `STATIONS=village:bend-65+field-65+forest-edge-65+patchwork-1100,generated:overview-2500+junction-65+town-edge-250+country-250+tree-line-65`. The village is muted farmland with a mixed wood and gravel roads; a generated map is a town of asphalt streets in fields, woods and tree lines, with a river.
- **The gate's verdict** ([C87](slices/C87-ground-composition-gate.md#outcome)): GPU +0.09 ms median against the lane's +3 ms. One fresh eye over fifteen frames: town streets, the farm track and crop close-ups, and the read from high up work; open country still reads as "a flat, outlined carpet", roads and the river as "lines laid on top".
- **Open, this lane's** (a later ground pass):
  - Current user grass feedback: [C81's approved blade-facing checkpoint](slices/C81-wild-grass.md#outcome) reduces dark streaks; the user likes the yellow dry patches. [C85's continuity work](slices/C85-field-texture.md#current-grass-continuity-feedback) remains open after a finest-grain prototype failed to improve visible ground structure. Its source is restored; yellow variation must be preserved.
  - The gravel-to-asphalt join where a country road enters a town reads as a smudge; it needs a term of its own.
  - Meadow reads as felt and ploughed earth as rippled sand; crop fields from 250 m as corduroy. Field boundaries are uniform outlines; a sliver of crop can lie between a field's edge and a wood.
  - A wide road at 250 m is a plain ribbon. Bare ground within 5 m of the village's gravel road reads about 6% darker than the field beyond, cause not found.
  - The bank is one flat band with no slope cue; the water's light lanes read as lane markings from high up. Neither bank slice has a frame-cost row.
  - Crowns are lumped, not open (cost). A tree line's ends stop on a full-size tree. Undergrowth is a thin sprinkle with headroom to spare.
  - The town's ground between buildings is one rough lawn.
  - Refactors the closeout review left because they need a scene to prove: two noise helpers written out twice and three ways to turn a lattice in `terrainMaterial.ts`; the rig builds the same world twice.
- **Open, the owner's:** forest floor bodies are off by default (with them on, the full village report's flank script is refused on seed 34); the log's 0.7 m thickness and the car wreck's 0.7 m height are the simulation's boxes; whether to spend more of the frame on open crowns.
- **Open, other lanes':**
  - Map: the authored rural-road cutover is complete ([C64](slices/C64-road-kinds.md#outcome)); the map names its road kinds. Street trees stand a metre from facades, crowns through walls; woods on the village are rectangles; street stubs end square; hedgerows are rare (one tree line in thirty fields).
  - Fog styles: `grey-veil` separates seen from unseen ground by about 1 unit at `default-wall`, which holds the town's yard to a dark green.
  - Scale: `movement`'s gap check and `village`'s scripted fight fail from merge `82564693`; main is about 5.6 FPS and 150 MiB of heap off the lane's start, the heap growing step by step across both lanes' merges.
  - Unattributed: two `village` panel-layout checks; the generated overview draws one tree short.
- **Interfaces that changed:** `BattleFrame` gained `setTreesShown`, `setDressingShown`, `setUnderstoreyShown`, `setRoadWearShown`, `setFieldTextureShown` and the `ground-classes` view; `post.encode` takes a mode; the world export's paved `kind` column indexes `surfaceAreaKinds`; `TerrainSurface` gained `strokes`; the biome's `roads`, `shore`, `water`, `grass`, `plots`, `field_rules`, `trees` and `forest_floor` rows changed shape; the prop catalog's street rows have art and `street_tree` is drawn by the forest. No pass was added to the frame.
- **Checkpoints:** every pass's shots were opened in Preview for five minutes. The user answered one (canopy closure at 0.23 is fine); the rest were kept as merged.
- **GPU budget for any further pass** (the user's): tune on two stations with at most three variants a round and two rounds a slice; shoot the full station set once per slice; one short paired cost run per slice.
