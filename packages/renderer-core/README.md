# Browser camera primitives

This package holds browser camera controls and CPU-side view queries. It has no
battle rules or knowledge of unit identities. The [shared Rust renderer](../../crates/renderer-core/README.md)
owns drawing, GPU admission contracts and resource accounting.
Dependencies remain owned by [the web application](../../web/README.md#build-and-serving).

[Camera projection](src/camera3d.ts) defines one live camera, and [the
viewport](src/viewportCamera.ts) pins its aspect to the pixels it is drawn
into, so a resize moves picking and drawing together. The matrices a frame
draws with (the view, its camera uniform, the sun's cascades) are the
presentation crate's, made from that camera and uploaded as they are; the
page's own math gives the same bits, so a clicked shape agrees with its drawn
location.

[Camera control](src/cameraController.ts) describes view movement. [Clearance](src/cameraClearance.ts)
uses caller-supplied authoritative geometry; model silhouettes and depth pixels
cannot substitute for physical picking or clearance.
