# Renderer core

This package holds the drawing primitives shared by the battle renderer and
CPU-side view queries. It has no battle rules or knowledge of unit identities.
Dependencies remain owned by [the web application](../../web/README.md#build-and-serving).

[Camera projection](src/camera3d.ts) defines one live camera. Resize changes
its aspect through the live viewport; picking and drawing use that same
projection. The matrices a frame draws with (the view, its camera uniform, the
sun's cascades) are the presentation crate's, made from that camera and
uploaded as they are; the page's own math gives the same bits, so a clicked
shape agrees with its drawn location. [Uniform packing](src/cameraUniform.ts)
writes only a camera the page poses itself, an impostor bake's view.

[Camera control](src/cameraController.ts) describes view movement. [Clearance](src/cameraClearance.ts)
uses caller-supplied authoritative geometry; model silhouettes and depth pixels
cannot substitute for physical picking or clearance.

[The depth contract](src/depthContract.ts) keeps projection, format, clear and
comparison consistent. A pass cannot change one of them independently or treat
the convention as a device fallback. [Device admission](src/device.ts) requests
the device once and exposes its failures and granted features and limits;
consumers read those instead of independently probing a different device policy.

[Allocation tracking](src/gpuAllocations.ts) measures device allocations, including
library-created resources. It counts in creation order, so an owner's teardown can be
checked while the next owner is already allocating. Destruction belongs to [the battle renderer's registry](../battle-renderer/README.md#resource-lifetime-and-bounded-work),
not to the tracker. [Presentation tables](src/kindTable.ts) share explicit fallback
selection for kinds not given their own row; they are not a gameplay type registry.
