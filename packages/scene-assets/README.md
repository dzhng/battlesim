# scene-assets

The one owner of appearance bundles: schema, validation, baking and loading. The workbench and the battle load through the same `AppearanceLibrary` (`src/loader.ts`). The CLI (`web/asset.mjs`) is only file IO around this package.

The contract is ported from `~/dev/game`'s soldier-assets (`ART_INPUT_CONTRACT.md`); the encoding is ours (`specs/done/battle-look/decisions.md`, "Bundle encoding").

## Principles

- **Atomic.** A bundle is complete or refused. The loader installs a whole catalog generation at once and keeps the old one when anything fails.
- **Content-addressed.** A bundle is one binary file named by the sha256 of its bytes, under `assets/runtime/<hash>/bundle.bin`. The bake is deterministic, so a hash changes only when content does. The loader checks every hash.
- **Clips once per skeleton.** Skinned bodies carry joints, binds and meshes. Animation lives in one `SkeletonClips` bundle per skeleton, shared by every body on that rig.
- **Engine space is decided at bake.** Z up, +X forward, +Y left, metres, origin on the ground. The bake applies glTF's Y-up conversion plus the catalog's `basis_yaw_deg` (90 for the Quaternius rig) as one transform above the roots. The loader never converts.
- **The simulation is the authority on fit.** The soldier frame every squad shares (height, eye, muzzle) comes from the fixture's `physics` block; each unit type's hull extents, mounts, pivots and muzzles from its resolved catalog type (`src/authority.ts`). How far art may sit from them is a catalog tolerance, not a rule: widen a tolerance, per appearance if needed, rather than weakening a check.

## Source conventions (what the validator expects of a GLB)

- Four tiers in one GLB, named by a `_LOD0`..`_LOD3` suffix on mesh objects, finest first. An unsuffixed mesh is in every tier.
- **Skinned** (infantry): one skin, at most four weights. Unweighted `_leaf` joints are dropped unless the skeleton has them. Kit parented to a bone is skinned rigidly to it at bake. Sockets are empties under a joint; infantry needs `eye` and `muzzle`. The skeleton needs the six clip roles (`INFANTRY_CLIPS`), each with an explicit loop flag, plus a standing-aim reference pose where the muzzle is measured. The battle draws the fallen at the bundle's `corpse_pose`: the body skinned there once into a static mesh (`posedMesh`, `src/pose.ts`), never posed per frame.
- **Articulated** (vehicles): one root empty. Every empty is a moving node; mesh objects fold into their nearest empty. Node frames are engine-aligned, so a turret yaws about its local +Z. The required names per unit are in `validate.ts`. Tracks carry `track_length_m` and `link_pitch_m` custom properties. Each mount must turn about its catalog row's pivot, carried by the mount it sits on, as the simulation's muzzle model does; `fit.muzzle_arc` checks it as an arc.
- **Which rig draws which mount is data:** a vehicle's catalog entry names, per unit-type mount, the rig that draws it (`"mounts": { "cannon": "gun", "HMG": "hmg" }`; the rigs and their nodes are `MOUNT_NODES` in `units.ts`). Nothing is inferred from a mount's name. The validator requires every mount with a muzzle to be declared once, by a real rig, and the rig's nodes to exist (`fit.mount_draw`, `nodes.missing`); the bake carries the declaration to the runtime catalog, and the pose driver and drawn muzzles read it through `mountRoles`.
- **Unit silhouettes** (`silhouette.ts`) are rendered on the CPU from each type's baked model, side-on, and traced into one filled SVG path; `asset icons` writes `assets/icons/units/<type>.svg`, and `asset check` and `icons.test.ts` fail when one is missing, stale or orphaned. Weapon, state and glyph icons are centred on their ink (`inkBounds.ts`), so each sits in the middle of the slot the panel gives it.
- **How vehicles move** is `articulation.ts`: the pose inputs (`Articulation`: turret and gun, HMG, each side's travel, deploy progress) mapped onto named nodes. Wheels roll about their local +Y; guns pitch about their local +Y. Deploying parts carry their own motion as custom properties (`deploy_start`, `deploy_end`, `deploy_move_{x,y,z}`, `deploy_turn_{x,y,z}`), and a supply truck's pads must reach the ground at deploy 1. An articulated bundle's `bounds` cover every pose this reaches; fit is still measured at rest.
- **Static** (buildings and scenery): one GLB per state. Buildings need `intact` and `ruin`.
- **Fit to a prop's box.** A building, and a scenery kind whose footprint is a simulation prop, declares `footprint_half_m`: the box its art is authored to, bottom on the ground. It must be a box the simulation places (a map building, a vehicle's hull for a wreck, a building's plan at the building row's ruin height (`props.building.destroyed.into.height_m`) for a ruin; `web/tests/sceneAssets/catalogFootprints.test.ts` holds the catalog to that). The validator measures every state against it (`fit.footprint`; a building's ruin at the ruin height), the runtime catalog carries it, and the battle fits each placed box from it. Vehicles' hit boxes judge the sides with `hull_extent_m` and the top with `hull_top_m`, so an antenna never loosens the sides.
- **Scenery** (`unit: "scenery"`): every prop, tree, hedgerow and grass kind, named by the entry's `scenery`. A scenery kind is one row of `SCENERY_KINDS` (`src/scenery.ts`), the art side's table: the states its art must carry, and its footprint, meaning what the simulation knows of it for the workbench's overlay. The footprint says it stands for prop types (box and blocking class; which ones is each prop type's `appearance.drawn_by`, never this table), a forest tree (trunk and canopy) or nothing. The validator, the bake, the loader and the workbench read it from there. A kind instanced by the hundred also carries its triangle budget per tier there (`tier_triangles`, finding `budget.tier_triangles`): a tree's is what a paired frame-cost run measured to fit, so a new species fits the budget rather than raising it. Which scenery kind draws a prop is the prop type's data, not this table's: a new prop type starts in the prop catalog ([`fixtures/README.md`](../../fixtures/README.md)), whose `appearance.drawn_by` names a scenery kind, and needs a new row here only when it needs new art.
- **Grass kinds** are the `grass` row: a clump of blade strips the battle's grass field instances and bends in the wind, so every tier must be the same blades in one layout (`grass.ts`, finding `structure.grass`). They are generated, not modelled: a catalog entry's `grass` spec is the generator's input, `asset grass` writes its GLB, and the bake takes the GLB like any other. A clump's mean colour stands for the ground it grows on; the field tints each clump by its own ground, so a spec's colours say how its roots, tips and heads differ from that ground, never the field's colour. Its vertex alpha is how far the kind answers the field's wind. No blade stands taller than its spec's `height_m`, and no field may draw any kind taller than `GRASS_MAX_HEIGHT_M` once the biome's scales are composed; the workbench shows each kind against a ruler at that height.

## City buildings

A town's building is not an appearance. It is a physical template (the contract's descriptor, `crates/contract/src/templates.rs`) dressed by rows that place a kit's shared modules on it, so thousands of buildings draw a few dozen meshes. The source format and the rules a script keeps are in [`blender/city/README.md`](blender/city/README.md). This package takes a set from there:

- **A kit is an appearance** (`unit: "kit"`), baked and loaded like any other. Its bundle's states are its modules, each in its own frame. A module is a piece of a building, so nothing measures it against the ground or a box; fit belongs to the templates that place it.
- **Every set packs into one template art library** (`templateLibrary.ts`): one content-addressed runtime file beside the bundles, named in the runtime catalog and installed by the one loader together with its kits, or not at all.
- **Art and physics have separate identities.** The library's `art_hash` moves with any module, material, row, tier mask, tint or status. `covers` is the hash of the physical catalogue it was made to. Neither is written into a map, so replacing a stand-in with real art moves no map, scenario or replay.
- **The resolver is pure** (`resolve`): a template id, the frame it is placed at and the state a side knows it in give its module placements in world space. Missing art is refused by name (`TemplateArtError`). Nothing stands in for it: no other template, state or family.
- **The simulation's contract judges the physical side.** What a legal descriptor is, and what the catalogue's hash is, come from the contract's own code through the WebAssembly (`PhysicalTemplates`), never from a copy here. So `bake` and `check` need the WebAssembly built.
- **Art is held to its descriptor** (`templateSource.ts`, the `templates.*` findings): everything a row draws, at every tier it draws at, stays inside its template's parts grown by the set's `fit`. What hides a unit in the simulation hides it on screen.
- **The physical catalogue and the sets cover each other.** Every template the map generator can place is dressed by exactly one set, and every set's descriptor is a row of the catalogue. The catalogue's rows are derived: `asset catalogue` rewrites them from the sets. That moves the catalogue's hash, and every generated map with it.
- **The prototype set is generated** (`prototypeSet.ts`, `asset prototypes`): each physical part as a tinted box, for every catalogue template no other set dresses, marked `prototype`. It shrinks as real sets land. To retire a stand-in template, delete its catalogue row and regenerate.

## Sides

Blue and red draw the same meshes. A material's `tint` (glTF material extras, 0..1) is the side-tint mask: how much of the side's colour it takes. The catalog's `sides` holds each side's linear RGB tint, baked into the runtime catalog. `AppearanceCatalog` (`src/appearanceCatalog.ts`) answers which appearance a unit kind draws and its side's tint: `resolve(kind, side, soldierId)`.

- A vehicle kind has exactly one appearance; a second is refused.
- An infantry kind may have several **variants** (catalog entries with the same `unit`: another head, kit, pack and colouring). They must share one skeleton, so they share one clip set; variants on two skeletons are refused. A soldier wears variant `id mod n` in name order, the same one alive and fallen, so consecutive soldiers (a squad's) never share one.

The renderer multiplies tint-masked albedo by the `ModelInstance`'s tint.

## Authored sources (`blender/`)

The infantry sources under `assets/source/infantry/` are exported by the Blender scripts in `blender/`, run with `bun run --cwd web asset -- blender ../packages/scene-assets/blender/<script> <arg>`:

- `clips_infantry.py <family>` bakes one clip set (a hold family) into `clips_<family>.glb`;
- `infantry_kit.py <kind> [a|b|c]` builds one kind's body, kit and weapon into `<kind>.glb` (variant `a`) or `<kind>_<variant>.glb`. The variants' looks are the script's `LOOKS` table: headgear, eyewear, vest colour, pack, pouches, skin and hair. The kind's own cue (the recon ruck, the launcher) stays in every variant.

They read the third-party packs from a local cache, never from the repo; `blender/packs.py fetch` downloads them, and every read is checked against the hash pinned in `blender/packs.json`, so a re-uploaded pack stops the build instead of changing the art. Their sources and licences are listed in [`assets/README.md`](../../assets/README.md). Exports are hash-stable: the same scripts and packs write the same bytes, so a changed hash means changed art.

The vehicles, village buildings, wrecks and props under `assets/source/vehicles/` and `assets/source/village/` are ours from scratch, one script per subject, and so are the trees and hedgerows under `assets/source/trees/` (`trees.py`: a branch skeleton carrying solid leaf clumps, the same clumps coarser on each nearer tier and a lobed volume built inside them as the far tier, because a tree's shadow is cast by the tier under the one drawn): `blender/build_sources.sh` rebuilds all of them. They follow the Muster technique: scripted parts with bevels, one mesh per `_LOD<n>` tier, and the look baked into vertex colour (paint, edge wear, grime, ambient occlusion). `parts.py` owns the primitives and the bake; `masonry.py` the village's paints and walls.

Wrecks are the live vehicle's own parts, worked over by `wreckage.py` before the bake: warped and dented plates, folded and torn panels, hulls hollowed and holed so openings show a burnt interior, and debris thrown round. Its booleans end in a canonical vertex and face order. The paint round each fire vent (`SCORCH`) blisters to rust and chars black; away from them the original paint survives, sooted. A bridge's piers, abutments and wing walls stand below its box, down to a channel's bed: the catalog widens that appearance's `ground_m` for them.

## Textures

A material may carry baked textures (bundle format 3), in three channels (`TEXTURE_CHANNELS`, `schema.ts`):

- **albedo**, sRGB; its alpha is the wear threshold;
- **normal**, tangent space;
- **ORM**: occlusion, roughness and metalness, with the side-tint mask in alpha.

A source embeds them as a standard glTF material's PNG textures, with a `TANGENT` attribute on its meshes. The bake (`texture.ts`) decodes each image, builds every mip level and addresses the texture by the sha256 of its content. It then stores the texture once per bundle, however many materials share it. The renderer keys textures by that address too, so a texture two bundles share is one layer on the GPU.

The validator's `texture.*` findings (`validate.ts`) hold every texture square, a power of two and fully mipped, and every vertex a normal-mapped material draws to carrying a tangent.

A textured material's vertex colour means something different. It is relative to the albedo texture's mean and stored at a third (`Material.colour_scale` 3), so dust, ash and rust can lighten or tint a surface as well as darken it. Its alpha says how worn the surface is. Where that rises past the albedo's wear threshold, the material's `wear` colour shows, as crisp chips on edges and spatter low down.

The textures are procedural recipes in `blender/textures.py`: camouflage prints, weaves, rubber, steel, burnt metal, wood, stone, concrete and markings. Each is evaluated on a periodic lattice, so it tiles seamlessly, and baked with fixed seeds. The scripts give a textured part UVs in metres by box projection (`box_uv`), so every part and every tier samples the recipe at its own scale. `attach` then writes the images into the exported GLB. Painted markings (tactical numbers, crate stencils, launcher nomenclature) are modelled as thin lettering (`parts.stencil`).

## Where things are

`schema.ts` holds the types and constants, `validate.ts` the rules and finding codes, and `codec.ts` the binary layout. `loose.ts` judges one GLB on its own, for `asset validate` and the model workbench's drop zone. Every validation also returns a `preview`, the bundle as built even when it has errors; the workbench installs previews through the same loader (`previewRuntime` plus `memoryFetch`), and the bake never writes them.
