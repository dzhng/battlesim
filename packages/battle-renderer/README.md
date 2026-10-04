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
that the simulation does not provide.

## Shared drawing contracts

[Renderer core](../renderer-core/README.md) owns the live camera and depth convention.
[Light](src/light/sceneLight.ts) shares one fixture-owned sun and environment;
[world shading](src/world/) and [shader functions](src/shaders/) reuse it.
Fog eyes use published positions while model motion may interpolate. Unknown
occluders cannot enter a side's fog. Seen ground and sun shadow must remain visually
distinct from unseen ground; [fog and light rationale](../../specs/done/battle-look/README.md)
records that design requirement.

World paint belongs to the lit ground. Display overlays compose after the world's
postprocessing, so grading and fog cannot change their information colors.
[Surface classification](src/models/surfaceParts.ts) separates opaque, cutout,
room and glass geometry; alpha cannot silently acquire a second meaning in a pass.
Numeric layouts and pipeline formats belong to their source owners, not this guide.

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
