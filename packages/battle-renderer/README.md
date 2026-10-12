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
passes. [Models](src/models/) and [fog inputs](src/frame/fogInputs.ts) for the
static world still consume immutable identities and published causes here.
Building damage follows the side's known physical remains, not a renderer guess
from height or floor count.

The app's [observation feeds](../../apps/battle-lab/README.md#observation-feeds)
translate publications into this boundary. [Scene assets](../scene-assets/README.md)
owns installed appearance generations and fit. A map must request its building
kits before drawing them; a catalog listing is not installed kit geometry.

[Terrain](src/terrain/) uses compiled map shapes, resolved in Rust for every
client ([the ground](../../crates/static-world/src/ground/mod.rs): forests, plots,
roads as drawn, surface fields, water cells); [scenery](src/scenery/) places
admitted appearances around them. Physical bounds and visual-only landscape remain
separate. Cosmetic geometry must not invent a physical obstruction or imply cover
that the simulation does not provide: a tree the side has seen felled falls on the
clock and lies pressed under cover height by its stump ([felled trees](src/scenery/felled.ts)),
since the simulation gives it no body. A map's region (its `regional_family`) is a
look, never physics: the biome's paved rows take that region's own finish through
one function (`regionalBiome` in [the biome](src/terrain/biome.ts)), and scenery
its region's appearances.

## Shared drawing contracts

[Renderer core](../renderer-core/README.md) owns the live camera and depth convention.
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
[Surface classification](src/models/surfaceParts.ts) separates opaque, cutout,
room and glass geometry; alpha cannot silently acquire a second meaning in a pass.
Numeric layouts and pipeline formats belong to their source owners, not this guide.

The ground's and the grass's look numbers that are not the biome's live in one
tuning table each ([terrain](src/terrain/terrainTuning.ts),
[grass](src/terrain/grassTuning.ts)), spliced into the WGSL as constants; hash
multipliers, decorrelation offsets and lattice turns stay in the shader as part of
the formula. A second client reads the same tables, and the frame's
`uploadedTables` hands it the ground's and grass's GPU tables word for word, each
with a layout derived from its struct ([word tables](src/frame/wordTables.ts)).
It hands over the buildings' module instances and the scenery's instances the
same way, expanded by the owners the layers draw from, so a second client places
nothing itself, and `drawnInstances` says what the last frame drew of them.

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

## Resource lifetime and bounded work

[The GPU registry](src/frame/registry.ts) owns allocations and nested scopes.
Destroying a TypeGPU root alone does not destroy all resources it created. Size-dependent
[targets](src/frame/targets.ts) swap as complete generations; failure or disposal
must release a late build rather than install it into a dead frame.

Update a resource at its actual frequency: immutable world data, changed
knowledge, view changes or frame animation. [Static chunks](src/frame/staticChunks.ts)
and the buildings' Rust walk (`crates/presentation`'s static world) bound work and residency
without changing authoritative populations. Streaming work reports when it remains
pending; a camera cut cannot be captured merely after an arbitrary number of frames.

Diagnostics describe the last drawn frame. Cost and resource comparisons need the
same camera, tick and installed state, plus a picture proving the intended subject
was drawn. [Browser verification](../../web/README.md#checks-and-evidence) owns
scene execution, while [the benchmark](../../web/src/battle/benchmark/README.md)
owns rendered cost. Healthy counters alone do not prove a correct image.
