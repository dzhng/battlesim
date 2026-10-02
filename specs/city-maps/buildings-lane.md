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

**2026-10-02. A generated town is real buildings.** Every building is drawn from the template art library through the static chunk owner; massing is gone. The picture is `throwaway/evidence/city-block/wide-1920x1080.png` after `scene -- city-block` (`/lab/city-block` is a block of a generated town with no battle). Buildings cost 0.4 to 4.1 ms of GPU from the tactical camera to the whole map on a Metro Large, the most in the two views with a horizon ([C22](slices/C22-placement-chunks.md#outcome), [C23](slices/C23-far-tier.md#outcome)).

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
- One static chunk owner, `frame/staticChunks.ts`, with scenery, corpses and buildings on it, and buildings drawn from the library with bounded residency ([C22](slices/C22-placement-chunks.md#outcome)).
- The far tier is each template's own tier 3: no tile builder ([C23](slices/C23-far-tier.md#outcome)). `/lab/city-lineup` is the picture a set is judged by: every template on flat ground at any tier, state and station, with a scene that holds each to its parts and fit and writes a sheet per category at each tier and each template across each tier boundary (`throwaway/evidence/city-lineup/`).
- Material coverage (opaque, cutout, blended) and the interior sheet a room surface names, in bundle format 4 ([C21](slices/C21-material-transport.md)).
- The interior atlas: ten rooms and ten shops, dim and daylight-only ([C15](slices/C15-interior-atlas.md)).

**Decided:** China's family ships first, and New York and Paris are later families through the same exporter, not part of closing this lane; every category the graphs do not cover is our own scripted source; a join between parts is never built, the outline of the abutting boxes is ([choices](choices.md#buildings-lane)).

**In flight:** cutout, glass and interiors in the model layer (C24 to C26); damage states for all five sets (C14); the village's houses onto the library (C37).

**Next:** the coarse tiers' art, set by set, from [C23's list](slices/C23-far-tier.md#what-the-coarse-tiers-must-keep-per-set) (tower windows that change colour at 128 m, apartment balconies that turn cream at tier 2, houses and farms with blank walls at tier 3); glass, rooms and cutouts restored in the kits once C24 to C26 land; ruin and gutted art drawn by knowledge, with the far tier's variants (C27).

**What an unprimed critic saw in the first real town** (eight frames of `/lab/city-block`, 2026-10-02), and where each finding went:

| Finding | Where it is being handled |
|---|---|
| Neighbouring buildings run into each other: a roof through a roof, an apartment block's stair house in a house | The map: the parcel pass places some buildings overlapping (the boxes overlapped too). For the map lane, below |
| A fallen building is a flat brown slab | C14 and C27, in flight |
| Buildings stand on bare lawn: no pavement, yards, fences or paths to doors | The ground lane (streets, C28 to C30) and street placement (C46) |
| Apartment roofs are the most saturated thing on screen and tile visibly; a dark ground storey reads as sunk in shadow; roof stains repeat as dots | The China kit's pass, in flight |
| Pitched roofs read as tartan from above | The shared roof recipe, in the scripted sets' pass, in flight |
| Opaque stand-in glass is charcoal on one face and pale on the next; doors do not read | Glass (C25), in flight |
| Fine detail (window cages, rails) aliases at the tactical camera; the whole-map view does not read as a town | [C23](slices/C23-far-tier.md#outcome): the thresholds stay, the cages and rails go to a cutout texture (C24), and the overview needs a built-up ground tint under settlements, which is the ground lane's |
| Identical neighbours; facing is hard to read from above | Open. A wall-colour palette per building and asymmetric roof details would help; neither is built |
| A building's shadow on a road reads as a second road material; bands and blotches on lawns | The ground lane and the light: not buildings |

**For the other lanes:**
- **The catalogue's hash is `6b0a5e8b…` and should now hold still.** It moves only if a template's physical shape changes. Each move needs, in one commit: the parity records' requests renamed to the new hash and re-recorded, `fixtures/maps/market-town` saved again (the three commands in the fixtures guide), and `fixtures/camera-lab.json` if it names a retired template.
- **Shapes against the old boxes.** Houses are taller (the box top is the ridge: about 5.2 m for one floor, 8.3 m for two, 12 m for three). Apartment slabs are 35 × 11, 47 × 11, 59 × 14 and 53 × 14 m with a 1.1 m parapet in the box; the courtyard block is 41 × 38 m. Towers are whole bays (56 × 14, 26 × 26, 23 × 32, 29 × 29 m). Industry is whole bays too (15 × 24, 48 × 24, 72 × 33, 90 × 39, and a works of a 54 × 27 hall with an 18 × 9 office on its street face, both 8 m). Farm buildings keep their plans. Districts' `ground_m` was not retuned.
- **Map lane: some buildings overlap.** In Mixed Small seed 1, near the main town's centre, neighbouring buildings' boxes overlap by metres (two corner shops through each other, an apartment block into a house). Boxes hid it; real roofs do not. Art may also reach past a part's faces by its set's `fit.side_m` (0.5 m for houses, up to 1.5 m for apartment balconies and tower canopies), so two buildings need at least the sum of their side fits between their boxes unless they are one template's joined parts.
- **Some bays have no opening** (barns, warehouses, a tower's blank columns): a soldier seated there fires through a drawn wall. The simulation seats every bay; whether to mark such bays is a rules question nobody has taken.
- **Ground lane: a town does not read from far off.** From 4.5 km out to the whole map a house is a pixel or less and most of a town's ground is lawn the colour of a field, so only the road grid says "town" ([C23](slices/C23-far-tier.md#outcome)). No building tier can fix that; a built-up tint under settlements would.
- `map-presets.json` `parcels.regional_families` is `["china"]`; its revision string was not bumped.
- To add or retire a template: retire its catalogue row, then `asset prototypes`, `asset catalogue`, `asset prototypes`, `asset bake`, `asset check` ([city kit readme](../../packages/scene-assets/blender/city/README.md), "From a set to a town"). `asset bake` and `asset check` need the WebAssembly built.
