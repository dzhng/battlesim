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
| current browser shadow-owner run | shared shadow resources | Browser allocates the shared four-cascade depth-array/receiver/camera owner and the shared core now clears and draws uploaded ground caster geometry into every fitted cascade; the scene records `shadowPasses: 484` across the recovery-inclusive run. Presentation model caster routing is still zero for this flat fixture, and lit shadow sampling remains open | PR #16 |
| current exported model-prepass run | shared model pipeline | Browser binds `typegpuCameraLayout` + shared `modelLayout` and executes the exported 48-byte `modelPrepass` over the fetched indexed mesh; scene records `sharedModelPrepass: true` | PR #16 |
| current browser fog-compute owner run | shared fog pipeline owner | Browser allocates all four exported fog compute bindings in the shared owner; scene records `sharedFogCompute: true` | PR #16 |
| current model-opaque contract run | shared model/environment/fog groups | Binding the complete exported `modelOpaque` contract with placeholder environment/fog data produced visibly invalid fixture output; the pass is retained as a diagnostic, not the shipped frame path | PR #16 |
| `35c5f19fd` model-packing split | browser shell shape | Production vertex packing now has a named owner under the file-size ratchet without changing the draw contract | PR #16 |
| `3c5415d81` lifecycle scene gate | browser shell | The real scene fails on missing surface-recovery telemetry; the later device-loss gate now covers full recreation | PR #16 |
| current device-loss recovery | browser shell | `Device::destroy()` triggers the browser callback; `recover_device` rebuilds every GPU owner and replays the BGAT asset plus frozen camera/environment/fog/overlay records before the next frame | PR #16 |
| current presentation-record bridge | shared shell | Browser uploads the frozen presentation camera (40 words), environment (16), fog (24), and overlay (4) through one API; camera is bound by the shared model prepass and the remaining records are staged for extracted production groups | PR #16 |
| current overlay target owner | shared target lifecycle | Native and browser `TargetOwner` now allocate overlay depth, paint, resolved colour, and half-resolution glow attachments together on resize/rebuild; the browser shared owner now renders against world depth and composites its glow | PR #16 |
| current environment/shadow browser owners | shared shell | Browser constructs the nine-resource environment bind group and uploads the frozen sun receiver plus cascade camera records; `CasterRouting` preserves per-cascade batch masks (the terrain-only fixture has zero admitted casters) | PR #16 |
| current fog-look browser owner | shared fog-mask | The 24-word `FogLook` record is written into the shared fog-mask owner and its screen-space passes encode before post; fog sight uploads now dispatch terrain, merge, whole and cull stages | PR #16 |
| current fog-sight browser owner | shared fog compute | Shared owner allocates params, eyes, foliage, occluders, maps, tiles, wholes, flags, all four compute bind groups, and the five native layer records; browser exposes packed upload validation and dispatches the four shared stages; full height-page and due-state parity remains open | PR #16 |
| current frozen fog upload | presentation/browser boundary | `PresentedGround.prepare_fog` supplies the authoritative packed fog upload; the browser reinterprets those records byte-for-byte, uploads them twice across device recovery, and the scene requires `fogSightUploads: 2`; compute dispatch now runs from the uploaded counts | PR #16 |
| current modelOpaque path | shared model/environment/fog groups | The browser now runs the exported `modelOpaque` path as part of every shared frame with rigid palette/material rows, real environment and fog groups, and correct 48-byte joint/weight packing. The remaining visual gap is bundle material classification and lighting assets | PR #16 |
| current environment atlas probe | exported environment contract | Replacing 1×1 diagnostic sky/PMREM/DFG textures with the exported atlas dimensions and top-mip metadata removes the resource-shape mismatch, but a direct production `worldOpaque` draw is still visually invalid because authored sky bake, fog/terrain records, and native world inputs are not all present; keep the fixture world path until those inputs are extracted | PR #16 |
| current shared sky-bake run | exported `bake` pipeline + presentation `SkyLight` | Browser now bakes the 40-word resolved sky record into the shared LUT and replays it after device loss; PMREM filtering, authored DFG, and full world receiver inputs remain open | PR #16 |
| current shared PMREM run | exported `pmremBake`/`pmremFilter` pipelines | Browser builds the same 336×256 atlas and filters roughness levels after the sky bake; the scene remains green, with authored DFG and full receiver parity still open | PR #16 |
| current authored DFG run | checked-in `dfgLut.ts` + shared environment texture | Browser uploads the native 16×16 half-float DFG table instead of a neutral placeholder; scene remains green, with full world/material input parity still open | PR #16 |
| direct production world receiver recheck | completed sky/PMREM/DFG environment + exported `worldOpaque` | The receiver no longer has a placeholder resource shape, but the direct world image is still invalid because native terrain/material/fog publication inputs are absent; keep the fixture world path and extract those inputs before claiming receiver parity | PR #16 |
| current shared-target presentation run | shared world target + browser composite | The shared row now stores the fixture world pass, runs shared fog/post/rim/overlay composition, and presents the shared display target through a fullscreen browser composite. This is a credible shared-target checkpoint, but not production BattleFrame parity: real caster batches, full model/world material passes and height-page fog parity remain incomplete | PR #16 |
| current presentation terrain upload | shared world input boundary | The browser uploads the frozen presentation mesh (xyz/normal/rgba records) and the first 16 camera words into the shared Rust world owner. The mesh is rebuilt after device loss from a CPU snapshot, and the scene still passes the resize/surface/device recovery gates. The shared screenshot now contains the actual terrain mesh rather than the old fullscreen fixture; the composite UV orientation was corrected so ground is below the black sky, while production props/models still need parity work | PR #16 |
| current presentation world upload | shared world input boundary | The same upload now concatenates the frozen terrain and static world-prop mesh. The shared screenshot shows the real ground plus the presentation props through the Rust target; clipping, sky treatment, shadows, units and overlays are still visibly incomplete | PR #16 |
| current overlay upload | shared browser overlay boundary | The browser uploads the frozen opaque/translucent/painted overlay meshes into the shared production overlay owner. Rings, order lines and paint now render against world depth, blur into the shared halo target, and composite after the fog rim; production unit/model draws remain pending | PR #16 |
| current model-instance upload | shared model input boundary | The browser now forwards the decoded authored instance stream into the shared vertex owner and replays it after device loss. The exported prepass and opaque model draw both run after device recovery; the remaining defect is material/lighting parity rather than transport or buffer lifetime | PR #16 |
| current full environment upload | shared model shading boundary | The browser now packs the 64-word exported `Environment` record (observer, sun, fill and aerial fields) instead of writing only the 16-word sun suffix. This removed the underspecified lighting input; the subsequent visibility fix makes the model appear, while full bundle material parity remains open | PR #16 |
| current model visibility fix | shared model lifecycle/vertex boundary | The authored vertex adapter packs a valid +Z snorm16 normal and the opaque model draw is part of the normal shared frame. The shared screenshot now shows the authored model after recovery; its material is still a neutral diagnostic row rather than the bundle's full material table, so texture/material parity remains open | PR #16 |
| current bundle material upload | shared asset/material boundary | Browser asset loading now writes up to sixteen decoded BGAT material rows, propagates mesh draw material ids, and uploads decoded sRGB/linear bundle textures into shared 2D arrays with mip levels. The model remains visually stable; exact native material classification and all appearance tiers remain open | PR #16 |
| current fog dispatch | shared visibility compute boundary | The browser now forwards `FogUpload.rebuild_dispatch` and whole-structure dispatch metadata, uploads the shared height-page header, then runs terrain, merge, whole and cull compute stages in the shared frame after the authoritative fog upload. The scene passes with no WebGPU validation errors; non-flat height-page/visibility image parity remains open | PR #16 |
| current fog height upload | shared static ground boundary | Presentation WASM now exposes the packed `Grid::pages()` payload; the browser uploads it into the shared `fog-heights` storage owner, replays it after device recovery, and the scene records the three-word flat-fixture header. The transport and binding are real; this fixture has no present height pages, so non-flat production page parity remains open | PR #16 |
| current shared shadow caster pass | shared cascade pass boundary | `renderer-core::ShadowResources` now owns cascade attachment clearing, camera binding, and ground caster draw order; the browser shell supplies the exported `worldCaster` pipeline and packed ground vertices. Four cascades execute every frame without changing presentation routing; model caster batches and receiver sampling remain open | PR #16 |
| current shared post/overlay composition | shared display target boundary | Browser post now receives the resolved bloom/grade records and presents `targets.display`; the shared overlay owner uses the exported translucent geometry pipeline against world depth, resolves the transparent overlay target, blurs its halo, composites it after the fog rim, and replays the records across resize/device rebuild. The scene remains green; exact BattleFrame pixels still differ because model/world material, receiver sampling, non-flat fog pages, and production framing are incomplete | PR #16 |
| current accounting run | three-way harness | Shared browser WASM is 1,341,053 bytes plus 84,761 bytes generated JS; the shared row encodes 121 frames at 5.5 ms with a 1.3 ms compositor drain, includes 484 ground shadow cascade passes, and replays recovery. Battle image distance remains 47.03 because model/world material, receiver sampling, non-flat fog pages, and production framing are not yet matched | PR #16 |

## Handoff

Incumbent: `9783fa7b8` plus the shared browser-owner commits on PR #16.
Active task: port native-equivalent shadow caster draws and exercise the shared
fog height-page path against a non-flat production ground, then compare the
matched production frame.
Regression set:
renderer-core/native-client WASM checks, presentation tests, and the interleaved
one-renderer scene. The latest rebuilt scene passes all eleven rows and executes
the fetched mesh through exported model prepass, restores records after device
loss, and reports the complete frozen record lengths. Next test: bind real
environment/fog records to production shading and compare the matched frame.
