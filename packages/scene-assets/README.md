# scene-assets

The one owner of appearance bundles: schema, validation, baking and loading. The workbench and the battle load through the same `AppearanceLibrary` (`src/loader.ts`). The CLI (`web/asset.mjs`) is only file IO around this package.

The [appearance rationale](../../specs/done/battle-look/README.md) records the imported techniques and their provenance. The [unit models rationale](../../specs/done/unit-models/README.md) records why units are built from references to their frames, with their own wrecks, strict tiers, material roles and loose budgets. [Schema](src/schema.ts), [validation](src/validate.ts) and [encoding](src/codec.ts) own the current contract.

## Principles

- **Atomic.** A bundle is complete or refused. The loader installs a whole generation at once, for a catalog load and for a request for kits alike, and keeps the installed one when anything fails.
- **Fetched when drawn.** The catalog load takes the scenery that is not fetched on request (`fetchedOnRequest`), the skeleton clips, the template art library (small, and naming every kit), and of soldier and vehicle art only what the page's session's units wear (`UnitCatalog.appearances`, selected by `catalogLoadNames`): a game page never fetches art only test or menu units wear. A later session on the same page with other units (a lab after the game) widens the load to theirs (`withUnits`), fetching nothing it already holds. Two kinds are fetched on request, when something that draws them asks. A kit is tens of megabytes and a page draws from a few: a map asks for the kits its buildings' templates place, a lab for the kit it shows. A regional look is drawn only on a map of its region: a map asks for its own family's looks, no other's. Each is fetched once however many ask. One not fetched is a named state, never a hole: the generation lists what the catalog fetches on request, a module of an absent kit is bound to no state, and a map whose kit is absent is refused by name rather than drawn without it.
- **A paint is a tint, not an appearance.** A prop whose art marks a material `tint` (a car's body) is drawn in one of its catalog `paints` per body, chosen by which authored body it is, so a shoved car keeps its colour (`battle-renderer` `models/propAppearance.ts`). Paints avoid the saturated blue and red the HUD gives the sides.
- **A region's look is an appearance, not a kind.** A shared piece (a bench) is one prop kind, the same body in every region; a scenery appearance tagged with a `regional_family` (one of the presets' `parcels.regional_families`, held there by the bake) is that region's look of it. A map draws a kind in its own region's looks where it has any, else in the looks of no region, never another region's (`battle-renderer` `models/propAppearance.ts`). Only scenery is regional: a body is the same everywhere, and a kit belongs to a region through its templates.
- **Content-addressed.** Every served file is named by the sha256 of its bytes. Every bundle, texture and the template library travels as gzip: encoded addresses change when transport changes, while original content hashes remain the art identity. The loader checks both before installation.
- **Clips once per skeleton.** Skinned bodies carry joints, binds and meshes. Animation lives in one `SkeletonClips` bundle per skeleton, shared by every body on that rig.
- **Engine space is decided at bake.** Z up, +X forward, +Y left, metres, origin on the ground. The bake applies glTF's Y-up conversion plus the catalog's `basis_yaw_deg` (90 for the Quaternius rig) as one transform above the roots. The loader never converts.
- **References are review inputs, held to their provenance.** A family's reference library ([`references.ts`](src/references.ts)) is read by `check` and the review sheet only. Every image names a licence we may redistribute, or is labelled generated with how it was made, and no view rests on a generated image alone.
- **The simulation is the authority on fit.** The soldier frame every squad shares (height, eye, muzzle) comes from the fixture's `physics` block; each unit type's hull extents, mounts, pivots and muzzles from its resolved catalog type (`src/authority.ts`). How far art may sit from them is a catalog tolerance, not a rule. A vehicle's parts that stand outside its hull (antennas, stowage, crew) are not a wider tolerance but dressing: nodes named `dressing_*`, held to their class's own allowance so they never read as cover the simulation lacks.

## Source conventions (what the validator expects of a GLB)

[Validation](src/validate.ts) owns structural, fit and budget findings; [schema](src/schema.ts)
owns bundle roles and limits. Source exporters carry explicit detail tiers, so a
coarser representation changes geometry rather than merely hiding a large mesh.
A unit or wreck must really switch with zoom: every mesh names its tier, and
each tier draws a recorded fraction of the one before. Those rules, the
dressing allowance and the budgets are kept per unit class (a vehicle's
physical class, or soldier) in [unit art](src/unitArt.ts), as scenery's are
per kind; each number there is a loose tripwire, raised when real art
outgrows it and nothing visibly suffers.

Skinned bodies share one skeleton's clip library. Bone-attached kit is baked into
that skin, and sockets locate observed effects on the drawn weapon. The fallen
body uses a fixed corpse pose instead of retaining a live animation workload.
[Pose construction](src/pose.ts) owns those transformations.

Articulated vehicles preserve moving-node frames and mount ownership. A mount's
catalog binding selects its drawn rig; its name cannot infer a pivot or muzzle.
[Unit bindings](src/units.ts) and [articulation](src/articulation.ts) own that mapping.
Bounds cover all reachable poses, while physical fit remains measured at rest.

Stationary supply service is a simulation capability, not evidence of deployment
hardware. Sources may remain ordinary cargo vehicles. A source that declares
deployment nodes or motion is held to the declared hardware chains and deployed
ground contact; the validator does not invent a mast or outriggers.

[Scenery kinds](src/scenery.ts) define art roles, states and per-tier budgets. A kind may also take pieces: states cut
from its whole where they lie in it (a tank wreck's hull and turret, for the
turret's throw), held inside the whole rather than to the ground or the box.
Every vehicle appearance names its own wreck (`wreck`): a `wreck` scenery
appearance on its unit's hull, cut into those pieces where it has a turret to
throw, so you can tell which unit died by looking. What the blast threw clear
(armour packs, doors, track runs) is the wreck's `debris` state, held to its
own allowance instead of the box: it is presentation, drawn only where the
side watched the death and sunk away after a while, never cover. Prop bindings belong to the
resolved simulation catalog; the renderer does not maintain a second prop roster. Art fits boxes the simulation actually places,
including a wreck's hull or a building's physical remains. Nonphysical dressing
must not look tall enough to grant cover or concealment the simulation lacks.
Trees share a physical size reference so species variation cannot change gameplay fit.

[Grass generation](src/grass.ts) builds source clumps from authored specifications.
Tiers retain one blade layout, and biome scaling must stay within the presentation
height bound. Clump colors describe variation relative to their ground; the field
owns the final terrain tint and wind response.

[Silhouette generation](src/silhouette.ts) and [icon construction](src/icons.ts)
derive unit icons from admitted models. Missing or stale icons fail the asset gate;
icons are not an independently authored unit catalog. Shared ink bounds keep glyph
placement tied to what the player sees rather than empty image margins.

## City buildings

A building is not an appearance, on any map: a generated town's, a test map's farms and a lab's one box are all drawn this way and no other. It is a physical template (the contract's descriptor, `crates/contract/src/templates.rs`) dressed by rows that place a kit's shared modules on it, so thousands of buildings draw a few dozen meshes. The source format and the rules a script keeps are in [`blender/city/README.md`](blender/city/README.md). This package takes a set from there:

- **A kit is an appearance** (`unit: "kit"`), baked like any other and loaded on request ("Fetched when drawn"). Its bundle's states are its modules, each in its own frame. A module is a piece of a building, so nothing measures it against the ground or a box; fit belongs to the templates that place it.
- **Every set packs into one template art library** (`templateLibrary.ts`): one content-addressed runtime file beside the bundles, named in the runtime catalog and installed whole by the one loader with the catalog. The load holds it to the kits the catalog names, by hash, so a library packed against another bake's kit fails the load before any kit is fetched.
- **One library covers two physical catalogues.** The map generator's (`fixtures/prototype-building-templates.json`: complete buildings, placed by their entrances and bays) and the authored maps' (`fixtures/building-templates.json`: solid boxes, pinned by hash). A set names the one its templates are rows of (`city_sets.<set>.catalogue` in `assets/catalog.json`: `generated` or `authored`), and each catalogue is held to its own sets and its own rule.
- **Art and physics have separate identities.** The library's `art_hash` moves with any module, material, row, tier mask, tint or status. `covers` lists the hashes of the physical catalogues it was made to. Neither is written into a map, so a change of art moves no map, scenario or replay.
- **The resolver is pure** (`resolve`): a template id, the frame it is placed at and the state a side knows it in give its module placements in world space. Missing art is refused by name (`TemplateArtError`). Nothing stands in for it: no other template, state or family. A template is found by its id alone, so no two catalogues share one.
- **A map fetches and installs the kits its own buildings draw from** (`templateKits`), not every kit: a test map downloads its own set and no tower's.
- **The simulation's contract judges the physical side.** What a legal descriptor is (valid for an authored map's box, complete for a building the generator places), and what a catalogue's hash is, come from the contract's own code through the WebAssembly (`PhysicalTemplates`), never from a copy here. So `bake` and `check` need the WebAssembly built.
- **Art is held to its descriptor** (`templateSource.ts`, the `templates.*` findings): everything a row draws, at every tier it draws at, stays inside its template's parts grown by the set's `fit`. What hides a unit in the simulation hides it on screen.
- **A template has the one damage state the simulation gives it.** The building prop type's rule (`Authority.collapse`, read from the unit catalog) says which: a `ruin` for a building that collapses, a `gutted` shell for one that stands. A template with the wrong one, both or neither is refused (`templates.state`). A `ruin` is held to the remains the collapse leaves (`ruinParts`: every part's plan at the one ruin height, with the fit's `ruin_top_m` above), a `gutted` state to the standing parts.
- **A physical catalogue and its sets cover each other.** Every template of a catalogue is dressed by exactly one set that names it, and every such set's descriptor is a row of it. The generator's rows are derived: `asset catalogue` rewrites them from its sets. That moves the catalogue's hash, and every generated map with it. The authored maps' rows are written as they are, and their set (`lab_boxes`) copies them: nothing rewrites a catalogue whose hash is its maps' identity.
- **Nothing stands in for a building; a box stands in for a prop.** A catalogue template without art is refused (`templates.coverage`): it gets art, or it leaves its set and `asset catalogue` drops its row. The one kit no set places is the stand-in kit (`standInKit.ts`, written by `asset stand-in`): a generated unit box, which the battle stretches to a prop whose kind has no model yet and tints by that kind (`battle-renderer` `models/propAppearance.ts`), so a body the simulation holds is never invisible.

## Transport and budgets

Every bundle, every texture and the template library is one gzip file. A bundle travels without its textures' pixels: each texture is its own file, named in the runtime catalog's texture table (`textures`: a bundle's texture addresses, in its order) and stored once however many bundles use it. A texture's file is the preimage of its content address (a header line, then every level), so its original content hash is its address. The runtime catalog's gzip records connect each original content hash to its encoded address and lengths (`schema.ts`). Encoded addresses make immutable caching safe even when the compressor changes; the art identity of a bundle stays the hash of its whole encoding, textures included, and the library's module bindings and `art_hash` continue to name original art.

`gzip.ts` owns packing and reading for the baker, the loader and the native readers (icons, model fit, `asset download`). A reader checks the encoded length and hash, inflates into the declared decoded length and cancels on overflow, joins a bundle with its textures, then checks the original hash before anything uses it. The loader fetches each texture once for every bundle that names it. A missing gzip record is an error. A failed request preserves the installed generation. The same publication owner supplies workbench previews, so a preview follows the battle's contract.

Download, decoded content and residency are separate budgets, each a loose tripwire against gross growth, not a target:

- **The catalog load** (`catalogLoadBytes`, `CATALOG_LOAD_MAX_BYTES`): what a page downloads before its first frame, the appearances its load takes (above) with the skeleton clips and the template library, and their textures, each file once. The loader refuses an oversized load, or widening, before fetching anything. `asset bake` and `asset check` print the game page's load and the largest any page could take (every unit's art), and hold the latter to the limit.
- **A map's download** (`downloadBytes`, `MAP_DOWNLOAD_MAX_BYTES`): what it fetches on request beyond the catalog load (the game page's, for `asset download`), each unique kit its templates need (with their damage states) and its family's regional looks, with the textures of theirs the load did not fetch. The loader refuses an oversized selection before requesting any of it; `asset download <map.json>` verifies the same objects and gate.
- **Texture layers** (`TEXTURE_ARRAY_LAYERS_FLOOR`): the renderer holds albedo and surface textures in one GPU array each, a layer per distinct texture. The device requests the adapter's own `maxTextureArrayLayers`; the bake counts each array's distinct layers over every appearance and refuses a count above the floor the target machine grants (`budget.texture_layers`).

The per-kit validator still limits the raw bundle's full geometry and texture mip chains. Compression changes neither their decoded size nor their GPU/resident cost; those remain the renderer and startup gates' concern. Budget constants live in `schema.ts`.

Gzip is an explicit file transport, not an assumed hosting optimization. The browser uses its own `DecompressionStream`; no HTTP `Content-Encoding` header is required for these files. Every source GLB, physical descriptor, template row, texture pixel and raw art identity survives unchanged.

## Sides

Both sides draw the same meshes; what differs is the army, not the side. A card
several factions field (the rifle squad) draws one appearance set, and a soldier
appearance may name its look per faction (`factions`): a soldier wears his side's
faction's look, so a US and an Eastern squad wear their own uniforms whichever side
each is on, and a battle of no factions (a lab) draws the base look. A faction look
loads with the appearance it belongs to and is held to the same rig.
[Appearance resolution](src/appearanceCatalog.ts) combines that with the catalog's
side tint and unit bindings and with observed carrier and active weapon assignments. A soldier's variant identity persists when an equipment hold
changes; the current hold supplies its own skeleton clips and muzzle socket.
Recovered equipment follows its living operator. A supported active pose is
stationary: moving operators show their carried kit, and the fallen keep that
carried identity through death and corpse presentation. The active kit's muzzle
socket must fit its declared physical bore in that supported pose; the shared
standing frame still owns body height, eye height and ground contact.

Variant sets must satisfy the [shared rig contract](src/validate.ts). Side tint
changes masked albedo, not geometry or the authoritative unit type.

## Authored sources (`blender/`)

[The source authoring guide](blender/README.md) owns Blender invocation, exporter
families, pinned input acquisition and the batch rebuild's scope. Exported GLBs
are committed source; rebaking them does not rerun their source generators. The
[city guide](blender/city/README.md) owns kit, interior and template authoring.

## Textures

A material may carry baked textures, in three channels (`TEXTURE_CHANNELS`, `schema.ts`):

- **albedo**, sRGB; its alpha is the wear threshold;
- **normal**, tangent space; its alpha is coverage (see "Coverage, rooms and roles");
- **ORM**: occlusion, roughness and metalness, with the side-tint mask in alpha.

A source embeds them as a standard glTF material's PNG textures, with a `TANGENT` attribute on its meshes. The bake (`texture.ts`) decodes each image, builds every mip level and addresses the texture by the sha256 of its content. A bundle names its textures by that address; each travels as its own runtime file, stored and fetched once however many bundles share it ("Transport and budgets"). The renderer keys textures by the address too, so a texture two bundles share is one layer on the GPU.

The validator's `texture.*` findings (`validate.ts`) hold every texture square, a power of two and fully mipped, and every vertex a normal-mapped material draws to carrying a tangent.

A textured material's vertex colour means something different. It is relative to the albedo texture's mean and scaled by the [material contract](src/material.ts), so dust, ash and rust can lighten or tint a surface as well as darken it. Its alpha says how worn the surface is. Where that rises past the albedo's wear threshold, the material's `wear` colour shows, as crisp chips on edges and spatter low down.

The textures are procedural recipes in `blender/textures.py`: camouflage prints, weaves, rubber, steel, burnt metal, wood, stone, concrete and markings. Each is evaluated on a periodic lattice, so it tiles seamlessly, and baked with fixed seeds. The scripts give a textured part UVs in metres by box projection (`box_uv`), so every part and every tier samples the recipe at its own scale. `attach` then writes the images into the exported GLB, at the recipe's size or, for a piece under a byte cap (the garden and court pieces), box-filtered down to a smaller power of two that keeps its tile. Painted markings (tactical numbers, crate stencils, launcher nomenclature) are modelled as thin lettering (`parts.stencil`).

## Coverage, rooms and roles

Every alpha has one meaning, and a material says the rest in words. Wear lives in the albedo texture's alpha and the vertex colour's, and the tint mask in the ORM texture's, on every material. How much of a surface is there is a separate statement, the material's **coverage** (`Coverage`, `schema.ts`):

- **opaque**, which every material is unless its source says otherwise;
- a **cutout**, drawn only where its coverage value reaches the material's cutoff (a grille, a sign's lettering);
- **blended**, partly there, with what is behind it showing through (glass).

The coverage value is the base colour's alpha times the normal texture's alpha: the two alphas nothing else used. So a cutout can still wear and take a side's tint, and nothing has to guess whether an alpha is damage or a hole. An opaque material ignores both.

A source says it the standard glTF way, `alphaMode` with `alphaCutoff`. The Blender helpers write those from a `coverage` argument on each material helper, and a recipe's coverage image rides its normal map (`textures.surface`, `Baked`).

A material may also be a **room**: a wall of the open box behind a window, which shows a cell of an interior atlas sheet (the atlas contract, and the UVs a room box carries, are in the [city readme](blender/city/README.md), "Interiors"). The material names the sheet (extras `interior`), and that is all it says: which cell, and its mirroring, is the drawer's choice per window. The bake gives the bundle the sheet itself (`interior.ts`): the catalog names each sheet's source picture under `interiors`, and a room's material comes out of the bake with that picture as its albedo texture, square, mipped and content-addressed like any other, so every kit that shows a sheet shares one layer of it on the GPU.

A material may also say what it **is**, its **role** (extras `role`, `MATERIAL_ROLES` in `schema.ts`): a vehicle's tyres are rubber, its optics and windows glass, its scheme paint, and the rest steel, track, fabric, skin or marking. A role is a statement about the real surface, so the validator can hold the surfaces players judge a model by to what they look like: rubber reads black and glass dark and smooth, whatever dust or paint a script lays on. It judges what the surface draws on average over its area, as the model shader does (albedo texture, vertex colour and its scale, base colour, and the wear colour where wear passes its threshold), against bounds in `schema.ts`. A vehicle's sight glass is opaque: it shows what it reflects, not what lies behind it.

The bundle only carries these statements. What draws them is the renderer's (`battle-renderer` `models/surfaceParts.ts`, which orders a mesh's triangles by kind, and `models/surfaceFragments.ts`, the stage each kind is drawn by), and a bundle of another format is refused, never read as if it were opaque.

The validator's `material.*` findings (`material.ts`) refuse what would be drawn wrong without anyone noticing. A cutout or blended material whose coverage value never crosses its own threshold has lost its coverage on the way (authored in the albedo's alpha, say). A blended surface cannot wear, since a worn patch has no coverage of its own. A room is opaque, has no textures or wear of its own in its source, and is on a static appearance; one whose sheet the catalog has no picture for does not bake. Rubber that draws lighter than black rubber (`material.role_rubber`), and glass that draws lighter or rougher than sight glass (`material.role_glass`), are refused.

`asset validate <glb>` prints each material as it would be baked, and what its rubber and glass draw.

## Where things are

[Schema](src/schema.ts) holds types and constants, [validation](src/validate.ts) the rules and finding codes, and [codec](src/codec.ts) the binary layout. `loose.ts` judges one GLB on its own, for `asset validate` and the model workbench's drop zone. Every validation also returns a `preview`, the bundle as built even when it has errors; the workbench installs previews through the same loader (`previewRuntime` plus `memoryFetch`), and the bake never writes them.
