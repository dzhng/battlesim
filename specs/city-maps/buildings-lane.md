# Buildings lane: real buildings in place of boxes

A separate session works this lane in parallel with the map lane. Generated towns stand today as plain massing boxes over a labelled prototype template catalogue (`fixtures/prototype-building-templates.json`). This lane makes the buildings: modelled, textured, drawn at scale, and wrecked.

## The contract

A generated town is drawn with real buildings from a reusable template library: every category the generator places (farmstead, detached home, attached home, apartment, highrise, industry) has at least one accepted template, built by scripts in the repo; a full map holds its frame budget near and far; and a collapsed or gutted building looks it.

The design, sources and settled decisions are in [procedural buildings](procedural-buildings.md) and the README's [pipeline and budgets](README.md). Each slice file is its own contract. Where a slice names a spike or gate (G0, S2, S5, S3, S6), answer that question on the way with the smallest experiment that settles it and record the verdict in the slice; nobody else will run it.

## Ownership

| This lane owns | Others own (stay out) |
|---|---|
| `packages/scene-assets/` building scripts, bake and library code; building sources and bundles under `assets/` | `crates/mapgen/`, `crates/sim/`, `crates/contract/` except template fields this lane needs |
| `fixtures/building-templates.json` and `fixtures/prototype-building-templates.json` (the catalogue the generator reads) | `fixtures/maps/`, `fixtures/map-presets.json`, `fixtures/encounters.json`, `fixtures/game.json` rules |
| `packages/battle-renderer/src/models/` and the building passes | `packages/battle-renderer/src/terrain/`, `scenery/` (the [ground lane](ground-lane.md)) |
| This file's Status section | `README.md`'s Next Agent Prompt and TODO; the menu and preparation code |

The frame function that orders every pass is shared with the ground lane: add a pass there in a small commit of its own. A change to a template's physical shape changes what the generator and the simulation build, so say so in Status when the catalogue's hash moves. No repo-wide renames.

## Work, in order

1. **Sources and the kit:** [C10 third-party sources](slices/C10-third-party-sources.md) → [C11 kit modules](slices/C11-kit-modules.md) → [C12 baked materials](slices/C12-baked-materials.md).
2. **Templates the generator can place:** [C13 placement bake](slices/C13-placement-bake.md) → [C32 template library](slices/C32-template-library.md). From here the generator's catalogue is the real library, not the prototypes.
3. **Draw them at scale:** [C22 placement chunks](slices/C22-placement-chunks.md) → [C23 far tier](slices/C23-far-tier.md). Maps reach 16,000 buildings. Step 3 can start on the prototypes while step 2 is in progress.
4. **Category coverage:** [C16 farmstead](slices/C16-farmstead.md) · [C17 detached home](slices/C17-detached-home.md) · [C18 tower](slices/C18-tower.md) · [C19 industry](slices/C19-industry.md), then [C37 house appearance](slices/C37-house-appearance.md) for the village's existing houses.
5. **Facades:** [C21 material transport](slices/C21-material-transport.md) → [C24 cutout](slices/C24-cutout.md) → [C25 glass](slices/C25-glass.md) → [C15 interior atlas](slices/C15-interior-atlas.md) → [C26 interiors](slices/C26-interiors.md).
6. **Destruction:** [C14 damage placements](slices/C14-damage-placements.md) → [C27 ruin and gutted art](slices/C27-ruin-gutted-art.md), over the lifecycle the simulation already has (C42, C43).

## How to work

Read [`AGENTS.md`](../../AGENTS.md). Load the `renderer` skill before renderer work. Art is programmatic: Blender scripts and code in the repo, reproducible from a clean checkout; nothing hand-edited.

This is the focused model pass: the models are judged by looking. For every model or look change, render it in the real lab at the camera a player uses (default tactical and close), get an unprimed `screenshot-critique`, compare against the references under `assets/reference/` with `compare-screenshots`, and show the user with `preview-shots`. The user gives feedback on pictures directly in this session; a checkpoint never blocks, so decide on the evidence if they are silent and record it.

Test in proportion: the narrow tests for what changed and the one or two scenes it can move. No full gate. No frozen-record tests. Branch from main, merge main often, and push small green passes to main. Add an Outcome to each slice file and decisions to [`choices.md`](choices.md) under a heading for the slice. Scratch renders go in gitignored `throwaway/`.

**A saved map pins the catalogue.** `fixtures/maps/market-town` is a generated map saved against the prototype catalogue's hash, so it stops resolving, and the catalogue tests fail, when that file changes. Re-save it with the three commands in the [fixtures guide](../../fixtures/README.md) in the same commit that changes the catalogue.

**One GPU, shared.** Several sessions are working at once. Run every scene, render and asset sheet through the GPU lock in the main checkout (the README's Checks section has the command), never two at a time, and keep heavy jobs (a full Rust test run, a long sweep) to one at a time.

## Status

Update this section, not the README, at the end of each pass: what landed, what a town looks like now (one picture path), what is next, and anything the other lanes need to know.

**2026-10-01.** A town in the game is still massing boxes (`throwaway/evidence/generated/building-1920x1080.png` after `scene -- generated`), now at the real templates' sizes: nothing draws the library yet. The art exists as source sets and is judged so far in Blender reassembly sheets (`city/assemble.py`).

- **Landed:**
  - The three source files, vendored with their licence ([C10](slices/C10-third-party-sources.md#outcome)), and the export spike ([S2](spikes/S2.md)).
  - The source format every building script writes ([city kit readme](../../packages/scene-assets/blender/city/README.md)), and its bake side: a `kit` appearance, the packed template art library, the pure resolver, fit and coverage checks, and the prototype boxes as a generated stand-in set ([C13](slices/C13-placement-bake.md), [C32](slices/C32-template-library.md)).
  - China's apartment kit: five block templates, 74 modules, nine ambientCG sets pinned ([C11](slices/C11-kit-modules.md), [C12](slices/C12-baked-materials.md)).
  - Our own houses: five detached and five attached templates, and `city/kit.py`, the helper every scripted set is written with ([C17](slices/C17-detached-home.md)).
- **Decided:** China's family ships first; every category the graphs do not cover, attached homes included, is our own scripted source; graph-made sides are 3n + 2 metres with 3 m floors ([choices](choices.md#buildings-lane)).
  - One static chunk owner, `frame/staticChunks.ts`, with scenery, massing and corpses on it; frames pixel-identical before and after ([C22](slices/C22-placement-chunks.md#outcome), first half).
  - **The catalogue cutover, first part.** The generator's catalogue now holds the ten real house templates and five real apartment blocks in place of their boxes, and stand-in boxes for farmsteads, towers, industry and the two compound apartment blocks: 29 templates, all of family `china`.
- **Decided:** China's family ships first; every category the graphs do not cover, attached homes included, is our own scripted source; graph-made sides are 3n + 2 metres with 3 m floors ([choices](choices.md#buildings-lane)).
- **In flight:** drawing buildings from the library through the chunk owner (C22, second half), which is the first real town picture; farmsteads (C16), towers (C18), industry (C19); the interior atlas (C15).
- **Next:** the far tier (C23), compound apartment blocks (S5), the facade passes, damage, the village's houses (C37).
- **For the other lanes:**
  - **The catalogue's hash moved** to `5416684f…` and will move again as each remaining set replaces its stand-ins (three more times at most). In the same commit: `map-presets.json` `parcels.regional_families` is `["china"]` (the revision string is unchanged; bump it if a request must not carry over), the two `crates/mapgen` tests that named the `prototype` family read the catalogue's own, the map-layout and encounter parity records name the new hash and are re-recorded (no record changed status), and the camera lab's map uses the real 59 × 14 slab.
  - **Shapes changed:** houses are taller (the box top is the ridge: 5.2 m for one floor, 8.2 to 8.5 m for two, about 12 m for three) and apartment slabs are 35 × 11, 47 × 11, 59 × 14 and 53 × 14 m with a 1.1 m parapet in the box. A ten-seed sweep of every type and size still generates 90 of 90 maps; districts' `ground_m` was not retuned.
  - To add or retire a template: `asset catalogue`, `asset prototypes`, `asset bake` ([city kit readme](../../packages/scene-assets/blender/city/README.md), "From a set to a town"). `asset bake` and `asset check` need the WebAssembly built.
