# Battle renderer

The renderer accepts a side-visible presentation feed. It draws evidence rather
than running rules: no simulation state, inferred enemy identity or invented
physical event belongs here. [The scene contract](src/scene.ts) defines the inputs
and diagnostics; [frame construction and rendering](src/frame/battleFrame.ts)
owns their composition. That function is the authority on phase order.

## Feed boundaries

[Models](src/models/), [effects](src/effects/) and [fog inputs](src/frame/fogInputs.ts)
consume immutable identities and published causes. Soldier poses follow actual
published soldiers rather than formation slots. Building damage follows the
side's known physical remains, not a renderer guess from height or floor count.
An effect on a barrel follows [the drawn muzzle](src/models/drawnMuzzles.ts);
its physical launch remains the simulation's event.

The app's [observation feeds](../../apps/battle-lab/README.md#observation-feeds)
translate publications into this boundary. [Scene assets](../scene-assets/README.md)
owns installed appearance generations and fit. A map must request its building
kits before drawing them; a catalog listing is not installed kit geometry.

[Terrain](src/terrain/) uses compiled map shapes; [scenery](src/scenery/) places
admitted appearances around them. Physical bounds and visual-only landscape remain
separate. Cosmetic geometry must not invent a physical obstruction or imply cover
that the simulation does not provide: a tree the side has seen felled falls on the
clock and lies pressed under cover height by its stump ([felled trees](src/scenery/felled.ts)),
since the simulation gives it no body. A hull the side watched die cooks off
([cook-off](src/effects/cookOff.ts)): fireballs from its ring, its hull jolted and
its turret thrown, both its wreck's own pieces, until they lie where the wreck has
them and the whole wreck takes over without a seam. A wreck without detachable
pieces jolts as one body. Every watched vehicle keeps its last live appearance
until the blast and follows its roll to the resting wreck; a wreck found
later is simply there. A map's region (its `regional_family`) is a
look, never physics: the biome's paved rows take that region's own finish through
one function (`regionalBiome` in [the biome](src/terrain/biome.ts)), and scenery
its region's appearances.

## Shared drawing contracts

[Renderer core](../renderer-core/README.md) owns the live camera and depth convention.
[Light](src/light/sceneLight.ts) shares one fixture-owned sun and environment;
[world shading](src/world/) and [shader functions](src/shaders/) reuse it.
The [sky](src/world/sky.ts) shows a fair-weather cloud layer over the clear
atmosphere it bakes; the environment light is baked clear, so clouds never dim
the sun on the ground.
Fog eyes use published positions while model motion may interpolate. Unknown
occluders cannot enter a side's fog. The fog fills the gaps too narrow to see
through exactly as sight does (the world's sight gaps, while the side knows both
their bodies stand), so it draws no sliver of sight the simulation would not give. Seen ground and sun shadow must remain visually
distinct from unseen ground; [fog and light rationale](../../specs/done/battle-look/README.md)
records that design requirement.

World paint belongs to the lit ground. Display overlays compose after the world's
postprocessing, so grading and fog cannot change their information colors.
[Surface classification](src/models/surfaceParts.ts) separates opaque, cutout,
room and glass geometry; alpha cannot silently acquire a second meaning in a pass.
Numeric layouts and pipeline formats belong to their source owners, not this guide.

Placement previews name the same installed appearance and pose as a real unit.
Their explicit ghost presentation draws its mesh after fog treatment and before
postprocessing, reading physical world depth without writing it or casting shadows.
The mesh stays legible over unseen ground while its translucency preserves that
ground's fog treatment. Authored material coverage
still defines holes and glass; instance opacity never changes that asset contract.
The app owns placement validity and colour, and the renderer receives only the cue.

## Resource lifetime and bounded work

[The GPU registry](src/frame/registry.ts) owns allocations and nested scopes.
Destroying a TypeGPU root alone does not destroy all resources it created. Size-dependent
[targets](src/frame/targets.ts) swap as complete generations; failure or disposal
must release a late build rather than install it into a dead frame.

Update a resource at its actual frequency: immutable world data, changed
knowledge, view changes or frame animation. [Model pools](src/models/placementPool.ts)
and [static chunks](src/frame/staticChunks.ts) bound work and residency without
changing authoritative populations. Streaming work reports when it remains
pending; a camera cut cannot be captured merely after an arbitrary number of frames.

Diagnostics describe the last drawn frame. Cost and resource comparisons need the
same camera, tick and installed state, plus a picture proving the intended subject
was drawn. [Browser verification](../../web/README.md#checks-and-evidence) owns
scene execution, while [the benchmark](../../web/src/battle/benchmark/README.md)
owns rendered cost. Healthy counters alone do not prove a correct image.
