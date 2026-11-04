# One-renderer choices ledger

Consolidated against the completed implementation. Verdicts judge architectural
choices; verification and measurement limits live in research.

The later [memory evidence](memory.md) records a related lifetime decision:
immutable texels share content identity across bundles and replacement drafts,
while drafts carry only requested source changes. The index holds weak
references so it cannot become an independent retention owner. Parsed browser
art remains the rebuild source; serialized geometry exists only during admission.
This spends repeat serialization work to remove a permanent duplicate payload.

Review these first: memory measurements describe several different kinds of
ownership; explicit native timestamp profiling changes pacing; shader composition
keeps a small custom build step. All final decisions below are judged sound.
Confidence ranks how certain the audit is that the user would choose the same
tradeoff. Superseded transport spikes and temporary migration gates are omitted.

## Sound — medium confidence

### Measure owned payload and wrapper release separately

**When:** shared-frame diagnostics and production viewport migration, 2026-10-28.

After leaving a battle, two different questions matter: how many bytes its
renderer owned, and whether its browser GPU objects remain reachable. The memory
visitor temporarily deduplicates buffer/texture handles and counts allocation
descriptors: buffer sizes and texture blocks across mips, layers and samples.
These are **owned GPU payload bytes**, excluding driver, pipeline and surface
storage. CPU snapshot payload counts retained external world/player geometry;
CPU bundle payload counts decoded model assets and effect sheets. These CPU
figures exclude presentation, selection caches and allocator overhead.
WebAssembly linear memory is **capacity/high-water size**, not live payload;
JavaScript managed heap and the separate simulation worker are separate domains.

Release observation uses weak object identities, drains queued work on departure
and explicitly collects browser garbage before inspecting old identities. A
strong registry would itself keep dropped Rust resources alive. Collection is a
verification operation, never gameplay. Wrapper disappearance establishes neither
driver reclamation nor process-memory reduction.

**Gap:** the plan required memory/lifetime evidence without defining these
measurement domains. **Reach:** future GPU owners must join the visitor; future
reports must preserve these definitions. **Verdict:** sound; measurement must not
change the lifetime it observes. **Confidence:** medium; these are useful bounded
facts, not a complete memory census.

### Pay the counter-resolution wait only in explicit native profiling

**When:** final native timestamp collection, 2026-10-28.

When a native benchmark submits a frame, Metal can expose a stale end timestamp
if counters are resolved too early. The explicit profiler now waits for that
render submission, submits counter resolution separately, then reads its result.
Ordinary rendering does not acquire this profiling wait. CPU preparation and
encoding spans end before it; measured frame intervals include its pacing cost.
Reports retain indexed raw samples and coverage and omit GPU aggregates if a
required sample is missing. Resolving immediately and averaging only successful
samples would produce a more flattering but incomplete number.

**Gap:** the protocol assumed same-submission counter resolution was reliable.
**Reach:** these profiled intervals cannot establish ordinary renderer FPS or a
speedup against old instrumentation; platform surface costs also remain distinct.
**Verdict:** sound; complete counters justify explicit measurement overhead.
**Confidence:** medium; the instrumentation changes the workload's pacing.

### Compose maintained WGSL with whole-identifier aliases

**When:** final shader ownership cleanup, 2026-10-28.

When terrain and models need the same lighting calculation but their uniforms
have different names, they select one authored WGSL block, the GPU shader source
language, with identifier aliases such as `params=terrainParams`. The build step
substitutes complete identifier tokens once and joins ordered named blocks;
binding declarations remain explicit. Copying the calculation into each pipeline
would create independently editable algorithms. Expanded pipeline files exist
only in Cargo output. Typed Rust declarations own actual pipeline state, and
diagnostics serialize those same depth states; shader layout reflection reads
the canonical presentation record metadata.

**Gap:** one shader owner was required, but its authoring/composition format was
open. **Reach:** future shader work maintains the block library and composition
index together. Both count as maintained source. Generated-export deletion is
separate: maintained authoring and generated expansion are counted separately in research;
no maintained-line reduction is established. **Verdict:**
sound; a small composition owner
avoids repeated algorithms and a second authoring toolchain. **Confidence:** medium;
the custom index is still machinery future contributors must understand.

### Use one 64px minimum for full-frame rendering and its camera

**When:** final review of tiny viewport handling, 2026-10-28.

If a hidden or shrinking viewport requests 1×1 pixels, a full production frame
still creates a bloom pyramid of progressively smaller images. Unclamped sizes
can create a zero-sized bloom texture. The canonical presentation records now own
`MIN_TARGET_PX = 64`, generated into TypeScript. Browser camera construction uses
it, and `renderer_core::render_size` applies the same floor to runtime creation,
resize and browser surface configuration. A 1×100 request therefore renders with
a 64×100 camera and targets. Separate constants could let the camera and render
attachments disagree. Raw offscreen `Targets` allocation still permits smaller
sizes; this decision governs the complete frame with post-processing.

**Gap:** the plan did not choose a minimum extent for the production pyramid.
**Reach:** full-frame consumers must use the shared size contract; smaller raw
targets remain available to specialized offscreen work. **Verdict:** sound; the
existing camera floor supplies a common valid production extent. **Confidence:**
medium; arbitrary tiny full-frame output would require a different pyramid policy.

### Admit the player's purchase roster and retain learned source kinds

**When:** final browser source-lifetime and purchase-response repair, 2026-10-28.

A tiny lab must not upload another battle's cached wrecks, while a player must
still get an immediate ghost when selecting an unfielded reinforcement. One
source closure includes the human's enabled purchase cards for its faction,
initial fielded units, authored map wrecks and everything the observer has learned.
It includes corpse, crash, remembered-contact, transit and wreck references even
when their live units are absent. Learned kinds stay for this battle and observing
side; a different observer or battle starts its own history. Other factions’ roster-only kinds enter on observed/pending demand, rather
than uploading every faction roster before play. The same closure feeds canonical fielding and GPU admission.

The page may retain downloaded art from earlier visits, but that cache is filtered
before canonical prop selection. A loaded catalog entry does not by itself admit
a mesh. Transport stays bounded per bundle. Review sheets also install their
actual ground and catch up the current world before framing.

**Gap:** catalog availability did not define admission scope, history retention
or immediate purchase readiness. **Reach:** this retains a faction roster plus
learned art for the battle's life, costing more memory than current-visible-only
loading and avoiding repeated source retirement. It does not load every faction
up front. **Verdict:** sound; complete scene demand determines residency, with
purchase responsiveness preserved. **Confidence:** medium; growing content still
needs measured admission budgets.

## Sound — high confidence

### Link browser presentation, rendering and audio into one WASM module

**When:** unified browser runtime integration, 2026-10-28.

When drawing a battle, Rust rendering synchronously borrows the actual Rust
presentation, the owner of what this side can currently see. Browser audio uses
the same initialized binding identity. Copying a frame through JavaScript into
a second WebAssembly memory would require another schema, decoder and recovery
snapshot. The simulation remains in its own worker; DOM, input and fetched bytes
remain browser responsibilities. An allocating rendering or audio call can grow
the shared page memory and detach an earlier JavaScript typed view, so callers
reacquire publication views after such calls and rendering republishes its header.

**Gap:** the plan inherited separate modules without settling their final
packaging. **Reach:** new page bindings must share the initializer and respect
the short view lifetime. **Verdict:** sound; canonical borrowing removes boundary
owners. **Confidence:** high. Packaging alone establishes no causal CPU saving.

### Keep one GPU device for the page and platform loading outside the core

**When:** production viewport and workbench migration, 2026-10-28.

Opening a detached workbench canvas uses the page's existing `BrowserGpu`, the
owner of the adapter, device and queue. Each renderer owns its canvas and scene;
closing one drops those resources without disposing the page device. Page
departure explicitly disposes it. Giving every canvas its own device would add
independent lifetimes and prevent this common ownership. Native still reads
files and the browser fetches bytes; the shared renderer receives source assets
and external geometry and never opens files or touches the DOM. JavaScript gets
owned diagnostics/readbacks rather than raw GPU handles.

**Gap:** the plan allowed platform shells without fixing device scope or the
asset handoff. **Reach:** future tool canvases share page admission; new asset
sources remain platform adapters. **Verdict:** sound; each lifetime has one owner.
**Confidence:** high.

### Finish synchronous borrowing before asynchronous GPU work

**When:** unified runtime and production appearance/query integration, 2026-10-28.

A model card bake or pixel query may finish after the user resizes, replaces art
or leaves the scene. Its synchronous setup captures owned GPU resources and
snapshot metadata, then returns a promise with no live presentation or renderer
borrow. For a pixel read, dimensions and bytes belong to the submitted image even
if a later resize changes the canvas. Holding a borrow until mapping finishes
would lock the live observer; looking up resources afterward could silently read
the replacement scene.

**Gap:** shared synchronous preparation did not settle asynchronous lifetimes.
**Reach:** future bakes, visibility probes and readbacks must own their pending
work and release live borrows before awaiting. **Verdict:** sound; pending work
cannot follow a later frame accidentally. **Confidence:** high.

### Commit only the newest complete appearance generation

**When:** production catalog and workbench migration, 2026-10-28.

While edited model A is baking, the committed model keeps drawing and receiving
pose updates. If edits B and C arrive, desired intent becomes C while at most one
candidate is preparing. When A finishes, it is discarded; C prepares next. At a
frame boundary, a complete candidate commits only if its device, runtime,
canonical world and desired request still match. The commit moves canonical art
facts, GPU assets and retained recovery sources together, reapplies current
controls and the latest poses/proxies, and requires a fresh canonical frame.
Committing each partial result could show A's geometry with C's facts or restore
old art after device recreation.

**Gap:** latest-generation behavior did not prescribe concurrency or commit scope.
**Reach:** asset changes replace asset owners while retained observation/history
stays live; topology changes use whole-scene admission. **Verdict:** sound;
bounded preparation and one commit preserve matching ownership. **Confidence:** high.

### Ignore obsolete successes and failures; terminate on current GPU failure

**When:** production admission lifetime review, 2026-10-28.

Suppose a viewport starts loading one world, then the user changes world or
quality before it finishes. Completion checks the captured presentation, source,
world and quality identities: `obsolete → dispose/ignore; current → install or
report failure`. The same check applies when preparation rejects. Late appearance
callbacks also check the current scene before touching presentation. Otherwise
an old failure could unmount the new scene or an old success could mutate it.
Invalid dropped art remains source validation with the current model visible;
a current required-GPU failure reaches the existing terminal application boundary
and manual Reload policy. Supersession does not create automatic GPU retries.

**Gap:** successful replacement was specified, but obsolete rejection and current
required-resource failure policy were not. **Reach:** reset, quality, departure
and future asynchronous admission follow the same identity rule. **Verdict:**
sound; stale work has no authority over current readiness. **Confidence:** high.

### Share immutable geometry while keeping routing rows distinct

**When:** production skirmish memory admission, 2026-10-28.

Two catalog rows can place the same authored model differently without needing
two copies of its vertices. Admission shares immutable GPU mesh tiers, the
authored levels of geometric detail, when source and resolved state/pose agree.
Rows retain placement, visibility, material routing and corpse metadata; different
corpse orientations remain distinct. A construction-local sharing map disappears
after admission, so later edits install a new generation rather than invalidate
a persistent cache. Allocating vertices for every routing row would multiply art
storage without changing what is drawn.

**Gap:** the plan chose a shared runtime, not immutable vertex ownership.
**Reach:** future row variants may share geometry while preserving their own
meaning; memory/submission diagnostics inspect actual resources and draws rather
than permanent migration counters. **Verdict:** sound; routing identity does not
imply mesh ownership. **Confidence:** high.

### Match canonical fielding to one stable complete request

**When:** final browser source/topology admission, 2026-10-28.

Canonical fielding resolves unit descriptions and animation inputs asynchronously.
A stable request object names its actual rules, catalog, kinds, factions and side.
New topology, map art and model art enter together only after that exact request
and its source closure are complete. While a replacement loads, the last complete
matching battle/observer generation keeps drawing; changing battle or observer
retires it immediately. Source reconciliation happens at the frame boundary,
after canonical framing, so a changed world rebuilds once instead of also
preparing an obsolete asset-only generation. Otherwise new art can meet old canonical rows, or a newly allocated
React dependency can continuously reload the observer and erase its effects.
The raw presentation still receives publications for audio while visual admission
waits; gating that raw reference would drop real sound evidence.

**Gap:** the plan required coherent source generations without choosing how the
session correlated asynchronous fielding results. **Reach:** future fielding
inputs belong in the request identity; source updates preserve history when the
request is unchanged. The roster order picture intentionally retains authored
dust that the original browser's repeated reloads erased. **Verdict:** sound;
readiness names actual completed inputs. **Confidence:** high.

### Complete frame requests only after current successful submission

**When:** final browser readiness and evidence repair, 2026-10-28.

Two animation callbacks do not prove a canvas drew: a scene can still be rebuilding,
waiting for art, or have no acquired surface. Rendering returns whether submission
actually succeeded. Viewport-owned waiters complete only after that draw uses the
current world and complete asset generation; initial readiness follows the same
rule. Drawn observation, clock, pick boxes, unit positions and projection camera commit
with submission. Pose-only updates reframe before their draw completes. Errors
and departure reject waiting explicit callers; obsolete internal rebuild failures
cannot refuse page-lifetime resources. Replacement inherits current controls.

**Gap:** frame completion and asynchronous refusal semantics were unspecified.
**Reach:** capture and diagnostic callers can trust completion without queue-drain
waits; visual submission still does not prove the display compositor presented it.
**Verdict:** sound; completion reports the operation it names. **Confidence:** high.

### Let canonical records select model ranges, materials and fog classes

**When:** canonical model/scenery extraction and production frame integration,
2026-10-28.

When a frame names an appearance, detail tier, instance range and shadow cascade,
the common model owner draws exactly that run. Held static markers refer to
shared static owners; they are not frame-local offsets. A run cannot silently
fall back to the first loaded mesh, and terrain faces do not send props through
terrain materials. The runtime reads the real Rust presentation record types for
models, effects, terrain and optional fog/scar updates. Absence preserves an old
optional table; an explicit empty table clears it. Fog resources initialize the
canonical surface-layer policy before the first upload. Flattening these records
to counts or local decoders would lose culling, routing or update meaning.

**Gap:** extraction left packed input interpretation and admission enforcement
open. **Reach:** future material/tier/static/caster work extends these named
shared contracts and the canonical ABI, the record layout consumers agree on.
**Verdict:** sound; presentation decides the run and rendering preserves it.
**Confidence:** high.

### Keep attachment and frame order in the common runtime

**When:** shadow, fog, post and production-frame extraction, 2026-10-28.

For a drawn frame, the shared runtime clears and fills the fitted shadow
cascades, builds physical depth, computes sight, draws materials/effects, composes
fog and completes post-processing and information overlays into its display
texture. It owns attachment clear/load/resolve choices, named resource inputs
and overlay/halo composition. Shells acquire and present output and may request
readback. Giving the browser its own shadow loop, fog attachments or final
compositor would allow different images despite shared shader text.

**Gap:** the plan required shared passes but did not settle attachment ownership
and the final composition boundary. **Reach:** a new pass joins one common frame
and resource owner instead of a platform's parallel graph. **Verdict:** sound;
one owner controls the complete image. **Confidence:** high.

### Rebuild all target-dependent bindings together on resize

**When:** shared browser resize and runtime integration, 2026-10-28.

When a viewport changes size, old textures cannot stay hidden inside bind groups,
the GPU records connecting shader inputs to resources. The shared runtime
replaces targets and rebuilds post, fog-mask, overlay, depth-dependent fog,
x-ray, high-air and effect bindings before the next draw, then asks presentation
to resend size-dependent fog work. Replacing textures alone would keep old
attachments reachable and let later passes read a different-sized frame.

**Gap:** required resize support did not specify the dependent-resource boundary.
**Reach:** every new size-dependent pass must participate in this common rebuild.
**Verdict:** sound; attachments and their consumers change together.
**Confidence:** high.

### Recover current knowledge and held lists from presentation

**When:** shared-frame recovery and unified runtime integration, 2026-10-28.

A recreated device has no fog textures, scar cache or held static GPU buffers.
The recovery path retains decoded assets and external geometry, creates ordinary
runtime owners on the new device, and asks live presentation to resend current
fog/scar knowledge and materialize its held static lists into the current frame.
It preserves that frame's motion, selection, effects and time, then publishes
fresh static generation data for subsequent frames. Replaying the latest small
upload cannot restore complete knowledge; rebuilding the whole UI frame might
choose another moment. The header is republished after materialization because
the record arrays can move.

**Gap:** recovery was required without choosing retained CPU representation or
knowledge reconstruction. **Reach:** decoded art/geometry retention costs belong
in memory accounting. A device-recreation caller supplies a fresh page GPU and
prepares assets; production's terminal failure/manual Reload policy remains the
policy above. **Verdict:** sound; recovery uses the current authority rather than
a second renderer or stale frame mirror. **Confidence:** high.

### Reuse model/effect streams for the same canonical frame

**When:** held-frame preparation integration, 2026-10-28.

When the browser redraws a paused frame, model and effect buffers already match
that frame and can be reused. A different frame index, static generation or
relevant control invalidation prepares them again; a newly installed world has
no reusable state. Fog and scars still prepare every draw because incremental
work must be consumed and completed rebuilds must not be replayed. Uploading
everything on every held draw repeats work; JavaScript hashes or dirty flags
would add another description of change beside presentation's own identity.

**Gap:** shared native preparation did not specify repeated drawing of a held
browser frame. **Reach:** future stream reuse must follow canonical identity and
the owner's invalidation rule. **Verdict:** sound; unchanged records permit
reuse without a second change protocol. **Confidence:** high. No whole-path
performance saving is attributed to this decision alone.

### Keep physical depth on the production camera and terrain path

**When:** production depth integration and obsolete-workaround removal,
2026-10-28.

When real terrain draws color, it compares against a prepass built from the same
production camera and geometry. The common reverse-Z convention stores closer
surfaces with greater depth, so terrain color uses `greater-equal`. The early
spike's `always` comparison hid incomplete input/depth plumbing; retaining it
would bypass the now-complete physical contract. Shadows likewise use the
presentation light's authored cascade resolution instead of the spike's tiny
diagnostic maps. Typed pipeline states also supply the reported depth facts.

**Gap:** migration did not say whether preservation included temporary depth and
shadow approximations after production inputs were complete. **Reach:** later
depth-sensitive layers and quality tiers start from the common physical contract;
quality changes must be deliberate. **Verdict:** sound; remove obsolete exceptions
when their missing inputs have a real owner. **Confidence:** high.

### Query production material functions with owned diagnostic inputs

**When:** production material-harness migration, 2026-10-28.

When a diagnostic asks for terrain distance, roughness or filtering results, it
appends query entrypoints to the actual composed terrain shader and uses the
installed runtime's material layout. It builds temporary canonical material
inputs and queues an owned result snapshot. If the query consumes scar uploads,
presentation is told to resend them so the live scene still receives its current
knowledge. A separately authored diagnostic shader could keep agreeing with
itself while the game's material behavior changed.

**Gap:** replacing JavaScript GPU tests left the diagnostic access contract open.
**Reach:** future material diagnostics exercise production algorithms without
exporting GPU handles or locking the live observer during mapping. **Verdict:**
sound; the question reaches the shader the game draws. **Confidence:** high.

### Bake authored environment light and keep lookup/sheet data neutral

**When:** environment parity and final shared-data cleanup, 2026-10-28.

When a scene installs its light, the shared owner bakes the sky from presentation's
light record and prefilters it into the reflection atlas used at different
roughnesses. It uploads the same authored split-sum lighting lookup on both
targets. Neutral placeholder texels would satisfy GPU types but change rough
surface lighting. The immutable lookup now lives as binary data, and effect
sheet ordering lives in a common JSON manifest; native reads neither value by
parsing TypeScript source. Both targets decode the same effect sheets, and the
common renderer keeps the third-party attribution beside the adapted algorithms.

**Gap:** the plan did not choose precomputed files versus startup GPU bakes, or
the permanent owner for data previously embedded in browser source. **Reach:**
future environment and effect changes update shared data and algorithms rather
than a platform-language parser or approximation. **Verdict:** sound; resource
dimensions, authored light and asset order have one platform-independent owner.
**Confidence:** high.


### Share decoded bundle ownership through admission

**When:** final browser memory repair, 2026-10-28.

When GPU admission reads a model, it receives an immutable shared handle to the
already decoded bundle. The source library owns that payload, and consumers may
retain the handle while preparing a replacement. Reading a source must not make
a fresh copy of all mesh and texture arrays. The same contract applies to native
filesystem sources and fetched browser sources.

**Gap:** the plan specified source lifetime without settling returned ownership.
**Reach:** bundle readers return `Arc<Bundle>`; intentional mutable fixtures copy
only when they change a bundle. This reduces duplicate payload without adding a
cache or a parallel asset registry. **Verdict:** sound; immutable data has one
payload owner rather than a new copy per admission read. **Confidence:** high.
