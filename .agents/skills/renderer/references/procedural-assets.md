# Procedural assets

Every model and texture is **made by code**. The user likes this and wants it kept:
- headless Blender scripts build the geometry and bake vertex colour;
- numpy recipes build tileable textures;
- the bake packs both into content-addressed bundles.

No hand-authored or downloaded art, except third-party sources with a known source and a licence that allows shipping. [`assets/README.md`](../../../../assets/README.md) holds the one list of them, with each one's source and licence; a new one is added there in the commit that brings it. Packs that are not committed are read from a local cache by `blender/packs.py`, each pinned by hash.

Owners:
- `packages/scene-assets/src/`: `codec.ts` (the bundle), `validate.ts`, `loader.ts`, `appearanceCatalog.ts`, `scenery.ts`;
- `packages/scene-assets/blender/`: `parts.py`, `textures.py`, one script per subject, and `build_sources.sh`; `city/` holds the buildings' scripts, with a readme of its own for their source format;
- `assets/catalog.json`;
- `web/asset.mjs`: the `asset` CLI.

The scene-assets README is the manual; this page is the lessons.

Drive Blender through a Blender MCP server when one is installed, e.g. to inspect a scene or try a change interactively. Land the result as a script change, because the scripts are the source of truth and the `asset` CLI runs them headless.

## The pipeline, in one breath

1. The Blender script (pinned version, `BLENDER` env override) builds the subject from `parts.py` primitives, one mesh per LOD tier.
2. `finish()` bakes paint, edge wear, grime and ray-cast AO into vertex colour.
3. `textures.py` embeds the albedo, normal and ORM PNGs.
4. It exports a GLB into `assets/source/**`, which is LFS.
5. `asset bake` makes the bundle (magic `BGAB`, a canonical JSON header and an aligned binary body) at `assets/runtime/<sha256>/bundle.bin`, mapped by name in `assets/runtime/catalog.json`.
6. `asset check` validates it against the fit authority (`src/authority.ts`: the fixture's soldier frame and each unit type's resolved catalog numbers), within the catalog's `tolerances`.
7. `asset sheet <name>` renders workbench views headlessly for review; `--accept` copies them to `assets/review/`.

A new **unit type** (vehicle or infantry) starts in the unit catalog, in [`fixtures/README.md`](../../../../fixtures/README.md): mounts, muzzles, fit checks, and what presentation anchors to.

## Adding a prop the battle draws

The fence, sandbags and dragon's teeth are worked examples:
1. Add the simulation's prop type (a `props` entry under `fixtures/props/`: its body row and its `appearance` binding; see [`fixtures/README.md`](../../../../fixtures/README.md)).
2. Add a `props.py` kind, authored to that box, with its origin at the box centre on the ground.
3. Add its line to `build_sources.sh`, then run `asset blender` and `asset bake`.
4. Add a catalog entry (`unit: "scenery"`, `states`, `basis_yaw_deg`, `footprint_half_m`).
5. Add a `SCENERY_KINDS` row.
6. Run `asset check`, the catalog footprint test and a sheet.
7. **Bind it in the prop type, not the renderer:** its `appearance.drawn_by` names the scenery kind, and `modular` makes it repeat along its box instead of stretching. The renderer reads the binding from the world layout (`propAppearance`); a type whose binding names a scenery kind with no appearance draws as nothing, and the catalog footprint test catches it.

## What made models read at RTS distance

- **Vertex colour alone read "toy-like"** in every critique round. Splitting faces for camo doubled the triangle count and still looked soft. **Textures fixed it:**
  - albedo, normal and ORM at 256 px RGBA8, with box-filtered mips;
  - periodic numpy noise (tileable, fixed seeds), with `box_uv` in metres so texel density matches across parts;
  - albedo alpha is a wear threshold; ORM alpha is the side-tint mask, so one texture serves both sides.
  
  Texture sampling was not measurable at battle scale. BC7 wasn't worth it for the memory saved.
- **Wrecks need deformed geometry, not textures.** Ten texture rounds failed, and `wreckage.py`'s deformation of the live vehicle's own parts succeeded:
  - operations: dent, sag, bend, ragged edges, holes, cuts and plates;
  - applied identically on every tier, with booleans ending in canonical vertex and face order;
  - a sooted camo base multiplied into pink blotches, so it was dropped.
- **Author tiers per part, don't decimate.** Segment counts scale by tier, and bevels live on the two finer tiers only. Find the part that dominates the triangles (the tank's 22 wheels are half its count): that part is the LOD budget.
- **Bake Cycles-only effects** (bevel shading, world-z gradients) into textures or vertex colour. The realtime path has no equivalent, so a Cycles render overstates the look.
- **Keep the 48 B vertex.** Joint weights were narrowed to unorm8 to make room for an snorm8 tangent. A wider vertex costs buffer memory across every tier and variant.
- **Break clones without scaling stature:**
  - infantry variants chosen by `id mod n` (they must share one skeleton);
  - a per-soldier animation phase;
  - small rest-pose manners.
- **Props fit per axis from the nearest `footprint_half_m`.** Walls, fences and sandbags repeat a module along their box instead of stretching.
- **Sim rules come first when they collide with art.** The tank's gun length is a simulation number (its cannon's `mounts` row: `pivot_m` and `muzzle_m`). Every mount's drawn muzzle matches the sim's arc (`fit.muzzle_arc`, the pivot turning with its carrier and the muzzle with its own bearing) to 1e-6 m, never the other way round.
- **A building drawn by the hundred is a few faces and a recipe.** One face a wall, split only as far as the vertex colour needs (occlusion, mud, streaks), with the material's texture carrying the look: a two-storey house is 5,000 triangles at tier 0 and 20 at tier 3. A kit module is baked alone (`parts.finish(objects=...)`), so its paint must not assume the ground is at its own z = 0 unless it stands there.
- **A tower's far wall is a picture of its near wall.** Near, every bay of every floor is a row placing a shared panel module; further off the template's shell draws a run of bays as one face sampling a facade recipe cut from the same table of openings (`city/towers.py`). Make the recipe's tile two bays by two floors with each of the four dressed differently, or the grid reads as a spreadsheet.
- **A template of several buildings is one shell module in the template's frame.** A row has one tint, so each building's wall colour goes into its own material (the same recipe, another vertex colour); the far tiers are then one row, with each fitting folded into the shell as a flat panel. A timber or a glazing bar a pixel wide crawls: drop the diagonal ones at the coarse tiers.
- **A pitched roof carries its own UVs** (u along the eave, v up the slope). Box projection lays the courses across the slope on every roof whose ridge runs the other way.
- **Per-piece tone in a recipe aliases into a quilt at 80 m.** A roof tile is 4 px there; a wide brick-to-brick or tile-to-tile spread reads as a checkerboard, and vertex-colour mottle on a 1.3 m grid adds a second one. Keep the spread narrow and let stains do the variation.
- **A big roof is the facade, and nothing countable goes in its recipe.** A recipe repeats every 6 to 8 m: pools and patches in a felt texture tiled into camouflage across a 48 m roof. The recipe carries what is smaller than a few metres (ribs, laps, fixings); rooflights, mended sheets and patches are faces of the shell placed once; rust, damp and fading are vertex paint on a grid the shell builds itself and keeps down to tier 2.
- **Vertex-placed marks must be wider than two cells of the coarsest grid that carries them,** or each tier's vertices sample different marks and the rust moves when the tier changes. The coarsest tier has only corners, so there a weathered material is swapped for a twin with the same marks averaged; split a wall at its foot so mud and ground shade stay there at every tier.
- **A building's tier is chosen by pixels to the metre, not by its height.** The boundaries are the same for a house and a tower (the city readme has the distances at the battle's camera), so a tower is at tier 1 or coarser in most frames. Judge a set at those three boundaries in the game's own renderer (the line-up lab), with the tier before and after side by side; a Blender sheet picks its tiers by the same rule.
- **A coarser tier is the same picture with less geometry.** Every pop the line-up found was art that differed across a boundary, never a threshold in the wrong place. What held: a shell keeps its fittings at the coarse tiers as flat panels where they were (a window is its pane over a pale surround between its shutters, a door its own paint, a chimney a block the colour of its cap, since from above a chimney is its cap); a facade recipe draws curtains in the same bays the near tier's rows hang them in, and no more; a part folded into a shell keeps its second material (a hut's concrete lid over its tinted walls); a column in another colour gets a recipe that carries the colour only where the paint is, because a baked tint multiplies the dark openings too.
- **A wall that is one face takes its corners' shade.** The ground's mud and the eaves' occlusion sit on a wall's four corners at a coarse tier and wash across it, so the wall changes colour with the tier. Cut it into a foot, a body and a head at every tier.
- **Roof marks at the coarsest tier need vertices, not an average.** A twin material with the marks averaged turned a rusty roof clean at a kilometre; a coarser grid (6.6 m) of the real paint keeps the patches where they were.
- **Per-vertex tone on a big flat roof draws the mesh.** A textured paint's fine grain, and lichen sampled at a tile's scale, are random from vertex to vertex on a 1.3 m grid: with a recipe that repeated a wide tile-to-tile spread every 2 m, roofs read as tartan from the game's camera. A roof recipe holds nothing wider than a tile, in one colour (a neutral slate for a recoloured one, so a tint is never a cast), and a roof's own stains are fields several metres across.
- **Soot is a shape, not a darkness.** Darkening a pale tinted wall to 40% read as a slightly dirty wall; a blackened hut at 60% read as its own colour. What reads as burnt: every opening an empty hole, a tongue of soot in the wall's own texture over each one (black at the head, narrowing and wavering upward, each bay a different strength so the wall is not one grey), bays blown out to pale slab edges, and all-over soot of 75% and more on what stood in the fire. Put it in the recipe the shell samples, so it is there at every tier.
- **A ruin fills its remains and keeps three things of its building:** the wall's own material and tint on the stumps (sooted from the foot up, with the openings broken to their sills where the intact rows hung them), pieces of the roof in the roof's own material lying on the heap, and the plan. The heap is a mound, low against the walls and high in the middle: level with the walls it read as a pool. Its recipe is dark and dusty; a pale one read as a salt flat. A steel shed falls as torn sheet and frames, and the sheet is the roof's own material gone to rust.
- **A damage state inherits its building's tier budget,** so the far tiers of a ruin are a box and a quad: its sides in the wall's burnt material, its top the heap's or the fallen roof's. That is enough: at a kilometre a ruin is a dark low patch in its building's colours.
- **A long band is faces, not a box.** A plinth modelled as one box the size of the building hid 8,000 paint-split triangles in its top and bottom.
- **A building that is one painted mesh pays its bytes on every page that draws its set.** A map fetches the whole kit of every template it places. The village farm (`house.py`) is a whole building of vertex paint, 70 to 110 thousand triangles at tier 0 and 6 to 8 MiB with its ruin: one for each of the authored maps' twelve boxes was 81 MiB, past a kit's 50 MiB budget. Painting twice as coarse saved a fifth, since most of it is geometry and bevels; taking a box's finest tier from the farm's second saved four fifths (`city/village.py`). Reach for shared modules and a recipe before a whole painted building.
- **A tier renamed before the bake is painted as its new tier.** `parts.finish` splits a mesh for paint by its `_LOD` suffix, so a mesh moved a tier finer is split as finely as that tier: scale `paint_scale` with the move.
- **Rigging.** The soldier is a CC0 body on a 65-joint rig. Clothing is shells cut from the body, so it skins for free. Rigid kit is bone-parented and skinned at bake; the rifle is IK'd in its own frame and baked to FK. The basis conversion happens at bake, never in the loader.

## Determinism and LFS

- **Scripts are deterministic:**
  - positions snap to 0.1 mm;
  - custom normals are cleared;
  - islands export in canonical order;
  - noise seeds are fixed.
  
  A few outputs (LOD3 collapse, wreck and variant GLBs) are still not byte-reproducible, so **the committed GLBs are the source of truth**.
- **Bundles are content-addressed** (sha256), and the loader verifies every hash. It installs a catalog generation atomically, keeping the old one if the install fails.
- **Third-party packs** are recorded by sha256, cached in `~/.cache/battlegame/packs`, and never committed.
- **Tests hash LFS pointers by oid,** so they need no pull. Golden-failure GLBs for the validator are generated in code.
- **In a worktree, pull only what you need,** e.g. `git lfs pull --include="assets/runtime/**"`. Never run a bare `git lfs pull`. A missing pull serves pointer files: effects flipbooks and bundles fail to decode, and nothing says "LFS".

## Baked pictures of rooms (the interior atlas)

`blender/city/interiors.py` renders rooms lit only through their own window; its contract is in the city readme.

- **Whatever hangs in the window's light is the brightest thing in the picture, however dark its paint.** A pale curtain, pelmet or shutter read as a lit panel or a strip light. Keep window-side things dark and give the tone curve a ceiling.
- **Tone the sheet in the game, not on the sheet.** The frame shows a cell as a matte surface in sun shadow, so a sheet that looks rightly dim as a picture (cell means near 0.2 sRGB) is a row of black holes at 80 m. Cell means of 0.3 to 0.4 under a ceiling of 0.15 linear read as rooms and stay under the facade scene's bound; the sheet itself then looks like a lit doll's house, and that is correct.
- **A room takes no sun, so look at it on a facade in its own shade.** There the wall darkens and the room does not: a ceiling that passes every check in the sun still read as "a dim light left on" in pale shops. The ceiling is set by that frame.
- **Bring the back of the room up with exposure under a low ceiling, not a shadow lift.** A lifted curve read as a grey veil over every room.
- **Walls and floors carry the read at distance.** From the steep camera a window is its cell's floor, so floors are light and differ, and wall hues are strong.
- **Give every room the same light, not the same sky.** A shopfront has three times an apartment's glass and at the same sky reads as lit.
- **Coplanar wall patches render black.** Give each its own thickness.
- **Cycles on the CPU with a fixed seed and no denoiser wrote the same bytes twice;** do the tone curve, the downsample and the PNG in numpy.
- **Judge it through the lookup, not on the sheet.** A small numpy ray-cast of the room box behind window-sized openings shows what the game will: looking down from the tactical camera, a window is mostly the cell's floor strip.

## Windows with rooms on a scripted kit

The helpers and their contract are in the city readme ("Interiors"); these are what the first four kits taught.

- **A room needs a hole.** A wall that is one face hides the room box behind it. Cut the opening after the rows are known (`kit.open_walls`), and run every cut across the whole wall and round the corner, snapped onto its plane: a cut that stops part way leaves a vertex on a neighbour's edge.
- **Never squeeze a room.** A box under 1.5 m deep, or as narrow as its window, shows its cell's pale walls just behind the glass and reads from above as a blank grey panel. Give the corner to one window and draw the other's blind, in a deep cloth so it is as dark as its neighbours.
- **Check the plan, not the picture, for overlap.** Two boxes sharing space fight for depth only from some cameras. `plan_rooms` fails the build on any shared space; that check found two faults the frames had not shown.
- **Match the far pane to the near window, by measure.** Glass over a room is three times brighter than the glass's own colour. Sample the pane's pixels either side of a tier boundary in the line-up's crops and set the far pane (or the far recipe's glass) to the near tone.
- **What is pale and thin near the glass decides the window's tone at distance:** bars drawn at tier 0 only made tier 1 darker; a pale reveal made a row of windows seen along its wall a row of white slabs.
- **A big pane with no bars reads as a hole.** Bars, a reveal and something standing just inside the glass are the cues the kit has; the frame gives no reflection.

## Impostors are baked by our own renderer

Soldier cards are baked at install by the frame's own model path:
- an orthographic view on the bounding sphere;
- 8 yaws × 2 pitches, 64 px cells, 2× supersampled;
- a CPU box filter, so the bytes are deterministic.

Baking at install means no bundle-format change and no LFS churn. Cards are alpha-tested, cast no shadow, and are relit from a normal atlas with the tint mask in alpha. Don't add three.js or any other renderer just to bake.
