# One-renderer auto-research record

## Contract

Objective: implement the one-renderer architecture and decide whether the
shared Rust `wgpu` runtime can replace the browser TypeGPU renderer without
breaking production visuals or lifecycle behavior.

Fixed comparison: the same frozen presentation frame, canvas size, adapter,
render scale, and authored runtime assets. The browser TypeGPU renderer is the
control; the browser and native shells call the shared Rust renderer candidate.

Acceptance is architectural and behavioral, not a microbenchmark win. The
candidate must preserve battle digests/publication contracts, match the
production frame within the existing screenshot gates, and pass resize, rebuild,
surface/device-loss recovery. Complexity is measured after deleting duplicate
TypeGPU owners; a small performance regression is acceptable when the total
renderer ownership and source complexity are lower.

Cheap evaluator: focused Rust/WASM checks, then the `one-renderer` browser
scene with interleaved repeats and screenshot/accounting output. Full native
parity and repository closeout run only after all production passes are wired.

## Hypothesis queue

1. **Shared resource owners first (active):** moving native model/material,
   shadow, and fog target ownership into `renderer-core` gives both shells one
   lifetime owner without changing GPU work. Falsifier: browser-safe types cannot
   represent the existing assets or native behavior without a second graph.
2. **Pass seams before browser integration:** extracting fog, effects, post and
   overlays as input traits/records will make the native frame callable from the
   browser shell while retaining pass order. Falsifier: the seam requires DOM,
   filesystem, or platform-specific resource types in the core.
3. **Lifecycle is the complexity gate:** shared resize/rebuild/reset/device-loss
   ownership removes more duplicated state than it adds. Falsifier: either
   shell needs an independent registry or cannot return to a clean baseline.
4. **Deletion is the final win:** after parity, deleting TypeGPU runtime owners
   produces the complexity reduction. Falsifier: keeping TypeGPU as a runtime
   materially lowers maintenance without duplicating rendering ownership.

## Parameter-effect map

| Strategy | Current evidence | Effect | Status |
|---|---|---|---|
| Standalone Rust production-shaped graph | Full frame distance 47.03; incomplete materials/shadows/fog/post | Fast diagnostic, invalid migration comparison | Reject as final architecture |
| Shared target/pipeline/depth/world owners | Native/browser compile; unchanged pass order/GPU work | Reduces duplicate allocation and attachment policy | Keep |
| Shared indexed model upload owner | Native/browser compile; queue upload seam | Establishes one model GPU lifetime owner; no measured extra draws | Keep, production integration pending |
| Shared fog-mask owner | Native/browser-safe core; same 2 optional distance passes + compose + rim | Removes native attachment/pass duplication with zero extra GPU work | Keep |
| Shared effects/post owners | Native/browser-safe resource and ordered fullscreen/instanced draws | Removes native pass/resource duplication; asset decoding remains shell-owned | Keep |
| Shared target lifecycle owner | Atomic target swap plus generation counter | Makes resize invalidation one contract with no extra GPU passes | Keep |
| Shared fog compute pipeline owner | Four exported compute bindings and stable dispatch labels/order | Removes pipeline/dispatch drift; presentation upload and bind groups stay shell-owned | Keep |
| Browser TypeGPU as permanent second renderer | Two pass/resource graphs | Preserves a known control at the cost of permanent duplication | Reject after parity |

## Attempt ledger

| Candidate | Parent | Result | Evidence |
|---|---|---|---|
| `a27ca5d27` shared targets/pipelines/depth/world/browser shell | native client before extraction | Compile and browser scene pass; parity incomplete | PR #16, one-renderer evidence |
| `91102bb19` + `e29e253bf` shared indexed model owner/queue seam | `a27ca5d27` | Focused native/browser checks pass; browser still uses compatibility draw path | PR #16 |
| `88155dc93` shared fog-mask resources/pass order | model-owner incumbent | Native adapter and renderer-core WASM/native checks pass; GPU work unchanged | PR #16 |
| `df66c2cd3` shared effects and post resources | fog-mask incumbent | Renderer-core WASM/native checks pass; pass counts and attachments unchanged | PR #16 |
| `dcd58d35d` shared target lifecycle owner | effects/post incumbent | Native/browser-safe checks pass; same allocation and pass count | PR #16 |
| `5218451be` shared fog compute pipeline owner | lifecycle incumbent | Native/browser-safe checks pass; terrain→merge→whole/copy→cull order unchanged | PR #16 |
| `417438f93` indexed model draw fix | shared model owner | Browser validation exposed an unbound index buffer; indexed draw now binds the full model and the eleven-row scene passes | PR #16 |
| `e0f7edfc5` + `857aeda54` embedded production pipeline export | fog compute incumbent | Browser WASM embeds all 63 pipeline descriptors and WGSL sources; fixture frame remains control | PR #16 |
| `1c5da75e8` browser export validation | embedded export | Browser-safe `Pipelines::build` validates the embedded production export; full resource graph is still separate | PR #16 |
| `48e059aac` + current scene run | embedded export | Shared browser row now validates all 63 exported pipelines before rendering; full eleven-row scene passes | PR #16 |
| `0da7aa468` shared model material bindings | model owner | Native palette/material/texture bind-group order now has one browser-safe owner; palette growth rebinds through it | PR #16 |
| current browser material-owner run | shared model owner | Browser binds the exported `modelLayout` against array texture views and real fetched albedo data; scene records `sharedMaterials: true` | PR #16 |
| current browser lifecycle run | target/surface/device owners | Browser rebuilds target-bound fog/post resources after resize, exercises one surface recovery, destroys the real device, and recreates the renderer with cached assets and presentation records; scene records generation 2, `surfaceRecoveries: 1`, `deviceLost: 1`, and `deviceRecoveries: 1` | PR #16 |
| current browser shadow-owner run | shared shadow resources | Browser allocates the shared four-cascade depth-array/receiver/camera owner; scene records `sharedShadow: true` while caster draws remain open | PR #16 |
| current exported model-prepass run | shared model pipeline | Browser binds `typegpuCameraLayout` + shared `modelLayout` and executes the exported 48-byte `modelPrepass` over the fetched indexed mesh; scene records `sharedModelPrepass: true` | PR #16 |
| current browser fog-compute owner run | shared fog pipeline owner | Browser allocates all four exported fog compute bindings in the shared owner; scene records `sharedFogCompute: true` | PR #16 |
| current model-opaque contract run | shared model/environment/fog groups | Binding the complete exported `modelOpaque` contract with placeholder environment/fog data produced visibly invalid fixture output; the pass is retained as a diagnostic, not the shipped frame path | PR #16 |
| `35c5f19fd` model-packing split | browser shell shape | Production vertex packing now has a named owner under the file-size ratchet without changing the draw contract | PR #16 |
| `3c5415d81` lifecycle scene gate | browser shell | The real scene fails on missing surface-recovery telemetry; the later device-loss gate now covers full recreation | PR #16 |
| current device-loss recovery | browser shell | `Device::destroy()` triggers the browser callback; `recover_device` rebuilds every GPU owner and replays the BGAT asset plus frozen camera/environment/fog/overlay records before the next frame | PR #16 |
| current presentation-record bridge | shared shell | Browser uploads the frozen presentation camera (40 words), environment (16), fog (24), and overlay (4) through one API; camera is bound by the shared model prepass and the remaining records are staged for extracted production groups | PR #16 |
| current overlay target owner | shared target lifecycle | Native and browser `TargetOwner` now allocate overlay depth, paint, resolved colour, and half-resolution glow attachments together on resize/rebuild; no visible overlay pass is enabled yet | PR #16 |
| current environment/shadow browser owners | shared shell | Browser constructs the nine-resource environment bind group and uploads the frozen sun receiver plus cascade camera records; `CasterRouting` preserves per-cascade batch masks (the terrain-only fixture has zero admitted casters) | PR #16 |
| current fog-look browser owner | shared fog-mask | The 24-word `FogLook` record is written into the shared fog-mask owner and its screen-space passes encode before post; fog sight visibility buffers and compute dispatch remain open | PR #16 |
| current fog-sight browser owner | shared fog compute | Shared owner allocates params, eyes, foliage, occluders, maps, tiles, wholes, flags, and all four compute bind groups; browser exposes packed upload validation, while dispatch remains opt-in until height pages and due-state handling are ported | PR #16 |
| current frozen fog upload | presentation/browser boundary | `PresentedGround.prepare_fog` supplies the authoritative packed fog upload; the browser reinterprets those records byte-for-byte, uploads them twice across device recovery, and the scene requires `fogSightUploads: 2`; pixels remain unchanged because dispatch is still opt-in | PR #16 |
| current modelOpaque probe | shared model/environment/fog groups | The browser now builds a valid exported `modelOpaque` command path with rigid palette/material rows, real environment and fog groups, and correct 48-byte joint/weight packing. It remains opt-in because the screenshot still does not show credible production model output | PR #16 |
| current shared-target presentation run | shared world target + browser composite | The shared row now stores the fixture world pass before fog/post and presents the shared resolved `lit` target through a fullscreen browser composite. The prior black frame was an orchestration bug (`kept: false` discarded the world colour before post); the rebuilt screenshot now shows the shared sky gradient and fixture geometry. This is a credible shared-target checkpoint, but not production BattleFrame parity: modelOpaque, real caster batches, fog dispatch and overlays remain incomplete | PR #16 |
| current presentation terrain upload | shared world input boundary | The browser uploads the frozen presentation mesh (xyz/normal/rgba records) and the first 16 camera words into the shared Rust world owner. The mesh is rebuilt after device loss from a CPU snapshot, and the scene still passes the resize/surface/device recovery gates. The shared screenshot now contains the actual terrain mesh rather than the old fullscreen fixture; the composite UV orientation was corrected so ground is below the black sky, while production props/models still need parity work | PR #16 |
| current presentation world upload | shared world input boundary | The same upload now concatenates the frozen terrain and static world-prop mesh. The shared screenshot shows the real ground plus the presentation props through the Rust target; clipping, sky treatment, shadows, units and overlays are still visibly incomplete | PR #16 |
| current overlay upload | shared browser overlay boundary | The browser uploads the frozen opaque/translucent/painted overlay meshes and redraws them after the shared target composite. Rings, order lines and paint now appear in the shared screenshot; this is still a shell-side diagnostic surface pass, with overlay target composition and production unit/model draws pending | PR #16 |
| current model-instance upload | shared model input boundary | The browser now forwards the decoded authored instance stream into the shared vertex owner and replays it after device loss. The exported prepass remains active; the opaque model is still not visible in the shared screenshot, so the remaining defect is in the model camera/material/fog contract rather than transport or buffer lifetime | PR #16 |
| current full environment upload | shared model shading boundary | The browser now packs the 64-word exported `Environment` record (observer, sun, fill and aerial fields) instead of writing only the 16-word sun suffix. This removed the underspecified lighting input; the subsequent visibility fix makes the model appear, while full bundle material parity remains open | PR #16 |
| current model visibility fix | shared model lifecycle/vertex boundary | The model diagnostic flag is preserved across device recreation, and the authored vertex adapter now packs a valid +Z snorm16 normal. The shared screenshot now shows the authored model after recovery; its material is still a neutral diagnostic row rather than the bundle's full material table, so texture/material parity remains open | PR #16 |
| current bundle material upload | shared asset/material boundary | Browser asset loading now writes up to sixteen decoded BGAT material rows and propagates mesh draw material ids into the authored vertex stream. The model remains visually stable; full multi-layer texture-array upload is the next parity step | PR #16 |
| current accounting run | three-way harness | Shared browser WASM is 1,328,627 bytes plus 82,501 bytes generated JS; the shared row encodes 121 frames at 5.6 ms with a 2.5 ms compositor drain, while forced device recovery is included in its 264.4 ms setup. Overlay submission reports 242 draw calls. Battle image distance remains 47.03 because production fog/shadow/overlay target composition is not yet matched | PR #16 |

## Handoff

Incumbent: `9783fa7b8` plus the shared browser-owner commits on PR #16.
Active task: consume production fog visibility buffers and overlay draws in the
extracted browser pass groups, then compare the matched production frame.
Regression set:
renderer-core/native-client WASM checks, presentation tests, and the interleaved
one-renderer scene. The latest rebuilt scene passes all eleven rows and executes
the fetched mesh through exported model prepass, restores records after device
loss, and reports the complete frozen record lengths. Next test: bind real
environment/fog records to production shading and compare the matched frame.
