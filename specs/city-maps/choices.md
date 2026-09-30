# city-maps choices ledger

Implementation decisions not already settled by the user or plan. Each entry names its gap, consequence, verdict and confidence; delegated experiment limits are recorded in the owning evidence instead.

## Sound

### Public height queries use samples independently of drawing triangles

**When:** SA1 terrain/export pass, 2026-09-30.

**Choice:** A browser querying the riverbed reads the simulation's compact height samples, even when drawing coalesces a large surrounding flat field into two triangles. The sample export describes its dimensions and present pages; omitted pages mean exactly zero height. Fog and grass share one GPU copy of those samples. The alternative would recover a rectangular height grid from drawing vertices, which stops working once equivalent flat triangles are coalesced.

**Gap:** SA1 required an exact compact public surface but left the browser encoding and shared GPU lifetime unspecified.

**Reach:** Drawing can change equivalent triangle layout without changing height lookup. Consumers must use the sampled-surface descriptor rather than assume vertices are row-major grid samples. The combined directory/height buffer keeps paging inside the existing device binding budget.

**Verdict:** Sound — the frozen query oracle and real Metal readback prove one physical surface across the consumer cutover; no approximate resampling or second simulation world is introduced.

**Confidence:** High.

### Systems proofs may unlock their production contract before complete art acceptance

**When:** S0 / systems-scope checkpoint, 2026-09-30.

**Choice:** A physical/resource contract can enter production after its own frozen proof passes while complete G0 and released appearance remain open. For example, a new terrain representation must reproduce the original heights/rays and fit full-size memory limits before simulation consumers use it; missing tower art does not block that code proof. The alternative would wait for finished models before any nonvisual implementation, so the requested specialist handoff could never be reached.

**Gap:** The original G0 bundled physical feasibility with source exports and visual frame approval. The user then explicitly deferred 3D design/modeling until systems implementation finishes.

**Reach:** Every mixed slice carries two evidence responsibilities: implemented systems and pending specialist acceptance. This does not mark the whole slice/release complete, waive resource failures or certify prototype art.

**Verdict:** Sound — respects the new execution order while keeping each original final requirement and a concrete parity/resource gate.

**Confidence:** High.

### Observation visibility shares the ground stream's side and epoch

**When:** SA4, 2026-09-30.

**Choice:** Switching from blue to red opens one new observation stream for both visibility and learned ground. The epoch is the number naming that stream; visibility keeps its own exact revision within it. A red fog snapshot therefore cannot be paired with blue's learned ground. The unbuilt alternative would maintain separate stream identities and coordinate their completion before a frame could be exposed.

**Gap:** The slice required explicit identity and no mixed epochs without choosing whether visibility should introduce a separate stream.

**Reach:** Future publication fields that depend on a consumer baseline should join this stream identity. Ground's payload and learning revisions retain their existing contract; transport revisions do not change battle digests.

**Verdict:** Sound — reuses the existing side/resynchronization owner and avoids a second coordination state machine.

**Confidence:** High.

### Return obsolete side-view credits without exposing their frames

**When:** SA4, 2026-09-30.

**Choice:** Blue's first frame arrives and the user switches the diagnostic view to red while a second blue frame is already in flight. The client records the requested side and invalidates its current epoch, returns that second frame's buffer as credit, and counts its returned tick toward an outstanding advance. It exposes the next red snapshot only when both visibility and learned ground belong to the new epoch. The unbuilt alternative would queue the requested view change until every old frame had been shown, leaving the old view visible longer.

**Gap:** The plan required unchanged authority tick/backpressure semantics and no mixed side views but did not specify the handling of an already-completed old-side frame. The integrating agent explicitly accepted discarding those stale view records while returning their credits.

**Reach:** A side switch is a diagnostic view reset, not ordinary tick coalescing. The authority still completes every tick in order, and an advance cannot stall merely because its final buffer became obsolete after the switch. Race regressions before and after the first callback pin both the side filtering and advance completion.

**Verdict:** Sound — follows the existing immediate learned-ground invalidation and preserves authority progress without introducing a deferred-switch queue.

**Confidence:** High.

### Publish changed ground as exact tile runs

**When:** SA3 / SA4 transport seam, 2026-09-30.

**Choice:** When a side learns a whole cleared tile, send one record saying which tile, which local cells and their exact ground bytes. The receiver keeps those bytes as a short run. When neighboring cells differ, their runs remain separate and a busy page uses dense bytes. A run never includes a cell whose learning stamp is outside the requested cursor. Local tile rows reconstruct global indices using the map width, so neither a row crossing nor a far map corner loses its address.

**Gap:** Sparse indexes alone leave a full learned snapshot larger than wasm can address. At 18 km, the original collected cell records and float publication exceed the experiment ceiling before the client receives them. The snapshot seam needed a shape that preserves knowledge without first enumerating every cell into a second array.

**Reach:** The native iterator, publication encoding, decoder and learned receiver change together. Frozen tests expand small outputs to canonical original tile/local order; production consumers query pages or consume runs directly. The transport carries the same epoch, side and cursor semantics.

**Verdict:** Sound — the run encoding is lossless and the focused seam tests preserve exact values, order, hidden edits and side replacement. Whole-map admission and arbitrary-entropy bounds still require executed resource evidence.

**Confidence:** High for correctness; resource acceptance remains pending.

### Learned cells persist while the GPU caches exact sampling regions

**When:** SA3, 2026-09-30.

**Choice:** Keep every learned cell in lossless CPU pages and reuse one bounded GPU cache for each admitted draw region. Exact uniform tiles live as packed directory words; varying tiles use filtered texture pages with their full halo. The rejected whole-known atlas could exceed two gigabytes, and the measured bounded atlas that materialized even uniform pages uploaded 2.15 GB per overview frame.

**Gap:** The original world-sized scar texture exceeds the actual device limit at each required map size; sparse JavaScript alone cannot fix that capability.

**Reach:** Camera residency cannot evict gameplay knowledge or turn known marks into zero. Draw batches submit before overwriting their shared cache. Foliage clearing and tree suppression query the same learned owner. Future nonuniform workloads must remain exact and fit the admitted region, not silently approximate.

**Verdict:** Sound ownership and source correction, supported by exact sampling tests and real Metal allocation/disposal evidence. The initial uniform directory arm was slow (373 ms at the 18 km overview); the common-word correction below resolves that measured uniform workload. Mixed/fragmented throughput stays open. Arbitrary-entropy and full native overlap remain separate admission evidence.

**Confidence:** High for the proved values/lifetime; full performance and production pixel acceptance remain open.
## SA2 sparse navigation proof

### Keep the original planner as an evidence oracle, never a runtime fallback

- **When:** SA2 representation correction.
- **Choice:** freeze the original planner under the proof assets and compile it only in the matched proof example. When a tank asks for a route, production uses the one spatial NavGrid; the example independently runs the frozen planner on identical safe inputs and compares its actual answer. A second runtime backend would split route ownership and hide scale failures on small maps.
- **Gap:** the slice required frozen equivalence evidence but delegated its executable form.
- **Reach:** future search corrections inherit an original-winner oracle without retaining dense production allocations.
- **Verdict:** sound. This keeps evidence reproducible and leaves one planner owner.
- **Confidence:** high.

### Use sufficient geometric certificates without changing route scheduling

- **When:** SA2 representation correction.
- **Choice:** an entirely closed row or column between the snapped endpoints proves NoRoute without flooding one map half. A clear, uniform endpoint rectangle can produce the same single smoothed goal as the original planner; faster terrain elsewhere vetoes that shortcut for Fastest. If either certificate cannot prove its case, the original A* priority, neighbour order, parent updates and sampled smoothing run. A broad stronger heuristic was tested and rejected after changing an original waypoint.
- **Gap:** exact search corrections were delegated, but neither a stronger priority nor approximate routes were authorized.
- **Reach:** these certificates cover specific geometric facts, not general city routing. The safe original-winner corpus and village trace match; long geometry and full native/wasm route parity remain proof obligations before full SA2 unlock.
- **Verdict:** sound within the recorded proof scope. The bridge failure remains red instead of treating a few cheap routes as a universal work bound.
- **Confidence:** medium.

### Representation discretion and rejected arms

The delegated storage choice is implicit open/edge cells with sparse exact exceptions, 32-cell clearance tiles with the inherited 16 m cap and derived 8-cell halo, and 32-cell visited scratch tiles. Retained capacities and both-side/rebuild overlap remain part of admission. Navigation's read-only storage/expansion diagnostics do not enter digests or publication. Per-cell visited hashes were replaced after their measured heap cost. A one-active-entry heap was rejected after 2.6% fewer instructions cost 20% more heap; the original heap remains. Asynchronous route jobs were documented only as an architecture candidate because changing the movement-start tick is a behavior decision. No new game rule or visual design was selected.


## Sound: ground publication admission and recovery

### Admit one complete record and fail explicitly when it cannot fit

**When:** SA3 integrated transport checkpoint, 2026-09-30.

**Choice:** A reconnect that learns all uniform Large ground and finest-grid fog sends one complete record within a provisional 64 MiB allowance. Before allocating output, the publisher adds every group and payload with checked arithmetic. A busier record exceeding that allowance fails visibly; it cannot drop distant cells or publish a partial observation. The alternative would fragment a record across several credits and retain a separate partially reconstructed observation state.

**Gap:** The run seam removed giant cell staging but did not select atomic-record admission or error behavior for higher-entropy snapshots.

**Reach:** Worker callers now receive a publication error, and C07 must measure/ratify actual active and high-entropy workloads. The current bound is derived from two approximately 20.25 MB payloads, not universal world admission. Fragmentation would need its own measured protocol change if actual required workloads fail.

**Verdict:** Sound as a provisional codec allowance — it has checked pre-allocation rejection and preserves the previous record/cursors on that rejection. Full-size overlap and workload acceptance stay explicit open gates.

**Confidence:** Medium.

### Validate paired baselines before committing, and make receiver failure terminal

**When:** SA3 integrated transport checkpoint, 2026-09-30.

**Choice:** A record with valid new fog but a wrong ground baseline is rejected before either decoder cursor moves. If decoding/application or the authority fails at runtime, every pending command, advance and replay promise rejects and held/incoming buffers return once. Later status messages cannot restart this client. The user starts a new client to recover. The alternative would retry/resynchronize automatically while retaining partly applied state and unresolved requests.

**Gap:** Independent review exposed a paired-cursor transaction bug and an existing failure path that settled only startup, leaving producer credits and runtime requests stranded.

**Reach:** Consumers cannot hang behind a failed publication or expose fog from a rejected ground record. Ground revisions may stay unchanged while fog advances. Automatic recovery is not implied by the protocol.

**Verdict:** Sound — the failing production decoder/worker-edge scenarios now prove baseline retention, explicit rejection and exactly-once credit return; no retry state machine is added.

**Confidence:** High.

### Cache an exact common word with complete exceptions

**When:** SA3 follow-up, 2026-09-30.

**Choice:** The learned receiver maintains page classification metadata. GPU misses inside an admitted cache return its exact common uniform word; every differing page or zero hole is explicitly represented. When the complete exception set fits the same bounded GPU pool, one global cache suffices. Larger sets use the existing bounded regional path. A 158 KB maximum tile-occupancy bitset removes first-page arrivals in constant work and enumerates missing tiles without copying/sorting all retained IDs. It is metadata, not dense ground/clearing values.

**Gap:** Exact uniform per-tile words still rebuilt 40 MB of directory data in each 18 km overview, taking 373 ms. A single missing tile also revealed whole-map temporary sorting in the first exception enumerator.

**Reach:** Defaults are valid only within cache bounds; clearing projection and partial-edge validity remain exact. Near-uniform learned maps can retain local nonuniform scars without materializing the common value as texture pages. Fragmented workloads retain their separate resource/frame gate.

**Verdict:** Sound common-word correction, with actual 18 km warm overview at 3.03 ms complete and zero repeated uploads. The bitset choice was explicitly admitted as per-tile metadata by the integrating owner and replaces a more complex interval-tree candidate. Full mixed/fragmented/native pressure and pixel acceptance remain open.

**Confidence:** High on the proved parity/lifetime; broader workload throughput remains under measurement.
