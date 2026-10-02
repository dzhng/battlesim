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

### Current integration triage

The closeout run below is historical. A focused street run passes all five checks with the current native validator and Wasm built from matching source; its first HTTP refusal came from an old validator missing the movement-rehearsal input. The hypothesized rural-road classification defect did not reproduce, so this triage changes no terrain code.

The movement scene now waits within its existing planning bound before inspecting each admitted route. It retains a completed route while the other squad plans, then binds both route and actual arrival publications to every acknowledged actor and its own destination. This avoids empty-array success and proves arrival at the published float32 marker without adding a distance tolerance. All 13 checks pass on the integrated build, including concurrent weapon changes. Retained original failures, publications, guard falsifications and current evidence live in ignored `throwaway/integration-triage/` in the main checkout.

Current field texture checks pass all four original conditions without changed thresholds. Panel layout passes after replacing a stale wait for an unselected card to disappear with the documented visible compact-card contract; leaving hover must close detail and clear hover state. Its original layout/hover/command checks remain. The boundary check now tests neutral stored paint and visible finished-frame contrast: grey paint can lower red over a yellow field, so positive colour rises were an invalid neutrality test. All 13 boundary samples pass; controls with missing paint or unchanged finished frames fail. The renderer is unchanged.

The diagnostic ballistics and weapons fixtures now stage the current physical flight: the low arc is blocked by the existing crest, moving bodies meet the solved flight time, and the hidden firing report is within grenade reach while the identified control is within both mounts' reach. The integrated scenes pass all 17 ballistics and nine original weapons checks without gameplay tuning. Their map identity changes intentionally. Retained red/green and guard controls live in `throwaway/integration-triage/lab-staging/`.

The lab panel scrolls within the viewport, including at 800 × 600, and every feed row is reachable without a tolerance. Four added checks pass alongside the original nine weapons checks. Crossing captures use the authoritative near-miss/impact events rather than obsolete frame numbers. Paired captures and a fresh image review confirm this bounded diagnostic improvement; the review still finds tiny actors, weak distant arcs and ambiguous diagnostic marks. These remain visual limitations, not accepted battlefield art. Evidence is in `throwaway/integration-triage/lab-panel/`.

Village-watch now checks the approach acknowledgement before searching for contact reports. The old centre destination refused all nine actors; the west approach admits five and refuses four, yielding the actual last-seen and firing reports. Four focused checks pass, including preferred label IDs and ground-centre anchors at both zooms. This is capture staging, not a contact-style fix: the fresh critic and user find the overlapping reports hard to read. The white outline marks last-seen evidence; the offset borderless patch marks firing evidence from the same tank. Both areas are drawn but only the preferred report is labelled. Small labels and dominant fog stripes remain visible. Evidence and controls live in `throwaway/integration-triage/contact-capture-review.md` and the village-watch captures.

The scripted village play tour passes all 17 checks on the current equipment-integrated build. Its far-card check follows the documented compact, depth-ordered overlap contract while retaining HUD clearance. The sound approach waits for its own settled acknowledgement and requires actual placement before waiting for heard cues; restart tolerates the route's legitimate remount. Missing cards, expanded far cards, HUD overlap, refused/empty placement and a previous command's acknowledgement all fail the extracted guards. Evidence lives in `throwaway/integration-triage/village-play-settled-ack.log` and `village-play-guard-controls.mjs`. This is focused village play evidence, not the generated encounter, landscape look or whole-spec gates.

### Buildings checkpoint

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
- The source format every building script writes ([city kit readme](../../packages/scene-assets/blender/city/README.md)), and its bake side: a `kit` appearance, the packed template art library over two catalogues, the pure resolver, fit and coverage checks ([C13](slices/C13-placement-bake.md), [C32](slices/C32-template-library.md)).
- One static chunk owner, `frame/staticChunks.ts`, with scenery, corpses and buildings on it, and buildings drawn from the library with bounded residency ([C22](slices/C22-placement-chunks.md#outcome)).
- The far tier is each template's own coarsest tier: no tile builder, and a whole frame stays under 11 ms on a Metro Large ([C23](slices/C23-far-tier.md#outcome)).
- **`/lab/city-lineup` is the picture art is judged by**: every template on flat ground at any tier, state and station, with a scene that holds each to its parts and fit.
- **One way to draw a building.** The village's and the labs' houses are templates of the library too, and the fitted house path is deleted ([C37](slices/C37-house-appearance.md)).
- Material coverage (opaque, cutout, blended) and the room sheet a surface names, in bundle format 4 ([C21](slices/C21-material-transport.md)); the model layer draws all three ([C24](slices/C24-cutout.md#outcome), [C25](slices/C25-glass.md#outcome), [C26](slices/C26-interiors.md#outcome)), judged in `/lab/facade`. Rooms draw at tiers 0 and 1. The interior atlas is ten rooms and ten shops, dim and daylight-only ([C15](slices/C15-interior-atlas.md)).
- **Every template has its damage state**: `ruin` for six floors or fewer, `gutted` above, held by the bake to the simulation's own collapse rule ([C14](slices/C14-damage-placements.md)). The same pass made the coarse tiers keep their openings and colours and rewrote the roofs.
- The China kit uses all three facade features: glass in windows and enclosed balconies, a room behind every window and shop front, and cages, rails, rain streaks and leaves as cutouts ([C11](slices/C11-kit-modules.md), [C25](slices/C25-glass.md)).
- **A building a side has seen destroyed is drawn destroyed**, from what the simulation published about it, with smoke; the other side sees it intact until it learns. `/lab/city-ruins` shells a block and a tower while one side watches ([C27](slices/C27-ruin-gutted-art.md)).
- **A page fetches only the kits it draws** ([choices](choices.md#buildings-lane)). A town still fetches more than the 50 MiB kit budget: see For the other lanes, below.

**Decided:** China's family ships first, and New York and Paris are later families through the same exporter, not part of closing this lane; every category the graphs do not cover is our own scripted source; a join between parts is never built, the outline of the abutting boxes is ([choices](choices.md#buildings-lane)).

**The lane's work is done.** Every slice in "Work, in order" has its Outcome. The scripted sets (homes, farmsteads, towers, industry) have glass and rooms like the apartment kit ([C26](slices/C26-interiors.md#outcome)). The whole-lane review has run: one owner for what a side knows of a map's buildings, shared scene probes, one ruin-height rule, the empty prototype set retired (what remains of it is the unit box a prop with no model is drawn as, `asset stand-in`), and the per-set Blender sheet scripts deleted. The decisions are one consolidated ledger, with the known shortfalls listed apart from the choices ([choices](choices.md#buildings-lane)).

**Open look questions**, recorded in their slices: glass adds almost no cue that a pane is there (C25); a gutted tower reads from far off as a darker intact tower, and its smoke carries the read at mid range (C27, C14); identical neighbours and unreadable facing in a town (the critic's table below).

**What an unprimed critic saw in the first real town** (eight frames of `/lab/city-block`, 2026-10-02), and where each finding went:

| Finding | Where it is being handled |
|---|---|
| Neighbouring buildings run into each other: a roof through a roof, an apartment block's stair house in a house | The map: the parcel pass places some buildings overlapping (the boxes overlapped too). For the map lane, below |
| A fallen building is a flat brown slab | Fixed: every template has ruin or gutted art (C14), drawn by knowledge (C27) |
| Buildings stand on bare lawn: no pavement, yards, fences or paths to doors | The ground lane (streets, C28 to C30) and street placement (C46) |
| Apartment roofs are the most saturated thing on screen and tile visibly; a dark ground storey reads as sunk in shadow; roof stains repeat as dots | Fixed in the China kit: a duller clay roof with large sparse stains, a mid-grey plinth (C14's pass) |
| Pitched roofs read as tartan from above | Fixed: the roof recipes are small staggered tiles and a neutral slate, with stains in the roof's own paint (C14's pass) |
| Opaque stand-in glass is charcoal on one face and pale on the next; doors do not read | [C25](slices/C25-glass.md#outcome): blended glass bounds what a pane mirrors and stays 0.25 to 0.42 of its wall under four suns. It lands on the town when the kits take it. Open there: a pane reads by darkening alone |
| Fine detail (window cages, rails) aliases at the tactical camera; the whole-map view does not read as a town | [C23](slices/C23-far-tier.md#outcome): the thresholds stay, the cages and rails go to a cutout texture (C24), and the overview needs a built-up ground tint under settlements, which is the ground lane's |
| Identical neighbours; facing is hard to read from above | Open. A wall-colour palette per building and asymmetric roof details would help; neither is built |
| A building's shadow on a road reads as a second road material; bands and blotches on lawns | The ground lane and the light: not buildings |

**For the other lanes:**
- **The catalogue's hash is `6b0a5e8b…` and should now hold still.** It moves only if a template's physical shape changes. Each move needs, in one commit: the parity records' requests renamed to the new hash and re-recorded, `fixtures/maps/market-town` saved again (the three commands in the fixtures guide), and `fixtures/camera-lab.json` if it names a retired template.
- **Shapes against the old boxes.** Houses are taller (the box top is the ridge: about 5.2 m for one floor, 8.3 m for two, 12 m for three). Apartment slabs are 35 × 11, 47 × 11, 59 × 14 and 53 × 14 m with a 1.1 m parapet in the box; the courtyard block is 41 × 38 m. Towers are whole bays (56 × 14, 26 × 26, 23 × 32, 29 × 29 m). Industry is whole bays too (15 × 24, 48 × 24, 72 × 33, 90 × 39, and a works of a 54 × 27 hall with an 18 × 9 office on its street face, both 8 m). Farm buildings keep their plans. Districts' `ground_m` was not retuned.
- **The library covers two catalogues now**: the generator's and the authored maps' (`fixtures/building-templates.json`); a set names which in `assets/catalog.json`. A building whose template has no art is refused (`template.missing`), no longer drawn another way.
- **`contract::catalog::PropAppearance` lost `remains_state`** (and the fixture rows with it): a presentation field only the deleted house path read. Digests are unchanged.
- **The one full run, at closeout (2026-10-02).** `check` passes: format, lint, typecheck, every Rust test and 911 web tests. `verify` ran every scene: 433 checks pass and 19 fail, none in a scene that judges buildings (`city-block`, `city-lineup`, `city-ruins`, `facade`, `generated` and the camera lab pass). The 19, each the same when its scene is run alone on main merged in: `ground` (seven: the town street's pavement, walk, kerb and paint, and the plain's fields; those stations are shot with models off, so no building is in them), `movement` (the rifle group's routes through the 5 m gap) and `village` (the scripted fight, two panel-layout checks), which the ground lane's status already traces to the scale lane's merge `82564693`, `ballistics` (the crest shot and the crossing tank), `weapons` (the grenade's area) and `village-watch` (the playable area's border, a labelled contact).
- **A page downloads only the kits it draws from** ([choices](choices.md#buildings-lane)): the village 38.8 MB, a lab with no buildings none, and a block of a generated town 94 MB (119 MB with a tower in it: apartments 43.5, towers 24.7, industry 19.4, farmsteads 16.1, homes 15.4). That is over the spec's 50 MiB for a shared kit download; glass and rooms grew the kits and nothing has shrunk them. A route that draws buildings takes its appearances from `useMapAppearances`; the bare catalog holds no kit. Every page still downloads about 240 MB that is not kits (soldiers, scenery, vehicles) before it starts, which nobody has taken.
- **Map lane: some buildings overlap.** In Mixed Small seed 1, near the main town's centre, neighbouring buildings' boxes overlap by metres (two corner shops through each other, an apartment block into a house). Boxes hid it; real roofs do not. Art may also reach past a part's faces by its set's `fit.side_m` (0.5 m for houses, up to 1.5 m for apartment balconies and tower canopies), so two buildings need at least the sum of their side fits between their boxes unless they are one template's joined parts.
- **Some bays have no opening** (barns, warehouses, a tower's blank columns): a soldier seated there fires through a drawn wall. The simulation seats every bay; whether to mark such bays is a rules question nobody has taken.
- **Ground lane: a town does not read from far off.** From 4.5 km out to the whole map a house is a pixel or less and most of a town's ground is lawn the colour of a field, so only the road grid says "town" ([C23](slices/C23-far-tier.md#outcome)). No building tier can fix that; a built-up tint under settlements would.
- `map-presets.json` `parcels.regional_families` is `["china"]`; its revision string was not bumped.
- To add or change a template: change its set, then `asset catalogue`, `asset bake`, `asset check` ([city kit readme](../../packages/scene-assets/blender/city/README.md), "From a set to a town"). `asset bake` and `asset check` need the WebAssembly built.
