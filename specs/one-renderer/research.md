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

## Current task — production feature integration (2026-10-28)

The committed incumbent is `ab0dc6629`, the unified runtime and presentation
module. The current pass adds player depth/paint/x-ray, separate high-air output,
canonical tooling poses, actual impostor atlases/cards, live controls and explicit
GPU queries. Production viewport/sheet integration and deletion remain open.

The captured browser street now includes external articulated poses and a
placement ghost alongside paint, pointer marks and selection. The source-pixel
mean RGB difference is `0.0000014467592592592592` on the 0–255 scale. All eight
inspector modes match their TypeGPU control: final/world have that same tiny
difference; both overlay backgrounds, both masks, classes and paint match exactly.
Leaving inspection restores both complete frames, and resize/device recovery
preserves both browser canvas outputs in this run. Evidence:
`throwaway/evidence/one-renderer/player-feature-review/capture/`.

Actual authored MRT impostor baking matches TypeGPU albedo, normal and mask
bytes, metadata and hash exactly. Native GPU regressions draw real relit cards,
exercise a card-only high-air frame and prove missing submissions fail. Asset
admission now explicitly separates synchronous presentation borrowing from async
GPU preparation. Configurable review bakes and probes queue owned snapshots
before returning their promises, so asynchronous completion retains no scene
borrow. Evidence: `throwaway/one-renderer-impostor/` and focused GPU test logs.

The native delta is isolated: exporting the actual TypeGPU pipeline removes
the earlier Rust-only terrain `depthCompare: always` workaround in favor of
reverse-Z `greater-equal`. With the same binary and WGSL, reverting only this
descriptor restores all six old images byte for byte. The correct common rule
changes eleven pixels / twenty-one RGB channels across 5,529,600 pixels, maximum
delta 4/255; all remaining channels and counts are preserved. Current-current
repeats and old/new camera words are bit-identical. Keep the shared depth rule
as a named correction, recorded in choices; do not widen a screenshot threshold.
Evidence: `throwaway/native-depth-control-result.json`.

Browser readback now proves a queued frame snapshot remains byte-identical to
the source while a different inspector view renders before the promise resolves.
Restoring controls restores the complete source image. The browser executes the
same real fog-shape shader and returns the required ordered samples. All default
eleven-row controls still pass. New neutral image review finds no visible A/B
difference; shared vehicle hatching, weak contact shadows and muted secondary
paint remain existing visual concerns, with no migration-specific change.

Independent integrated Codex review found three correctness gaps and completed
successfully. All are resolved: recovered fog queries restore retained knowledge
before consuming canonical uploads; skinned lab bodies/corpses publish their
actual card capability; resolved paint clears independently before the MSAA pass
because Metal may skip untouched tile resolves. The new recovery query failed
with false visibility behind walls, then matched after the Runtime fix. The lab
card regression failed on tier 3, then produced actual tier 4 while ghosts kept
mesh tiers. Existing hidden-paint GPU clearing, all eight browser views, queued
readback and full captured recovery remain green. Review and red/green evidence:
`throwaway/review/player-core/`, `throwaway/fog-recovery-probe-{red-2,green}.log`
and `throwaway/lab-cards-{red,green}.log`.

These are feature correctness experiments, not a new performance cohort. The
43.2% changing-frame CPU saving below predates this pass. The earlier separate-
module CPU saving also predates module unification; neither isolates a causal
contribution from module packaging or an individual rendering optimization.

## Historical native owner replacement (2026-10-28)

Incumbent: PR #16, based on `ab00ccef5`, with the current shared native model,
static-batch and scenery extraction. The browser no longer has its own model
vertex/material/state interpretation. The native client directly reexports the
same owners. Replaced model probes, range decoders and nonindexed pass modules
are deleted; the browser no longer depends on the native platform crate.

The normal eleven-row scene passed with the shared native model owner. The
focused captured pair now installs the same complete appearance generation and
calls BattleFrame's separate static-frame upload as production does. The first
captured runs omitted that call and scoped scenery to trees; their timing/image
results are **invalid as matched production comparisons**. They exposed missing
buildings and proxy parked cars in the control and were corrected, not promoted.

The valid `plate-240` development run uses 151 appearance rows, 36 unique bundles,
30,538,294 unique compressed asset bytes, 2,850,537 resident model vertices,
227,838 scenery vertices, 232 prepared model runs and seven scenery runs. It
renders ten effects and passes resize, surface recovery and real device recovery.
Raw results and images: `throwaway/evidence/one-renderer/plate-240.png/`.

| Browser target | Setup | CPU encode, 120 frames | Queue completion after encode |
| --- | ---: | ---: | ---: |
| TypeGPU BattleFrame | 810.1 ms | 50.3 ms | 386.8 ms |
| Shared native Rust core in WASM | 706.9 ms | 12.1 ms | 558.6 ms |

Three interleaved repeats; setup excludes forced recovery on the candidate, but
fixture construction/cache order still affects startup. Queue completion now uses
`onSubmittedWorkDone` in both paths, rather than a compositor animation frame.
These are development diagnostics, **not a migration verdict**: sky is absent and
lighting/fog/ground still differ. No GPU timestamp or complete shared memory
accounting is established. The old synthetic `drawCalls += 5` telemetry is deleted.

An independent Codex review found another shared-fixture defect: the browser
control consumed the presentation's incremental fog upload before a fresh Rust
renderer attached. A captured-scene assertion reproduced the missing full
snapshot (red), then `resend_fog()` before the initial snapshot made it green.
The corrected candidate restores the adjacent parked car and removes broad
hatching from the street/sidewalk. The table above is this corrected run.

Matched screenshot metrics still reject parity: grayscale MAE
`48.86802`, pixelmatch ratio `0.79827`,
parity distance `0.43178`. Sky remains absent, shaded buildings too
dark, and fog/material treatment differs. Full-frame/cropped reports and fresh
critique live under `throwaway/evidence/one-renderer/paris-240-review/fog-after/`.
A green transport/lifecycle scene does not override this red visual gate.

This extraction checkpoint deletes 1,185 net physical source lines (including
comments, excluding docs), before TypeGPU removal. That is incremental deletion
against `ab00ccef5`, not the final migration source budget. Asset admission,
model/scenery/held-batch routing now each have one Rust owner; native and browser
call those same owners. Remaining native shell rendering is still counted.

The next experiment reused the actual native sky, environment, ground, grass,
scars and fog owners, then replaced both callback frame adapters with one concrete
`FrameRenderer`. **Keep:** the captured `plate-240` control/candidate RGB MAE is
`0.000010049120099044128`, with zero grayscale/pixelmatch/edge distance in the
full frame and the architecture, sky, tank and street crops. The native client
also reproduces all six pre-extraction native plates byte for byte. A fresh
unprimed critique found no candidate-specific regression; diagonal vehicle
striping and weak tank grounding are visible in both controls and remain
existing visual concerns. Evidence: `shared-frame-review/`,
`native-shared-frame-diff/` under `throwaway/evidence/one-renderer/`.

The detailed development protocol uses serial queue completion between frames,
three rotated repeats and 120 actual GPU timestamp samples in every repeat.
Both controls use the same marker-pass technique; instrumentation remains off
in the default batched-throughput run. CPU includes each renderer's preparation
and encoding, including the candidate's cross-WASM fog/scar transport. The
normal eleven-row regression and captured lifecycle checks pass.

| Browser target, frozen 640×360 street | CPU p50 across repeats | CPU p95 across repeats | GPU mean across repeats | GPU p95 across repeats |
| --- | ---: | ---: | ---: | ---: |
| TypeGPU BattleFrame | 0.390–0.410 ms | 0.445–0.610 ms | 3.635–3.653 ms | 3.804–3.829 ms |
| Shared Rust runtime in WASM | 0.120–0.130 ms | 0.140–0.230 ms | 3.945–3.955 ms | 3.998–4.017 ms |

Owned GPU payload is `408,256,869` bytes for TypeGPU and `372,836,805` bytes for
the Rust candidate, including profiling allocations. Rust additionally reports
`185,333,300` decoded asset payload bytes and `11,155,820` recovery snapshot bytes
on the CPU. These are ownership/payload figures, not whole-process memory or
driver residency; the TypeGPU control has no equivalent CPU heap measurement.
Raw per-repeat/per-frame evidence is in `shared-frame-profile/samples.json`,
with compact results in `shared-frame-timing-summary.json`.

These timings support a roughly threefold CPU reduction and an approximately
eight-percent GPU cost on this frame, not the final migration verdict. Browser
paint/x-ray/high-air/diagnostic features remain unported. The candidate still
needs canonical effect-atlas mip ownership and fog-look enable flags; startup
includes fixture/cache-order effects and is not a cold-download comparison.
Next: preserve these pixels while finishing the production feature seams, then
expand to an effects-heavy plate and matched native/browser resolution. Final
source accounting must include deletion of TypeGPU and the standalone graph.

A transient browser startup failure also occurred: a fresh static data page
failed before game code; after bounded GPU-disabled and Metal probes the default
browser loaded again. Those attempts have no renderer timing interpretation.
The first detailed timestamp run also timed out before any canvas reported a
stage. Adding progress stages to the TypeGPU control produced a complete repeat;
the timed-out attempt provides no timing result and remains an infrastructure/
initialization uncertainty rather than an assigned renderer regression.

## Next task — one browser module and complete runtime ownership

The separate-module shared frame is the incumbent. The CPU reduction above was
measured **before module unification** and is not evidence of a benefit caused
by it. It compares the complete measured TypeGPU/Rust paths on a frozen frame;
the savings have not been isolated to individual mechanisms.

A bounded scratch experiment linked the existing presentation bindings as an
rlib into the actual renderer. Both full class APIs, sound bindings and free
functions can share one WASM module; Node exercises canonical frame/fog borrowing
and exact camera records in that memory. Borrowing actual model slices into the
renderer compiles/exports. An exports-only control confirms that no artificial
probe exports are needed to retain the existing classes.

| Optimized WASM bytes | Separate modules, summed | Unified exports-only |
| --- | ---: | ---: |
| Raw | 5,035,788 | 4,952,412 |
| gzip level 9 | 1,631,450 | 1,567,702 |
| Brotli quality 11 | 1,102,643 | 1,025,680 |

Identical empty-world initialization/frame preparation retains `3,145,728` bytes
of separate WASM memories versus `2,031,616` unified. Freeing the presentation
does not shrink those memories. This is an empty-world Node control, not a
full-battle GPU or process-memory claim. All builds use matching optimization,
SIMD and wasm-opt settings, isolated scratch target/output, with no product
source changes. Detailed commands, hashes and evidence boundaries are in
`throwaway/unified-wasm-spike/REPORT.md`; the scratch build target was removed.

**Keep for implementation:** one browser module and direct canonical borrowing
can delete cross-memory frame/fog/scar protocol and recovery replay machinery.
**Unproven:** actual unified GPU pixels, frame cost, full-workload memory and
complete runtime ownership. Renderer allocations now also invalidate JavaScript
presentation views when memory grows, so the integration must refresh them and
keep every class on the same generated initializer.

The next discriminating test is [slice 03](slices/03-runtime.md)'s real unified
GPU path against the saved separate-module street, native plates and lifecycle
incumbents. Share resource composition and preparation, including atlas mips
and live fog-look flags, before deleting the extra bridges. A separate direct
output-target trial may explain the remaining GPU cost; do not attribute that
or any combined gain to packaging without an isolated comparison.

The independent shared-frame review found an existing overlay-loss bug during
device recreation. The harness now renders before loss, removes manual scene
replay after recreation, and compares raw before/after pixels. The missing marks
were reproduced red, retained overlay source geometry makes the check green,
and an unprimed critique confirms continuity. Evidence: `recovery-pixels-red.log`,
`recovery-pixels-confirmation.log`, and `recovery-review/` under the renderer
evidence folder. One attempted green run overlapped lint-staged hiding the
unstaged harness and is invalid; its workload lacks the new comparison field.

## Unified runtime development results

The actual browser integration now links presentation/audio/rendering in one
artifact and calls the common Runtime for composition/preparation/rebuilding.
Its first captured GPU run preserves the same browser street distance
(`0.000010049120099044128` mean RGB, 0–255), exact recovery pixels and all six
native incumbent plates byte for byte. New neutral critique finds no
candidate-specific regression; both controls retain the previously reported
vehicle hatching and weak tank grounding. Evidence: `unified-runtime-first/`,
`unified-runtime-review/` and `unified-native-plates.json` under renderer evidence.

The first held-frame trial exposed repeated full model uploads: the new Runtime
prepared the same materialized publication on every draw, unlike the old frozen
control which uploaded it once outside measurement. Its 223.5 ms CPU total over
120 frames is retained as a rejected preparation policy, not a packaging cost.
The runtime now reuses model/effect buffers until the canonical frame index
changes or its static generation is invalidated. Fog/scar work still prepares
every actual draw. Actual counters confirm one model/effect preparation across
121 held-frame preparations after recovery. Changing-frame validation remains a
separate gate; it must include every changed publication's preparation.

Three rotated serial/timestamp repeats after held-frame reuse:

| Frozen 640×360 street | CPU p50 range | CPU p95 range | GPU mean range |
| --- | ---: | ---: | ---: |
| TypeGPU in current cohort | 0.595–0.735 ms | 0.760–0.990 ms | 3.758–3.894 ms |
| Unified shared Rust runtime | 0.275–0.300 ms | 0.330–0.400 ms | 4.069–4.150 ms |

Both current CPU controls are slower than the saved separate-module cohort, so
those historical absolute values cannot isolate the effect of packaging. The
current pair supports a whole-path CPU saving and a modest GPU overhead on this
held frame only. Raw repeats: `unified-runtime-profile/samples.json`.

The integration also adopts canonical effect PNG decoding/mips and live typed
fog settings, so it is a coupled runtime change, not an isolated module toggle.
Decoded atlas payload moves into asset accounting; its full mip chain adds GPU
storage compared with the old browser's base-only atlas. Owned memory excludes
allocator/driver/canvas residency. The shared module's linear memory remains at
its high-water size after payloads are freed; per-stage values are recorded in
the raw workload rather than described as live retained payload.

Changing-frame development confirmation now replays the exact same 120 captured
frames and 60 authority publications in both browser backends. It includes
publication decode, canonical framing and each backend's per-frame preparation
in the measured loop. Current model/effect counters are 121 each including the
warm frame, and final camera/view/shadow/caster words and presentation probe
match exactly. Source 640×360 output differs in 13 channels, maximum 2, mean RGB
`0.00002170138888888889` (mean RGBA `0.000016276041666666668`). Fresh visual
critique found no candidate-specific difference.

| Changing 120-frame street window | Per-repeat CPU mean | Per-repeat GPU mean |
| --- | --- | --- |
| TypeGPU | 1.1385 / 1.1570 / 1.2805 ms | 6.543 / 4.035 / 4.743 ms |
| Shared Rust | 0.6808 / 0.6465 / 0.7052 ms | 6.736 / 6.646 / 4.376 ms |

The CPU cohort mean is 1.1920 ms versus 0.6775 ms (43.2% lower for Rust). GPU values
have substantial between-repeat drift and a per-repeat reversal; their cohort
mean is not a stable GPU verdict. Evidence and complete raw samples:
`changing-plate240-attempt3/`. Final dynamic-vs-frozen history is not byte-identical;
those are different presentation histories and are not compared as equal outputs.

Two earlier dynamic attempts are retained. Attempt1 saved the TypeGPU image
after destroying its device, allowing a prior compositor frame and a misleading
CSS image difference 2.42082. Attempt2 found raw post-destroy TypeGPU canvas pixels
were black. Attempt3 retains each final native-size canvas before disposal,
outside the measured loop. It preserves CSS captures as secondary evidence and
proves the actual final source pixels. Neither failed capture is renderer parity
or timing confirmation.

## Hypothesis queue

1. **Unified runtime (verified):** one browser module and direct canonical
   preparation should remove transport/recovery owners while preserving held
   and changing images. Held/recovery/changing/native checkpoint gates pass.
   Module packaging alone has no isolated frame-time verdict.
2. **Browser feature completion (active):** shared paint, x-ray, high-air, lab
   models, quality and diagnostics must preserve every production consumer.
   Core paint/x-ray/lab/inspector captures pass; actual impostor baking matches.
   Production viewport, proxy/sheet admission, diagnostic bindings and final
   quality controls remain open. Native depth-policy delta is isolated and named.
3. **Final measured/deleted architecture:** compare both browser backends and
   native at matching workload/resolution, then delete TypeGPU and the separate
   spike graph. Source totals must count the resulting shared runtime and shells.
4. **Direct output target (optional, separate experiment):** removing the extra
   browser display-composite pass may reduce GPU overhead. No causal result yet;
   preserve output formats and readback when testing it.

## Parameter-effect map

| Strategy | Current evidence | Effect | Status |
| --- | --- | --- | --- |
| Standalone Rust production-shaped graph | Incomplete production materials/shadows/fog/post | Useful transport probe, invalid migration comparison | Reject as final architecture |
| Shared native resource/frame owners | Matched frozen browser street; all six native incumbent plates unchanged | Removes duplicated resource and pass policy | Keep |
| One browser module + common Runtime | Actual GPU recovery/frozen/changing parity; Node memory/header/audio checks | Deletes canonical frame/world/fog/scar copies, bridge callbacks and recovery replay | Keep |
| Reprepare every held model/effect stream | First unified frozen trial repeatedly uploaded large unchanged streams | Unnecessary CPU work, 223.5 ms over 120 held draws | Reject |
| Reuse unchanged canonical model/effect frame | Actual counters 1 model / 1 effect across 121 held preparations; every changed frame prepared in dynamic cohort | Restores bounded held-frame upload work; fog/scar preparation stays live | Keep |
| Canonical PNG decode/full atlas mips | Shared native/browser source; frozen/native images preserved | Removes browser decoding and atlas owner; adds mip storage versus old browser atlas | Keep |
| Owned allocation visitor | Descriptor-based deduplicated GPU handles, transient diagnostic lifetime | Measures owned payload without keeping dropped resources alive | Keep; excludes allocator/driver residency |
| Browser TypeGPU as permanent second renderer | Two runtime graphs | Retains duplicate policy and lifetime owners | Reject after production parity |
| Canonical lab poses and ordinary model batches | Actual articulated/ghost browser capture and canonical frame regressions | Shares authored poses, batching, fog and model GPU ownership | Keep; viewport migration open |
| Actual MRT impostor atlas/cards | Native vs TypeGPU bytes/hash exact; real card/tint/high-air GPU regressions | Replaces coarse-mesh substitution with production far-model behavior | Keep; final startup/memory measurements open |
| Shared live inspector/quality controls | All eight captured browser views match; raw/mask/paint GPU negative controls | Keeps debug depth/color/post policy in the same renderer | Keep; remaining production wiring open |

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

## 2026-10-27 captured Paris workload

The shared browser row now has a second diagnostic input selected by
`ONE_RENDERER_CAPTURE=1`. It reads the committed `paris-corner` capture, decodes
every publication through the `plate-660.png` tick, resolves the capture unit
catalog and the Paris map through the same browser WASM adapters, loads the
map's city kits and biome scenery, and sends the resulting `ScenePresentation`
frame through the shared Rust core. The one-shot run completed 120 measured
frames plus the recovery frame with `791.7 ms` setup, `8.8 ms` CPU encode and
`0.1 ms` queue-boundary drain. It carried 662,508 model words, 4,048 effect
words, 240 material words, 37/41/41/41 caster batches and 484 shadow passes;
the full record is saved in `throwaway/evidence/one-renderer-capture/result.json`.

This run found and fixed two real ownership defects: the adapter had been
passing simulation unit ids as appearance requests, and the shared browser
model instance buffer was a fixed one-megabyte probe allocation. The loader now
resolves unit ids to their worn appearance set and the Rust shell grows its
renderer-owned instance buffer before upload, preserving the same recovery
snapshot contract.

The captured run is intentionally still open. The browser shell's asset bootstrap
loads only the `test_tank` drawable into the shared model owner, so the capture
frame's decoded appearance/material records are not yet all admitted to GPU
drawables; the two reported model runs are therefore not a complete capture
visual comparison. Attempting to drive the full capture through the existing
BattleFrame control exceeds the scene's 60-second gate while installing the
complete appearance generation. The next experiment is to stream every
appearance/tier referenced by the capture into the shared drawable table, then
rerun the browser-control versus shared-Rust screenshot and GPU comparison on
the same plate. No substitute model or relaxed gate is acceptable.

The model geometry slice now packs all non-empty tiers from the loaded
appearance into one vertex stream and routes each accepted batch to its tier's
vertex range. The shared row reports 49,020 model vertices, 240 authored
material words, and the same recovery gates. This closes the tier-range seam;
multiple appearance bundles, articulated transforms, and real presentation
batch selection remain open.

The real-frame slice now closes that last transport-only model gap. The browser
constructs a deterministic simulation battle, publishes one real observation,
resolves its `fieldedBattle` bundle records and clip tables, and frames the
result through `ScenePresentation`. The shared browser upload receives the
actual batch, instance, palette and authored material arrays, plus the real
terrain/fog/shadow records; the latest row reports 920 model-record words and
240 material words and passes resize, surface recovery and device-loss replay.
This is still an input/lifecycle gate: the ground remains the lab's frozen
visual mesh, and the native renderer has
not yet rendered the identical production frame for image comparison.

Auto-research follow-up `c574cfd97` removed those explicit fixture eyes. The
same publication now supplies the blue unit sight records through
`ScenePresentation.setFog({ sensors })`; the complete eleven-row scene still
passes with no WebGPU errors, `fogEyes: 2`, 920 model words and 240 material
words. The candidate is kept because it removes a lab-only visibility input
without changing the shared lifecycle/workload gates. Setup/encode/queue timing
remained within run-to-run noise (candidate 364.9/6.9/1.2 ms versus the prior
346.6/6.7/0.6 ms), so no performance claim is attached to this change.

The next visual probe fixed the browser asset adapter's vertex ABI. Its prior
packer kept positions and colours but synthesized normals, malformed UV bytes,
rigid weights and a zero tangent. It now preserves the bundle's normals, UVs,
colours, joints, normalized weights, tangents and material ids in the same
48-byte layout as `modelLayer.packVertices`; the before/after shared-frame PNGs
are byte-different and the full scene remains green. The fresh visual review
now sees textured authored vehicles, but still rejects the frame as production
parity: the vehicles remain low-contrast, terrain marks alias heavily, the
ground ends in black, and contact/cast shadows and gameplay anchors are absent.
This keeps the ABI fix and makes those remaining defects explicit next work.

The ABI packer is now `renderer_core::model_vertex::pack`, with the browser
bundle reader reduced to a thin attribute adapter. A renderer-core unit test
pins the 48-byte offsets and exact weight/tangent narrowing; the full WASM scene
still passes unchanged. This removes one browser-only owner and leaves the
native shell able to consume the same pack contract when its model upload is
moved across the boundary.

The model prepass and opaque draw encoder now also live in
`renderer_core::model_pass`; the browser shell only adapts assets and calls the
shared pass functions. The six renderer-core tests, WASM check, clippy and full
eleven-row scene remain green. This is a structural simplification with no
separate performance claim; the visual review still gates terrain, lighting and
shadow work.

Accounting after the move is 3,423 lines across the shared Rust core and 2,403
in the browser shell, versus 3,173 and 2,541 before it. The combined total is
slightly higher because the generic core seam and its validation are new, but
the browser-only owner shrinks by 138 lines and the pass code is now available
to the native shell. This is the intended direction; the final deletion audit
must still remove the standalone spike and the remaining duplicate native
adapters.

The real-publication visual probe now reuses the frozen lab's complete static
props and overlay mesh while retaining the simulation publication's model,
material and fog records. The shared row still passes all lifecycle gates and
now reports eight fog-height words plus the authored overlay workload. This
removes the previous sparse-scene confound; fresh review still finds terrain
aliasing, weak contact shadows and a black map edge, which are genuine shared
world/material parity defects rather than missing fixture inputs.

Fresh review of the enriched frame confirms the remaining defects: repeated
white terrain artifacts dominate the scan; the selected vehicle's silhouette is
still a brown slab under a clearer cyan ring; the right vehicle is nearly lost
under a detached red marker; barriers and vehicles lack readable contact
shadows; the orange range arc is clipped; and the finite ground falls into a
black void. These are now the active visual parity queue, not transport gaps.

The shared render probe also removed the duplicate compatibility ground draw
whenever production terrain tables are present, and initialized the no-scar
directory/page textures explicitly. The duplicate draw was a real ownership
bug: it put the same mesh through the fixture and production terrain passes at
the same depth. The full scene remains green; the remaining dark terrain
pattern persists, so terrain material/fog composition is still an open parity
issue rather than being hidden by a second ground owner.

An auto-research shadow probe disabled shared cascade encoding for one run. The
terrain artifact was unchanged while the expected shadow-resource gate failed,
so the artifact is not caused by the shadow map itself. The probe was restored
immediately; the next active hypothesis is the shared terrain/fog material ABI
or its synthetic ground table inputs.

The shared frame-order pass moved native and browser encoding behind
`renderer_core::frame::encode_frame`. The canonical order is sun shadow,
depth, fog sight, world, effects, fog mask, overlays, post and fog rim. Native
adapts its existing pass vector; browser adapts both the production and fixture
resources. Surface acquire, display composite, overlay composite and present
remain shell operations. The full eleven-row scene stays green after the
change, including resize, surface recovery, device loss and 484 shadow passes.
The browser effects slot is intentionally empty until the effect resource owner
moves into the shared runtime, so this proves orchestration only and does not
close production pass parity.

The next effects probe constructed the browser's shared `EffectsResources`
owner, bound it to the production depth, camera, environment and PMREM records,
rebuilt it after resize, and replayed it during device recovery. A dummy one
layer atlas plus the frozen 16-word effect instance was deliberately rejected:
the fresh screenshot showed black voids swallowing most of the terrain and
units, so the atlas ABI is not proven by a bind-group that merely validates.
The browser scene now keeps the owner present but uploads zero effect instances
until the real flipbook fetch/decode path is wired. This preserves the clean
frame boundary without accepting a visual regression.

The terrain-hole probe used the current shared browser row as the fixed parent.
Skipping fog-mask compose left the holes unchanged, so fog composition was
discarded. Skipping production terrain depth removed all terrain, proving the
color pass depends on that reverse-Z coverage. Adding depth bias `1` to either
`worldPrepass` or `worldTerrain` also left the image unchanged. Changing only
the exported `worldTerrain` color pass to `depthCompare: always` restored the
full ground while models, static props, resize, surface recovery, device
recovery and 484 shadow passes stayed green. Keep the shared pipeline change;
the exact browser/native depth-value mismatch remains a lower-priority root
cause investigation.

The first real-atlas trial used an invalid hand-written effect record: its
shape word selected a streak, so the screenshot showed a detached white
rectangle. The record was corrected to the presentation ABI for a flipbook
(`a = position/radius`, `b = turn/frame/layer/softness`, `colour`, then
`misc = shape/emissive/lit/pad`). The atlas and upload owner are kept. The
browser row now removes that fixture record entirely and uploads the copied
`ScenePresentation` frame effect stream, including after device recovery. The
current deterministic battle publication produces zero effect words at its
sampled tick, which is evidence that transport is no longer hiding a fake
effect, not evidence of visual effect parity; a publication with a real launch
or impact remains a required workload.

An environment-input A/B copied the native `View.view` matrix and observer
words into the browser's 64-word environment record instead of the previous
identity view. The shared scene stayed green and its production image distance
remained `47.034991`, so this fixes an ABI mismatch without explaining the
dark model/material result. A second A/B skipped fog-mask compose for one run;
the shared image and distance were unchanged, so fog compose is not the source
of that darkness. The fog probe was restored; model material/shadow lighting
parity remains the active hypothesis.

The browser workload now reports resolved model-run admission separately from
uploaded vertex words. The real frozen frame resolves one admitted presentation
model run (`presentationModelRuns: 1`) while the shared model prepass executes;
the dark silhouettes therefore are not an empty-draw artifact. The bundle-row,
vertex-colour, instance-stream, static-world-colour, shadow-floor, and
fog-compose A/Bs all left the `47.034991` production image distance unchanged.
The remaining visual defect is inside the shared model/world lighting ABI or its
authored resource values, and is still open for the migration verdict.

The native parity harness also completed a real `paris-corner` control capture
after fetching the runtime LFS textures: six 1280×720 plates, 2,461 frames,
1–28 vehicles/soldiers per plate, and 10–253 effect instances per plate. This
is a usable native control workload for the final matched-frame comparison; it
is not yet wired into the one-renderer browser row.

Native replay of that same committed capture produced six Rust plates and matched
the browser control's counts on every plate. The fixed-image comparison is also
near-identical: parity distances are `0.00478`, `0.00641`, `0.00144`, `0.00011`,
`0.00166`, and `0.00007` (worst `plate-660`), with pixelmatch ratios at or below
`0.00308`. This validates the native control and capture contract; it does not
yet compare the extracted shared browser core, which still lacks the full
appearance/world upload for this workload.

The matched `paris-corner` performance controls are now recorded as well. The
native release replay runs 2,400 post-warmup frames at 84.6 FPS, with CPU p50
`1.13 ms`, CPU p95 `6.76 ms`, GPU mean `11.57 ms`, and GPU p95 `14.14 ms` on
Apple M6 AC power. The browser production capture runs 2,341 post-warmup
frames at a paced 60 FPS, with CPU p50 `3.24 ms`, CPU p95 `3.80 ms`, GPU mean
`9.10 ms`, and GPU p95 `12.08 ms`. These are controls for the same native
renderer workload; the shared browser Rust row still needs this workload before
its performance tradeoff is final.
