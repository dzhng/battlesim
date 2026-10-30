# Battle render inputs

This package owns the browser's plain world geometry, observed model requests,
styles, picking and information marks. It carries no GPU runtime. The
[shared Rust renderer](../../crates/renderer-core/README.md) owns pipelines,
resource lifetime and drawing for both native and browser; the
[browser scene](../../web/src/battle/render/scene.ts) transports these inputs.

The simulation is authoritative. Render inputs describe what a side knows;
geometry queries and cosmetic detail cannot infer hidden enemies, manufacture
cover or change a physical outcome. [Presentation](../../crates/presentation/README.md)
resolves world placement, frame batches, camera, light, fog and learned scars.
The generated [record readers](src/records.gen.ts) carry that Rust-owned numeric
layout across the boundary without a second schema system.

[World inputs](src/worldMesh.ts) carry physical bounds, traversal faces and lab
overrides to the [shared world constructor](../../crates/renderer-core/src/runtime/world.rs).
Rust assembles the decorative surround as well as the submitted terrain and water. [Terrain inputs](src/terrain/) hand canonical exported shapes and the
biome to presentation. [Model requests](src/models/) name admitted appearances,
poses and cues. The workbench may author a private pose, while battle poses
remain presentation's. A map must load the art it actually draws: a catalog name
alone does not provide geometry.

[Scene inputs](src/scene.ts) distinguish display overlays from painted ground
marks. Display information composes after lighting and grading; ground paint
belongs to the lit, shadowed, fogged world. [Stroke sizing](src/strokeWidth.ts)
keeps authored dimensions in CSS space and its readability floor in actual render
pixels, so lowering resolution cannot erase an information mark.

[Styles](src/overlayStyle.ts) type fixture-owned appearance; the
[presentation settings](../../crates/presentation/src/settings/validate.rs)
admit it.
[Fog](src/fog.ts) and [grass](src/grass.ts) contracts describe
canonical inputs and explicit diagnostic snapshots; they own no device or
resources. Actual GPU behavior is checked through the shared renderer's native
tests and the [browser scenes](../../web/README.md#checks-and-evidence), with a
picture proving that the subject still appears correctly.
