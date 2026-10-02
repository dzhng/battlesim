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

**2026-10-02.** Steps 1 to 6 of the work order and the town streets are on main; the forest floor, the street models and the composition slice are in flight; the composition gate is last.

- **Landed** (each slice's Outcome has its seam, numbers and open items):
  - Rig: [C62](slices/C62-ground-evidence-rig.md#outcome). `STATIONS=village,river,generated bun run --cwd web scene -- ground` writes each station's shot, bare ground, class mask and a sheet into `throwaway/evidence/ground/`; `STATIONS=map:station+station` shoots only those.
  - Country roads: [C66](slices/C66-road-core.md#outcome), [C67](slices/C67-road-shoulder.md#outcome), [C68](slices/C68-ruts-and-centre-strip.md#outcome), with [SG3](slices/SG3-road-wear-read.md)'s verdict: each kind's own surface; the shoulder is a wash of hue with thinned grass, not a band; ruts and a centre strip on the dirt track only.
  - Town streets: [C28](slices/C28-pavement.md#outcome), [C29](slices/C29-curbs.md#outcome), [C30](slices/C30-markings.md#outcome): asphalt, a drawn walk, a shaded curb, centre dashes and crossings.
  - Water's edge: [C70](slices/C70-river-bank-bands.md#outcome), [C71](slices/C71-river-bank-roundness.md#outcome).
  - Trees and forest: [C73](slices/C73-tree-skeleton.md#outcome) with [SG1](slices/SG1-tree-and-dressing-cost.md)'s tree verdict, [C74](slices/C74-tree-species.md#outcome), [C75](slices/C75-forest-mix-and-colour.md#outcome), [C76](slices/C76-canopy-closure.md#outcome): seven kinds from one generator inside a per-tier triangle budget, stands of one family, closure at 0.23 of the floor seen (accepted by the user; a unit under a closed crown is found by its x-ray).
  - Grass and fields: [C80](slices/C80-grass-presets.md), [C81](slices/C81-wild-grass.md#outcome), [C82](slices/C82-within-field-variation.md#outcome), [C83](slices/C83-crops.md#outcome), [SG4](slices/SG4-palette-vs-shadow-floor.md), [C84](slices/C84-field-palette.md#outcome), [C85](slices/C85-field-texture.md#outcome), and fields that lie along their roads on generated maps.
- **The ground now:** shoot `STATIONS=village:bend-65+field-65+forest-edge-65+patchwork-1100,generated:overview-2500+junction-65+country-250`. The village reads as muted farmland with a mixed wood and gravel roads; the generated town has asphalt streets with walks and crossings between the buildings lane's buildings.
- **Open, for C31, C87 or a later pass:**
  - A gravel country road runs through the generated town and reads as asphalt in shadow; the ground between town buildings is a saturated lawn green (SG4: no palette can desaturate it without failing `fog-look` under `grey-veil`). Both are C31's.
  - Meadow reads as felt and ploughed earth as rippled sand; crop fields from 250 m read as corduroy. Wedge plots where a curving road cuts the land need field polygons from the map.
  - A wide road at 250 m is a plain ribbon; the track's bend and the village's road elbow are too tight (C65's rounding).
  - The bank is a flat tan stripe; its outer line is lobed at 250 m; the water's light lanes read as lane markings from high up.
  - Crowns are lumped, not open (cost); trunks stand in rows (the forest rule's grid).
  - Bare ground within 5 m of the village's gravel road reads about 6% darker than the field beyond, cause not found (`choices.md`, "Ground lane integration").
  - Frame cost: every pass measured paired on/off under a loaded machine and most rows did not resolve; C87 owns the lane's budget on a quiet run.
- **In flight:** C78, C79 and the drawn half of C86 (forest floor); C45 (street models); C31 (urban and plain composition).
- **Next:** C87, the composition gate.
- **For the other lanes:**
  - Map lane: the village map should name its roads `country_road` (then `drawnKind` in `terrain/surfaces.ts` goes); generated towns export no `sidewalk` and nothing says "urban"; `land_regions` reaches no consumer; C69's "no layout writes a river" is stale; the trunk grid reads as rows.
  - Fog styles' owner: `grey-veil` separates seen from unseen by about 1 unit at `default-wall`; no palette widens it.
  - Interfaces that changed: `BattleFrame` gained `setTreesShown`, `setRoadWearShown`, `setFieldTextureShown` and the `ground-classes` view; `post.encode` takes a mode; the world export's paved `kind` column indexes `surfaceAreaKinds`; the biome's `roads`, `shore`, `water`, `grass`, `plots`, `trees` and `forest_floor` rows changed shape. No pass was added to the frame.
- **Checkpoints:** every pass's shots were opened in Preview for five minutes. The user answered one: canopy closure at 0.23 is fine. The rest were kept as merged.
- **GPU budget for every pass** (the user's): tune on two stations with at most three variants a round and two rounds a slice; shoot the full station set once per slice; one short paired cost run per slice.
