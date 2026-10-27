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
| current non-flat height-page run | frozen fixture mesh + one 2×2 presentation page | Shared browser uploads an eight-word page payload, dispatches fog, and replays it after device loss; scene remains green, with full production terrain/material parity still open | PR #16 |
| fog-before-world ordering probe | native pass order + exported `worldOpaque` | Moving shared fog dispatch before the production world draw does not make the direct fixture image valid; retain native ordering in the eventual extraction, but do not claim receiver parity until world/fog publication inputs are complete | PR #16 |
| current shared-target presentation run | shared world target + browser composite | The shared row now stores the fixture world pass, runs shared fog/post/rim/overlay composition, and presents the shared display target through a fullscreen browser composite. This is a credible shared-target checkpoint, but not production BattleFrame parity: real caster batches, full model/world material passes and height-page fog parity remain incomplete | PR #16 |
| current presentation terrain upload | shared world input boundary | The browser uploads the frozen presentation mesh (xyz/normal/rgba records) and the first 16 camera words into the shared Rust world owner. The mesh is rebuilt after device loss from a CPU snapshot, and the scene still passes the resize/surface/device recovery gates. The shared screenshot now contains the actual terrain mesh rather than the old fullscreen fixture; the composite UV orientation was corrected so ground is below the black sky, while production props/models still need parity work | PR #16 |
| current presentation world upload | shared world input boundary | The same upload now concatenates the frozen terrain and static world-prop mesh. The shared screenshot shows the real ground plus the presentation props through the Rust target; clipping, sky treatment, shadows, units and overlays are still visibly incomplete | PR #16 |
| current overlay upload | shared browser overlay boundary | The browser uploads the frozen opaque/translucent/painted overlay meshes into the shared production overlay owner. Rings, order lines and paint now render against world depth, blur into the shared halo target, and composite after the fog rim; production unit/model draws remain pending | PR #16 |
| current model-instance upload | shared model input boundary | The browser now forwards the decoded authored instance stream into the shared vertex owner and replays it after device loss. The exported prepass and opaque model draw both run after device recovery; the remaining defect is material/lighting parity rather than transport or buffer lifetime | PR #16 |
| current full environment upload | shared model shading boundary | The browser now packs the 64-word exported `Environment` record (observer, sun, fill and aerial fields) instead of writing only the 16-word sun suffix. This removed the underspecified lighting input; the subsequent visibility fix makes the model appear, while full bundle material parity remains open | PR #16 |
| current model visibility fix | shared model lifecycle/vertex boundary | The authored vertex adapter packs a valid +Z snorm16 normal and the opaque model draw is part of the normal shared frame. The shared screenshot now shows the authored model after recovery; its material is still a neutral diagnostic row rather than the bundle's full material table, so texture/material parity remains open | PR #16 |
| current bundle material upload | shared asset/material boundary | Browser asset loading now writes up to sixteen decoded BGAT material rows, propagates mesh draw material ids, and uploads decoded sRGB/linear bundle textures into shared 2D arrays with mip levels. The model remains visually stable; exact native material classification and all appearance tiers remain open | PR #16 |
| current fog dispatch | shared visibility compute boundary | The browser now forwards `FogUpload.rebuild_dispatch` and whole-structure dispatch metadata, uploads the shared height-page header, then runs terrain, merge, whole and cull compute stages in the shared frame after the authoritative fog upload. The scene passes with no WebGPU validation errors; non-flat height-page/visibility image parity remains open | PR #16 |
| current fog height upload | shared static ground boundary | Presentation WASM now exposes the packed `Grid::pages()` payload; the browser uploads it into the shared `fog-heights` storage owner, replays it after device recovery, and the scene records an eight-word non-flat fixture page. The transport and binding are real; full production page/material parity remains open | PR #16 |
| current shared shadow caster pass | shared cascade pass boundary | `renderer-core::ShadowResources` now owns cascade attachment clearing, camera binding, and ground caster draw order; the browser shell supplies the exported `worldCaster` pipeline and packed ground vertices. Four cascades execute every frame without changing presentation routing; model caster batches and receiver sampling remain open | PR #16 |
| current shared post/overlay composition | shared display target boundary | Browser post now receives the resolved bloom/grade records and presents `targets.display`; the shared overlay owner uses the exported translucent geometry pipeline against world depth, resolves the transparent overlay target, blurs its halo, composites it after the fog rim, and replays the records across resize/device rebuild. The scene remains green; exact BattleFrame pixels still differ because model/world material, receiver sampling, non-flat fog pages, and production framing are incomplete | PR #16 |
| current accounting run | three-way harness | Shared browser WASM is 1,347,401 bytes plus 84,761 bytes generated JS; the shared row encodes 121 frames at 6.0 ms with a 0.9 ms compositor drain, includes 484 ground and model shadow passes, replays recovery, and uploads an eight-word non-flat page. Battle image distance remains 47.03 because model/world material, receiver sampling, and production framing are not yet matched | PR #16 |

## Handoff

Incumbent: `9783fa7b8` plus the shared browser-owner commits on PR #16.
Active task: extract native terrain tables, material rows, fog publication state,
and static/model batch inputs behind the shared frame boundary, then compare the
matched production frame. The sky/PMREM/DFG environment and non-flat page gates
are already green; the direct exported world receiver remains diagnostic until
these inputs are real.
Regression set:
renderer-core/native-client WASM checks, presentation tests, and the interleaved
one-renderer scene. The latest rebuilt scene passes all eleven rows, executes the
fetched mesh through exported model prepass, replays authored environment assets
and a non-flat page after device loss, and reports the complete frozen record
lengths. Next test: bind real terrain/material/fog publication records to the
extracted production passes and compare the matched frame.

## 2026-10-27 terrain and model-record pass

The browser shell now has one shared `TerrainResources` owner for the native
`terrainLayout` contract. `GroundTables` crosses the presentation WASM seam as
params, nodes, plots, surfaces, surface index and scar-look words; the browser
adds the native four-word frame-view suffix, binds the exported `worldTerrain`
pipeline, and snapshots the upload for device recovery. A first run exposed a
uniform-size mismatch because the suffix was omitted; appending it fixed the
WebGPU validation error without changing shader work. This confirms the real
terrain input seam, but the scar directory/pool textures are still diagnostic
and the mesh is not yet selected by native terrain partition rules.

The same pass exposes a recovery-safe model-record upload for palettes,
instances and batches, and reports their word count. The frozen fixture sends
24 model-record words, while material rows remain zero and the full production
batch/material draw path is not yet selected. This is transport evidence, not
material parity.

The rebuilt `one-renderer` scene passes all eleven rows with no WebGPU
validation errors. The shared row reports 121 frames, 484 ground and model
shadow passes, two resize rebuilds, one surface recovery, one device recovery,
eight fog-height words, and 63 validated embedded pipelines. Its latest run
was approximately `176.4 ms` setup, `6.4 ms` encode and `0.2 ms` queue-boundary
drain for the 120 measured frames; setup includes asset loading and recovery,
so it is not a native startup comparison. The production BattleFrame distance
remains `47.03`, and fresh visual review still finds flat lighting, missing
readable contact shadows, clipped overlays and weak model detail.

The focused checks remain green: formatter, workspace clippy, renderer-core
check, renderer-browser WASM check, browser typecheck, shared-renderer WASM
build, and the full one-renderer scene. `cargo test -p presentation` has 111
unit tests passing; the known `ally_xray` integration prerequisite still points
at a Git LFS runtime bundle and is not weakened.

**Next experiment:** transport the authored material rows and static/model
batch ranges from the presentation frame into the shared model owner, bind
those records to the production model passes, and compare the same frozen
BattleFrame against native and browser TypeGPU output. Keep the terrain
directory/pool and receiver sampling as separate follow-up variables so a
visual improvement can be attributed.

The fresh screenshot review of the current shared capture adds concrete visual
regressions to that gate: high-frequency pale terrain noise overwhelms the
lime ground and weakens prop/overlay contrast; the dark bars read as floating
because contact shadows are absent; the right and lower slabs and the orange
arc are clipped; and the map boundary falls into black at the frame edges.
These findings reinforce that the current green harness is an input/lifecycle
proof, not a visual acceptance result.

The model-record follow-up also corrected the native ABI at the shared
boundary: `Instance` is 16 floats, `Batch` is 12 words, `Joint` is 12 floats,
and each material is 20 floats. The shared browser path now keeps dynamic
batch `first/count` ranges as separate draws and skips `NONE` static batches
until a browser static holder exists. This removes the earlier half-stride and
flattened-range errors. The current fixture still has no authored model runs
from `ScenePresentation`, so the next model gate must supply a real dynamic
frame or explicitly label a one-bundle fixture as transport-only.

The browser harness now uses a one-bundle frozen model fixture when the
presentation frame has no dynamic model rows: it creates 16-float instances,
12-word view batches, and one identity joint, then sends those through the
same shared upload and range-selection path. This is explicitly a transport
fixture, not a claim that `ScenePresentation` produced those rows. The latest
shared row reports 68 model-record words, 312 terrain-param words, 18,928
terrain-node words, 56,808 plot words, 432 surface-index words, and all
lifecycle gates green. The next production gate remains a real `PresentedFrame`
with appearance/tier drawable mapping and static-batch ownership.

The accounting snapshot after this pass is `1,371,265` bytes for shared WASM,
`88,332` bytes for generated JS, and `93,941` bytes for the one-renderer route
chunk. The latest measured shared row is `186.1 / 6.6 / 0.1 ms` for setup,
encode, and queue-boundary drain; these remain browser-compositor measurements,
not GPU timestamps.

The static-batch follow-up now moves held-range lifetime into the shared model
boundary. `BatchState` records dynamic ranges by key and resolves later
`first == NONE` records from that state; the snapshot is restored before the
browser replays model records after device loss. Renderer-core now has five
focused tests, including a held-range recovery contract. This closes the static
range lifecycle seam, while appearance/tier drawable mapping and a real
production frame remain open.

The latest model slice adds an explicit `(appearance, tier)` drawable
admission. The browser only accepts a batch for the loaded `test_tank` tier;
unknown rows are dropped instead of drawing the first available mesh. This
keeps the shared pass contract honest while the full appearance library is
still pending. The one-renderer scene remains green with no WebGPU validation
errors; this is still not a heterogeneous production-frame comparison.

The browser material accounting now retains the authored BGAT rows loaded from
the bundle instead of overwriting the count with the empty presentation-row
fixture. The latest shared run reports 240 material words alongside 68 model
record words, 121 frames, two resize rebuilds, one surface recovery and one
device recovery. This proves authored material transport and lifetime; it does
not yet prove that a real `PresentedFrame` selected those rows by appearance.

The latest accounting rerun keeps the same visual distances (`model_rust_vs_browser`
`0.0492`, `battle_rust_vs_browser` `47.035`) and reports the shared row at
`180.5 / 6.8 / 0.5 ms` setup, encode, and queue-boundary drain. These remain
submission-boundary measurements, not GPU timestamps.
