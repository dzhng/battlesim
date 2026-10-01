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
### Physical catalogue identity uses canonical named records and SHA256

**When:** C00, 2026-09-30.

**Choice:** Sort templates, parts, edges, entrances and declared joins by stable names, normalize signed zero, then SHA256 the physical template array. Materialized named records use that same ordering. Category, family and missing physical facts affect this identity; appearance and source readiness have no hashed field. Height and floor count derive from their authoritative facts instead of separate counters.

**Gap:** C00 required canonical physical identity without choosing its encoding, ordering or digest algorithm. The integrating root explicitly kept status outside physical identity.

**Reach:** Renaming a physical reference or adding formerly unknown physical data creates a new catalogue identity. A materials/LOD/readiness change does not. `sha2` adds a standard hashing dependency; the simulation's state digest is not repurposed as an external content identity.

**Verdict:** Sound — one typed physical owner, verified reload, and native/wasm byte-value parity without art in battle data.

**Confidence:** High.

### Missing source facts remain optional, with an explicit completeness boundary

**When:** C00, 2026-09-30.

**Choice:** Preserve the existing house boxes while leaving absent floors, entrances and bay patterns unresolved. `require_complete()` checks those facts and complete face coverage before a dependent consumer selects them. Neither house.py trim/windows nor the old capacity-driven slots supplies invented physical metadata. Complete labelled API fixtures prove the schema separately; physical completeness does not certify source fit or art.

**Gap:** The existing owner contains only boxes and old slots. C00's required future metadata has no authoritative source yet. The integrating root approved explicit absence and deferred fit rather than inferred floors or source-ready flags.

**Reach:** C01 may preserve current slots over exact shells; C40/C53/fit consumers must reject unresolved facts. Real-source bay/floor/entrance additions change physical identity. Source readiness remains provenance/evidence outside that identity.

**Verdict:** Sound — carries known facts without changing gameplay or fabricating release coverage.

**Confidence:** High.

### Supported joins pair equal vertical envelopes; bay work has a cumulative allowance

**When:** C00 review, 2026-09-30.

**Choice:** Keep arbitrary oriented box parts and independent elevations. A declared internal join must match span, opposing normals and equal base/top; unequal-height joins are rejected until a vertical exposure contract is proved. Z-disjoint boxes cannot hide each other's facades. Bay lattices emit points strictly inside their spans, and at most 65,536 positions may be materialized across one descriptor, checked before any bay vector allocation.

**Gap:** Exposed edges had one full-height boolean. The first compound incorrectly hid a taller part's upper wall, and finite tiny pitch could request enormous allocation. The root approved rejecting the unproved join and the explicit 65,536 technical allowance.

**Reach:** Coordinate payload is bounded to 1 MiB per materialized descriptor; this is not catalogue/world residency admission or a source dimension. A nominal 3 m lattice around an 18 km square fits. Actual source compliance with the ~3 m policy, external terrace capabilities, overlap allowances and unequal-height fitted joins remain S2/S5/C13 evidence responsibilities. The rejected arm retains its original data, output and source identity.

**Verdict:** Sound — the smallest proved join contract keeps independent primitive geometry, and bounded failure prevents accidental source profiles from exhausting memory.

**Confidence:** High for the supported contract; real-source join coverage remains unproved.

### Lossless template numbers use raw-token parsing without changing scenario parsing

**When:** C00 preservation proof, 2026-09-30.

**Choice:** Parse template numeric tokens with Rust's correctly rounded f64 reader, using JSON's raw-value facility only on template fields. Encode ordinary JSON numbers. Avoid global float-reader features that would also change existing scenario parsing.

**Gap:** The default parser rounded `40.000000000168804` by one ULP, so a canonical physical catalogue could not verify its own hash after reload.

**Reach:** Template/source and materialized numeric records retain exact f64 values across native/wasm JSON boundaries. The private numeric codec and `raw_value` feature are necessary maintenance surface; they do not establish a second descriptor format. Original complete observation/digest replay remains green.

**Verdict:** Sound — fixes the measured identity failure at its owner without moving existing battle inputs.

**Confidence:** High.

### Geometry admission is relative; floating point lattices must remain representable

**When:** C00 numeric review, 2026-09-30.

**Choice:** Compare joins and facade coverage after subtracting part centres, with separate height-based allowance for vertical arithmetic roundoff. When both parts move a very large distance from the origin, that shared offset cannot become permission for a one-metre gap or disjoint storeys. Bay indices must be distinct exact f64 integers; bounded checks then require every computed point to advance strictly inside its span, in source coordinates and again after placement, before allocating any bay vector.

**Gap:** The count allowance bounded memory but not numeric representability. The first comparison allowance also used absolute XY magnitude for vertical equality. A tiny pitch emitted repeated points and points on boundaries; a common coordinate shift admitted invalid joins.

**Reach:** A source or placement too precise for its numeric frame fails deterministically rather than publishing altered physical geometry. There is no arbitrary metre, minimum-pitch or world-origin restriction. The integer ceiling is f64's exact consecutive-integer range, and checks iterate only the already bounded descriptor count. Common offsets that preserve representable geometry remain legal. Valid physical identity and emitted coordinates are unchanged; rejected inputs and their original outputs retain separate evidence.

**Verdict:** Sound — numeric admission now enforces the promised geometry rather than treating finiteness and allocation size as sufficient.

**Confidence:** High.


### Learned ground visits edited truth and newly visible tiles

**When:** SA3 native learning work, 2026-09-30.

**Choice:** Truth keeps one latest-edit entry per touched tile, including unsealed
dirty tiles. Side knowledge keeps previous visibility words and the source cursor.
Changed truth and newly visible ground select exact learning candidates; broad reveal
walks occupied tiles instead of allocating a world-sized candidate list. Fully visible
pages may share immutable truth after exact value/stamp comparison.

**Gap:** Sparse storage fit, but stable full visibility still scanned millions of fog
cells and an initial uniform learning pass expanded 324 million cells. The bounded
index trades memory for changed-work lookup; previous visibility costs one bit per fog
cell and is transport-independent.

**Reach:** Both hints are outside the battle digest. Hidden changes do not modify a
learned page; unchanged values receive no new stamp. Fractional grid partitioning,
unsealed stimuli and partial edge holes are checked against the original semantics.
The [evidence](assets/ground-learning-work/README.md) passes uniform native recurring
work at fixed extents and preserves original observations/digests. Full entropy, broad
churn, active Battle and browser/GPU acceptance remain separate.

**Verdict:** Sound for the proved learning owner; no rule, wire-schema or extent change.

**Confidence:** High for exact values and the focused recurring-work bound; complete
resource/frame admission remains open.

## SA2 exact search work

### Supply optional cost bounds without creating another route owner

- **When:** SA2 search pruning and lookup pass.
- **Choice:** when a tank approaches a narrow crossing, try a few monotone walks through its opening and check every step against the same footprint, crossing and cost rules as A*. A successful walk supplies only a cost ceiling; it never becomes the returned route. Opening size and candidate-count allowances limit this optional preparation, and failure continues the original planner. Faster terrain uses conservative all-cell and whole-rectangle discounts instead of a persistent second routing graph. The alternative would maintain another planner or terrain potential cache just to construct the bounds.
- **Gap:** the plan required feasible upper bounds and admissible lower bounds but did not choose their discovery or storage.
- **Reach:** navigation gains no new route API, retained cache, tick scheduler or failure reason. These bounds can be loose or unavailable on general maps; they do not establish the still-open full-size work gate.
- **Verdict:** sound within the recorded proof scope — one owner still chooses the route, and optional proof limits cannot reject a requested route.
- **Confidence:** medium.

### Reuse a certified flat neighbourhood instead of retaining another cache

- **When:** SA2 search pruning and lookup pass.
- **Choice:** far from conservative surface/body regions and temporary traffic, certify the current cell and all eight neighbours as the existing implicit flat cells. Their footprint, crossing and cost answers can then be reused for that one expansion. Near a region or map edge the normal queries run. The alternative would keep per-goal or per-mover cost/heuristic arrays whose residency and invalidation would become another owner.
- **Gap:** lookup reuse was delegated, but the spec did not choose a cache or a geometric certificate.
- **Reach:** the certificate inherits the conservative region contract and exact cost arithmetic, adds no retained residency and leaves future map producers responsible for complete region coverage.
- **Verdict:** sound — reduces repeated queries without changing parent ordering or introducing another state lifetime.
- **Confidence:** high.


### Physical materialization uses one portable math evaluator

**When:** C00/S6 canonical-runtime proof, 2026-09-30.

**Choice:** Physical template materialization and admission use `libm=0.2.16` with
default architecture features disabled for sin/cos and facade norms. Keep one software
implementation for frame/local rotations and the geometry predicates they feed.

**Gap:** A wider placement corpus exposed seven exact native/wasm normal mismatches.
Host math's permitted platform precision cannot define canonical generated geometry.
The [rejected and corrected receipts](assets/template-geometry/runtime-rotation/README.md)
retain separate source/output identities, and the original small records remain frozen.

**Reach:** Descriptor facts and their catalogue hashes retain the same identity. Derived
values at the failing angles adopt the shared software result; complete map identity and
generator provenance must capture generated bytes. Simulation arithmetic and ordinary
scenario parsing are unchanged. Full S6 generation coverage and preexisting whole-battle
cross-runtime numerical differences are separate obligations.

**Verdict:** Sound — a narrowly owned numerical correction establishes the required
canonical seam without changing physical dimensions or adding a second geometry format.

**Confidence:** High for the checked native/wasm evaluator and complete finite corpus;
future dependency or algorithm changes require fresh canonical-output evidence.


## C02 — reject nonfinite map resolution before grid construction

**When:** per-map fog ownership pass.

**The choice.** A map asks for an infinite fog cell. The world refuses it by the map field's name before dividing the map into foliage and visibility cells. A merely positive check permits infinity and later reaches arithmetic overflow in the visibility grid.

**The gap.** C02 specifies ownership and required input, but leaves numeric admission implicit.

**The reach.** Every Battle and public WorldView uses the same admission; future generated maps must supply a finite positive resolution. This sets no minimum resolution or release performance promise.

**Verdict: sound.** The outer Battle regression falsifies the weaker check, and valid resolutions still reach the same common owners.

**Confidence: high.** Invalid geometry should fail at its input boundary with an actionable name.

### Keep scratch admission and accepted parent writes under one entry

- **When:** SA2 measured repeated-work pass.
- **Choice:** when a neighbor offers a better route cost, hold its existing scratch-table entry through the cost check, optional pruning bound and parent write. The alternative was another retained per-cell cache; the measured heuristic cache saved too little work and increased memory and latency, so it was removed.
- **Gap:** exact lookup reuse was delegated without choosing how to combine reads and writes.
- **Reach:** one scratch owner retains its original arrays/generation, strict tie behavior and public route interface. The matched arm reduces instructions without extra payload; it does not establish the still-failed full-size planning budget.
- **Verdict:** sound — the source proof and complete original-output checks support the measured correction, while resource failures stay explicit.
- **Confidence:** high.

## SA3 bounded sampled-word source

### One compressed pool owns exact GPU ground samples

**Choice:** Store visible uniform words directly in the directory and lossless runs
or dense words in one bounded pool. A dominant exact word plus every exception,
including implicit zero holes, permits complete-map admission when that actual
representation fits; otherwise existing bounded region draws reuse the same owner.
CPU learned values remain authoritative and never leave because of a camera move.

**Gap:** The sparse-ground slice delegated internal storage but did not choose GPU
encoding or residency. Materialized halo pages fit dimensions while repeatedly
uploading gigabytes. An arbitrary resident-page count also rejected ordinary mixed
uniform workloads despite small sampled data.

**Reach:** Buddy allocation and largest-first compaction trade a bounded reupload
for exact fitting residency. Directory identity changes refresh bindings. Immutable
normalization words share the allocator reservation through churn. No second world,
LOD/default-loss approximation or new draw subsystem is introduced. Full arbitrary
entropy may still require regional work or fail resource admission; it is not waived.

**Verdict:** Sound for the measured owner and preserved values. **Confidence:** high
for lookup/storage; full frame workload acceptance follows separate evidence.

### Observe hardware coordinate boundaries and contraction explicitly

**Choice:** Decode normalized f32 UV bits before the rounded product, then apply the
measured coordinate/color quanta and exact normalization table. Preserve original
cubic coefficients and tap values; make its outer fused multiply-add explicit.

**Gap:** Replacing hardware sampling exposed compiler cancellation and contraction
changes in the actual production path. Four exact taps still produced three one-ULP
cubic differences until the outer contraction became explicit. Local-window tests
were insufficient to prove the original full-texture coordinate path.

**Reach:** The actual production browser regression compares an independent original
2D hardware texture and fails the known cancellation mutant before restored zero-ULP
results. The Large virtual extension is labelled separately; no original 18 km
texture is claimed. No numerical tolerance or pixel threshold is loosened.

**Verdict:** Sound for the measured Metal corpus. **Confidence:** high within that
corpus; compiler/platform generality remains an explicit capability obligation.

### Admit measured compatibility once per device

**Choice:** Await a finite source-owned hardware probe before frame consumers start,
cache successful compatibility for the device lifetime, dispose probe resources in
finally, and remove rejected cache entries. A separate explicit frame construction
may retry on that device; no automatic retry or public profile schema is added.

**Gap:** Hardware filter precision is not portable, and startup cannot silently
sample a different rule. Attempts to make this finite probe a compiler cancellation
guard failed falsification, so the production regression owns that contract.

**Reach:** Incompatible initialization fails through the existing frame cleanup.
Other-device support/generic fallback stays open under C07. Sampled axes over 65,536
are rejected before source state or receiver changes mutate, because the present
f32 cell position cannot retain every Q8 quantum beyond that boundary; this is not
a physical map or game rule limit.

**Verdict:** Sound as an explicit, reversible renderer capability boundary.
**Confidence:** medium for compatibility inference; high for failure/lifetime and
numeric admission behavior exercised through actual frame creation.
## C01 — one placed building across physical states

### Keep authored facts separate from the current live integrity owner

**Choice:** A compound building collapses and leaves two weaker shells. Its immutable building ID still names the authored template, while its live parts share one fresh integrity owner. A removed original ID has no live owner or HP. Historical keys remain private to remembered knowledge and orders. The alternative would make the weaker remains inherit an already exhausted damage key or make an obsolete body appear live.

**Gap:** C01 chose one owner but did not specify destroyable-remains chains or owner lifetimes.

**Reach:** Initial associations and the existing immediate replacement links establish each state's membership. Source ancestry is copied at creation for every authored prop; genuinely dynamic bodies retain no source. Browser appearance lookup consumes that source without reconstructing chains. Immutable map/classification/catalogue identity remains in scenario_digest; nonidentity mutable associations are hashed and extra physical seat/eye facts get a cached fingerprint. Original implicit-box digests remain exact, without a family-name compatibility branch.

**Verdict:** sound. **Confidence:** high for the frozen singleton chains, compound chains and authored/dynamic distinction; future composite motion is not admitted.

### A blast damages one structure by its strongest part contribution

**Choice:** An explosion beside a wing also reaches the main house. The house takes the stronger contribution once, rather than summing both physical boxes. A terminal transition replaces every part together; a side that sees any part learns all replacements in the same observation. A wholly hidden side keeps its previous knowledge until reveal.

**Gap:** The aggregate plan did not select how multiple parts contribute to one blast or define partial visibility at collapse.

**Reach:** Direct hits and blast results resolve through the same integrity owner. This extends the singleton rule without increasing damage just because an author divided a house into more boxes.

**Verdict:** sound. **Confidence:** high; unequal-distance damage and partial/hidden reveal regressions distinguish the alternatives.

### Preserve physical exposed spans through seating and sensing

**Choice:** Soldiers on a stepped exterior face keep their actual edge ordinal. Their sensing eyes average within that held edge, so two east-facing spans cannot average through the wing's interior. Materialized geometry carries its exact part-local interval alongside world spans; admission checks that they agree. Generic seating uses the part frame and local interval once.

**Gap:** C01 required exposed seats but did not choose eye grouping or settle the arithmetic needed to preserve rotated singleton seats.

**Reach:** Existing capacity and seats remain exact. Partial faces gain no invented bay spacing. Approach, entry and shots inspect all physical parts; a shot leaving its occupied owner skips the whole current state.

**Verdict:** sound. **Confidence:** high for frozen singleton seats/eyes and asymmetric stepped geometry; new band/cap mechanics remain C40.

### Admit only capabilities that the placed physical owner can perform

**Choice:** Authored building parts must be immovable until motion can transform an entire compound. Garrison bodies require the explicit aggregate and at least one exposed span. Ordinary movable props still use their existing movement path.

**Gap:** The schema otherwise permits a one-prop shove to tear a placed compound apart, or a garrison with no legal seat/eye.

**Reach:** Admission reads body capabilities, not kind names or invented source dimensions. Unknown floors, doors and bays are permitted for prototype geometry but do not become complete source facts.

**Verdict:** sound. **Confidence:** high for current authored capabilities and rejection proofs; composite motion remains separate work.

### Cut over the exact public ID representation with its readers

**Choice:** All physical prop IDs, optional replacement/building/live-owner/source IDs and garrison IDs use the established pair of 16-bit limbs. Public building rows contain metadata and named part references to the existing physical prop export, without a second geometry table.

**Gap:** The old scalar prop and garrison fields lost integer identity beyond float32's exact range; C01 delegated the public owner/export representation.

**Reach:** All consumers switch together. The stream epoch, tick, credits and ground/fog semantics stay in their existing transport owner. Original animation words and physical columns remain frozen; only the declared identity columns normalize.

**Verdict:** sound. **Confidence:** high for production pack/decoder wide-ID and full observation oracles; integrated visual scenes remain a closeout gate.

### Author existing boxes at the producer boundary, with unresolved source facts

**Choice:** Fixtures, endurance and analytic test preparation explicitly use the shared solid-box physical constructor. Current physical dimensions and IDs remain exact. Prototype category/family labels provide deterministic eligibility metadata, while missing floors, entrances and bays stay absent. The world does not silently convert ordinary props.

**Gap:** Current source art does not establish the complete reusable physical catalogue or accepted regional classification.

**Reach:** Sim consumes trusted final placed geometry and validates internal geometry/ID consistency. Verification against the pinned catalogue belongs to C09/C13/C32 preparation. Actual source/category acceptance will produce its own physical identity later; no art binding or design readiness is fabricated.

**Verdict:** sound within the nonvisual systems scope. **Confidence:** medium for provisional classification; complete template/source/art G0 remains open.

### Check saved geometry in its actual world-coordinate precision

**Choice:** A valid template placed kilometres from the origin can have world endpoints emitted in a different transform operation order from part-local verification. Saved interval/span admission allows four coordinate ULPs plus local rotation roundoff; bay collinearity is checked by projecting back in world coordinates. Source catalogue joins keep their stricter translation-invariant contract.

**Gap:** C01 needed to reject contradictory saved local/world intervals without rejecting valid owner-emitted geometry. A positive 12/15 km placement exposed an overstrict relative-dot check.

**Reach:** The materialized record remains the one physical owner; validation does not change its coordinates. Prototype source facts are still subject to C00 representability and cumulative bay admission before allocation.

**Verdict:** sound for the admitted saved-geometry seam. **Confidence:** medium; the labelled positive/contradictory pair proves this rounding arm, while catalogue-versus-placement provenance verification remains a separate preparation gate.

### Make the physical materializer and loader share representability admission

**Choice:** A local rotated part placed at an enormous finite yaw can lose its local angle when the final yaw is added, while its composed edge normals keep that rotation. The materializer checks its final record through the same admission as the loader and rejects the contradictory primitive pose by name.

**Gap:** Finite input alone does not ensure that one world-yaw box can express the composed physical geometry; review found a successful producer output that its consumer rejected.

**Reach:** No angle normalization or simulation math changes. The original canonical runtime corpus and frozen fixture records remain intact. Bounded bay allocation still precedes this final consistency check; complete source fit remains separate.

**Verdict:** sound. **Confidence:** high for the public materialize-to-WorldView red/green arm and unchanged canonical corpus.

### Keep existing capacity across physical directional groups

**Choice:** Exposed normals are assigned to the nearest cardinal direction of the placed building frame. Existing capacity is shared across nonempty groups, with remaining seats assigned by total exposed length; seats then sample those physical intervals in deterministic group/edge order. Each seat retains its edge for sensing.

**Gap:** Multiple differently oriented parts no longer provide exactly one box face per original facade direction, while C40's new band and capacity policy is explicitly deferred.

**Reach:** The original full box retains its exact seats and four eyes. Compound proofs cover unequal spans and stepped exterior faces. Arbitrarily rotated parts at directional quantization ties still require wider paired battle-runtime evidence under S6; canonical materialization alone does not establish that simulation claim.

**Verdict:** sound within this physical systems proof. **Confidence:** medium for unrestricted rotated compounds; C40 and full S6 runtime admission remain separate gates.

### Resolve holdable remains and observed extents from current physical parts

**Choice:** A damaged but still holdable compound uses its current members and the world's immutable authored-source association to reach the original exposed-edge records. Its next terminal transition releases the one garrison once. The observed footprint is the current members' tight envelope in the immutable building orientation, measured about a live pivot; existing ring/ruler consumers use its bounding radius.

**Gap:** Review found that authored part IDs no longer existed after replacement, leaving an admitted garrison with empty seats, and that an owner-part-only footprint omitted the wing.

**Reach:** No second geometry or lineage owner, new visual style or schema column is added. Original singleton observations and animation words remain exact. Public static picking still addresses the immutable map owner; choosing a fresh live replacement from side-known geometry is the later C43 interaction contract. The current owner is already available to direct commands through KnownProp.

**Verdict:** sound. **Confidence:** high for native/public-WASM red/green, chained ownership and original singleton preservation; actual rendered-frame verification remains a separate gate.

### Verify aggregate pixels through disposable production-path staging

**Choice:** Reuse the real garrison route/authority/decoder/renderer with labelled C00 asymmetric geometry and existing accepted house/ruin bundles. Falsify the old footprint and immediate-predecessor appearance owners separately, then restore every staged tracked byte.

**Gap:** The pre-aggregate loader cannot load a compound; the ordinary shipped scene cannot reach a two-generation held prototype. Its passing singleton checks alone cannot establish the new capability.

**Alternatives:** Ship another debug scenario/renderer hook, or claim the source/Node checks proved pixels. Both leave an unnecessary owner or inadequate evidence. Disposable staging exposes the actual changed frames without adding a production branch.

**Reach:** The saved complete observations/digests match across the isolated arms except the declared envelope fields; cameras/ticks/assets remain matched. The actual ring and terminal models change. Prototype roof/source fit, final art G0 and current-state picker design remain open. The team thread limit blocked a fresh visual agent, so screenshot-critique's explicit implementing-agent adversarial fallback is disclosed and root separately inspects the frames; no unprimed review is claimed.

**Verdict:** sound for focused capability evidence. **Confidence:** high for exact public states and visible ring/source-state differences; lower for broader appearance/scan acceptance, which remains deferred.


## C03 shared surfaces

### Admit simple rings and export the exposed polygon union

**Choice:** Each authored paved polygon is one finite simple ring of either winding,
without a repeated closing endpoint or holes. Native triangulation provides membership;
a separate native stream provides only the exposed boundary of their union. It splits
intersecting edges, removes covered fragments and shared interior edges, and keeps one
coincident exterior edge with road priority. Drawing never treats fill diagonals as edges.

**Gap:** C03 selected polygons but did not settle admission or the boundary information
needed for material feathering. Compiler diagnostics and complexity limits remain C04/C63.

**Rejected:** Combining completed per-polygon signed distances removed triangulation
seams but retained shared-edge seams. The actual touching-polygon material probe returned
roughness .90 instead of .85. Overlapping rectangles also exposed an interior boundary at
2.5 metres instead of the union's 5 metres. Those receipts remain labelled as rejected;
they are not acceptance for the final union export.

**Reach:** The authored rings still own movement. Native union edges own the polygon
material boundary, including holes created by the composition of simple rings. The GPU
uses any-triangle membership and distance to those edges once. This is a polygon-union
contract: joining strokes to polygons still uses the original maximum of their distances,
and C63 must correct/admit mixed joins before generated mixed streets ship.

**Verdict:** sound for this capability. **Confidence:** high for single, touching and
overlapping polygon native/GPU regressions; unrestricted complexity and f64-to-GPU
boundary precision remain separate admission work.

### Pack the native streams without changing village stroke arithmetic

**Choice:** Public rows are six floats per stroke, seven per membership triangle and
five per exposed boundary segment. The renderer pads each to one eight-float GPU table;
counts identify the three regions. The original stroke coordinates, arithmetic and order
remain unchanged. No polygon-end flags or triangle edge masks survive this contract.

**Gap:** C03 required a shared export but did not choose its packing. The terrain
uniform's unused diagnostic lane now carries triangle count; boundary count is explicit.

**Reach:** Native geometry supplies membership and the polygon boundary. The later bounded
distance field replaces shape scans through this same source. Current scans establish
correctness, not final city-scale frame admission.

**Verdict:** sound. **Confidence:** high for the combined public-WASM/GPU checks;
paired village frames and later field admission own their respective claims.

### Keep surface overlap and unfinished pavement appearance explicit

**Choice:** A road overlies a sidewalk, so movement classifies it as road. Otherwise
a sidewalk keeps ground movement cost. Existing water and bridges retain precedence.
Both paved kinds use the existing paved material while the specialist's road/sidewalk
appearance work is pending. Agricultural plot cuts use road strokes and actual outer
polygon edges, while sidewalks do not subdivide fields.

**Gap:** The slice did not select road/sidewalk overlap order or an interim appearance
for the new sidewalk capability. New pavement styling is outside this systems pass.

**Reach:** Adding sidewalks cannot silently grant the road speed bonus. Later material
work may distinguish them without changing their physical boundary or movement cost.

**Verdict:** sound within the systems scope. **Confidence:** medium for the interim
material and plot guides; finished pavement design remains C28/C64 and the specialist.

### Refuse removed physical fields at the map boundary

**Choice:** MapDefinition decoding refuses unknown fields. The removed `roads`
field cannot silently default to an empty `surfaces` list. Frozen historical
inputs use explicit test-only translation; production accepts the current schema.

**Gap:** Derive decoding had ignored the removed field even after the source
cutover, silently erasing authored physical roads. The native parser regression
earned red for that exact loss and passes with strict map admission.

**Verdict:** sound. **Confidence:** high for the shared native decoder; combined
WASM preparation is checked at the wave gate. No valid current map geometry moves.

## C09/C60 source provenance before adapter cutover

### Record repository history and supplied input bytes honestly

**When:** C09 core, before filesystem/HTTP consumers change; approved by the
coordinating agent from the authorized systems scope.

**Choice:** A source receipt is either `repository { path, revision, sha256 }`
or `supplied { label, sha256 }`. When the compiler reads a request from a temporary
directory, it knows the bytes and their hash; it does not know a historical Git
revision. It therefore records a supplied input labelled request or catalogue.
Cut-over shipped fixtures record their actual repository path, revision and byte
hash. Both use the existing shared SHA256 owner. Neither receipt certifies an art
source or authenticates historical content not supplied to the resolver.

**Gap:** C04 saved a bare generation identity and accepted external input files.
C09 requires a common source record; forcing every input into a repository receipt
would fabricate history or prevent a valid preparation API input.

**Reach:** `SOURCES.json` now has one `MapSources` envelope containing identity,
physical-library selection and input receipts. The old bare file format is refused;
there is no legacy decoder. Compiler stdout and the existing `GenerationIdentity`
remain unchanged. Metadata/provenance changes do not change physical map hashes or
the original replay contract. Actual write-to-resolver evidence owns the file seam;
source/art readiness and complete preparation admission remain separate gates.

**Verdict:** sound. **Confidence:** high; these records state only the origin facts
the producer actually knows and avoid a second source or hashing contract.

### Keep acquisition identity beside the battle definition

**When:** C09 core checkpoint; coordinating-agent approval before implementation.

**Choice:** A saved map has an authored content identity; a compiled generated
map carries the existing generation identity, including its lossless seed and
pinned configuration/catalogue/map hashes. Moving an unchanged file into a
catalogue folder changes its storage address and source record, while the battle
still receives exactly its physical definition. Putting repository paths into
`ScenarioDefinition` would instead alter old scenario/replay identities merely
because a file moved. The pure resolver returns definition plus identity, performs
no filesystem/browser IO and constructs no world. The generated request variant
waits for the real C55 configuration rather than introducing a placeholder.

**Gap:** C09 did not fix the provenance lifetime relative to the existing replay
contract or the division between acquisition and world preparation.

**Reach:** Native and browser adapters must call this owner; C33 can consume its
result without an extra main-thread world. No second Battle loader is authorized.

**Verdict:** sound. **Confidence:** high; it preserves the existing battle boundary
and gives current source identity one owner without inventing generator behavior.

### Rebuild selected physical catalogues through their existing owner

**When:** C09 core checkpoint; approved after the actual Endurance producer audit.

**Choice:** Endurance uses one house template and has a different catalogue hash
from the complete shared physical library. Its source record can name that one
template ID; the existing catalogue owner rebuilds and hashes the selection.
Loading the entire library unconditionally would silently change Endurance's
identity. Copying its descriptor into a second catalogue file would create two
physical owners. The actual resolver preserves the frozen single-template hash;
the labelled analytic proof is not a seeded Endurance scenario cutover.

**Gap:** The producer inventory exposed different legitimate catalogue selections
that a single global-library hash could not preserve.

**Reach:** Saved placements are checked against exact materialization at their
frames, including category/family and optional physical facts. A matching map
hash alone cannot authorize invented floors. Source/art fit remains separate.

**Verdict:** sound. **Confidence:** high; one descriptor owner supports the original
selection identities and the mutant using the whole library is rejected.

### Separate physical header validity from caller resource policy

**When:** C09 core checkpoint, following independent source-review findings.

**Choice:** A saved map with zero fog spacing remains invalid even if its content
hash is recomputed. The contract therefore owns finite positive sizes/spacings and
the slope range, and both acquisition and compilation use that owner. The compiler
separately enforces the playable architecture envelope. Each resolver caller must
state its allowed authored parts and cumulative emitted bay positions; the resolver
checks these before additional authored geometry cloning and bay materialization.
This narrow allowance does not admit tree rasterization or complete world memory.
Nonempty shape validation waits for C72's shared owner rather than reproducing
simulation polygon rules in acquisition.

**Gap:** Content matching is not physical validity, while a library cannot silently
select every caller's startup/memory policy. The shared shape owner was not yet
integrated when this checkpoint was frozen.

**Reach:** C33 must still admit rules-specific capabilities and complete resource
work. No adapter may describe this result as complete physical/world validation.

**Verdict:** sound. **Confidence:** high for the proven narrow gates; full preparation
and nonempty shared-shape admission remain explicitly open.

### Admit catalogue addresses before adapters perform source IO

**When:** C09 core checkpoint; coordinating-agent approval of the concrete seam.

**Choice:** A catalogue request carries one directory ID, not a path. Lowercase
ASCII letters/digits, hyphens and underscores are allowed, with a letter/digit
first. A request for `../village` or a drive path is refused by the shared type
before a future adapter reads files. Repository receipt paths separately require
normalized relative components, reject drive colons/control characters and carry
a nonempty revision; supplied receipts use a nonempty human label. These syntax
checks do not prove the unavailable original bytes or certify an art source.

**Gap:** The spec named ID lookup without fixing its boundary grammar or how
repository-relative receipt syntax handles platform-specific paths.

**Reach:** Native and browser adapters inherit one address grammar. Existing route
IDs remain registry-owned; the ID type does not manufacture map entries.

**Verdict:** sound. **Confidence:** high; the boundary has explicit refusals and
keeps storage addresses independent of physical identity and encounter naming.

### Keep constant uniform material costs on the plan stack

- **When:** SA2 measured material-work pass.
- **Choice:** reuse the existing implicit flat cell's two exact step costs for one plan, instead of recalculating them at every certified uniform expansion. The alternative would add another persistent cost cache or leave the measured repeated arithmetic.
- **Gap:** repeated-work reduction was delegated without selecting a cache lifetime or material owner.
- **Reach:** two stack values preserve the existing cost evaluator, arithmetic order and route interface; exceptional cells still call the normal owner. The measured instruction reduction adds no retained residency, while timing is flat and the full-size work gate remains red.
- **Verdict:** sound — frozen outputs and the pure-input source proof support the small reduction without another invalidation lifetime.
- **Confidence:** high.


## SA2 indexed refinement proof

### Index geography and keep one private active clearance page

- **When:** proof-only road hierarchy preparation and delayed physical refinement.
- **Choice:** a deterministic segment BVH preserves actual min/max coordinates and canonical candidate order; exact signed-zero-normalized coordinate identity replaces linear node deduplication. Physical refinement owns one private current clearance page with counted construction, rather than depending on the shared planner cache's warm history.
- **Gap:** accepted road preference/planning delay does not select preparation structure, cache ownership or scheduling units.
- **Reach:** no production owner, command, route or Battle digest changed. The page bounds per-job residency and replay work independently of map area; dense overlapping road candidates and per-request topology work remain open.
- **Verdict:** suitable for the recorded proof cut; not full architecture admission.
- **Confidence:** medium.

### Schedule captured leg work and name the representative speed trial

- **When:** independent resumable physical/refinement and midpoint transit tracer.
- **Choice:** one job per unit, stable round robin, explicit supersede/cancel and side-revision invalidation; a physical decline remains distinct from NoRoute. The trial Jeep uses road 110/off-road 55 km/h while retaining its existing body/steering/follower, matching the accepted light-vehicle cap without another multiplier.
- **Gap:** counted quantum and off-road trial value were delegated tunables; the user selected a representative midpoint outcome, not every edge/vehicle arrival promise.
- **Reach:** external proof scheduler only, with no Battle pending/digest selection. The single straight 4,998 m arm arrives in 166.3 s including 2.733 s planning. Moving-start/traffic integration, preparation/search scheduling and general fallback are still required.
- **Verdict:** proof result supports this trial; production selection awaits the stated integration contracts.
- **Confidence:** medium.

## SA2 standalone counted core

### Share prepared geography and overlay each request's endpoints

- **When:** standalone counted topology/query/search proof; production selection pending.
- **Choice:** two units requesting routes borrow one immutable junction graph. Each request adds its own road-entry points and keeps its own costs, parents and blocked directed arcs. A wall known in one captured grid can therefore decline that request's corridor without closing the other request's road. The unbuilt alternative would copy/split the complete graph for every unit before credited work starts.
- **Gap:** the accepted delay and road preference did not select request data ownership.
- **Reach:** preparation becomes a separate loading owner; side-known snapshots must still establish complete physical identity and bounded capture/retirement before Battle integration. Stroke-only connectivity cannot qualify polygon streets.
- **Verdict:** sound within the small proof — actual concurrent requests share topology and produce independent physical results; loading and full-scale admission remain open.
- **Confidence:** medium.

### Use fixed-node trees for credited search operations

- **When:** standalone counted core proof.
- **Choice:** when a request grows its search frontier, an ordered tree allocates fixed-size nodes rather than copying an expanding flat array. Its queue compares coarse cost, node identity and insertion serial deterministically; accepted improvements remain strict. Endpoint traversal stores a fixed-depth index cursor and projects one candidate at a time. The alternative would perform a whole query/sort or a variable-size capacity copy inside one nominal credit.
- **Gap:** credits were required, but their collection/allocation atoms were not chosen.
- **Reach:** tree operations still cost logarithmic work, so maximum admitted depth, allocator cost and latency need measurement. These records establish actual resumability, not a universal constant-time credit or a completed-tick budget. Ordinary destruction remains unbounded and has a separate handoff.
- **Verdict:** sound as a falsifiable proof representation — one-credit outer tests reject the synchronous wrapper and exercise real cursors; production admission remains blocked on the named remaining owners.
- **Confidence:** medium.

### Rank coarse journeys and validate the chosen directed path incrementally

- **When:** standalone counted core proof.
- **Choice:** a unit first ranks road corridors with cheap estimated costs, then checks every segment with its existing physical evaluator. If a directed arc fails, that request excludes it and searches again; unchanged failed arcs cannot be rediscovered forever. Exhausting roads reports CorridorDeclined, so a future general planner can continue. The alternative would refine every road arc before selecting any corridor or falsely call road exhaustion destination NoRoute.
- **Gap:** accepted automatic-road intent does not select a detailed routing priority or quality tolerance.
- **Reach:** this can choose different routes from the original fine-grid optimum. Physical segment sums are exact in the proof, while journey quality, mode usefulness, general fallback and active movement remain unselected integration contracts. The accepted >2 km threshold is the user's rule and is not an invented tuning choice.
- **Verdict:** sound within declared proof scope — real wall/water/deck cases and the directed goal-connector regression demonstrate passage and retries without a synchronous escape. No production behavior is activated.
- **Confidence:** medium.

### Update semantic fingerprints with state mutations

- **When:** standalone counted core proof.
- **Choice:** as a request changes one parent or a physical transform changes one sample, update its keyed fingerprint immediately. Per-poll digest combines those fingerprints with actual cursors and scalar state instead of scanning every retained map/page. A verification-only canonical fold scans the same state after each poll and must match. The alternative would hide a full scratch scan in every Battle digest.
- **Gap:** future-affecting progress must be hashed, but the active digest's work owner was unspecified.
- **Reach:** these are ordinary deterministic digest values, with ordinary collision limits. Actual Battle scheduler, retirement and snapshot authority must include their future-affecting state too; the reference scan and complete JSON serialization remain outside the credited actor.
- **Verdict:** sound for the frozen corpus — complete Native/Wasm poll traces and canonical comparisons agree; this does not admit unimplemented Battle state.
- **Confidence:** high.

## C72 one forest rule

### Hold the GPU forest query to the sim's membership, not to exact distance bits

**Choice:** The browser check of the forest-floor query asserts that the GPU puts every probe point on the same side of the forest boundary as the sim, and that its signed distance agrees within 0.1 mm. It no longer asserts bit-equal distances.

**Gap:** The candidate's check demanded `===` on a GPU distance and failed by one f32 step at a capsule endpoint (`-1` vs `-1.0000001`). WGSL does not promise correctly rounded `sqrt`/`length`, so bit equality is not a contract any GPU can be held to; membership is what the sim and the drawing must share.

**Verdict:** sound. **Confidence:** high; the check is stricter on the property that matters, since it now compares the GPU's inside/outside answer with the sim's directly.

### Rewrite frozen parity inputs once and regenerate their outputs for the named change

**Choice:** Frozen oracle inputs in `fixtures/parity/` were rewritten to the current map format (roads as surfaces, forests as shapes, one forest rule, `fog_cell_m` on the map), and the test-only converters that did this on every run (`migrate_original_ground`, `originalGroundInput`) were deleted. Outputs that the forest rule changes were regenerated with `BLESS_PARITY=1`: the village arm of the building oracle, the fog-delivery oracle (digests and visibility from native, decoded frames from Wasm), the foliage exports and the terrain queries.

**Gap:** These oracles froze outputs of implementations that no longer exist (dense terrain, pre-aggregate buildings), so the named C72 change could not be re-derived from them. Their historical-equivalence proofs stand at `9a88280` (tag `city-maps-evidence-2026-09-30`); from here they pin the current build, and the fog oracle still holds native and Wasm to the same digests.

**Verdict:** sound. **Confidence:** high; only the arms with light or dense woods moved, and the garrison-collapse arm, garrison seats and line-of-fire queries came out byte-identical.

### Replace the light-versus-dense spotting scenario with open-versus-forest

**Choice:** `t1-spotted-light-vs-dense` became `t1-spotted-open-vs-forest`: a squad on open ground is spotted at least 1.3× farther than one in the forest. The map is 800 m long so the open squad starts beyond detection range.

**Gap:** With one rule there is no light or dense forest to compare; the moment that still matters is that a forest hides a squad that open ground would not.

**Verdict:** sound. **Confidence:** high.

### Leave the forest export's redundant columns for the next forest-export slice

**Choice:** The forest metadata export still carries a canopy height per forest (every row now holds the rule's 12 m) and an `id` that equals its row index, and a rectangular forest still travels as a rect row, a rect-to-forest ID row and a metadata row.

**Gap:** Trimming them changes the export layout and its renderer readers for no behaviour change, and the rectangle path exists to keep the village's float order.

**Verdict:** provisional; collapse them when C77/C78 next touch the forest export. **Confidence:** medium.

## C64 road kinds

### A dirt track is three quarters of road speed

**Choice:** `surfaces.dirt_track.speed_factor` is 0.75: a jeep does 13.5 m/s on a track against 18 on a road and 9 across a field; a tank 9 against 12 and 6.

**Gap:** The slice delegated the factor.

**Verdict:** sound for now; C50 owns tuning. **Confidence:** medium.

### A surface is never slower than the open ground beside it

**Choice:** Speed on a road kind is the unit's road speed times the factor, floored at its off-road speed. A squad on foot (road speed 1.3× its walk) gains nothing from a dirt track and loses nothing; a sidewalk's factor of 0 means "not a road".

**Gap:** The slice said the factor multiplies road speed. Taken literally, infantry would walk slower on a track than in the field next to it, and a sidewalk could not be given one number that suits every unit.

**Verdict:** sound. **Confidence:** high.

### Every carriageway is one kind to the sim; only its speed differs

**Choice:** The sim's own surface kinds stay ground, road, water, bridge and sidewalk. A country road and a dirt track are both "road" for trunk clearance, navigation regions and the exported tag, and differ only in `road_factor`. Where kinds overlap, the earlier kind in `SurfaceKind` wins (road, country road, dirt track, sidewalk).

**Gap:** The slice did not say how overlapping kinds resolve or whether the renderer needs the kind yet.

**Verdict:** sound for the physical half; C66 adds a per-kind exported tag when the look needs it. **Confidence:** high.

## Scale direction: wheeled road speeds

### Wheeled vehicles do three times their off-road speed on roads

**Choice:** The jeep's road speed is 27 m/s (97 km/h, from 18) and the supply truck's 21 m/s (76 km/h, from 14), three times their off-road speeds. Tracked vehicles stay at twice. Each unit type keeps its own two speeds in the catalog; there is no shared multiplier.

**Gap:** The user asked for wheeled vehicles to gain more from roads than tracked ones, "maybe 3x". `scale-direction.md` caps light vehicles at 110 km/h and wants a Large map's centre reached from an edge in about three minutes.

**Verdict:** sound; reversible data. One of six quick village trials changes digest, with the same outcomes and losses. `t3-jeep-takes-a-road-bend-at-speed` holds 27 m/s on the straight and rounds a right-angle bend about a metre wide of the road. **Confidence:** medium until the transit proof runs on a generated map.

## C65 round centerlines

### Sample only the bends; a straight stretch stays one segment

**Choice:** The curve emits points at most 2 m apart inside a bend and nothing between bends. A 5 km straight road is one segment, not 2,500.

**Gap:** The slice said the loader densifies strokes "to points ≤2 m apart". Subdividing a straight line changes no geometry, and every consumer (the sim's surface index, the exports, the terrain shader until C63 lands) pays per segment.

**Verdict:** sound. **Confidence:** high. Rivers whose width varies along a straight reach may need their own samples in C69.

### Round each corner through its control point, inside the stroke's own corridor

**Choice:** Paired cubic Béziers meet at the authored corner with the bisector's tangent. The bend reaches at most half the stroke's width from the authored runs and a third of the shorter adjacent run.

**Gap:** The slice delegated the spline family after centripetal Catmull–Rom wandered up to 70 m from the village's road. The prototype's local Bézier was the tested alternative.

**Verdict:** provisional on looks. The road swings about 2 m to the outside of a right-angle turn instead of cutting the inside, as a real fillet would; a fillet would leave the authored corner point off the road. The visual pass decides. **Confidence:** high for the physics, medium for the look.

### Refusal messages are not the contract

**Choice:** A degenerate stroke (zero-length or unrepresentable runs) may be refused by the centreline or by the stroke check; the test asserts the refusal, not which message.

**Gap:** The existing test pinned one owner's wording.

**Verdict:** sound. **Confidence:** high.
