# Procedural assets

Every model and texture is **made by code**. The user likes this and wants it kept:
- headless Blender scripts build the geometry and bake vertex colour;
- numpy recipes build tileable textures;
- the bake packs both into content-addressed bundles.

No hand-authored or downloaded art, except third-party sources with a known source and a licence that allows shipping; [`assets/README.md`](../../../../assets/README.md) lists them. Two kinds exist today:
- the Quaternius base packs the soldiers are built from (a CC0 body, its 65-joint rig and library clips), read from a local cache by `blender/packs.py`, never committed;
- effect flipbooks under `assets/third-party/effects/`, layers of the effect atlas.

Owners:
- `packages/scene-assets/src/`: `codec.ts` (the bundle), `validate.ts`, `loader.ts`, `appearanceCatalog.ts`, `scenery.ts`;
- `packages/scene-assets/blender/`: `parts.py`, `textures.py`, one script per subject, and `build_sources.sh`;
- `assets/catalog.json`;
- `web/asset.mjs`: the `asset` CLI.

The scene-assets README is the manual; this page is the lessons.

Drive Blender through a Blender MCP server when one is installed, e.g. to inspect a scene or try a change interactively. Land the result as a script change, because the scripts are the source of truth and the `asset` CLI runs them headless.

## The pipeline, in one breath

1. The Blender script (pinned version, `BLENDER` env override) builds the subject from `parts.py` primitives, one mesh per LOD tier.
2. `finish()` bakes paint, edge wear, grime and ray-cast AO into vertex colour.
3. `textures.py` embeds the albedo, normal and ORM PNGs.
4. It exports a GLB into `assets/source/**`, which is LFS.
5. `asset bake` makes bundle v3 (magic `BGAB`, a canonical JSON header and an aligned binary body) at `assets/runtime/<sha256>/bundle.bin`, mapped by name in `assets/runtime/catalog.json`.
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
- **A tower's far wall is a picture of its near wall.** Near, every bay of every floor is a row placing a shared panel module; further off the template's shell draws a run of bays as one face sampling a facade recipe cut from the same table of openings (`city/towers.py`). Make the recipe's tile two bays by two floors with each of the four dressed differently, or the grid reads as a spreadsheet. Tiers go by projected height, so a 60 m tower is at tier 0 out to 500 m: its tier 0 is what a battle sees, and the far tiers are for the zoomed-out camera.
- **A template of several buildings is one shell module in the template's frame.** A row has one tint, so each building's wall colour goes into its own material (the same recipe, another vertex colour); the far tiers are then one row, with each fitting folded into the shell as a flat panel. A timber or a glazing bar a pixel wide crawls: drop the diagonal ones at the coarse tiers.
- **A pitched roof carries its own UVs** (u along the eave, v up the slope). Box projection lays the courses across the slope on every roof whose ridge runs the other way.
- **Per-piece tone in a recipe aliases into a quilt at 80 m.** A roof tile is 4 px there; a wide brick-to-brick or tile-to-tile spread reads as a checkerboard, and vertex-colour mottle on a 1.3 m grid adds a second one. Keep the spread narrow and let stains do the variation.
- **A big roof is the facade, and nothing countable goes in its recipe.** A recipe repeats every 6 to 8 m: pools and patches in a felt texture tiled into camouflage across a 48 m roof. The recipe carries what is smaller than a few metres (ribs, laps, fixings); rooflights, mended sheets and patches are faces of the shell placed once; rust, damp and fading are vertex paint on a grid the shell builds itself and keeps down to tier 2.
- **Vertex-placed marks must be wider than two cells of the coarsest grid that carries them,** or each tier's vertices sample different marks and the rust moves when the tier changes. The coarsest tier has only corners, so there a weathered material is swapped for a twin with the same marks averaged; split a wall at its foot so mud and ground shade stay there at every tier.
- **A long band is faces, not a box.** A plinth modelled as one box the size of the building hid 8,000 paint-split triangles in its top and bottom.
- **A building that is one painted mesh pays its bytes on every page.** The loader fetches every bundle the catalog names, whatever map is played. The village farm (`house.py`) is a whole building of vertex paint, 70 to 110 thousand triangles at tier 0 and 6 to 8 MiB with its ruin: one for each of the authored maps' twelve boxes was 81 MiB, past a kit's 50 MiB budget. Painting twice as coarse saved a fifth, since most of it is geometry and bevels; taking a box's finest tier from the farm's second saved four fifths (`city/village.py`). Reach for shared modules and a recipe before a whole painted building.
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
- **Don't lift the shadows to avoid black holes.** A lifted curve read as a grey veil over every room. Raise the exposure instead.
- **Give every room the same light, not the same sky.** A shopfront has three times an apartment's glass and at the same sky reads as lit.
- **Coplanar wall patches render black.** Give each its own thickness.
- **Cycles on the CPU with a fixed seed and no denoiser wrote the same bytes twice;** do the tone curve, the downsample and the PNG in numpy.
- **Judge it through the lookup, not on the sheet.** A small numpy ray-cast of the room box behind window-sized openings shows what the game will: looking down from the tactical camera, a window is mostly the cell's floor strip.

## Impostors are baked by our own renderer

Soldier cards are baked at install by the frame's own model path:
- an orthographic view on the bounding sphere;
- 8 yaws × 2 pitches, 64 px cells, 2× supersampled;
- a CPU box filter, so the bytes are deterministic.

Baking at install means no bundle-format change and no LFS churn. Cards are alpha-tested, cast no shadow, and are relit from a normal atlas with the tint mask in alpha. Don't add three.js or any other renderer just to bake.
