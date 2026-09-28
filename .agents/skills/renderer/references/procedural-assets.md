# Procedural assets

Every model and texture is **made by code**. The user likes this and wants it kept:
- headless Blender scripts build the geometry and bake vertex colour;
- numpy recipes build tileable textures;
- the bake packs both into content-addressed bundles.

No hand-authored or downloaded art, with two licensed exceptions, each hash-pinned in the reuse manifest's `third_party` list with its licence:
- the Quaternius base packs the soldiers are built from (CC0 body, 65-joint rig and library clips), read from a local cache by `blender/packs.py`, never committed;
- the third-party effect flipbooks under `assets/third-party/effects/` (CC0 Unity Labs fire and dust sheets), layers of the effect atlas.

The validator refuses any source whose hash has no such entry with an allow-listed licence.

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
6. `asset check` validates it against the fit authority (`src/authority.ts`: the fixture's soldier frame and each unit type's resolved catalog numbers), within the catalog's `tolerances`. It also checks the reuse manifest's licences.
7. `asset sheet <name>` renders workbench views headlessly for review; `--accept` copies them to `assets/review/`.

A new **unit type** (vehicle or infantry) starts in the unit catalog, in [`fixtures/README.md`](../../../../fixtures/README.md): mounts, muzzles, fit checks, and what presentation anchors to.

## Adding a prop the battle draws

The fence, sandbags and dragon's teeth are worked examples:
1. Add the simulation's prop type (a `props` entry under `fixtures/props/`: its body row and its `appearance` binding; see [`fixtures/README.md`](../../../../fixtures/README.md)).
2. Add a `props.py` kind, authored to that box, with its origin at the box centre on the ground.
3. Add its line to `build_sources.sh`, then run `asset blender` and `asset bake`.
4. Add a catalog entry (`unit: "scenery"`, `states`, `basis_yaw_deg`, `footprint_half_m`).
5. Add a `SCENERY_KINDS` row and the `project-owned` provenance record.
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

## Impostors are baked by our own renderer

Soldier cards are baked at install by the frame's own model path:
- an orthographic view on the bounding sphere;
- 8 yaws × 2 pitches, 64 px cells, 2× supersampled;
- a CPU box filter, so the bytes are deterministic.

Baking at install means no bundle-format change and no LFS churn. Cards are alpha-tested, cast no shadow, and are relit from a normal atlas with the tint mask in alpha. Don't add three.js or any other renderer just to bake.
