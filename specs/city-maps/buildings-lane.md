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

**2026-10-01.** A town in the game is still massing boxes (`throwaway/evidence/generated/building-1920x1080.png` after `scene -- generated`), at the real templates' sizes: the pass that draws the library is in flight. The art is judged so far in Blender reassembly sheets (`city/assemble.py`).

**Every category has real art.** The generator's catalogue is 29 templates, all `release`, all of family `china`, and no stand-in box remains:

| Category | Templates | Set, and its slice |
|---|---|---|
| Urban apartment | Four slabs, a point block, a U block and a courtyard block | `china_apartments`, from the vendored graph ([C11](slices/C11-kit-modules.md), [C12](slices/C12-baked-materials.md), [S5](spikes/S5.md)) |
| Detached and attached home | Five houses; a townhouse, two terraces, a shop row, a corner shop | `homes` ([C17](slices/C17-detached-home.md)) |
| Farmstead | Three farms | `farmsteads` ([C16](slices/C16-farmstead.md)) |
| Highrise | A ten-floor slab and three point towers | `towers` ([C18](slices/C18-tower.md)) |
| Industry | A shed, two warehouses, a works, a depot | `industry` ([C19](slices/C19-industry.md)) |

**Also landed:**
- The three source files with their licence ([C10](slices/C10-third-party-sources.md#outcome)) and the export spike ([S2](spikes/S2.md)).
- The source format every building script writes ([city kit readme](../../packages/scene-assets/blender/city/README.md)), and its bake side: a `kit` appearance, the packed template art library, the pure resolver, fit and coverage checks ([C13](slices/C13-placement-bake.md), [C32](slices/C32-template-library.md)).
- One static chunk owner, `frame/staticChunks.ts`, with scenery, massing and corpses on it ([C22](slices/C22-placement-chunks.md#outcome), first half).
- Material coverage (opaque, cutout, blended) and the interior sheet a room surface names, in bundle format 4 ([C21](slices/C21-material-transport.md)).
- The interior atlas: ten rooms and ten shops, dim and daylight-only ([C15](slices/C15-interior-atlas.md)).

**Decided:** China's family ships first, and New York and Paris are later families through the same exporter, not part of closing this lane; every category the graphs do not cover is our own scripted source; a join between parts is never built, the outline of the abutting boxes is ([choices](choices.md#buildings-lane)).

**In flight:** drawing buildings from the library through the chunk owner (C22, second half).

**Next, in order:** the far tier (C23); cutout, glass and interiors (C24 to C26); damage states and their art (C14, C27); the village's houses (C37).

**For the other lanes:**
- **The catalogue's hash is `6b0a5e8b…` and should now hold still.** It moves only if a template's physical shape changes. Each move needs, in one commit: the parity records' requests renamed to the new hash and re-recorded, `fixtures/maps/market-town` saved again (the three commands in the fixtures guide), and `fixtures/camera-lab.json` if it names a retired template.
- **Shapes against the old boxes.** Houses are taller (the box top is the ridge: about 5.2 m for one floor, 8.3 m for two, 12 m for three). Apartment slabs are 35 × 11, 47 × 11, 59 × 14 and 53 × 14 m with a 1.1 m parapet in the box; the courtyard block is 41 × 38 m. Towers are whole bays (56 × 14, 26 × 26, 23 × 32, 29 × 29 m). Industry is whole bays too (15 × 24, 48 × 24, 72 × 33, 90 × 39, and a works of a 54 × 27 hall with an 18 × 9 office on its street face, both 8 m). Farm buildings keep their plans. Districts' `ground_m` was not retuned.
- **Some bays have no opening** (barns, warehouses, a tower's blank columns): a soldier seated there fires through a drawn wall. The simulation seats every bay; whether to mark such bays is a rules question nobody has taken.
- `map-presets.json` `parcels.regional_families` is `["china"]`; its revision string was not bumped.
- To add or retire a template: retire its catalogue row, then `asset prototypes`, `asset catalogue`, `asset prototypes`, `asset bake`, `asset check` ([city kit readme](../../packages/scene-assets/blender/city/README.md), "From a set to a town"). `asset bake` and `asset check` need the WebAssembly built.
