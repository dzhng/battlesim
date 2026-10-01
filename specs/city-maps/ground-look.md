# Ground look: unknowns map (roads, rivers, forests, grass, farms, map catalogue)

Walked with the user on 2026-09-28. Retained ground appearance/rule decisions are givens for SG1–SG6, GG and C60–C87. The [completed procedural-map walk](procedural-maps.md) supersedes map scope/scale: all catalogued maps gain real urban/plain composition, generation uses fixed type/size presets, and full-extent memory/residency gates cover the ground lane. Sparse woodland varies coverage under the same forest rule. Rivers retain local shaping on initially flat generated ground. The references and their licences are in [`assets/reference/ground/SOURCES.md`](assets/reference/ground/SOURCES.md).

**Scope:** open-country ground on every map (roads, rivers, forests, grass, farms), plus the structure that catalogues maps. The village is the proving ground; city pavement stays in C28–C31.

## 1. Known knowns: settled ground

**References and what we may take:**

| Reference | Shows | Licence | Take |
|---|---|---|---|
| Forest road (Unity 6.6, driven by OpenAI's GPT-6 Astra model) | A dirt track fading into the forest floor, ruts, dense undergrowth, rocks and logs | No source | Look only |
| Selo Empire, "Manor Lords for the web" ([repo](https://github.com/SeloSlav/medieval-settlement-threejs)) | Soft worn paths, mottled meadow, sunken creeks | Code under a custom MIT wrapper; art all-rights-reserved | **Technique only:** feathered road shoulder (`src/roads/RoadMeshBuilder.ts`); per-vertex road-wear blend (`src/terrain/TerrainRoadWear.ts`); shore signed-distance field with noise (`src/rivers/organicShoreField.ts`); bank patches (`RiverBankMesh.ts`) |
| dryad ([repo](https://github.com/owenyuwono/dryad)) | Broken-up broadleaf crowns | MIT; WebGL GLSL, three.js r160; LOD not wired; leaves are alpha cards | **Technique only:** pipe-model branch radii, gravity droop, parallel-transport tube bark |
| Grassworks | Grass species presets | Commercial, closed | Look only (the species idea) |
| `~/dev/game` | Lobed crowns, a conifer and an aspen, pyramid rocks, one grass species, `mapCatalog.ts` | Our sibling project | **Technique:** conifer and aspen crown parameters; the `mapCatalog.ts` entry shape. Its rocks are too crude. Its crowns, grass and terrain are already ported (reuse-manifest `technique` entries) |

**Our ground today** (a code sweep of the files cited):
- **Fully procedural:** no albedo textures and no splat maps. Per pixel: plot colour → mottle → crop rows → verge → forest floor → shore → road → water bed → scars (`packages/battle-renderer/src/frame/terrainMaterial.ts:430-490`).
- **Roads:** one flat pale grey-tan (sRGB [0.46, 0.43, 0.39]) with a ~0.2 m feather at the sim rule's edge (`:479-484`). Battle-look chose "pale, reads as a line like WARNO's" and removed ruts because they read as a ghost outline (`specs/done/battle-look/decisions.md:90, :93`).
- **Grass:** GPU-compute clumps in 5 kinds (meadow, pasture, crop, wheat 0.95 m, stubble), 1–1.2 ms (`frame/grassPass.ts`, `terrain/grassField.ts`).
- **Trees:** solid lobed crowns, 3 species plus a hedge (`packages/scene-assets/blender/trees.py:48-70`). Leaf cards were rejected: they shimmer under 4× MSAA and need a discarding prepass (`specs/done/battle-look/choices.md:1433-1441`). The forest floor is litter, moss and humus with a sun dapple. **No undergrowth, rocks or logs.**
- **Forests in the sim:** each has `canopy_height_m` 12, `trunk_height_m` 10 and a density row (light, medium or dense: spacing 14/9/6 m, concealment, canopy radius) (`fixtures/game.json`).
- **Water:** axis-aligned rects, impassable, crossed at bridges only (`crates/sim/src/world/mod.rs:221-238`). The village has none.
- **Farms:** about 4,000 seeded split plots with palette jitter and crop rows (`terrain/plots.ts`). Only the summer biome exists.
- **Maps:** no category field. The village is `fixtures/game.json`; there are 11 `fixtures/*-lab.json` files and 25 entries in `apps/battle-lab/src/fixtures.json`, and `MainMenu.tsx` hand-lists Play, Watch, Benchmark and Labs.
- **Every drawn thing is a workbench appearance** (renderer `SKILL.md:184`).

## 2. Known unknowns: decision ledger

| # | Decision | Why | Closed by |
|---|---|---|---|
| Q-G1 | **This work folds into city-maps** as a ground lane, and the village is its proving ground. City-maps' village firewall gains an exception for **visual ground changes**. Village digests move only where a slice names it. | One spec shares the surface bake, biome and prop systems. | user |
| Q-G2 | **Roads get a wear gradient in the terrain.** C28's surface bake gains a distance-to-road channel. The terrain paints a packed-dirt core, then a worn shoulder 2–4 m wide each side with a noise-jittered edge, then grass. Grass clumps thin across the shoulder. The sim rule stays exactly the core. No road mesh. | One owner of "where the road is"; no transparent pass. | user |
| Q-G3 | **Soft ruts:** two slightly darker, sunken tracks inside the core, broken by noise, **fading out below ~2 px per rut**. Critique gate: "does the road read as an outline?" | Detail up close without battle-look's ghost-outline failure. | user |
| Q-G4 | **Two rural road kinds, in map data.** A *dirt track* is warm brown, with a grass centre strip on narrow tracks. A *country road* is gravel or asphalt grey with a crisper edge and a worn shoulder. The village's 12 m roads are country roads. Speed per kind stays in the sim, so the village keeps today's speeds. | Different roads read differently. | user |
| Q-G5 | **Rivers are centerline polylines** (width and depth per point) in `MapDefinition`, **replacing water rects on every map.** The sim carves a sloped bank into the height grid and classifies water by distance to the centerline. The renderer shades bed, wet bank, mud and grass from the same distance field, with no hard edge. Water stays impassable and is crossed at bridges only. **Minimum river width 12 m** (3 cells on the 4 m grid), checked by map validation. Banks get a gentle ramp at bridges. Narrow creeks wait for a finer height grid in a later spec. | Rivers meander, and one distance field serves sim and look. | user |
| Q-G6 | **The village gets no water.** Rivers are proven on a new `/lab/river` map. | Village layout and balance stay put. | user |
| Q-G7 | **Trees use a branch skeleton plus solid leaf clumps** in our Blender tree script: pipe-model branches, droop, bark furrows (dryad's technique), with leaf clusters as solid geometry, not alpha cards. The LOD tiers stay, and the solid-crown far tier is kept for distance. | Broken-up crowns without shimmer or a prepass. | user |
| Q-G8 | **Species, as much variety as possible:** conifers (spruce and pine), the 3 broadleaf kinds reworked, birch and saplings, and dead trees (standing snags plus fallen trunks). We port only `~/dev/game`'s conifer and aspen crown parameters, as technique; everything else is built new. | Variety; `~/dev/game` has no birch, dead trees, logs or undergrowth. | user |
| Q-G8b | **One forest rule.** Every forest has the same density and canopy height. Trees are all **about the same height and trunk thickness.** Variety comes only from shape and colour (conifer against broadleaf, crown shape, leaf tone, birch bark, the odd dead snag). The village's "light" forest becomes the one rule (medium), a **named village digest change.** | Simple to play and draw; the art can't lie about sight. | user |
| Q-G9 | **Saved map catalogue:** every materialized map (playable, lab, benchmark, test) lives in `fixtures/maps/<id>/` with `map.json`, `SOURCES.json`, `meta.json` and `encounters/<name>.json`. The `meta.json` fields are: `category` (playable, lab, benchmark or test); `status` (draft, released or retired); `label`, `character`, `biome`, `size_m`, `tags`; `source` (imported, generated or authored); `seed`, `encounters`, `benchmarks`. A schema test enforces it. Saved listings come from the catalogue; C55 transient runtime maps use equivalent immutable identity through the same resolver and need no persistent folder. The menu, lab app, benchmark and scene runner have no hand-kept saved map lists. The labs move out of `fixtures/*-lab.json`. | Many maps are coming, and per-folder metadata avoids merge conflicts across worktrees. | user |
| Q-G10 | **Grass species presets** (blade shape, height, width, colour ramp, clumping, wind response) as clump-model variants from our own generator. Wild ground: short meadow, tall rough grass, dry prairie, weedy verge. **Crops:** wheat, barley, rapeseed, hay and stubble, pasture, ploughed or fallow. **Every crop and grass is ≤0.9 m (half a soldier)**, and there's no maize. Each plot kind picks its crop in the biome. | A Broken Arrow patchwork of crops, not recoloured lawn. Short crops imply no concealment buff and are easy to draw. | user |
| Q-G11 | **Crops are visual only.** Concealment in crops is a later rule pass. | | user |
| Q-G12 | **Forest floor:** fallen logs and large boulders are **sim bodies** (they stop rounds, give cover, block vehicles), placed sparsely by the map. Ferns, bushes, saplings, small rocks and litter are **presentation-only dressing** (workbench models) inside forests. Concealment stays owned by the forest. | Cover you can see is real, without thousands of bodies. | user |
| Q-G13 | **Fields are desaturated and textured** (olive, tan, brown), with texture inside each field. | Broken Arrow's farm read. | user |
| Q-G14 | **Grass varies within a field:** colour, height and clumping. | Not a mown carpet. | user |
| Q-G15 | **Tree lines between fields are optional per map.** Inside the playable area they're **thin forest strips in map data** under the one forest rule, so they block sight as drawn. Past the map edge they stay backdrop decoration as today. | A tree line you can see must be real to the sim. | user (optional); the sim-real placement was disclosed to the user |
| Q-G16 | **Forests read dense but looser than Broken Arrow's.** The canopy mostly closes, but you can still see down to units and the floor. | Readability under trees. | user |
| Q-G17 | **Tuned and gated at the default battle camera (65–250 m).** The 25 m ground view must hold up. Fine detail (ruts, ferns, blades) fades out gracefully with distance. | That's where fights are played. | user |
| Q-G19 | **Nothing reads blocky.** Road and river centerlines are **splines, densified to points ≤2 m apart by the contract's loader**, so the sim rule and the shading follow the same round curve. Shading reads the **exact distance field**, never a grid cell. Carved banks stay **gentle**, and bank shading hides the 4 m flat-shaded terrain facets. Critique gate on every curve: "does any road, river or bank read as blocky or stepped?" Otherwise the simplest solution for now (user). | A turning river or road must be round. | user |
| Q-G18 | **Summer only.** Palettes and species live in the biome, so later seasons are biome files, not code. | | user |

## 3. Unknown knowns: what the user revealed

- **The road "grey blob" is the pain point.** Transitions (road to grass, ground to river) matter more than the hero objects.
- **Farms matter:** most maps will be open farmland like Broken Arrow's, so grass and crops are core, not polish.
- **Keep it simple.** One forest rule and one tree size, with variety in looks only. Pushback on over-engineering is a signal to cut sim coupling, not add it.
- **Variety is good:** more species, more grass kinds.
- **Many maps are coming,** including test and benchmark ones, so they need a catalogue from day one.
- **No false buffs:** nothing may look like it grants something the sim doesn't (hence crops ≤0.9 m).

## 4. Unknown unknowns: landmines

| # | Finding | Evidence | Status |
|---|---|---|---|
| L-G1 | Tree height and density are sim data, and art that differs lies about sight. | `game.json` `map.forests`, `forests.densities` | **Decided:** Q-G8b (one rule, one size) |
| L-G2 | A narrow creek on the 4 m grid comes out jagged, and steep banks go untraversable. | `height_grid_m` 4, `slope_cutoff_deg` 35 | **Decided:** Q-G5 (≥12 m, bridge ramps) |
| L-G3 | The avoid-list bans a **dark wet shore** and a **darker, wider road verge**: they read as shadow or fog. | `.agents/skills/renderer/references/fog-and-light.md` ("Avoid these patterns") | Sharp edge: shoulder and mud at or above the grass's luminance, differing by hue. The standing critique question gates it |
| L-G4 | Desaturating fields toward brown lowers luminance, which can break the "darkest seen ground stays lighter than, or apart in hue from, unseen ground" scene check. | `fog-and-light.md` (`light.shadow_floor`) | Sharp edge: that scene check gates C71 |
| L-G5 | Thousands of dressing pieces per forest can't use the per-model draw path. | `modelLayer.ts` per-instance palette | **Corrected in synthesis:** dressing rides the scenery layer's existing chunks (`frame/sceneryLayer.ts`), which C22 promotes into the one static-chunk owner (G-R1). SG1 measures the cost |
| L-G6 | Branch-skeleton trees add triangles on hundreds of trees (one per sim trunk). | `scenery/placement.ts` | Sharp edge: a per-tier triangle budget, the solid far tier kept, a frame-cost row |
| L-G7 | ~~Grass species multiply indirect draws~~ | `frame/grassPass.ts:21` | **Corrected in synthesis:** grass is already one draw per tier over every kind. The real limits are 8-blade padding, 16 kinds and 16 growth rows, and one appearance per row; C82 fixes the last |
| L-G8 | Replacing water rects changes `geometry-lab.json` and `movement-lab.json`. | fixtures | Sharp edge: named; the village has no water |
| L-G9 | The catalogue cutover touches 11 lab fixtures, 25 lab-app entries, web scene ids and `MainMenu.tsx`, and overlaps hud-chrome's planned typed router. | `apps/battle-lab/src/fixtures.json`, `web/scene.mjs` | Sharp edge: C09 moves every map at once; C60 (data) and C61 (listings) follow; SG6 dry-runs it; scene ids stay stable; routes stay separate from maps |
| L-G10 | Selo Empire's code licence is a custom wrapper, and dryad is WebGL GLSL. | their repos | Sharp edge: both are **technique-only** manifest entries, rewritten from reading |
| L-G11 | Tree lines in the playable area must block sight as drawn. | first-principles rule | **Decided:** Q-G15 (thin forest strips in map data) |

**Slices:** spikes SG1–SG6, gate GG and C60–C87 in `slices/`, from a four-draft synthesis (see `decisions.md`, "Ground synthesis"). Refinements made in synthesis:
- saplings are dressing only;
- fallen trunks are log bodies only;
- forest bodies are generated by the forest rule;
- the catalogue lists maps, and routes stay separate;
- rounding the corners moves the village digest (C65).
