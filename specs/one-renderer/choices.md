# One-renderer choices ledger

## Sound

### Shared shadow resources own cascade attachment and ground-caster order

The browser had allocated four shadow layers and their camera records but did
not draw anything into them. The shared shadow owner now clears each fitted
cascade, binds its camera, and draws the shell-supplied packed ground caster
geometry. The native and browser shells can therefore use one cascade order and
depth policy; only the geometry source and platform resource setup remain at the
boundary. The alternative would have kept a browser-only shadow loop beside the
native pass.

**Gap:** the spec required one shadow owner but did not say whether attachment
clearing belonged to the owner or the shell. **Reach:** model/scenery caster
routing and receiver sampling must extend this same pass boundary. **Verdict:**
sound; the scene records four cascades per frame and the shared owner contains
the order. **Confidence:** high.

### Shared core owns post and overlay composition order

The browser used to copy the lit HDR target to the canvas and draw a diagnostic
overlay directly on the surface. That bypassed the production tone map, fog rim,
depth-tested overlay target, and halo blur. The shared core now owns the same
target sequence as the production frame: post writes the display target, the
rim overlays it, then overlay geometry resolves into its transparent target and
the halo/composite pass draws over the surface. The alternative would have kept
another browser-only compositor that could drift from native.

**Gap:** the spec required one pass owner but did not prescribe where the
surface composite should happen. **Reach:** native and browser must use this
ordering as later caster, model, and fog parity work lands. **Verdict:** sound;
the browser scene passes and the shared target owner is reused. **Confidence:**
high.

### Shared Rust owns attachment policy and pass ordering (fog-mask pass)

When the native client created the fog distance, compose, and rim passes, it
also decided which textures were cleared, resolved, or loaded. The shared core
now owns those choices, while native supplies only the presentation look bytes
and the browser shell can use the same resource owner. The alternative would
be two implementations that can silently disagree about which image is shown.

**Gap:** the original extraction plan named the owner but did not prescribe the
fog-mask API. **Reach:** future fog, post, and browser integration must reuse
this target/pass contract. **Verdict:** sound; the GPU work and order are
unchanged. **Confidence:** high.

### Keep filesystem and DOM asset loading outside the renderer core

Native reads effect sheets and runtime bundles from disk; browser fetches them.
The renderer core receives device-ready resources and records instead of
opening files or touching the DOM. This keeps one renderer while allowing two
platform shells. **Gap:** the spec allowed shell asset policy without naming
the exact handoff. **Reach:** effects and post extraction follow the same seam.
**Verdict:** sound. **Confidence:** high.

## Needs-user (provisional calls)

### Keep TypeGPU as a control until matched production parity

The implementation preserves the TypeGPU renderer while Rust gains production
passes, so screenshots and timings have a trustworthy control. The provisional
call is to delete it only after the shared Rust path owns every required pass
and lifecycle gate; this is reversible until that deletion commit.

**Gap:** the user delegated the migration decision but not the exact deletion
checkpoint. **Reach:** determines when browser source files and the dependency
can be removed. **Verdict:** needs-user, provisionally keep until parity.
**Confidence:** medium.

## Sound

### Rebuild target-bound shared resources together on resize

When the browser changes from 640×360 to 320×180, the target textures change.
The fog-mask and post bind groups point at those textures, so keeping the old
groups would submit work against stale attachments. The browser now swaps the
target owner and rebuilds those shared groups in the same operation. The
alternative would be a target swap with hidden old-resource ownership.

**Gap:** the spec required resize recovery but did not say which dependent
resources must be rebuilt atomically. **Reach:** every future size-dependent
pass must join this rebuild boundary. **Verdict:** sound. **Confidence:** high.

### Keep device-loss recreation as an explicit remaining gate

The browser shell can reconfigure an outdated or lost surface, and the scene
now exercises that call. A true device loss invalidates the device and every
GPU resource; the current shell reports the condition but does not recreate the
device and all owners. We keep that as an open gate rather than claiming the
surface recovery proves device recovery. **Gap:** the existing spike had no
device-loss injection contract. **Reach:** blocks final TypeGPU deletion until
the shell can rebuild the device/resource set or the product explicitly owns a
browser reload policy. **Verdict:** sound to leave open. **Confidence:** high.

### Shared shadow routing preserves each admitted model instance range

The presentation record does not merely say that a cascade has some casters;
each model batch names the first instance, the number of instances, and the
cascades that admitted that run. The shared Rust shadow owner now keeps those
ranges and issues one draw for each admitted run inside the cascade's render
pass. A model that was culled from a cascade therefore cannot cast there just
because another model did. The alternative would have kept only per-cascade
counts and drawn the whole browser instance buffer, which would make a valid
looking fixture lie about culling and would diverge from native's `Run` list.

**Gap:** the spec required presentation-owned caster routing but did not say
whether the browser proof could collapse it to counts. **Reach:** native and
browser must use the same `first/count/cascade_mask` contract before shadow
receiver sampling can be judged against matched pixels. **Verdict:** sound;
the shared scene now reports model-caster passes after the range-aware draw
path, and malformed or empty ranges are rejected at the boundary. **Confidence:**
high.

### Shared renderer boundaries use named input contracts

The first version of the range-aware change passed nine positional numbers for
the packed batch layout and eight more for the model draw. That made a swapped
offset or resource easy to compile and hard to review. The shared core now
receives a named `CasterBatchLayout` and a named `ModelCaster` input bundle.
The alternative would be a shorter call site with more positional ambiguity;
the new shape makes the browser shell's platform adaptation visible while
keeping the pass implementation in one owner.

**Gap:** the spec required a platform-independent boundary but did not prescribe
how to keep a packed-record API auditable. **Reach:** every later extracted pass
should use the same named-contract rule instead of growing argument lists.
**Verdict:** sound; clippy's argument-count gate passes without suppression.
**Confidence:** high.

### Fog fragment groups start with the native layer contract

The fog compute owner allocates five fragment bind groups. A zero-filled layer
record looks harmless, but the shader uses those four words to decide whether a
surface is ground, painted, or always visible. Native initializes the groups to
the five fixed layer records; the shared owner now does the same at allocation,
before the first presentation upload. The alternative would have left the
browser's production pipeline with an all-zero fog policy and made its output
depend on a later upload that only carries sight data.

**Gap:** the extraction plan named the buffers but did not say which owner
initializes the static layer policy. **Reach:** every shared world/model pass
can bind the same layer indices during startup, resize, and device recovery.
**Verdict:** sound; the records match native's fixed layer table and do not add
per-frame work. **Confidence:** high.

### Production environment textures use the exported atlas dimensions

The production lighting shaders sample the sky lookup, PMREM atlas, and DFG
table using dimensions and a top-mip value chosen by native. A browser 1×1
placeholder satisfies WebGPU's type checker but feeds those coordinate formulas
the wrong space. The browser shell now allocates the exported atlas dimensions
and writes neutral half-float texels, and the frame bridge supplies the matching
top-mip metadata. This is a valid resource contract, not a claim that neutral
texels reproduce authored sky lighting.

**Gap:** the spec allowed shell-owned asset loading without spelling out the
minimum texture contract. **Reach:** model/world parity can now replace the
neutral texels with a baked sky/PMREM/DFG set without changing bind groups or
shader layouts. **Verdict:** sound as a prerequisite; the authored bake and
production world receiver remain open. **Confidence:** high.

### The browser bakes the sky from the shared presentation record

The native client turns the resolved `SkyLight` record into a sky lookup with
the exported `bake` pipeline before lighting samples it. The browser shell now
does the same on startup and after device recovery: it uploads the 40-word sky
record, renders the exported fullscreen bake into the shared LUT, and keeps the
same environment bind group. The alternative would have kept a neutral texture
forever and made a production receiver comparison meaningless even though the
resource dimensions were correct.

**Gap:** the spec named shell-owned asset loading but did not say whether the
sky bake was a shared pass or a precomputed file. **Reach:** later PMREM and
world/model parity work receives authored sky radiance without changing the
browser/native resource boundary. **Verdict:** sound; the scene remains green
and the bake survives the recovery replay, while PMREM filtering and full world
inputs remain open. **Confidence:** high.

### PMREM filtering stays in the shared browser environment owner

Native renders the baked sky into a cube atlas and filters successive roughness
levels with the exported `pmremBake` and `pmremFilter` pipelines. The browser
environment owner now performs that same atlas construction after its sky bake,
using the same face geometry, viewport layout, and top-mip convention. The
alternative would have left rough surfaces sampling a neutral placeholder even
after the sky itself became authored.

**Gap:** the spec did not say whether PMREM was an asset file or a GPU startup
pass. **Reach:** model/world receivers share one environment atlas contract and
can later receive authored DFG data without another browser-only lighting path.
**Verdict:** sound; the one-renderer scene still passes all rows and lifecycle
gates, while authored DFG and production world input parity remain open.
**Confidence:** high.

### The shared browser environment uses the authored DFG LUT

The production BRDF lighting shader reads a 16×16 split-sum DFG table. The
browser shell previously filled that table with a neutral half-float value,
which was valid GPU data but not the game's light. It now parses the checked-in
`dfgLut.ts` words into the same texture format native uploads. The alternative
would have left every roughness/metalness comparison in the browser on a
diagnostic approximation.

**Gap:** the spec named the environment bind group but did not name the DFG
asset's source of truth. **Reach:** shared model/world pixels now use the same
authored BRDF lookup on both platforms; the remaining gap is full world and
material input parity. **Verdict:** sound; the browser scene remains green.
**Confidence:** high.

### The height-page gate uses a non-flat page, while the mesh stays frozen

The old fixture's height payload was a three-word header with no present pages,
so uploading it proved only that an empty map could cross the boundary. The
fixture now keeps its authored render mesh and camera but gives presentation a
single 2×2 page with four distinct height samples. The shared browser row
uploads eight words, dispatches the same fog stages, and replays that page after
device recovery. The alternative would have changed the visible mesh to make a
transport check look like terrain parity, coupling two independent contracts.

**Gap:** the spec required non-flat height-page parity but did not prescribe a
fixture that isolated page transport from mesh generation. **Reach:** future
production terrain work can replace this page with real map pages without
changing the renderer boundary. **Verdict:** sound; the scene remains green
and reports the non-flat payload through recovery. **Confidence:** high.

### Terrain material tables cross one explicit shared owner

The browser now receives the same resolved terrain tables that native uploads:
params, plot nodes, plot records, surface records, surface index and scar look.
`renderer-core::TerrainResources` owns the `terrainLayout` bind-group order,
while the browser shell owns only the WASM conversion and recovery snapshot.
The native frame-view suffix is appended at that boundary because the native
owner adds it before binding the uniform. This keeps one ABI and avoids a
browser-only terrain-material layout.

**Gap:** the first attempt omitted the suffix and WebGPU rejected the uniform
as too small. **Reach:** the table seam is now valid, but scar directory/pool
textures and native terrain partition selection still need real inputs before
the pixels are a parity result. **Verdict:** keep this owner and contract;
finish the missing resources in the production parity pass. **Confidence:**
high.

### Model frame records are uploaded before full batch/material selection

The browser shell accepts presentation palettes, instances and batch words in
one recovery-safe upload and reports their size. This establishes the record
transport without inventing a second model schema. The current fixture sends
the frozen records but leaves material rows empty and still draws the fetched
asset through the existing shared model path.

**Gap:** wiring the bytes into the complete production material and batch
selection path is still required. **Reach:** the upload and recovery contract
can remain unchanged while native material classification and static batches
are added. **Verdict:** retain as an input seam, not as evidence of visual
parity. **Confidence:** medium.

### Dynamic model batches keep their native ranges

Presentation `Instance` records are 16 floats and `Batch` records are 12 words.
The shared browser upload now validates those strides, preserves the complete
instance stream, and turns dynamic `VIEW_PASS` batches into explicit draw ranges
for the shared prepass and opaque pass. Batches with `first == NONE` are held
static batches; the browser skips them until it has a static holder keyed by
`Batch.key`, matching native's `Statics` owner rather than pretending their
range is frame-local.

**Gap:** the current fixture has no authored model runs from `ScenePresentation`
and still loads one test-tank drawable, so this proves range handling rather
than heterogeneous appearance selection. **Reach:** an appearance/tier
drawable table and static holder are still required for full production parity.
**Verdict:** keep the native record strides and explicit run contract; do not
flatten batches into a synthetic instance stream. **Confidence:** high.

### The empty lab frame uses an explicit one-bundle transport fixture

The frozen lab presentation has no dynamic unit rows because it does not load
a battle publication. The harness therefore creates a small record stream from
the already loaded `test_tank` asset: 16-float instances, 12-word view batches,
and an identity joint. These records exercise the same upload, validation,
recovery, and per-range draw code as a real frame, while the report labels the
fixture as transport evidence. We keep this separate from the eventual
production gate instead of pretending that hand-authored rows came from the
simulation.

**Gap:** it does not prove appearance/tier lookup, static batch holding, or
multi-bundle material routing. **Reach:** the next real-frame fixture must come
from `ScenePresentation` with those records populated. **Verdict:** useful and
honest as a bounded seam test; insufficient for parity. **Confidence:** high.

### Batch decoding belongs in the shared model core

The browser previously decoded packed `Batch` words locally. The shared model
core now owns the 12-word decoder and returns named appearance, tier, pass,
range, and static-held fields. Browser code only filters the dynamic view runs
it can currently draw. The decoder rejects partial records and explicitly
recognizes `first == NONE`; this keeps the static-cache gap visible rather than
turning it into an accidental instance read.

**Gap:** static batches still need a browser holder and appearance/tier rows
still need drawable buffers. **Reach:** native and browser now agree on the
packed batch contract before that mapping is added. **Verdict:** keep the
decoder in `renderer-core`; do not duplicate record parsing in either shell.
**Confidence:** high.

### Static batch ranges survive frame rebuilds through the shared batch state

Native treats `Batch.first == NONE` as a held static range. The shared model
core now keeps those ranges by `Batch.key`, reuses them when a later frame
publishes the held marker, and snapshots the state with the presentation upload
so device recovery rebuilds the same draw ranges. Dynamic batches still use
their frame-local `first/count` records and the full instance stream remains
available to the shadow owner.

**Gap:** the browser still needs an appearance/tier drawable table for
heterogeneous static rows; the holder only solves range lifetime. **Reach:**
this removes one lifecycle divergence without adding a second renderer graph.
**Verdict:** keep the holder in the shared model boundary and add drawable
mapping next. **Confidence:** high.

### Model runs require an explicit appearance/tier drawable admission

The browser shell now keeps a shell-owned `(appearance, tier)` admission table
when it loads a bundle. Shared model runs are filtered through that table before
they reach the prepass and opaque draws, so a batch cannot silently use the
currently loaded geometry for a different appearance or tier. The one-bundle
fixture admits `test_tank` tier 1; expanding to heterogeneous production rows
still requires loading their bundles and adding their vertex ranges.

**Gap:** the current table is deliberately one appearance/tier and does not
yet replace the native world's complete appearance library. **Reach:** the
contract makes that expansion an asset-loading concern at the shell boundary,
while the shared pass remains unchanged. **Verdict:** keep the admission check;
do not fall back to the first loaded drawable. **Confidence:** high.

### Authored tiers share one packed vertex stream with explicit ranges

The browser now packs every non-empty tier of the loaded appearance into one
48-byte vertex stream and records its `(appearance, tier) -> vertex range`
mapping. Model runs carry both the instance range and the selected vertex
range into the shared prepass and opaque draws. This preserves the native tier
contract while keeping geometry allocation shell-owned.

**Gap:** the current loader still installs one appearance and does not combine
multiple bundles or articulated node transforms. **Reach:** adding more
appearance bundles extends the table and texture/material rows without a new
pass graph. **Verdict:** keep the packed-range contract; the next real-frame
slice should populate it from the complete appearance library. **Confidence:**
high.
