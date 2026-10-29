# Battle renderer

The renderer accepts a side-visible presentation feed. It draws evidence rather
than running rules: no simulation state, inferred enemy identity or invented
physical event belongs here. [The scene contract](src/scene.ts) defines the inputs
and diagnostics; [frame construction and rendering](src/frame/battleFrame.ts)
owns their composition. That function is the authority on phase order.

## Feed boundaries

Units, corpses, ghosts, a cook-off's pieces, the combat effects and their cast
lights arrive as [the presentation crate's](../../crates/presentation/README.md)
frames ([records](src/records.gen.ts)): batches already culled and tiered, joint
palettes, effect instances in draw order and the lights chosen for the view. The
renderer draws them as given and owns only what is per vertex, texel and pixel:
the meshes and textures an appearance names, skinning by the given palette, the
passes. Fog of war's sight is decided there too (`presentation::fog::sight`):
[fog visibility](src/frame/fogVisibility.ts) uploads each frame's eyes,
occluders, rebuilds and parameters as given and runs the compute.
The static world (buildings, scenery, props) arrives the same way, as the
frame's static batches, walked by the presentation crate and written only when
a list changes; [models](src/models/) and the scenery draw them.
Building damage follows the side's known physical remains, not a renderer guess
from height or floor count.

The app's [observation feeds](../../apps/battle-lab/README.md#observation-feeds)
translate publications into this boundary. [Scene assets](../scene-assets/README.md)
owns installed appearance generations and fit. A map must request its building
kits before drawing them; a catalog listing is not installed kit geometry.

[Terrain](src/terrain/) and [scenery](src/scenery/) draw the static world Rust
resolves for every client from the map's compiled shapes and the biome
([the ground](../../crates/presentation/src/static_world/ground/mod.rs): forests,
plots, roads as drawn, surface fields, water cells, the terrain's and grass's GPU
tables, and the scenery's walk: what trees, shrubs and dressing a view draws and
casts at which tier, as the frame's static batches); the renderer uploads the
tables and batches as given and draws admitted appearances. The shared [terrain partition](../../crates/presentation/src/static_world/ground/partition.rs)
reorders whole triangles once, preserving their vertex words. Terrain, water,
and skirt/debug meshes reuse that owner; the constant-size backdrop needs no index.
Both clients query
its hierarchy with the actual GPU camera and each caster matrix independently;
selection adds no far cutoff. Water selects only for the camera. It blends
over the world's kept samples in a pass of its own, after glass, reading depth
and writing none, and shows its bed from a copy of the world before it: by
depth on every tier, refracted and lit by its waves' caustics on the tiers
that draw it `full` (`WaterDetail`; Low keeps the depth colour alone). Its
waves are a height field the renderer steps from each frame's packet
(`Presentation.waves`: the steps, the epoch, the shift, then the wakes'
`WaveSplat` records); a step takes a bounded number of splats, so a big
battle's wakes cost the same however frames batch the steps. Camera ranges serve depth and color, including each
scar-region submission. Traversal work is distinct from issued draw counts.
Physical bounds and
visual-only landscape remain
separate. Cosmetic geometry must not invent a physical obstruction or imply cover
that the simulation does not provide: a tree the side has seen felled falls on the
clock and lies pressed under cover height by its stump (`ground::walk`), since
the simulation gives it no body. A map's region (its `regional_family`) is a
look, never physics: the biome's paved rows take that region's own finish through
one function (`Biome::load` in
[the biome](../../crates/presentation/src/static_world/ground/biome.rs)), and
scenery its region's appearances.

## Shared drawing contracts

[Renderer core](../renderer-core/README.md) owns the live camera and depth convention.
The frame draws with what the presentation made of that camera (`setCamera`):
its view, its uniform and the sun's cascades, uploaded as they are; each batch
names the tier and cascade mask it casts with and the fog group it binds, so the renderer routes
rather than decides.
[Light](src/light/sceneLight.ts) shares one fixture-owned sun and environment;
[world shading](src/world/) and [shader functions](src/shaders/) reuse it.
What the sky, the haze, the sun and post make of the light is resolved in Rust
(`presentation::light`): the frame is built with those uniform records and
uploads them as they are, so both clients light a battle from one derivation.
The [sky](src/world/sky.ts) shows a fair-weather cloud layer over the clear
atmosphere it bakes; the environment light is baked clear, so clouds never dim
the sun on the ground.
Fog eyes use published positions while model motion may interpolate. Unknown
occluders cannot enter a side's fog. A ray squeezing between occluders, one on
each side, with less than `sensors.min_sight_gap_m` of room on its two sides
together is blocked there, as the simulation's sight is (`WorldGeometry::squeezed`),
so the fog draws no sliver of sight between houses. The horizon build counts the
nearest occluder either side of each ray, so it may close a squeeze a little later
along the ray than sight does. Seen ground and sun shadow must remain visually
distinct from unseen ground; [fog and light rationale](../../specs/done/battle-look/README.md)
records that design requirement.
The rendered margin outside the playable rectangle is always unseen; fog ray
marching stops at that boundary because the margin carries no gameplay sight.

World paint belongs to the lit ground. Display overlays compose after the world's
postprocessing, so grading and fog cannot change their information colors.
Their [stroke sizing](src/strokeWidth.ts) keeps authored widths and glyph bodies
in CSS space, but the readability floor in actual render pixels: lowering render
resolution must not erase a thin information mark. The viewport supplies its
actual canvas-to-CSS ratio, so resizing and quality changes share this rule.
High-air models (`HIGH_PASS` batches) can be drawn apart: given a second
target, the frame draws them there alone after the world, lit by the same
pipelines and graded by the same look, premultiplied for the page to lay
over the world's canvas between its card planes. Without one they draw with
the world.
[Surface classification](src/models/surfaceParts.ts) separates opaque, cutout,
room and glass geometry; alpha cannot silently acquire a second meaning in a pass.
Numeric layouts and pipeline formats belong to their source owners, not this guide.

The ground's and the grass's look numbers that are not the biome's are data,
`presentation.terrain` and `presentation.grass` in the game fixture (documented
in [the ground's tuning](../../crates/presentation/src/static_world/ground/tuning.rs)),
spliced into the WGSL as constants; hash multipliers, decorrelation offsets and
lattice turns stay in the shader as part of the formula. The water's and the
shore's are uniforms instead (`TerrainParams.waterLooks`, `shoreTuning`): each
river and each swamp draws one of its biome's water looks for its kind, picked
in Rust (`StandingWater`, the one owner of standing water), and a look retunes
without rebuilding a shader. A second client draws the
same world from the same Rust output, and `drawnInstances` says what the last
frame drew of the buildings and the scenery.

The shaders are the same text for every battle and every canvas size: a
battle's light (sun, sky, clouds, haze, bloom) and a size's texel steps reach
them through uniforms, never spliced in, so one compiled set serves every map.
Every pipeline has its own name. A second client compiles this text, not a
port: [the export](pipelines/) is each pipeline's WGSL and settings, written
from this package's code and held to it by its test ([pipelines](../../crates/pipelines/README.md)).

Placement and order previews (a unit being placed, a held facing drag, every
ordered destination with Space held) name the same installed appearances and pose
as a real unit, one per soldier for a squad.
Their explicit ghost presentation draws its mesh after fog treatment and before
postprocessing, reading physical world depth without writing it or casting shadows.
The mesh stays legible over unseen ground while its translucency preserves that
ground's fog treatment. Authored material coverage
still defines holes and glass; instance opacity never changes that asset contract.
The app owns where they stand, their validity and colour, and the renderer receives
only the cue.

The visual backdrop surrounds the scene's terrain by default. A composition with
remote disconnected patches can supply separate local terrain for that backdrop
in [the scene contract](src/scene.ts). This changes only the decorative surround;
terrain selection and shadow receivers still consume the complete drawn mesh.

## Resource lifetime and bounded work

[The GPU registry](src/frame/registry.ts) owns allocations and nested scopes.
Destroying a TypeGPU root alone does not destroy all resources it created. Size-dependent
[targets](src/frame/targets.ts) swap as complete generations; failure or disposal
must release a late build rather than install it into a dead frame.

Update a resource at its actual frequency: immutable world data, changed
knowledge, view changes or frame animation. The static world's Rust walks
(`static_world`: buildings, scenery, props) bound work and residency without
changing authoritative populations. Streaming work reports when it remains
pending; a camera cut cannot be captured merely after an arbitrary number of frames.

[Submitted geometry](src/frame/submitted.ts) counts issued draw calls, before
clipping, scissor rejection or fragment discard. Its instances are per draw,
not distinct objects: repeated surfaces and terrain scar regions repeat work.
Depth, color, separate high-air passes and individual cascades stay separate.
Models include props and impostor cards; water and backdrop ground have their
own counters, while backdrop scenery belongs to scenery. The map skirt shares
a mesh and counter with traversal-view debug boxes. This is partial world
accounting, excluding effects, grass, sky, paint, proxies, x-ray and overlays.
Installed mesh sizes and presentation selection statistics remain distinct.

Diagnostics describe the last drawn frame. Cost and resource comparisons need the
same camera, tick and installed state, plus a picture proving the intended subject
was drawn. [Browser verification](../../web/README.md#checks-and-evidence) owns
scene execution, while [the benchmark](../../web/src/battle/benchmark/README.md)
owns rendered cost. Healthy counters alone do not prove a correct image.
GPU measurement windows start at the scene's timing reset: only frames begun
after it may contribute, even when older readbacks finish later. Settle a camera
or workload change before resetting; reset neither waits for nor warms that state.
