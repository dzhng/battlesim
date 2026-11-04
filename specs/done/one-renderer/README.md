# One renderer

One Rust `wgpu` runtime owns rendering policy for browser and native. Both
targets consume canonical presentation and authored assets through the same
resource composition, preparation, materials and frame encoder. Platform shells
provide devices, surfaces, asset transport and platform services. The simulation
remains the battle authority; rendering observes a side's presentation.

The purpose is to stop paying for two implementations of the game's look and
resource lifetimes. Sharing shader text alone would still leave two places to
decide which assets, passes and attachments constitute a frame. Sharing only a
frame-order callback would permit the same divergence behind each callback.
The concrete [Runtime](../../../crates/renderer-core/src/runtime/) and
[FrameRenderer](../../../crates/renderer-core/src/frame.rs) therefore own both
preparation and drawing. The [core contract](../../../crates/renderer-core/README.md)
defines that boundary; the [browser shell](../../../crates/renderer-browser/README.md)
and [native client](../../../native/client/README.md) own platform admission.

## Canonical presentation, one page module

Presentation owns poses, visibility, model tiers, cascade membership and held
static batches. A second browser interpretation of those records would recreate
the ownership problem inside the renderer. Synchronous renderer calls therefore
borrow the actual Rust presentation instead of copying records between memories.
The [presentation bindings](../../../crates/presentation-wasm/README.md) own the
shared-module and memory-view contract.

The existing browser entrypoint links the bindings: an umbrella package would
add a name without adding an owner. The [page initializer](../../../web/src/battle/sim/presentation.ts)
owns initialization.

## Lifetimes follow the authority that can rebuild them

The following constraints prevent apparently valid frames from using stale art,
knowledge or GPU objects:

- Acquire drawable output before consuming incremental presentation work.
  [Browser rendering](../../../crates/renderer-browser/src/render.rs) enforces
  this ordering; an unavailable surface must not eat the next fog/scar update.
- Keep presentation borrows synchronous. Asset baking and queued readbacks own
  the resources and metadata they need while promises await completion. See
  [asset generations](../../../crates/renderer-core/src/runtime/assets.rs) and
  [readback](../../../crates/renderer-core/src/readback.rs).
- Commit canonical appearance facts and matching GPU resources together. Reject
  stale device/runtime/catalog candidates and require a fresh canonical frame
  before drawing newly installed rows. The viewport's newest desired request
  controls scheduling; completion order cannot choose the active catalog.
- Follow the core's [device reconstruction contract](../../../crates/renderer-core/README.md#device-reconstruction).
- Reuse model/effect uploads only while the canonical frame and renderer static
  generation remain unchanged. Fog and scar preparation still runs for every
  draw; their incremental work must be consumed rather than replayed.

Full-frame surfaces, targets and browser camera projection share a minimum
extent through `presentation::records::MIN_TARGET_PX` and
`renderer_core::render_size`. A collapsed canvas must remain a usable render
target: otherwise the bloom pyramid creates zero-sized textures. Raw small
offscreen targets remain available for isolated passes.

The browser page owns one GPU shared by battle and detached tooling canvases.
Dropping a scene releases its scene owners; page departure disposes the device.
The production application treats unexpected GPU failure as terminal until
Reload. Explicit device recreation remains a tested harness capability, rather
than a hidden replacement of the shared page device.

Immutable geometry belongs to its resolved authored state/pose, rather than to
each appearance routing row. Sharing it reduced GPU-owned payload in the
measured generated skirmish from 2.670 GB to 1.224 GB with identical pixels.
This is an admission-memory result, not a frame-time attribution. Decoded assets
and external geometry remain available for reconstruction and have a CPU cost.

## One shader and pipeline owner

Maintained [WGSL modules](../../../crates/renderer-core/src/shaders/README.md)
own algorithms; the typed Rust [pipeline catalog](../../../crates/renderer-core/src/pipeline_registry/)
owns layouts and state for both targets. Composition expands shaders only into
Cargo build output. Whole-identifier aliases adapt binding/local names without
duplicating algorithms or changing expression order. Neither shell supplies a
production scene-shader loader or a second descriptor manifest. The browser's
final surface-copy shader is platform presentation transport; its display
pipeline is built by the shared core outside the scene catalog.

[ABI reflection](../../../crates/renderer-core/tests/pipeline_abi.rs) checks the
assembled shaders against canonical presentation layouts. Material diagnostics
query the actual composed terrain shader rather than maintaining a test shader
that could agree with itself while production drifted. Environment lookup data
and effect-sheet ordering also have platform-independent owners.

## Tradeoffs and limits of the evidence

The measured benefit is fewer independent rendering-policy owners and lower
browser CPU cost, with a small GPU-cost increase in the frozen matched cohort.
The ownership reduction does **not** establish a source-line reduction. Authored
WGSL counts as maintained source; deleted generated expansion is separate, and
both endpoints of renamed paths count. [Research](research.md) owns the exact
cohorts, artifact identities, baseline-relative accounting and verification limits; the
[inventory tool](../../../tools/renderer-accounting.mjs) reports current owner and
artifact sizes. The architectural benefit is fewer independent policy and
lifetime owners, with additional Rust, bindings and verification code to maintain.

Native timestamp collection resolves counters after render completion to avoid
stale Metal end stamps. This profiling synchronization changes pacing; native
also omits surface work included in browser measurements. Its complete sample
coverage is useful, but these separate cohorts establish no native speedup.
Owned GPU payload, retained CPU payload, WASM linear-memory high-water size,
route transfer and whole-process residency are different measurements. Strong
diagnostic handle registries would themselves prolong Rust-created WebGPU
lifetimes; memory visitors retain handles only during the snapshot.

Startup measures application readiness over loopback, not actual display
presentation or external network latency. Startup and long-battle peak memory
are separate measurements. [Whole-browser memory](memory.md) compares
the browser, GPU process, pages and workers before/after the port and subsequent
memory fixes. Its process totals are the primary memory result; comparing only
the old presentation WASM with the shared renderer WASM misses the old path's
JavaScript art and GPU allocations.

## Pitfalls worth retaining

The standalone production-shaped Rust spike proved transport and submission,
but omitted material, lighting and lifetime owners. It could neither establish
production parity nor justify migration through a smaller line count. Synthetic
models and empty effects streams were similarly transport evidence only.

Loading the whole installed catalog exhausted browser WASM address space.
Admission follows the human's enabled purchase roster and the observer's learned
unit kinds, including remembered contacts, corpses, crashes and wrecks. A page
cache cannot broaden GPU residency. Loading only visible models instead broke
immediate purchase ghosts; retaining the own-faction roster deliberately pays
more memory to preserve that response. [Source selection](../../../apps/battle-lab/src/gameAppearances.ts)
owns this lifetime, reset by battle and observing side.

Downloading additional art must not withdraw the running battlefield. One
complete topology/map-art/model-art generation remains admitted while its
replacement resolves. Audio continues receiving publications. [Fielding](../../../apps/battle-lab/src/useBattleFielding.ts)
correlates each asynchronous result with its request and battle/observer scope.
The viewport then reconciles sources after canonical framing, choosing one
world rebuild or one asset transaction. Eager feed listeners previously started
both and reached WASM's address limit. Immutable bundle reads now return shared
handles rather than deep copies of decoded mesh and texture payloads.

Picking, unit positions, observation and projection camera commit with a
successful draw. A skipped surface cannot advance interactions ahead of the
image. Changes to existing external poses reframe before their draw completes.
The [viewport tests](../../../web/tests/viewport.test.tsx) pin these ordering and
lifetime contracts through the renderer boundary.
Waiting for an observed frame before admission deadlocked because publications
began only after GPU readiness; staged topology must be admitted first.

Repeated preparation of an unchanged held frame uploaded large streams on every
draw. Canonical frame identity supplies the bounded reuse rule without hashes or
another JavaScript dirty protocol.

An early terrain-color `depthCompare: always` workaround hid incomplete depth
inputs. The shared production descriptor uses reverse-Z `greater-equal`.
An isolated descriptor control explains the named native change: eleven pixels
across six plates, maximum channel delta 4/255; the old descriptor restores the
old images byte for byte. This correction does not justify widening visual gates.

## Visual provenance

This is a preservation task. The approved browser
[foundation](../../../web/scenes/baselines/frame/foundation.png),
[geometry](../../../web/scenes/baselines/frame/geometry.png) and
[river](../../../web/scenes/baselines/frame/river.png) screenshots define the
existing production appearance. Their unchanged spatial gates cover terrain,
authored geometry, fog and player marks; small raw raster differences in those
scenes must not be described as byte identity.

The browser TypeGPU `paris-corner` capture supplies the matched historical
street control; native replay supplies six before/after plates. Whole images
and local sky, vehicle, facade, street, material and shadow crops guard against
missing content hidden by an average. An effects-heavy plate supplements this
coverage; an explosion obscuring the street cannot establish model parity.
The roster order picture deliberately preserves movement dust. A controlled
TypeGPU run fixing only its unstable faction dependency produces the shared
renderer’s exact effect records; the original repeated world reloads erased
those effects. The other roster references and all visual thresholds remain
unchanged. Fresh unprimed reviews found no candidate-specific missing-model or
depth defect. Shared vehicle
hatching, weak contact shadows and muted secondary paint remain visual concerns.

The retained [TypeGPU street control](assets/paris-240-typegpu.png) and
[shared Rust result](assets/paris-240-rust.png) are native-size captures from the
matched frozen production plate240 cohort. They preserve the historical visual
standard after the TypeGPU implementation was removed. The [native control plates](assets/native/before/) and
[shared-runtime plates](assets/native/shared/) preserve the six-plate extraction
comparison, while the [TypeGPU effect state](assets/paris-effects-typegpu.png)
and [Rust effect state](assets/paris-effects-rust.png) preserve its supplemental
effect coverage. Detailed crops, metrics and critiques remain evidence under ignored
`throwaway/`; these are preservation comparisons, not a new art direction.


The deliberate roster reference decision retains its matched full order views:
[original TypeGPU](assets/roster/typegpu-dev.png),
[TypeGPU with stable fielding dependency](assets/roster/typegpu-stable-dev.png),
and [shared renderer](assets/roster/current-dev.png). All show the same tick and
complete observation; the stable TypeGPU/shared pair also has exact effect
records. This preserves both the original dust-free result and the control that
explains why authored dust remains in the approved order picture.

Air-hover’s historical TypeGPU and shared-renderer full-frame references retain
[selection](assets/air-hover/typegpu-frame-selected-1280x800.png) and its
[shared view](assets/air-hover/shared-frame-selected-1280x800.png), the
[ghost](assets/air-hover/typegpu-frame-ghost-1280x800.png) and its
[shared view](assets/air-hover/shared-frame-ghost-1280x800.png), and the
[unheld ghost](assets/air-hover/typegpu-frame-ghost-unheld-1280x800.png) and its
[shared view](assets/air-hover/shared-frame-ghost-unheld-1280x800.png). They are
preservation controls, not newly approved UI designs. Their original cameras and
clipped silhouettes remain intact. Unrecorded rotor/muzzle animation phase
precludes a general exact-pixel claim; [research](research.md) records the
broader paired comparison and its limits.
