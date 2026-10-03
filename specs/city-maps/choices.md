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

**Gap:** These oracles froze outputs of implementations that no longer exist (dense terrain, pre-aggregate buildings), so the named C72 change could not be re-derived from them. Their historical-equivalence proofs stand at `9a88280` (tag `city-maps-evidence-2026-09-30`); from here they pin the current build, and the fog oracle still holds native and Wasm to the same digests. **Superseded 2026-10-01:** those one-sided oracles are deleted; `fixtures/parity/` now holds only records a native and a Wasm test both read (see "Data files pruned" below).

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

### Every mover states `offroad_kmh` and `road_kmh`

**Choice:** The catalog's mobility rows read the same for foot, tracked and wheeled movers: two top speeds in km/h. Vehicles no longer use `mps`/`road_mps`, and infantry no longer uses a speed and a `road_multiplier`. The catalog refuses a type faster than 130 km/h or slower on a road than off it. Values: rifle squad 11/14, tank 22/43, supply truck 25/76, jeep 32/110.

**Gap:** The user found the two schemas odd and set the 130 km/h cap and the jeep's 110.

**Verdict:** sound. Rounding to whole km/h moved each speed by up to 2%, so every village digest changes; captures stay 2/3 (flank) and 0/3 (ambush), while losses over three seeds swing (flank 790 → 175, ambush 120 → 200), which is seed noise C50 should average over more seeds. **Confidence:** high for the schema; tank and truck values are today's, not yet checked against real vehicles.

## C04 pass 2: ground in the compiler

### Bound ground by authored points inside the map and one point allowance

**Choice:** A road or forest is refused if any authored point lies outside the playable rectangle; a stroke's width may overhang the edge. One caller limit, `max_ground_points`, counts polygon vertices plus rounded stroke samples across surfaces and forests.

**Gap:** C04 named "bounded-complexity failures" without saying what is counted, and did not say whether a road may touch or cross the map edge.

**Verdict:** sound for now. Generated roads end at the playable edge; scenery beyond it is a later rendered-surroundings contract. The limit does not bound trees a forest stands at load, which the scale work owns. **Confidence:** medium.

## C63 surface distance field

### An exact bucket index, without running SG5

**Choice:** The field is a segment-bucket index over the simulation's exported primitives, read with the distance functions the terrain material already had. Nothing is baked or resampled. SG5 was not run as a separate spike.

**Gap:** The slice waits on SG5 to choose between a signed-distance bake and an exact index. The work was started with the index already chosen.

**Verdict:** sound. The index keeps every village terrain pixel byte-identical, which a bake cannot, and its error at joins and width changes is zero by construction, so SG5's error tables have nothing left to measure for this arm. SG5's cost and memory questions are answered in the slice's Outcome. **Confidence:** high for exactness; medium for cost at 6–10 km until a generated map exists.

### A ladder of grids by pixel width, because the reach grows with the pixel

**Choice:** The index is a ladder of grids. Each level serves pixels up to twice as wide as the level below and lists primitives out to that pixel's reach. `groundCell(xy, footprint)` picks the level; `groundSite`, `groundWater` and `groundDapple` take the cell it returns.

**Gap:** The slice asks for one reach `R`. But every reader feathers over "a pixel at least" (the road's edge, the verge, the forest floor, the water bed), and a pixel's footprint has no upper bound: ground seen edge-on a kilometre off spans hundreds of metres. One fixed reach either changes those pixels or lists far too much for every other pixel.

**Verdict:** sound. `groundReach` in `terrainMaterial.ts` names each reader and how far it reads, from the biome's numbers. **Confidence:** high.

### On a dense map the ladder stops at a list budget

**Choice:** The ladder ends before a level whose cells would list more records on average than its budget: 24 at the finest level, growing by √2 a level (96 for 32 m pixels, 384 for 512 m). A wider pixel reads the last level, and so no farther than that level's reach. On a sparse map (the village with its 103 records, every lab) the ladder runs to a single cell that lists everything, so every pixel reads exactly and nothing moved.

**Gap:** An exact answer for a pixel 500 m wide reads every primitive within 500 m. On a 4,300-record town the few rows of pixels under the horizon would cost more than the rest of the frame. The slice's bar is cost bounded by what is near the fragment.

**Alternatives:** A flat budget of 64 was the first rule. C65's rounded bends took the village from 7 records to 103 and that rule stopped the village's own ladder at 256 m pixels. The budget grows with the pixel because wide pixels are rare: in a ground view the rows F metres wide thin as 1/√F, so √2 more records a level costs the same per frame, a few hundred records a screen column at most.

**Reach:** On a dense map, ground seen edge-on from far away loses the forest floor and verge smeared in from farther than the last level's reach (about 35 m on the 4,300-record test town, 70 m on the 6 and 10 km ones); the road's own edge stays exact to twice that. No shipped map is dense enough to stop its ladder: the village has a margin of 3.7×. The first generated town is where to judge the look.

**Verdict:** provisional: the budget and where it bites are untested on real generated ground. **Confidence:** medium.

### Finest cells of 8 m, serving pixels to 2 m; at most 262,144 cells

**Choice:** Level 0 has 8 m cells and serves pixels up to 2 m wide. A level's cell is at least four of its footprints. A map too large for 262,144 finest cells doubles its cell (16 m at 6 km, 32 m at 10 km).

**Gap:** Delegated: resolution and packing.

**Verdict:** sound; measured in the slice's Outcome. Halving the cell-to-footprint ratio would halve the records wide pixels visit for 1.7× the index. **Confidence:** medium.

### Beyond its reach a distance keeps only its side

**Choice:** A point farther outside than the reach reads a large negative; a point deeper inside a polygon than the reach reads at least the reach. Rects and strokes stay exact inside.

**Gap:** The slice says "exact-enough" without saying what a reader may assume far from an edge.

**Reach:** The two GPU distance probes in the ground scene asserted exact distances 5 to 30 m from an edge. They now ask twice: at a pixel as wide as the map, where the original exact assertions hold unchanged, and at a play-camera pixel, where the distance must be exact within the reach and on the right side beyond it.

**Verdict:** sound. **Confidence:** high.

### Rects moved into the records table

**Choice:** Forest and water rects are records beside the strokes, triangles and edges. The index takes the binding the rect table had.

**Gap:** The grass build is at the default limit of eight storage buffers, so a fifth terrain table was not available.

**Verdict:** sound. **Confidence:** high.

### Mixed stroke and polygon joins are still the larger of the two distances

**Choice:** Not changed. Where a stroke meets a paved polygon the field returns the maximum of the stroke's distance and the polygon union's, as before.

**Gap:** C03's ledger left "correct or admit mixed joins" to C63. This slice was scoped to the same distance functions and unchanged village pixels, and a corrected join moves pixels.

**Verdict:** open. It needs its own decision before generated streets mix strokes and polygons. **Confidence:** high that it is still open.

## C52 layout generator

### The approach rule is measured on the main settlement, and the generator reserves the ground for it

**Superseded in part** by [Town model and edge roads](#town-model-and-edge-roads): the rule still holds for the main settlement in each half, but an approach is now a corridor measured from where the districts end, not a fan of rays from the centre.

**Choice:** A plan passes when its main settlement (the highest class; the city in Metro) has, in the top half and in the bottom half, a wedge of ground open for 1,800 m beyond its edge across a front of 400 m. The generator picks one bearing in each half where such a wedge fits between the settlement and the playable edge, and keeps later settlements and woods out of it. The finished plan is then measured the way S7 measured: rays from the settlement, a hundred metres apart at full depth, stopped by other settlements, forests and the map edge.

**Gap:** M19 says "open approaches … to settlements in both halves" without saying which settlements. Counting any village would let Metro pass on a satellite's field while the city had none, which is what M18 was written to prevent.

**Verdict:** sound. It is one rule for every type and size. On Small Metro the wedges often run toward a corner, where the city leaves the most room; they are still 1,800 m of open ground inside the playable area. **Confidence:** high for the rule, medium for whether a corner approach plays as well as one from an edge.

### Metro's city covers 16–24% of the playable area at every size

**Choice:** The city's envelope (its outline, fields between districts included) is a seed-drawn 16–24% of the map, the same range at Small, Medium and Large. Built ground on Small Metro comes to 11–22%.

**Gap:** M18 says to tune Metro's share down until Small meets the approach rule, without a number, and does not say whether Medium and Large use the same share.

**Verdict:** sound for Small; open for Large. Over 300 Small Metro seeds, a city of 24% or less passed every time; 28% passed 98%, 32% 95%, 36% 88%, 40% 59% and 45% under 1%. Past 24% the first failure is room for the satellite towns, not the approach. Medium and Large could hold a larger city and still pass (their edges are farther away), but one range keeps Metro's building count down, which M18 names as the main scale risk. **Confidence:** medium. Raising the share on Medium and Large is a one-line preset change if the user wants a bigger city there.

### The transit rule is a time, 183 s, and it fixes the Large crossroads

**Superseded** by the 215 s entry below and by [Town model and edge roads](#town-model-and-edge-roads): only the top and bottom edges are held to a road, and Large maps vary.

**Choice:** Every plan must have, from the middle fifth of each edge, a road journey to the centre of at most 183 s at 110 km/h, of which 15 s is an allowance for planning and turns. Edge roads are built to that budget: each runs to the main junction by the centre or, when time allows, joins an earlier edge road up to 1.5 km out.

**Gap:** The target is "about three minutes" on Large, and M21 asks for road patterns that are not the same crossroads every time. 183 s is the worst case S7 measured and the user accepted with M20.

**Verdict:** sound as a rule, but it does not deliver M21 on Large. The budget is 5.13 km of road for a 5 km line, so on Large every plan has four roads meeting at the centre; only their skew, the settlements on them and the secondary network vary. On Small and Medium 22–41% of plans are a central crossroads and the rest fork or meet in T-junctions. **Confidence:** high on the arithmetic. The user's dial is `transit.max_s`: about 200 s would allow forks 600 m from the centre on Large.

### A town is rings of sectors, each one district of one use, with some left open

**Superseded** by [Town model and edge roads](#town-model-and-edge-roads): a district is a block cut along the settlement's roads. One use per district, the ids, areas and anchors, the open ground and the woods on it all remain.

**Choice:** A settlement's outline is cut into bands from the centre out and each band into sectors. A sector is one district with one dominant building category and at most one minor one (a garden suburb is 90% detached homes; an industrial district is all industry). In the outer band of a town or city some sectors are left unbuilt and the built ones stop at different depths, so fields reach in between districts; a wood stands on up to half of those open sectors. A sector that a main road enters the town through picks from a second set of weights that favours industry. Each district has an id (`settlement-3/district-2`), its area and an anchor point inside it.

**Gap:** The slice asked for "settlement plots" with category mixes and left their shape to the implementer. S7's districts were a jittered grid, each a blend of three or four categories. The user's Broken Arrow references then asked for single-use districts, green gaps, a main-road spine with industry on it, and districts an encounter can address.

**Verdict:** sound as zoning; provisional as a look. Sectors need no polygon clipping, never overlap and are cheap, but the picture reads as a dartboard. C53 cuts blocks and streets inside them and can replace the shapes without touching the rules. The anchor is a point half-way out along the sector's middle, not the centroid, because a ring sector's centroid can fall outside it. **Confidence:** medium.

### "Built ground" is the districts; a settlement's size class is its envelope

**Superseded in part** by [Town model and edge roads](#town-model-and-edge-roads): built ground is still the districts. A class's size is now the ground the settlement may build on, and its outline is the edge of what it built.

**Choice:** Fairness, the urban share and "the main settlement holds N% of the built ground" count district area only. A class's size in hectares is the envelope's, so a town is the same footprint at every map size (M05) however many of its sectors are open.

**Gap:** Once towns have fields inside them, "town area" has two meanings.

**Verdict:** sound. **Confidence:** high.

### Settlements stand on the main roads; a road that reaches a settlement carries on through it

**Superseded in part** by [Town model and edge roads](#town-model-and-edge-roads): the main settlement now stands on the main junction, a road crosses a settlement in a straight line between gates, and a joining road ends only where it makes a plain junction.

**Choice:** The straight lines of the edge roads are fixed first. Each settlement is then sited on one of those lines (40–55% of attempts, by type), beside the main settlement (10–30%), or anywhere, and the edge road is built through the centre of every settlement on or near its line. Any other settlement is joined to the nearest road good enough for its class, and that road continues straight through its centre to the far edge as a main street. A road that joins stops at the first road at least as good as itself, in a T-junction.

**Gap:** S7 joined every settlement to the nearest road and noted towns hanging off tracks, stubs and a fixed crossroads. The references asked for towns strung on a road.

**Verdict:** sound. A town never hangs off a track, and no joining road crosses a better one. The main street reads as a stub on the picture until C53 gives the settlement streets. **Confidence:** medium.

### Fairness is built in, then checked

**Superseded in part** by [Town model and edge roads](#town-model-and-edge-roads): the forest half of this stands. Built ground is now balanced in three places, since districts exist only after the roads do.

**Choice:** Each settlement goes in the half with less built ground, and when that half is behind by an amount its class can make up it is drawn at that size. A town's open sectors are taken turn about from its north and south sides. Each half is given the same forest target and filled to it. A few settlements or woods sized to the remaining difference close what is left. The finished plan is measured against S7's tolerances and refused if it fails.

**Gap:** S7's balancing step passed 98.2% of layouts; the misses were Small maps where one large wood outweighed the repair.

**Verdict:** sound: 2,700 of 2,700 plans pass, and the halves usually agree to within a few hundredths of a square kilometre. That is tighter than the tolerance asks and nothing mirrors. **Confidence:** high.

### A search that runs out refuses the request

**Choice:** If a settlement finds no site, the main settlement has no room for its approaches, or the finished plan fails a measured rule, generation returns a diagnostic naming the feature, the preset cell and the seed. It does not drop the settlement, shrink the town or try another seed.

**Gap:** S7 left up to four Metro satellites unplaced and carried on.

**Verdict:** sound. No seed in the sweep was refused with the shipped presets; the refusals appear when a preset asks for more than the map holds. **Confidence:** high.

### The plan records approaches, not a plain polygon

**Choice:** `MapPlan.approaches` lists each measured open wedge: the settlement, the half, the two bearings, the depth and the front. Open plain is whatever is neither district nor forest; no polygon for it is written.

**Gap:** The slice asks for "connected traversable plains" in the plan. The compiler does not admit land regions yet.

**Verdict:** sound for now. The approaches are what an encounter planner needs (where to deploy against a town). C54 can derive region polygons from the compiled map when land regions are admitted. **Confidence:** medium.

### One seed is nine maps, and every number is a whole centimetre

**Choice:** Each random stream is keyed by the seed, the map type, the size and the stream's name. Plan coordinates are rounded to centimetres and approach bearings to microradians.

**Gap:** The slice named streams by purpose only. An unprimed review of the first gallery found that seed 3 gave Mixed and Metro the same road skeleton and Small and Large Metro the same city, scaled.

**Verdict:** sound. Rounding keeps the plan short and lets any JSON reader read it back exactly; a centimetre is far below anything a parcel can tell apart. **Confidence:** high.

### Settlement counts differ from S7's proposals

**Choice:** Open has only villages and hamlets around its one small town (S7 had extra small towns on Medium and Large). Mixed has 1–2, 2–3 and 3–4 towns by size, against S7's 2, 4 and 6, and a larger main town (360–520 ha).

**Gap:** S7 itself reported that Open Large read as a thin Mixed and that Mixed Large had no dominant town (M21).

**Verdict:** sound: Mixed's main town holds 28–50% of the built ground on Large and is at least twice the footprint of any other settlement. **Confidence:** medium; these are the numbers most likely to change on review.

### Parity records hold a hash of each outcome

**Choice:** `fixtures/parity/map-layout/paired-records.json` stores each request and the SHA-256 of the native outcome. The web test hashes what Wasm returns.

**Gap:** The compiler's records store whole outcomes, which are a few kilobytes. A generated plan is tens of kilobytes.

**Verdict:** sound. A hash holds every byte to account; on a mismatch the two outcomes have to be regenerated to see where they differ. **Confidence:** high.

### Edge-to-centre by road within 215 seconds, so Large maps can vary their roads

**Superseded in part** by [Town model and edge roads](#town-model-and-edge-roads): 215 s stands, and now binds the top and bottom edges only.

**Choice:** `transit.max_s` in `fixtures/map-presets.json` is 215 (revision `layout-presets-2`), up from 183.

**Gap:** The "about three minutes" target allows 5.13 km of road for a 5 km line on Large, so every Large plan had four roads meeting at the centre, against M21's varied road patterns. The user said the numbers are guesses to tune unless called hard requirements.

**Evidence:** Over 100 seeds per Large cell, a central crossroads appears in 100% of plans at 183 s, about 60% at 200 s and 30–40% at 215 s, the range Small and Medium already had (22–41%). Nothing was refused at any value.

**Verdict:** sound. One global number, no per-size exception. The worst edge on Large is now up to 215 s at 110 km/h including the 15 s allowance. **Confidence:** medium until a vehicle drives it in a battle (SA2).

## C53 parcels and buildings

### A parcel is its template's footprint plus the district's setbacks

**Choice:** Walking along a street, the pass draws a template and cuts the parcel that template needs: its own footprint, a front setback from the street, a setback on each side and one at the rear. If that parcel would leave the district, touch a carriageway, a forest or another parcel, it tries up to two more templates of the same category and then moves 4 m on, leaving the gap as open ground. The template is placed whole, turned so its entrances face the street.

**Gap:** The slice says "fit parcels to legal envelopes or choose another eligible template; never stretch", and leaves parcel subdivision to the implementer. Cutting blocks into fixed lots first and then looking for a template that fits each would leave most lots with the wrong shape for anything in the catalogue.

**Verdict:** sound. Nothing is ever scaled, a building cannot stand on its neighbour because parcels do not overlap, and what does not fit shows as a yard. The cost is that blocks are not tiled edge to edge: corners and block ends stay open. **Confidence:** high.

### A place along a street draws one category, then tries that category's templates

**Choice:** At each place the pass draws a category by the district's weights and then tries up to three of that category's templates. A large template that does not fit gives way to another of its own category, not to the district's minor one.

**Gap:** `DistrictPlan.categories` is documented as "shares of its ground" (C52). Redrawing the category on every failed fit let small homes take the places where farmsteads did not fit: a farm district zoned 80% farmstead came out 62%. Weighting the draw by each category's parcel size was tried as well and was no closer (within 13 points against 10).

**Verdict:** sound: over eighteen maps every district kind's dominant category holds within fifteen points of its share of the built ground. **Confidence:** medium; the catalogue's mix of sizes moves it.

### Streets are a grid in the district's own frame; parcels front every carriageway

**Superseded in part** by [Town model and edge roads](#town-model-and-edge-roads): the grid, its bends and the parcels stand. The frame no longer runs out from the settlement's centre: it runs with the settlement's main street.

**Choice:** Each district gets long streets `block_depth_m` apart and cross streets every `block_length_m`, laid in a frame that runs out from the settlement's centre (the central district's runs with the road through it). A district kind may bend both families with a sine swing (garden suburbs, villages and farms do; centres, apartments, the core and industry are straight) and may leave out cross streets at random. A stretch of street that would run within half a block of an existing carriageway and within 30° of it is dropped, so no street doubles the main road. Parcels are then cut along every carriageway in the district, the country road or track included.

**Gap:** The slice delegates subdivision. C52 left districts as sectors with a road through the settlement's centre and nothing else.

**Verdict:** sound as a system, plain as a look. The patterns differ by numbers, not by code: a garden suburb is an 84 × 190 m grid swinging 16 m, an industrial estate a 210 × 280 m one. Every district is recognisably a grid; there are no crescents, squares or cul-de-sac loops. **Confidence:** medium.

### Every piece of a district's grid is joined; pavement connectivity is measured on the finished plan

**Choice:** Where a road or an earlier street crosses a district's grid, the grid is joined there. Any piece of grid that nothing crosses gets one straight link from its nearest node to the nearest street. `measure` then builds the road graph from the rounded centrelines the surfaces are made of and the plan is refused if any street has no way to the centre.

**Gap:** The slice asks that "vehicles can cross town into plain". A grid clipped to a sector falls into pieces, and a road graph built from authored points (as C52's was) misses a street that meets a bend's rounded surface.

**Verdict:** sound. A separate test floods the compiled map's surfaces from the roads that leave the map and reaches every street. The links are short diagonals that look improvised on the picture. **Confidence:** high for connectivity, low for the look of the links.

### A street that stops within a block of another carriageway runs on to it

**Choice:** After a settlement's streets are laid, each dead end is carried straight on for up to one block depth. If it crosses a carriageway in that distance (and not at a shallow angle) a short street joins the two.

**Gap:** A grid is kept only inside its district, so every street stopped at the district's edge: a few metres short of the main road, or facing the next district's streets across a gap. An unprimed review of the first gallery named this the largest defect in the pictures.

**Verdict:** sound. Districts now meet each other and the roads beside them at many points instead of one. A run-on may cross a field or a wood between two districts. **Confidence:** medium.

### Aprons are paved polygons of kind `road`; nothing else in a parcel is drawn

**Choice:** A district kind may pave the front of each built parcel: 12 m of parking before an apartment slab, the whole 30 m yard before a shed. The apron is a four-point polygon surface of kind `road`, as wide as the building, from the carriageway's edge. Lawns, gardens and courts are simply ground left open.

**Gap:** The brief asks for aprons "if the surface contract supports it". It does, but C63's ledger still lists the look of a stroke meeting a polygon as open.

**Verdict:** sound physically (a vehicle drives the yard at road speed, and no building stands on one); the drawn join is C63's open item. **Confidence:** medium.

### A terrace is one template; separate buildings never share a wall

**Choice:** Row houses and shop rows are single templates of three to five units joined by the descriptor's own declared joins. Two separate buildings always stand at least twice the side setback apart (1.5 m in a town centre).

**Gap:** The slice says "terraces use only S5/C13's baked compatible edge variants". A wall shared between two placed buildings would be an exposed, bay-carrying facade on both, with seats facing a wall 0 m away.

**Verdict:** sound. A terrace is one building with one integrity, which is the accepted rule for a compound. **Confidence:** high.

### The prototype catalogue is a descriptor list, apart from the library the authored maps pin

**Choice:** `fixtures/prototype-building-templates.json` is a list of 29 complete descriptors, regional family `prototype`, ids `prototype-…`: three farmsteads, five detached homes, five attached homes and shop rows, seven apartment blocks (4–8 floors), four highrises (10–20 floors) and five sheds. Every entrance is on one side. `mapgen catalogue` prints the list's canonical `{hash, templates}` form for a request to pin and the loader to resolve against.

**Gap:** `fixtures/building-templates.json` is already a canonical library, but every authored map pins its hash, so adding to it would change their identity. The CLI and Wasm boundaries take a descriptor list.

**Verdict:** sound for a systems proof. These are labelled boxes: they certify no real dimension, join or appearance, and the specialist's sources replace them through the same contract with a new hash. **Confidence:** high.

### Every placed building is the prop type `building`

**Choice:** `parcels.prop_kind` in the presets names one prop type for every category: today's garrisonable house body (400 hp).

**Gap:** M17 says categories do not change combat rules. Nothing says a warehouse and a tower share a hit-point row.

**Verdict:** open for C50 (durability balance). It is one preset string. **Confidence:** low that 400 hp is right for a twenty-floor tower.

### An empty district refuses the request

**Choice:** If no parcel fits anywhere in a district, generation fails with a diagnostic naming the district and its preset row.

**Gap:** "If nothing fits, leave a yard" covers one parcel, not a whole district.

**Verdict:** sound, by C52's rule that a search that runs out is a refusal and never a thinner map. No seed in the sweeps was refused with the shipped presets. **Confidence:** high.

### `generate` takes the catalogue; the plan records its parcels

**Choice:** The CLI's `generate` and the Wasm `generate_map_plan` now take the catalogue and return the whole plan: layout, streets, aprons, parcels (`lots`, plan-only) and buildings. A layout alone is the library's `generate_layout`. The generator is `layout-2` and the presets `layout-presets-3`, so an old request is refused. A building's id is its parcel's (`settlement-3/district-2/lot-14`).

**Gap:** The slice names workbench layers for parcels and rejected fits. A place where nothing fitted is not a parcel, so it is not recorded.

**Verdict:** sound; the rejected-fit layer is not built. **Confidence:** high.

### The template contract refused most placements on a full-size map

**Choice:** `contract::templates` now holds a placed bay or entrance to its facade with the rounding of the larger world coordinate, not each coordinate's own.

**Gap:** C00's proofs placed buildings near the origin. On a 10 km map a turned building far along X and near the origin in Y was refused as "off its facade": 3,477 of 4,000 placements of the asymmetric fixture.

**Verdict:** sound; a contract defect the first full-size town exposed. **Confidence:** high.

### Metro's core is apartment blocks with a few towers

**Choice:** The `core` district's mix is 85 apartments to 15 highrise (presets revision `layout-presets-4`), from 75 highrise to 25 apartments.

**Gap:** C52 chose the first mix. On Metro Large it stood about 100 towers in a field; the Broken Arrow reference has one tower among apartment slabs.

**Verdict:** sound. Seed 1 of Metro Large now has 21 towers among 9,276 buildings. Highrises remain Metro-only. **Confidence:** medium until real tower art exists.

## SA2 navigation at full extent

**When:** SA2 implementation (counted planning, road journeys), 2026-10-01. The user had already decided the hold while planning, the 2,000 m rule and the three-minute jeep target; these are the decisions the spec left open.

### A long leg becomes a fast move, written on the order

**Choice:** When a move or attack-move leg starts and its goal is strictly more than `navigation.road_leg_m` (2,000 m) away as the crow flies, the order's own route policy becomes `fastest`, exactly as if the player had double-clicked. It stays that way for the life of the leg, however short the leg has become when the unit plans again. A queued leg is measured from where it starts, not from where it was ordered. The alternative was a separate hidden "road preference" flag beside the order.

**Gap:** The spec said the preference is inferred and frozen, not where it lives, nor whether attack-move counts as an ordered leg.

**Verdict:** Sound. One mechanism (the fastest policy) instead of two, and the player's panel and order line show the truth: the unit is on a fast move. Attack-move is included: it is an ordered leg that happens to halt on contact, so a long advance to contact goes by road until it meets something. Pursuit and a building's approach are never legs and never change. **Confidence:** High for moves; medium for attack-move, which the user may prefer to keep cross-country.

### The fastest policy means "by road if a road journey beats the straight line"

**Choice:** A fastest leg first asks the map's road graph for the quickest way: straight to a road, along roads, straight off again. If that is quicker than driving straight there across country, the unit takes it; otherwise it plans straight across country. Both are judged as the crow flies, with the unit's own speeds, over the ground the line crosses: a wood or a hillside on the line counts as slow, bodies do not count. There is no detour tolerance number: the comparison is the rule. The old planner instead searched the whole grid for the provably fastest route, which is what flooded.

**Gap:** The spec asked for a "useful" road journey without defining useful.

**Verdict:** Sound. Against the frozen exact planner on 12,064 small cases, fastest routes average 0.5% slower, 15 of 4,757 are more than 10% slower and the worst is 17% slower. In the movement lab the fast tank takes the road round the wood and beats the tank that cuts through it, as before. **Confidence:** Medium: the straight-line estimate knows woods and slopes by 64 m tiles, not rivers or walls, so a road that only pays because the direct way is cut off is judged against a direct way that does not exist.

### Roads within 1,000 m of each end are considered

**Choice:** `navigation.road_access_m` is 1,000 m. A leg looks for road within that distance of its start and of its goal.

**Gap:** The spec left "nearby" to be measured.

**Verdict:** Provisional. On the probe maps (a road every 1.5 to 2 km) a unit is at most about 850 m from a road, so 1,000 m lets nearly every long leg find one; the journey-time comparison then discards roads that do not pay. Re-measure on generated maps. **Confidence:** Low to medium.

### The road graph is built from road strokes only

**Choice:** At load the map's road strokes become a graph: a node at each bend and wherever two roads' surfaces touch (their centrelines pass within the sum of their half-widths), an arc for each straight run. A run the terrain does not carry (water with no deck, ground too steep) is left out. Roads authored as polygons keep their speed but are not in the graph.

**Gap:** Polygon-road connectivity has no source contract yet (the spec names it as missing).

**Verdict:** Sound for generated maps, whose roads are strokes. A polygon plaza or junction slab does not join the roads that end in it; if generated maps use them, they need centrelines too. **Confidence:** Medium.

### The road says where to go; the grid says what fits

**Choice:** The graph is public terrain and knows no bodies. The chosen way is then checked on the side's own grid, 64 m at a time, for the unit's footprint. Where a known body stands on the road, the unit looks for a way round it within 128 m and rejoins the road. If nothing gets round (a wall across a bridge deck), that stretch of road is closed for this one plan and the graph is asked again. If the unit cannot reach the road at all, it plans straight across country. A body the side has not seen changes nothing until the unit drives up to it, learns of it and plans again, still on its fast move.

**Gap:** The spec required physical refinement and failed-arc retry without fixing the pieces.

**Verdict:** Sound: a truck goes to the other bridge, a hidden collapse does not leak. **Confidence:** High for the cases tested (wall on a road, wall on a bridge, no bridge); medium for long columns of wrecks, where a way round more than 128 m long closes the road for that plan.

### Vehicles keep to the right of the road

**Choice:** A road journey runs down the right-hand side of each road: the unit's left side half a metre from the middle, but its own middle never nearer than half a metre to the road's edge, so on a narrow track a wide vehicle runs with its right side off the track. It takes 32 m along the road to get over to its side, and the same to come back to the middle where it leaves.

**Gap:** Not in the spec. The first edge-to-edge run put both sides on the same centreline and they met head-on and stood there for the rest of the battle.

**Verdict:** Sound on 8 m roads (a jeep and a tank pass without either waiting) and on a 5 m track in the open (a jeep and a tank each way all get past). Through a wood there is no room beside a 5 m track, and two columns can still jam there. **Confidence:** Medium.

### A vehicle stuck in traffic plans round the whole knot

**Choice:** A vehicle that has waited too long for another already planned a way round that one vehicle. It now plans round every vehicle within 40 m of it, as they stand.

**Gap:** Not in the spec. Two columns meeting on a track are four vehicles, and a way round one led straight into the next.

**Verdict:** Sound: the two-columns test passes with it. The lower-numbered vehicle of a pair still just waits, as before. **Confidence:** Medium; 40 m is a guess that covers a short column.

### A unit joins and leaves the road at a slant

**Choice:** A unit 300 m from a road heads for a point 300 m further along it, not the nearest point, and leaves the road the same way.

**Gap:** Not in the spec.

**Verdict:** Sound: it reads as a crew cutting across to the road, and avoids a right-angle turn onto it. **Confidence:** High.

### The cross-country search walks its line

**Choice:** The grid search now estimates the way still to go as the walk over open ground, at the pace the ground straight ahead allows (a wood or a hillside ahead counts as slow), and of two equally good cells takes the one nearer the goal. Each cell is expanded once. Across open ground it searches about one cell per cell of route. The exact planner's start-up proofs (the separating-line proof, the uniform-rectangle shortcut, the cost-bound walks and the map-wide totals behind them) are deleted: they existed to make an exhaustive search affordable.

**Gap:** The brief asked for a bounded local search and left the estimate open.

**Verdict:** Sound. Shortest routes average 0.4% longer than the exact planner's on the 12,064 cases, none more than 10%, worst 7.8%. A search still has to fill the pocket in front of an obstacle before it finds the way round, as any grid search does. **Confidence:** High.

### A search gives up at a limit and says the route is blocked

**Choice:** A search may expand `search_cells_base` (20,000) cells plus `search_cells_per_m` (200) for every metre between its ends. Reaching the limit ends the plan as blocked, with its own reason inside the simulation (`SearchLimit`, distinct from "no route exists"); the player sees ROUTE BLOCKED either way and the order is kept and retried when the side learns something.

**Gap:** The user ruled that running out of a tick's work is never "no route"; an indefinite search still needed an exit.

**Verdict:** Sound but blunt. A goal across an unbridged river costs the whole limit before the unit reports blocked: about 5 s of planning for a 2.8 km leg. A reachable goal whose only way round lies outside roughly a 400 m band either side of the line is also reported blocked. A cheap proof that a goal is cut off (searching from both ends) would shorten the first case. **Confidence:** Medium.

### A straight pull past the middle of a tight cell needs the room it gives up

**Choice:** A cell's room is measured from its centre. A vehicle's straight route segment, or the point it is told to stand on, that passes to one side of a cell's centre now has that much less of the cell's room. Infantry is unchanged (it already reads half-metre sub-cells).

**Gap:** Not in the spec. With the new search the jeep-at-the-garden-fence scenario routed diagonally through the 4 m gate, clipped the post by 14 cm and stood there re-planning; and a jeep sent to a point among trees stopped 0.9 m short of it for good.

**Verdict:** Sound: it is the footprint rule applied where the route actually runs. Routes near obstacles keep a waypoint more, and 15% of the comparison's goals near map edges and bodies move to the middle of their cell. **Confidence:** High for the rule; the general gap (the grid judges a disc, the hull is a box) remains.

### Planning work is shared equally, a tick at a time

**Choice:** `navigation.work_per_tick` is 4,000 units, about one searched cell each (roughly 25 to 30 million instructions a tick when fully used, 3 to 4 ms on this machine unloaded). Every waiting unit gets an equal share each tick, round and round in unit order. A tick may overrun by one indivisible step (at most 576 units); the overrun comes off the next tick. A plan that fits in its share commits on the tick it was asked for, as before.

**Gap:** The spec asked for a budget and fairness, not the numbers or the unit.

**Verdict:** Sound. Twelve 900 m plans at once start moving after 0.13 to 0.27 s, twelve 9 km ones after 1 to 2.2 s; a short order is never starved by long ones. **Confidence:** Medium on the number: it is tuned on a loaded development machine and should be re-measured in the browser.

### What a unit does while it waits

**Choice:** A unit with a plan pending has no route: it holds, and the published move state is `planning`. A squad takes cover and posts as it would standing still, and its soldiers join the corridor from wherever they are when the route arrives. A vehicle that is planning a way round traffic keeps its old route to resume if no way round is found. A pursuit whose target has moved more than 5 m asks again, dropping the unfinished plan. The panel shows no row for `planning` yet. Because a blocked verdict can now arrive after the order's flash of order marks has faded, the marks flash once more when a unit's route turns out blocked.

**Gap:** The spec accepted the hold and proposed the published state, leaving the consumer review open.

**Verdict:** Sound. The order line appears at once (the goal is published); only the unit's first metre waits. A panel row is a UI decision for the client pass. **Confidence:** High.

### A side that learns something while a route is planned

**Choice:** The plan carries on against the side's new picture. When it finishes, it stands only if it still fits what the side now knows (and shoves nothing new); otherwise it is planned again from scratch on the new picture.

**Gap:** The spec asked for invalidation of affected work and warned against restarting on every change.

**Verdict:** Sound: unrelated changes never restart a plan, and a route through a newly learned body is never handed out. **Confidence:** High.

### A finished search hands its memory to the next

**Choice:** A search's bookkeeping (the cells it reached) is never freed when the search ends or is cancelled: it goes to a pool and the next search reuses it under a new generation number. Cancelling is a constant-time hand-over.

**Gap:** The standalone prototype freed a cancelled request's maps in one go, which the spec named as its open gap.

**Verdict:** Sound. Memory is bounded by the largest search (the limit above) times the most plans ever pending at once. **Confidence:** High.

### Each side's planning grid is built with the battle

**Choice:** `Battle::new` builds both sides' grids. Before, the first order built them in its tick.

**Gap:** Found by measurement: about 21 G instructions of S1's "12 s first tick" on the 6 km map were the two grids being built, not planning.

**Verdict:** Sound for loading, and it exposes the next blocker: the grid is still rebuilt whole whenever a side's knowledge changes (a tank fells a tree, a wreck appears), at the same cost. See the SA2 outcome. **Confidence:** High.

## C69 rivers

### The rule, and the moments it was judged by

**Choice:** Water is the ground within half a river's width of its rounded centreline. No ground mover stands on it; a deck over it is ground like any other; a round stops at its surface.

**Gap:** The slice asks for the neighbouring rules to be walked (tweak-mechanics) and says only "impassable and crossed at bridges".

**The moments:**
- *A squad reaches a river.* It walks down the bank to the water and stops. Ordered across, it goes round by the bridge; with no bridge its route is blocked and it stays. A film squad would wade a shallow stream, but depth is not read by the rule: every river is deep enough to stop everyone. A ford, when one is wanted, is a deck at the bed's height, with no new rule.
- *A tank reaches a river.* The same. It does not ford.
- *On the bank.* The bank is gentle and can be stood on to the waterline. Men at the water stand below the land, so the land's lip hides them from far eyes by the terrain's own line of sight, with no cover rule added.
- *Across the water.* Eyes and rounds cross it as open ground. A round falling into it stops at the surface: a shell bursts on the river and its blast reaches men on the bank; it digs no crater and leaves no scorch.
- *Ordered into the water.* A squad's goal is moved to the nearest ground it can stand on, so within about 15 m of a bank it walks to the bank; farther out, and for any vehicle, the route is blocked and the order kept, as for any goal that cannot be reached. A group's places are spread round the click and can land on both banks, each unit going to its own. A film tank would drive to the bank too; that is the unreachable-goal rule's to change, for cliffs as well.
- *Shoving.* A wreck pushed off a deck's side drops to the bed; pushed along the deck it stays on the deck. (It used to follow the bed under the deck.)
- *Woods and roads.* Water is neither. Trunks keep the forest rule's clearance from it, as from a road.
- *What it rules out.* Standing in water, a road under water giving road speed, a forest's concealment or slow going over water, a crater in a river.

**Double counting, freezes, cheap tricks:** none found in the rule. A river is a wall with gates, which is what it is for, and the other side sees the same gates. The freeze that does exist is not the rule's: see "Squads at a bridge" below.

**Verdict:** sound. **Confidence:** high.

### The minimum width comes from the grid: three height samples

**Choice:** A river point is refused under three times `height_grid_m`: 12 m on every shipped map.

**Gap:** Q-G5 says 12 m and gives the reason in brackets (three cells on the 4 m grid).

**Verdict:** sound. The reason is the rule: a channel narrower than three samples falls between them and is not carved. A coarser grid needs a wider river and says so. **Confidence:** high.

### The cross-section is a V, and its grade is depth over half the width

**Choice:** The bed falls in a straight line from the waterline to `depth_m` at the centreline. The bank carries the same grade on above the waterline. A point is refused when that grade, as the grid can draw it (up to √2 steeper, with a tenth to spare), would reach the slope cutoff: at 35° a 12 m river may be 2.67 m deep.

**Gap:** Delegated: bank profile. The slice asks for "a bank steeper than the slope cutoff" to be refused without saying what sets a bank's steepness.

**Alternatives:** A bank width per river, or a flat-bottomed channel with its own bank grade. Both add an authored number. One grade through the waterline has a property the others lack: the ground is one plane across the water's edge, so the grid's triangles cross the surface exactly on the rule's edge wherever the river runs straight.

**Verdict:** sound. **Confidence:** medium: a wide, shallow river has a long gentle bank (30 m wide and 2 m deep gives 1 in 7.5), which may want its own number once banks have art.

### A bank is cut below the land at the water's edge, not below a level

**Choice:** On the bank the ground is the land's own height, less whatever the cross-section still lacks to reach the land's height at the nearest point of the water's edge. The cut ends where the bank has climbed that far.

**Gap:** The slice says "a gentle bank from distance".

**Alternatives:** The first version took the lower of the land and the V. A ridge 60 m from the geometry lab's river was then sliced flat by the V's plane, and a bank could run on as far as any hill near it was high.

**Verdict:** sound. Land beside a river keeps its shape, and the strip that moves is bounded by the bank's own height. Where the land already slopes into the water its slope adds to the bank's, and only the bank's own grade is validated. **Confidence:** medium for rivers through relief, which no map has yet.

### `surface_z` is authored, and the land must stand at or above it

**Choice:** Each river has one surface height. A map is refused if the land at any sampled point of the water's edge is below it.

**Gap:** Delegated: per-river `surface_z`.

**Verdict:** sound. The geometry and movement labs use half a metre of freeboard (a 2 to 3 m bank) and the river lab 1.2 m (a 4.8 m bank). A river flowing downhill wants a surface per point; no map needs one yet. **Confidence:** medium.

### A bridge's ramp is the bank made steep

**Choice:** Along a bridge's approach (its deck, lengthened by the bank's reach and widened by a height sample each side) the cross-section takes the steepest grade the grid can draw under the slope cutoff (0.45 at 35°), bed and bank alike, and eases back to the river's own over at least three samples beside it. A bridge is refused when that bank cannot reach the land's height by the deck's end.

**Gap:** Delegated: ramp length. The slice asks for "a ramp at bridge ends".

**Alternatives:** A mound falling away from the deck's end stood in the water at the deck's corners and left a dip between the deck and the land. A steeper bank only above the waterline bent the ground at the water's edge, and the grid drew dry ground 0.4 m inside the water all along the abutment.

**Verdict:** sound: a deck 6 m past the water is stepped onto from the land's height (the 0.1 m every lab deck stands above its road), and the ground beside the ramp is a centimetre off the water's surface at worst. **Confidence:** high.

### The water is drawn as squares along the river, cut by distance

**Choice:** The water surface is a strip of 8 m squares at the river's surface height, one for every square the water or a 4 m margin touches. The fragment cuts the edge by the shared distance, feathered over a pixel.

**Gap:** The slice says "a ribbon along the centerline".

**Alternatives:** A ribbon of quads with mitred joins folds over itself on the inside of a bend tighter than the water is wide, and the blended surface is then drawn twice there. Squares never overlap.

**Verdict:** sound. **Confidence:** high.

### SG2 was answered here, and its second fallback is in

**Choice:** The bank and the bed are lit by a normal from the cross-section (`groundBank`), not by the triangles' own, out to a triangle past the bank's top. Nearby stretches share it by how far inside each puts the point.

**Gap:** SG2 was never run. Its kill check and fallbacks were to be applied in order.

**Evidence:** Lit by its own triangles a bank steps by up to 20.6% of the flat ground's luminance between neighbours (bar: 8%). Cutting the edge by distance and laying the wet band over it, both already in the frame, leave the steps on the bed through the water and on the bank past the band. With the shading normal the largest step walking a bank is 2.1%. Two things were tried and dropped on the way. Taking the nearest stretch alone left 16% at the inside of bends, where the bank's face turned at a stroke. Lighting the bed as the V it is showed through the water as one bright half and one dark, so the bed is lit flat.

**Verdict:** sound for "not stepped". It overlaps C71 (bank roundness), which now starts from a round bank. How soon the shading ends is a constant in the material, to become biome numbers when C70 gives banks a look. **Confidence:** medium until the specialist judges it beside bank art.

### The wet band's edge is plain

**Choice:** The wet, bare band beside the water ends a fixed distance from the water's edge (the biome's `shore.width_m`). Its ragged line is gone.

**Gap:** The slice says "there's no new look yet"; the band's ragged edge was the existing look, drawn for straight water rects.

**Evidence:** Two unprimed critiques called the band's outer edge scalloped or stepped, with high confidence, at the play camera and closer: the raggedness was value noise on a square lattice, which shows as lumps and straight runs beside a round river.

**Verdict:** sound as the plain drawing: a fourth critique, on the plain band, found no stepping on it. The band's art is C70's; fresh eyes also said the plain band reads as a painted outline. The geometry and movement labs' bands lose their ragged line too. **Confidence:** high for not stepped.

### The lab's meander is authored every 4 m

**Choice:** `river-lab.json`'s meander is 127 points along a smooth curve, none turning the river by more than 8°.

**Gap:** The slice asks for "a meander". C65's curve rounds a third of each run at a corner, so a river authored as a dozen long runs is straight for a third of every run.

**Evidence:** Authored as twelve 50 m runs it read as a chain of straights from above. Unprimed critiques named the corners in the waterline of a 9 m version, and in the wet band of a 5 m version whose points were rounded to half a metre.

**Verdict:** sound for the lab, and it says what a generator has to write: a river is a dense line, and the loader only takes the corners off. **Confidence:** medium; a curve family that rounds a whole run would let rivers be authored sparsely, and is C65's to decide.

### Fields are cut along a river's long runs, not every authored run

**Choice:** `river_runs()` joins authored points, leaving out those within half the river's narrowest water of the run between their neighbours.

**Gap:** The slice says "plots cut along river control runs". A meander authored point by point has a run every 4 m.

**Evidence:** Cut along every run, the lab's fields fanned into slivers at each bend.

**Verdict:** sound: the cuts stay under the water. **Confidence:** medium; the plot cutter may want its own rule for curves.

### Squads at a bridge: found, measured, left to movement

**Choice:** Not changed here. The river scenarios carry the failing checks as pending on SA2.

**Gap:** The slice asks for a squad and a tank to route over the bridge and never enter the water. They do both. But a soldier whose next step is onto ground he cannot enter does not sidestep; he stays. A squad's files spread wider than a 10 m deck, and a route that meets the deck at an angle runs along the deck's edge, so some soldiers stop at the water beside the deck and never arrive.

**Evidence:** The same scenario on the base commit's water rects strands the same four soldiers (`throwaway/c69/base-bridge/`). Straight up the road every soldier crosses in the scenario table (34 reversals at the deck's edges), and six of eight in the lab's scene, where a tank leads them over.

**Verdict:** open, and it will be met on the first generated map with a river. It needs soldiers to slide along ground they cannot enter as they do along bodies, or routes that keep a squad's width from a deck's edge. **Confidence:** high that it is not the river's contract.

### The lab is registered the way the others are

**Choice:** `river` is an entry in `apps/battle-lab/src/fixtures.json` with its own route, scene and `fixtures/river-lab.json`.

**Gap:** The slice depends on C60's catalogue, which has not landed; the dependency was relaxed.

**Verdict:** sound; it moves with the other labs when C60 does. **Confidence:** high.

## Integrating SA2 with C69

### A straight segment may clip the corner of any cell a mover could stand in

**Choice:** When a route segment passes more than half a cell from a cell's middle, it only clips that cell's corner, and the segment check asks whether a mover could stand in the cell at all. That is the rule the search already holds a diagonal step to. Closer than half a cell, the segment still loses that much of the cell's room, as before.

**Gap:** SA2 made the straight-segment check stricter than the search's diagonal rule. Along a curved river bank the search then gave a tank a route its own check refused (the river lab's long way round), so the unit would plan the same route again; the same thing explains SA2's "jeep among trees" note.

**Verdict:** sound: the search can no longer emit a step the route check refuses. Tightening the search to the stricter rule instead was tried and rejected: followed through, it needs almost 4 m of clearance and would keep tanks out of ordinary 7 m streets. The two navigation tests that judged a route point by point with the off-centre standing rule now judge it by the engine's route check plus "every cell on the way can be stood in". **Confidence:** medium; vehicles among trees should be re-measured.

## Navigation grid updates

**When:** the grid-update pass after SA2, 2026-10-01. The brief fixed the goals (local updates, one shared base, digests unchanged); these are the decisions it left open. Numbers are in [SA2's outcome](slices/SA2-counted-route-integration.md#grid-updates).

### The shared base is the map as authored, bodies included

**Choice:** Both sides start from one planning picture, built once: the terrain, and every body the map was authored with (trees, buildings, walls, authored wrecks), standing where the map put it. A side then keeps only what it has come to know differently: a body it has learned of, one it has seen moved, one it has seen go.

**Gap:** SA2 said public terrain is shared and "never includes secret side-specific bodies", but did not say whether authored bodies are public. In the battle they already are: every side plans with every authored body from the first tick, in view or not.

**Verdict:** Sound. Nothing about who knows what has changed. A side still plans round a tree it has not seen fall, and still knows nothing of a wreck it has not seen. A battle test holds both sides' pictures to that at every twentieth tick, with one side watching the changes and the other two kilometres off. **Confidence:** High.

### A change is taken in at once, not spread over ticks

**Choice:** When a side learns something, its picture is brought up to date the next time one of its units reads it, in full, before the unit plans. Only the cells under the bodies that changed are worked out again.

**Gap:** The brief allowed a burst to be spread over ticks, with units planning on the old picture meanwhile.

**Verdict:** Not needed. Fifty trees felled in one tick are about 1,200 cells and about 1.7 million instructions, a twentieth of what a full tick of planning costs; there is no window in which a unit plans on a stale picture. **Confidence:** High.

### Planning work is counted as before

**Choice:** A search still pays for its first read of each 64 m clearance tile after its side learns anything, although the tile is no longer worked out again unless a body near it changed.

**Gap:** Before, the whole picture was rebuilt on every change, so every tile really was worked out again and the charge was honest. Now it is a charge for work mostly not done.

**Verdict:** Kept, so that every route arrives on the tick it did and no battle digest moves. Charging only for tiles really worked out would shorten the hold after a side learns something and is a small named digest change for later. **Confidence:** Medium.

### Cleared forest ground reaches a side's picture when the side next learns of a body

**Choice:** Left as it was. Ground a tank or a shell has cleared stops counting as forest (the slower speed) in a side's picture at that side's next change of knowledge, whoever cleared it and whether or not the side saw it.

**Gap:** The old rebuild read the true cleared ground each time; nobody decided that. It is a small leak (speed only: what a side can pass is never affected) and a delay for the side that did the clearing.

**Verdict:** Kept, because changing it moves digests. The right rule is the one the rest of the battle uses: a side's picture of the ground is the ground it has seen. **Confidence:** Low. The user should decide.

### The world keeps two short lists of what changed

**Choice:** The world records the bodies it added, moved, removed or made known to all since the battle last asked, and every ground cell cleared, in order. A side's picture reads both; nothing else tells it which bodies to look at again.

**Gap:** SA2's outcome said cleared ground "needs a change feed first".

**Verdict:** Sound. The first list is emptied every tick; the second grows by one entry per cleared square metre and is the cleared ground itself in another order. **Confidence:** High.

### Each unit still checks its whole route when its side learns something

**Choice:** Left as it was, and made cheaper: a route is read a cell at a time instead of a sample at a time.

**Gap:** With the rebuild gone, this is what a change of knowledge costs: every unit with a route walks all of it to see whether the new body is in the way, 3 to 4 million instructions for a 9 km route. A hundred units a side would be a slow tick. Checking only the routes that pass near the change gives the same answers, but it changes which clearance tiles later searches pay for (see "Planning work is counted as before"), so routes arrive a tick earlier or later and digests move.

**Verdict:** Open. It is the next cost to remove, together with the check of a finished plan against newer knowledge, which reads a whole route in one uncounted step (a tick's planning reached 15,029 against an allowance of 4,000 on the 10 km probe, before and after this pass). Both need the counting decision above. A trial of the first (not committed; the patch is `throwaway/navgrid/route-recheck-near-change.patch`): a unit checks its route only when a change lies within 24 m of it. All six village quick digests moved, with like outcomes (the flank 2 of 3 captured both ways, 938 lost against 950). On Metro Large the crossing's planning work fell from 7.1 M to 2.0 M, because today a vehicle whose route shoves anything plans the whole route again whenever its side learns anything anywhere. **Confidence:** High that it is the next cost; medium on the fix.

### Surface lookups pass by a primitive on its bounds

**Choice:** A surface lookup skips a road segment or a polygon whose bounds the point lies outside by more than a thousandth of a millimetre.

**Gap:** Found by measurement: over half of the picture's build on a generated town was containment tests against every primitive of a 128 m bucket.

**Verdict:** Sound: the margin is far above rounding, so the answer is the same. It is in its own commit because it touches a file the rivers work is also changing. **Confidence:** High.

### A unit re-checks its route only when its side learns something near it

**Choice:** When a side learns of a change, a unit re-checks its remaining route only if the change lies within 24 m of that route (the widest clearance a footprint is judged by, and a cell more). If the side's change log no longer reaches back to the unit's plan, it re-checks as before.

**Gap:** After the grid-update pass, every unit still re-checked its whole route whenever its side learned anything anywhere: 3–4 M instructions per unit on a long route, and a vehicle whose route shoves anything re-planned each time. The pass trialled this change and left it uncommitted because it moves digests (routes are the same; later searches read different clearance tiles on different ticks).

**Verdict:** sound; a named digest change. On Metro Large (10 km, 9,276 buildings), twelve units crossing with wrecks and shelled trees appearing: planning work 7.1 M → 2.0 M, slowest tick 9 ms, none over 33 ms. Quick village report: flank 2/3 captured with 938 lost (950 before), ambush 0/3 with 0 lost. **Confidence:** high.

## Generated map in the lab

### The map and its plan come from two generator calls

**Choice:** The worker calls `generate_map` for the map and `generate_map_plan` for the plan, with the same request. The scenario carries the generator's own map text, spliced out of the outcome, never a re-serialised parse.

**Gap:** The encounter stands on the plan's settlements, districts and roads, which the compiled map does not carry, and `generate_map` returns no plan. A parse and re-serialise in JavaScript turns `-0.0` into `0`, so the scenario's map would no longer be the generator's.

**Verdict:** Sound for now. The second call costs 0.7 s on Metro Large (measured in the worker). A single export returning both would halve it; it waits for C59, which decides what the planner reads. **Confidence:** Medium.

### The developer encounter is written in TypeScript and reads the plan

**Choice:** `developerEncounter` puts blue in a column on the country road that enters the map nearest the main settlement, its tail 150 m in from the edge and 30 m between units, a jeep leading. Red's six units stand outside an entrance of the building nearest each district's anchor of the main settlement, in plan order; the three rifle squads are ordered into those buildings by the village's defender policy. The hold zone is 150 m about the settlement's centre. Hold time, the defender's thresholds and every rule are the village's.

**Gap:** C58 wants an authored encounter on one saved map and C59 a recipe planner in Rust; neither exists, and the task asked for a simple deterministic stand-in.

**Verdict:** A stand-in, and labelled as one. It checks nothing about range, cover or fairness, and red's vehicles stand 10 m out from a door without asking whether that is road. It is deleted when C59 lands. **Confidence:** High that it is the right size; the numbers are guesses.

### The page still builds its own static world

**Choice:** The page builds a `WorldView` from the scenario's map for drawing and picking, as every other route does.

**Gap:** C33 forbids a second world on the main thread and has not been built.

**Verdict:** Kept. On Metro Large it is part of the 1.4 s the page spends between the worker's answer and its meshes, and of the page's 550 MiB ([S3](spikes/S3.md)). C33 is still the owner of removing it. **Confidence:** High.

### A building with no art is massing, by its catalogue's own label

**Choice:** `presentation.massing` in `game.json` lists the regional families drawn as massing (`prototype`) and a tint for each building category. Every physical part of such a building is one plain box at the simulation's size. A part the side has seen fall is drawn as its remains' box in the ruin tint, or not at all when nothing is left. No model stands for either, and the village's house art is no longer loaded for a map that does not use it.

**Gap:** The handoff allows "labelled massing" for developer checkpoints and C13 says prototype rows carry their status; nothing said how a renderer tells them apart or what a fallen one looks like. Before this, the village's house was stretched over every footprint, a 72 m warehouse included.

**Verdict:** Sound. The label is the catalogue's, so real art replaces massing by publishing templates of another family, with no code change. **Confidence:** High.

### Massing draws through the scenery layer's chunk path, with a scale per axis

**Choice:** The static chunk path's instance record became pose, a scale per axis, a tint and a seed (it was one horizontal scale). Trees fill both horizontal scales alike; a massing box is a unit box scaled to its part. Massing passes tier thresholds of infinity, so every box draws from the static buffer at any distance.

**Gap:** C22 says the chunk owner is "promoted from the scenery layer's existing chunk path" and leaves the record's shape to the implementer.

**Verdict:** Sound. One instance layout, one vertex stage, one set of buffers and draw ranges; the massing material is a fragment of its own. Corpse chunks have not moved onto it (C22's other half). **Confidence:** High.

### A tree is drawn into a cascade only where its shadow can land in view

**Choice:** Every population is culled to the camera's side planes. A chunk out of view is still drawn into the sun's cascades when its box, swept along the fall of its tallest instance's shadow, meets the view within the shadows' reach; it draws at the coarsest tier.

**Gap:** The forest was never culled, "since its trees cast shadows into view from off screen". On a 10 km map that drew 6.5 million triangles into each of six passes from any camera.

**Verdict:** Sound, and it changes the village's frame too: off-screen trees whose shadows cannot reach the view no longer draw. The village scene pinned the old rule ("every forest tree is drawn at some tier in every framing"); that check now asserts the new one: the trees in view are drawn, every drawn tree casts, nearly all of both forests draw up the road and one forest alone at its edge. **Confidence:** High.

### The grass build and the plot split read bucketed tables

**Choice:** The grass build asks only the props listed for a clump's grid cell whether one covers it. The plot split keeps, for each plot, only the roads whose box meets its own, and asks a grid whether a building stands near.

**Gap:** Both loops were over every prop or every road on the map, which the village never noticed.

**Verdict:** Sound. The plot tree is byte-identical on the village and on three generated maps (hashed before and after), and the prop table gives the answer a walk over every prop gives (tested on 20,000 points). **Confidence:** High.

### Past 2 km of orbit, the haze stretches with the camera

**Choice:** `haze.overview_from_m` (2,000): beyond that orbit distance, the distances the haze is computed from are divided by how far past it the camera is. The lab's camera for a generated map reaches out to 1.5 map widths and tilts to 1.3 rad at that distance.

**Gap:** The camera stopped at 2 km and the haze was tuned for it; a whole-map view of a 6 km map was white.

**Verdict:** A guess to tune. The village never orbits past 2 km, so its frames are unchanged. **Confidence:** Medium: the overview reads, but nobody has judged its look.

### A URL that names no map is refused

**Choice:** `type`, `size` and `seed` default to mixed, small and 1 when absent. A value that is not one of the three types or sizes, or not a whole number that fits a u64, shows a refusal naming the parameter; nothing is generated.

**Gap:** The task gave defaults and did not say what a wrong value does.

**Verdict:** Sound: a silent default would start a battle on a map the URL did not ask for. **Confidence:** High.

### The battle's own seed is the village's

**Choice:** The battle's random seed is `village.seed`, whatever the map's seed.

**Gap:** C55 keeps the two seeds apart and gives no player control for the second.

**Verdict:** Sound for a lab route. **Confidence:** High.

### Massing tints stay off the fields' palette, and each box has its own value

**Choice:** Brick for attached homes, off-white for detached, greys for apartments and industry, blue-grey for highrises, taupe for farmsteads. Each box's value strays up to 8% from its tint, seeded by its prop id, so it is the same standing, fallen and in the next battle.

**Gap:** Massing only had to be "tinted by category". The first tints put an orange on terraces that the straw fields also wear, and one flat colour a category made a row of terraces one slab.

**Verdict:** A readability fix after an unprimed critique, not art: the numbers are guesses in `presentation.massing`. **Confidence:** Medium.

## Rivers in the layout generator

The layout generator now writes a river and its bridges. The seam and the measurements are in the [C52 outcome](slices/C52-procedural-generator.md#rivers).

### A river map has one river, and it runs from the north edge to the south

**Choice:** A map has no river or one. The river enters at the north edge and leaves at the south, so it crosses the line between the two halves and each half holds a like length of it (within 20%, or 5% of the whole, and the generator aims for half that).

**Gap:** The task said "zero or one river", "edge to edge" and "fair". It did not say which edges. A river from west to east would lie in one half, or along the line between them, and one side would have a water obstacle the other has not.

**Verdict:** sound for fairness, narrow for variety: every river map is a valley across the front, which both sides must cross or fight along. **Confidence:** medium. A river along the front line (west to east through the centre) is a different kind of map and wants its own decision.

### How often: half of Open and Mixed maps, four in ten Metro maps

**Choice:** `river_chance` is 0.5 for Open and Mixed and 0.4 for Metro. The seed decides, on a random stream of its own, so a seed that draws no river is byte for byte the map it was before rivers existed.

**Gap:** "A seed-chosen share of maps per type" with no numbers.

**Verdict:** a guess. Nothing was measured that favours these shares over others; they are one number per type in the presets. **Confidence:** low. The user should set them.

### The river is drawn straight after the main settlement, before anything else

**Choice:** The order is: main settlement, river, the two approaches, the other settlements, roads, forests. A course is taken only if it clears the main settlement and its junctions, keeps 500 m from the east and west edges, and is fair. If no course fits in 150 tries the main settlement is drawn again, and the map is refused with the diagnostic `river` when that runs out too.

**Gap:** The task put the river "after sites and before roads" in spirit (settlements respect it) but did not say what happens when the ground is already full.

**Verdict:** sound. The first version drew the river after every settlement and approach, and found no course on 38% of Mixed Small and 68% of Metro Small seeds. Drawn second, nothing is refused in 2,700 layouts. The cost is that a river never passes through a settlement (next entry). **Confidence:** high.

### No town stands on the river

**Choice:** Settlements keep 40 m from the water and the main settlement more. A share of the others (`siting.beside_river`: 0.2 on Open, 0.1 on Mixed and Metro) is placed along a bank on purpose. None is split by the river and no street runs to the water.

**Gap:** The task said the river is "not through the centre settlement unless presets allow". Presets do not allow it for any settlement, because a town on both banks needs streets that cross by bridges, blocks cut along a bank, and a waterfront, none of which the parcel pass does.

**Verdict:** the largest thing missing from river maps. A riverside town here is a town with a field between it and the water. **Confidence:** high that it is missing; it is a pass of its own in the parcel code.

### Water is not open ground for the approach rule

**Choice:** The 1,800 m approach to the main settlement stops at water, as it stops at a wood or another settlement. The generator picks the two approaches where no river lies in them.

**Gap:** The task asked for this to be decided and recorded.

**Verdict:** sound. The rule exists so a force can advance on the main settlement over open ground; nothing advances over water, and a bridge is a defile, the opposite of an open approach. Every river map still has an approach in each half. River maps count fewer approaches in all (see the table in the outcome), because the river ends the ones that would have run across it. **Confidence:** high.

### A road keeps to its bank and crosses once

**Choice:** A road whose ends are on the same bank never touches the water: where its line would cross and come back, it follows the bank 22 m from the water instead. A road whose ends are on opposite banks crosses once. It takes a bridge already built if one lies within 600 m of its line (1,200 m for a dirt track); otherwise it gets its own, on the road's own line when that is within 45° of square to the river, and square across the nearest straight stretch when it is not. When the generator chooses which settlement to join a new one to, one on the far bank counts as 800 m farther away.

**Gap:** The task asked for "few, well-placed crossings" and for minor tracks to be led to existing bridges, without distances.

**Verdict:** works, and looks mechanical in places. River maps have a median of two or three bridges. Two things look wrong: a road led to a bridge it did not aim for turns sharply at both ends of the deck, and a main road beside the river follows its bends for a long way. The four distances are preset numbers. **Confidence:** medium.

### A bridge is 14 m wide and ends 9 m past the water

**Choice:** One deck type (`bridge_deck`), 14 m wide, 0.8 m thick, its top 10 cm above the land, spanning at most 90 m. Each end lies 9 m past the water's edge and the road runs straight for 12 m beyond that.

**Gap:** The contract says a deck's ends must be over land that can be stood on. It does not say how far.

**Verdict:** the 9 m is measured, not chosen. With 6 m the contract accepted the bridge, but the simulation's ground, which is drawn on a 2 m grid, still stood 12 cm below the deck at one corner, more than a unit steps up. At 9 m the step is the deck's own 10 cm at every corner of every bridge tested. The presets now refuse a landing shorter than the bank's fall plus one grid diagonal. **Confidence:** high.

### A river's course must leave the main roads in time, and a road is timed along its rounded line

**Choice:** Before a course is taken, the generator lays a trial main road from each edge to the centre across it and refuses the course if any takes longer than the transit limit (215 s). When the real roads are built, each candidate is timed along the rounded line the road surface is made on, not along the straight runs between its points.

**Gap:** The transit rule was written for roads that run nearly straight. A bridge can put two corners in a main road, and a rounded corner is longer than the two straight runs it joins.

**Verdict:** sound. Without the trial road, 3 of 2,700 river layouts were refused for transit. Timing along straight runs then let one more through the generator and into a refusal: Mixed Medium seed 115, whose east road was 23 cm longer rounded than authored and 3 cm over the limit. Both are now tests. Maps without a river did not move: all are the maps they were. **Confidence:** high.

### The river is written as many close points, and a road beside it as few

**Choice:** A river's points lie at most 16 m apart and turn at most 8° each, so the contract's rounding of corners cannot pull the water away from where the generator measured it. A road that follows the bank is written with only the points it needs to stay within a couple of metres of that line.

**Gap:** None in the task; this is what the shared curve code needs. It rounds every corner, and takes more room the sharper the corner.

**Verdict:** sound, with a cost: a river adds 4,000 to 7,500 ground points to a map (a river-free Open Small has about 1,800 in all). They count toward `max_ground_points`. **Confidence:** medium. Fewer, longer river runs would work if the contract's river rounding were given a tighter tolerance, which is a contract change.

### Streets do not cross water

**Choice:** In the parcel pass, a street is joined to the nearest street it can reach without touching the water or its bank. A filled plan with a street in the water is refused.

**Gap:** Found by test. On Metro Medium and Metro Large seed 11 a town near the river joined its streets to a road on the far bank, straight through the water.

**Verdict:** sound for now, and it is the other half of "no town stands on the river". **Confidence:** high.

### Bridges are not shared evenly between the two halves

**Choice:** `measure` reports the bridges in the top half and the bottom half. No rule holds them equal.

**Gap:** The fairness rule covers town, forest and river length. The task did not name bridges.

**Verdict:** open, and it matters for play. Of 2,700 river maps, 1,098 have a half with no bridge of its own (649 of them have one bridge in all), and in 620 the halves differ by two bridges or more. A side whose half has the only bridge holds the only crossing. **Confidence:** low. The user should decide whether to hold it to a rule (at least one bridge in each half is the simplest); the generator would then add a crossing or refuse.

### Long moves on river maps can fail to plan in the simulation, and columns can jam

**Choice:** Left as it is. The generator's maps are valid and connected, and the simulation's road graph reaches every settlement over the bridges; this is a limit of the simulation's route search, which this work may not change.

**Gap:** Found by running `city_report` on generated river maps.

**Verdict:** open, in two parts. Planning: six units a side were sent across six river maps for 300 s. On the 8 km and 10 km maps two units were refused a route and two were still planning at the end, and three ticks on one map retired about 1.0 G instructions each; none of that happened on the same seeds without a river. Jamming: given 900 s, seven of eight vehicles crossed Mixed Small, but on Open Small only three of eight did, because four vehicles of one side stood waiting behind one of the other side on a road 700 m from the water. Without the river seven of eight arrived. A river sends both sides down the few roads that lead to a bridge. **Confidence:** high that both are the simulation and not the map: units do cross by the decks, a test proves each deck can be walked onto and across, and the jam was on dry ground. Low on how often the jam happens: it was one map of two, and an earlier run of the same seed had six of eight arrive.

## C57 camera clearance

What was built, the tuning values and the measurements are in the [C57 outcome](slices/C57-camera-clearance.md#outcome).

### The eye moves; the point looked at never does

**Choice:** Clearance changes only where the camera's eye is. The drawn pose always looks at the target the player or the script asked for; its distance, pitch and yaw are whatever looks at that target from the clear eye.

**Gap:** The slice says "nearby pose" and "pushback" without saying which part of the pose may give.

**Verdict:** Sound. Panning, picking and the screen's centre keep meaning what the player set, and the drawn pose is still an ordinary orbit pose for the one projection owner. The cost is that a held camera's view turns to follow its target, and its distance and pitch can leave the wheel's limits while it is held. **Confidence:** High.

### Pushback pushes the eye out of the building, on the side it is on

**Choice:** When no lift or slide is clear, the eye is pushed out of the box it is in through one of its four sides or its roof: the one nearest where the camera already is, so it slides along a wall and over a roof edge. A camera placed by a cut has no side yet and comes out on the side its target is on, so it sees it.

**Gap:** "Smoothly pushes to a safe pose" does not say along what. The first version pushed along the view ray, toward the target or away from it.

**Verdict:** Sound, chosen on the lab's trajectories. Along the ray, an eye that grazes a tower sideways is centimetres from clear space and a tower's depth from it along the ray: the eye jumped 17 m in one frame on the tower pass and 58 m in the corner. Pushed out through the nearest side, the same trajectories never move the eye more than 25 m/s faster than it was asked to move. **Confidence:** High.

### A building in the way is gone over if a lift could reach, and cut past if not

**Choice:** A camera held at a wall, whose goal is on the other side, rises over the building when its roof is within the lift's reach (16 m) above the eye, and otherwise cuts straight to the goal. The cut is reported (`ClearanceState.cut`), and a lift or slide is only taken if the camera can get to it the same way.

**Gap:** The slice asks for no unsafe pose and for smooth recovery; it does not say what happens when the only way to the pose asked for is through a tower.

**Verdict:** Sound, with a visible cost. In the lab, the only cuts are one past the 60 m tower on each of the two trajectories that send the eye through it, and the placement trajectory's own five. Until it cuts, the camera waits at the tower's wall for 1.0 to 1.5 s with the wall filling much of the view. Going round the back was tried by accident (the ray version did it) and whipped the eye 16 m in 0.15 s. **Confidence:** Medium. A player who dislikes the wait would want the cut sooner, or a slide wide enough to pass a tower (it cannot be: at the closest zoom the eye's orbit is 29 m wide and the tower 32 m).

### The camera looks 0.75 s ahead along its own motion

**Choice:** The policy tests each candidate pose where the camera is and along the straight line to where the same motion puts it 0.75 s later, for buildings only. A lift therefore starts before the wall. It applies to motion with a rate: held keys, the screen edge and scripts. A wheel notch or a drag has none and is not looked ahead for.

**Gap:** Not in the slice. Without it the eye reaches a wall, stops, and climbs it.

**Verdict:** Sound, bounded by the benchmark. With no lookahead the eye is pressed against the lab's wall for 0.70 s (and against the courtyard block for 1.4 s); with 0.5 s for 0.35 s; with 0.75 s for 0.27 s; with 1.25 s for 0.05 s. But a straight-line guess overshoots an eased scripted swoop: from 1.0 s the benchmark tour's approach to the village at 51 s of its 60 predicts an eye in a house it never reaches, and the tour is lifted off its keyframes. 0.9 s leaves the tour untouched; 0.75 s was chosen for the room. Looking ahead for the ground was dropped for the same reason: it put the tour's swoop at 31 s under the terrain. **Confidence:** Medium on the number.

### A goal keeps the release margin more than a drawn pose needs

**Choice:** A drawn pose keeps 1.82 m from every building (the near plane's 1.32 m envelope plus `margin_m`); a goal the camera heads for keeps `release_m` (0.75 m) more, and the camera is released back to the pose asked for only when that pose is clear by the same.

**Gap:** The slice names hysteresis and leaves its form open.

**Verdict:** Sound. One number does two jobs: a pose grazing a roof line no longer switches the adjustment on and off (the test's wobbling pan switches once with it and more than twenty times without), and the eye, which closes on its goal like a spring, reaches clear space in finite time instead of creeping up to a roof edge for ever. **Confidence:** High.

### Buildings and the ground block the camera, as the side draws them

**Choice:** The obstacles are the ground and every part of every building on the map, as the side knows it: standing where it has not seen it fall, its remains' box where it has, nothing where it saw nothing left. They are the same list massing draws. Trees, walls, fences, wrecks and units do not block. A box is grown by the clearance on each axis, so corners keep a little more than faces.

**Gap:** The slice says "public authored building/terrain geometry plus changes learned by the viewing side".

**Verdict:** Sound. A building with art and a building drawn as massing block alike, and what blocks is what is drawn. A ruin blocks as a 2 to 6 m box, which the eye rises over. **Confidence:** High.

### The harness's raw framing keeps the name `__lab.setCamera`

**Choice:** `window.__lab.setCamera` draws exactly the camera it is given, through neither the rig's limits nor clearance; its doc says so. `placeCamera` is the scripted path through both. No playable or benchmark path calls `setCamera`.

**Gap:** The slice allows a raw framing API if it is "explicitly named".

**Verdict:** Kept under its old name rather than renamed. About a hundred scene calls use it, in files two other sessions are editing; a rename would break their branches at run time with no type error. **Confidence:** Medium. Renaming it `setRawCamera` in a quiet moment is mechanical.

### The lab has no battle; its tower falls by a switch

**Choice:** `/lab/camera` draws a compiled map of prototype buildings with no simulation. "Blue has seen the tower fall" hands the same known-prop list to massing and to the camera's obstacles.

**Gap:** The slice asks for hidden versus observed destruction to respect knowledge.

**Verdict:** Enough for the lab. The boundary itself is tested where it lives, in the list both read (`knownStanding`): an unseen fall is no entry, so nothing changes. No scene shells a building and watches the camera through a real battle. **Confidence:** Medium.

## Town model and edge roads

The layout generator's roads and settlements were reworked for M22 (a road from the bottom edge to the top) and M23 (a town is not a dartboard). The model as it stands, the seam and the measurements are in the [C52](slices/C52-procedural-generator.md#outcome) and [C53](slices/C53-parcels-and-buildings.md#outcome) outcomes and the [crate guide](../../crates/mapgen/README.md). Generator `layout-4`, presets `layout-presets-6`. Every share below was measured over 100 seeds for each type and size, 900 maps through the parcel pass and the compiler, none refused.

### Only the top and bottom edges are held to a road; a road across the middle is drawn on half of maps

**Choice:** A road runs from the bottom edge and one from the top to the main junction by the map's centre, each inside `transit.max_s` (215 s), and so from one edge to the other. `roads.cross_road_chance` (0.5) draws a road from the left edge to the right through the junction. On the other maps `roads.side_road_chance` (0.6) draws one road in from the left or the right, leaving its edge anywhere in the middle 70%; the rest have no side road. The extra edge roads a rich network adds still come in from any edge.

**Gap:** M22 says a side-to-side road is optional and "not every map", without a share, and says left and right exits vary.

**Verdict:** sound. 40–63% of maps in a cell have a road from the middle of the left edge to the middle of the right, and 18–30% have four roads meeting at the centre, Large included. **Confidence:** medium. The two chances are guesses; each is one number.

### Two main roads that meet at the main junction from opposite edges are one straight line through it

**Choice:** When the bottom and top roads both end at the main junction, the second leaves its edge where the first's line through the junction arrives. The same holds for the left and right roads. A road that forks onto another before the junction is free.

**Gap:** None in the brief. Two roads that met at the junction a few degrees off straight left a sliver of ground between one road and the line of the other, which no block can be cut from.

**Verdict:** sound, and it costs some variety: a through road cannot dog-leg at the junction itself. It can still fork onto the cross road up to 1.5 km away, which is the staggered crossing M22 describes. **Confidence:** high.

### The main settlement stands on the main junction

**Choice:** The junction is drawn up to 450 m east or west of the exact centre and up to 80 m north or south (`roads.hub_offset_m`), and the main settlement is centred on it. `measure` times the top and bottom journeys to the junction, which must lie within `transit.centre_reach_m` (500 m) of the exact centre.

**Gap:** C52 offset the main settlement from the centre and left the junction at the centre, so the main roads crossed the town off to one side. A town that grows from its roads has to have them through its middle.

**Verdict:** sound for the town, and it fixes the template: every map has its largest settlement in the middle with a road through it from top to bottom. M22 asks for that road; nothing asks for the settlement to sit on it. **Confidence:** high on the rule, low on whether every map should look like this.

### A settlement's ground is cut along its roads into blocks, and a block is a district

**Choice:** A site is the ground a settlement may build on: a convex shape with a few straight sides. Once the roads are laid, that ground is cut along every road that crosses it, each piece is cut into a row a block deep behind the road it fronts (`classes.<class>.block.depth_m`), and each row across into blocks (`block.length_m`). A cut between blocks turns up to 8° off square (`towns.block_skew_deg`). Each block is one district of one use. A hamlet's blocks are 65–85 m deep, so it is one row of lots each side of its lane; a city's are 220–380 m deep.

**Gap:** M23 asks for irregular blocks laid out along the town's roads. It does not say how they are cut.

**Verdict:** sound. Every block is a convex polygon bounded by roads, streets and the settlement's edge, so no two overlap and each has a middle to anchor on. A town on one straight road is still a patchwork on that road's axis: 8° is enough to stop it reading as one grid and not enough to hide the axis. **Confidence:** medium on the look; high on the geometry.

### A road crosses a settlement's ground in a straight line and turns outside it

**Choice:** A main road that runs through a settlement enters by a gate 30 m past its limit (`roads.gate_margin_m`), runs straight through its centre and leaves by another. A settlement that no main road runs through gets a main street along the long axis of its ground, and the road that joins it to the network leaves by the end of that street, unless it would have to turn more than 50° there (`roads.turn_max_deg`): then the main street runs straight toward the network instead. A road that only passes over a settlement's ground is straightened across it.

**Gap:** None in the brief. A road that bends inside a town cannot be a block's edge.

**Verdict:** sound. A road is still ruler-straight inside a settlement and bends at its edge, which shows on a zoom. **Confidence:** medium.

### A road ends on another only where the two make a plain junction

**Choice:** A main road runs through a settlement only if no other already does. A road that ends at a settlement, or joins the network, ends on a point where another road passes, coming in at 50° or more to it, or on a road's end that it carries on from with a turn of 50° or less (the same `roads.turn_max_deg`). It never ends where three roads already meet, nor at the map's edge. Where no such point is in reach it takes the nearest point with fewer than three roads. A road whose two ends would lie on one road is not laid, and an extra road in from an edge is laid only to a settlement that lies ahead of it, within 50° of square to that edge.

**Gap:** The user's picture had every road converge on one centre point. An unprimed review then found hairpins at settlements' ends, roads running beside each other and a road along the map's edge.

**Verdict:** sound. No more than a crossroads forms anywhere; a test holds that from a settlement's centre district edges run out along four bearings at most. Near-parallel pairs and acute joins still occur where the fallback is taken and where a track is led to a bridge. **Confidence:** medium.

### A settlement grows outward from where its roads meet, along its roads first, farther one way than another

**Choice:** The blocks about the point where a settlement's roads come nearest the middle of its ground are built first. After that the nearest block is built next, where "nearest" is the distance from that centre, drawn up to 60% longer or shorter for each patch of ground three blocks across (`towns.growth_noise`, `growth_patch_blocks`), plus the distance from the nearest road times `classes.<class>.ribbon`. Growth stops at the class's `built_share` of the ground: 35–50% for a hamlet, 58–80% for a town or city. A block is built only if a road or the edge of a block already built leads to it, and never so that it touches the built ground at a corner only. What is left open is field.

**Gap:** M23 asks for a ragged outline with fields and woods pushing in between districts, and larger towns looser than small ones.

**Verdict:** sound. The outline is whatever the built blocks make, with lobes and bays. A city is still a compact mass about its crossroads. **Confidence:** medium; `ribbon`, the noise and the shares are guesses.

### Blocks take their use a few at a time, and zones are counted in the town's own distance

**Choice:** A class's `zones` say what its blocks are from the centre out, by share of built ground: a city's first 6% is core, its next quarter an even mix of apartments, terraces and suburb, the rest mostly suburb. Blocks are counted outward in the same stretched distance the settlement grew by, so a zone reaches out where the town did. A block draws its kind with a few neighbours (`classes.<class>.neighbourhood`, 4–8 blocks in a city), so uses come in patches. A block at the settlement's edge on a country road draws from the zone's `roadside` weights, which is where industry is.

**Gap:** M23 asks for one dominant category per district, a centre of a few blocks where the main streets meet, industry on a road at the edge, and a Metro city with a clear core.

**Verdict:** provisional. Unprimed reviews of the pictures first found rings (a core, a band of apartments, a rim of suburb), which mixing the middle zone and counting in the town's own distance removed. The last still calls a city a disc with a core bullseye and patches of other uses round it. A core in the middle is what M23 asks of Metro, so part of that is the brief. Uses are now patchy more than graded: apartments can stand on the outer edge with suburb nearer in. Industry is thin: many cities and large towns draw none. **Confidence:** low on the weights.

### A settlement's outline is the edge of its districts

**Choice:** `SettlementPlan.outline` is the boundary of the built blocks and of any open ground they enclose. It is a simple ring, and a ray from the centre may cross it more than once. `SettlementPlan.center` is the point of the settlement's roads nearest the middle of its ground, and `districts` are listed nearest the centre first.

**Gap:** C52's outline was an envelope drawn before the districts; M23 says the outline follows the districts.

**Verdict:** sound. A class's `area_ha` is the ground a settlement may build on, and the outline is smaller than it. Mixed's main town is at least half as large again as the next settlement, in outline and in built ground (it was twice on the envelopes; the sizes allow 1.55). **Confidence:** high.

### Streets run between blocks; the edge of town is a street only where a block needs one

**Choice:** The layout writes an avenue (`towns.avenue_width_m`, 10 m; a dirt lane in a hamlet) along every block edge shared by two built blocks that no road already covers. An edge that faces open ground gets one only if a street between blocks can be reached no other way, or the block would otherwise have no frontage wide enough for a parcel. A street that would end in the fields is dropped where nothing needs it.

**Gap:** M23 says districts are bounded by roads and streets. Ringing every block with a street put a paved road round the whole town.

**Verdict:** sound. Avenues are surfaces of kind `road` in the plan, so `measure` counts them as streets and the parcel pass builds along them. **Confidence:** medium.

### A district kind says how much ground it needs, and a block that cannot hold it is left open

**Choice:** `districts.<kind>.ground_m` is the rectangle of ground, against a street, that the kind's largest parcel takes. A block is built only if one of its class's kinds fits on a stretch of its edge that carries a road or an avenue, and it takes a kind that fits. A block with a corner sharper than 50° is left open (`towns.corner_min_deg`). In the parcel pass a district's first parcel is always built on; `coverage` applies to the rest.

**Gap:** C53 refuses a request when a district takes no parcel. Districts used to be large enough that this never happened; a block can be small or a triangle.

**Verdict:** sound: no request in the sweep was refused. The numbers must be kept in step with the catalogue by hand: nothing checks `ground_m` against the templates, and the one refusal met on the way was a triangle of 1.8 ha that the rectangle test passed and the parcel pass could not build. **Confidence:** medium.

### The approach rule is a corridor, measured from where the districts end

**Choice:** An approach is a corridor of open ground 400 m wide and 1,800 m deep along one bearing, starting at the last of the settlement's own ground inside the corridor. The generator keeps a corridor 650 m wide clear from the main settlement's centre to 1,920 m past the limit of its ground, in each half. `ApproachPlan` keeps its fields; `from_rad`..`to_rad` is a run of bearings whose corridors are open and `front_m` is the corridor's width.

**Gap:** C52 measured a fan of rays from the settlement's centre, 400 m wide half-way out. With an outline that follows the districts, the same open ground measured narrower the nearer the town's edge lay to its centre, and the fan a town of any shape needs was too wide for a Small Metro map.

**Verdict:** sound. It is M19's own wording (a front of 400 m, 1,800 m deep) and stricter than the fan at the town's edge. Counts of approaches are not comparable with C52's. **Confidence:** medium.

### Fairness is steered in three places, then checked

**Choice:** Sites are placed to balance the built ground each is expected to have (its ground times its class's mean `built_share`). A settlement that straddles the midline grows on whichever side has less of its built ground. Last, settlements build or leave open up to four blocks at a time (`retries.repair_blocks`) until the halves agree to half the tolerance. The finished plan is measured against the tolerance and refused if it fails.

**Gap:** C52 balanced exact sector areas at placement. Districts now exist only after the roads do.

**Verdict:** sound: every plan in the sweep is fair. **Confidence:** high.

### A district's streets run with the settlement's main street

**Choice:** The parcel pass lays each district's grid along one of the district's edges: the edge that runs longest in the direction of the carriageway through the settlement's centre. A carriageway within its own half width of a district's edge counts as frontage for that district.

**Gap:** C53 ran every grid out from the settlement's centre. Running each along its own longest edge turned the grid 90° from one block to the next.

**Verdict:** sound as a rule. Neighbouring grids run the same way; their streets still do not join across the avenue between them. **Confidence:** medium.

### A hamlet's lanes are dirt tracks

**Choice:** `districts.<kind>.streets.surface` is `road` or `dirt_track`. The farm district's is `dirt_track`, and a hamlet's avenues are tracks too.

**Gap:** C53 listed paved 7 m streets in hamlets as wrong.

**Verdict:** sound. A village's streets are still paved. **Confidence:** high.

### A wood beside a town is a wood of its own shape

**Choice:** On half of the blocks a settlement leaves open beside its districts (`forests.infill_chance`) a wood is tried, up to six times (`retries.infill`): a wood of the usual shape, half to 1.6 times the block's size (`forests.infill_cover`), set about the block and kept only if it stands 25 m clear of every district, as any wood does.

**Gap:** M23 asks for woods pushing in between districts. Clipping a wood to its block gave green chips with straight edges.

**Verdict:** provisional. The woods are whole shapes and some reach into a town's bays, but every review still says woods never touch a settlement: the 25 m gap and the bare field round it read as a halo. **Confidence:** low.

### Not done

Apartment slabs still stand along their streets, not in ranked rows across a lawn. An industrial district is one block on a road at the town's edge with streets of its own, but still one shed to a parcel. A town centre has no square; its main street is the country road or a 10 m avenue. Near-duplicate bridges and sharp turns onto a bridge are as they were. Open ground has no field pattern, and every map has its main settlement in the middle.

## Simulation lane decisions

Review first: the provisional building HP coefficient, legacy source-fact bridge,
and the bridge-approach tradeoff below. This section supersedes the earlier
map-lane decision to leave river planning limits unchanged; the simulation lane
now owns those mechanics. Art acceptance and complete G0 remain separate work.

### Sound, with provisional values or a retained source gap

#### C42/C43 — bulk integrity has a provisional coefficient

When two building parts overlap, their shared ground area counts once toward
integrity. The coefficient multiplies that union footprint by the first three
floor bands, the same band policy used for fighting seats. Computing area once
from immutable source geometry avoids changing bulk when a damaged shell loses
height. Summing part rectangles would give overlapping compounds extra HP.
The coefficient is currently one in the rules fixture; C50 owns final balance.
**Confidence:** high on area/ownership, medium on that starting balance value.

#### C40 — preserve only the identified legacy source bridge

Old authored box fixtures have no authoritative floor heights or bay positions.
They keep one ground band with approximately three-metre facade spacing.
Integrity and collapse classification also use the one-band legacy fallback;
this is a compatibility policy, not a claim that the source has one floor. Once
floor heights are supplied, an unresolved bay list supplies no seats: the
simulation cannot invent upper-floor windows. Resolved facades remain usable.
Removing the bridge today would remove fighting positions from those frozen
inputs; extending it to known floors would create a competing facade owner.
Remove it when preparation supplies complete physical facts. **Confidence:**
high on preserving those inputs, low on their eventual source-window fit.

#### C40 — demonstrate the firing seam with the existing rifle squad

The catalog has no dedicated infantry MG unit. The physical recording therefore
uses its existing rifle squad firing from floor three over a five-metre obstacle;
the gun-agnostic seat/muzzle contract is exercised without inventing a new unit
or borrowing a weapon model. The exact MG/window visual moment remains a later
unit/source presentation task. **Confidence:** high on the physical seam, medium
on substituting this demonstration for the slice's named MG shot.

#### SA6 — prefer a finishable aligned bridge approach

A move whose direct terrain leg crosses water asks the road graph for a bridge,
even under shortest policy. It joins and leaves through authored road approaches,
rather than cutting diagonally onto a deck where the formation or turning hull
cannot finish. Shortest compares distance; fastest compares travel time. The
unbuilt alternative is a more globally optimal approach that still satisfies
those physical constraints. This can make a cross-river route longer than a dense
planner's route; it does not redefine ordinary shortest moves or relax body fit.
The worst observed routes in the frozen corpus cost about 20% more than the
dense planner's route; this is an observed tradeoff, not a universal error bound.
**Confidence:** high on physical completion, medium on global optimality.

#### C77 — density is a sparse candidate ceiling

Logs and boulders each use an independently seeded jittered lattice. Candidates
that cannot fit are omitted without retrying or moving a trunk. Logs are placed
first, boulders second, after all existing trunks. A narrow or densely planted
wood may admit no floor bodies. The alternative, hunting until an exact count
fits, would add unbounded startup work and disturb the established tree layout.
Configured densities are candidate ceilings, with a hard admission cap of
100 per hectare to preserve sparse startup work. **Confidence:** medium;
actual density depends on the available physical gaps.

#### SA5 — direct page lookup spends directory memory

A terrain or foliage sample addresses a directory of sparse pages. Missing
terrain pages mean exactly flat ground; missing foliage pages mean open ground.
Existing samples, interpolation and clearing remain authoritative. Keeping the
old tree/hash lookup at every ray sample would spend less directory memory but
repeat more work. A larger extent or finer cells must measure this trade again;
empty height fields allocate no directory. **Confidence:** medium on the memory
trade, high on preserving sample semantics.

### Sound owner and behavior decisions

#### C40/C41 — one occupied seat plan supplies eyes and muzzles

The capped plan distributes seats across building-frame directions and samples
physical bay lists evenly, highest eligible band first. Each direction sees
from its highest occupied living seat, with stable seat-index ties. Weapon
assessment uses actual living carriers' muzzles; launch owns window-swap delay.
A hypothetical nearest vacant window could allow a shot no participating soldier
can fire. Whole-squad admission is checked again when entry finishes because
supply may replenish the squad during its entry timer. **Confidence:** high.

#### C40 — replacement height clips seats without revealing hidden changes

An optional holdable damaged shell reuses only source bays below its remembered
remaining height. Physical entry checks the live shell. Original authored IDs
retain their source plan while remembered, so an unseen collapse cannot change
command admission. Holdable replacement states share one height across parts;
full per-part heights belong to terminal ungarrisonable gutted shells. The
alternative, reading live replacement height at command time, leaks hidden
changes. **Confidence:** high.

#### C42/C43 — building policy belongs in catalog data

An explicit HP-scaling enum is independent of permission to garrison. The
optional building-remains policy supplies the collapse fraction, cap, floor
threshold and terminal gutted kind. Ordinary props retain fixed HP and ordinary
destruction chains. A per-kind simulation branch would split the catalog owner
and make new building types require code. Aggregate replacement states remain
immovable until one owner can move a whole compound coherently. **Confidence:**
high.

#### C42/C43 — placement owns the whole destruction chain

A catalog row may be usable in a building aggregate but cannot be spawned as an
ordinary crate, trunk, deck or vehicle wreck if any later state needs building
seats or scaled HP. One catalog traversal checks the ordinary/aggregate placement
context for every declared state. Boot validates actual bindings; dynamic births
validate their own context; a replacement inherits its old body's retained
geometry owner. This replaces separate guards and closes the later-state hole
without adding a serialized field, cache or recurring movement check. Unused
aggregate-capable rows can remain in the same catalog. **Confidence:** high.

#### C42/C43 — retain floor ownership after an occupant dies

A soldier who dies on an elevated fighting floor retains that floor's building
owner even after the entire squad dies and releases its hold. When support is
removed, existing and new deaths settle on the existing ground/deck surface at
their XY position. The tall gutted exterior also loses fighting floors. Identity
and facing remain; settling does not reroll survival. Keeping only the live
squad's hold would strand previously dead occupants in the air. This is internal
simulation state, with no new public corpse fields. **Confidence:** high.

#### C42/C43 — a corpse position follows side knowledge

The observing side remembers a movable floor corpse's last-seen pose. A hidden
collapse updates its own casualties immediately but cannot move a remembered
enemy corpse until sight returns. Ordinary ground deaths retain their immutable
pose path and original digest bytes. The existing knowledge owner stores these
conditional snapshots; a separate corpse-knowledge subsystem would duplicate
that owner. The renderer refreshes dying, resting and fading anchors without
restarting death or bringing a faded body back. **Confidence:** high.

#### SA5 — reuse indexes and invalidate only visited occlusion tiles

Eyes query existing body/forest buckets; a visited four-by-four fog-cell tile
rasterizes the same cell centres as before. An obstacle revision invalidates
cached tops, but only subsequently visited tiles rebuild. Forest candidates are
sorted before unchanged span integration. Each side indexes its authoritative
remembered bodies through the same two mutation owners; a read-only accessor
prevents unsynchronized writes. A second sweep-only copy or whole-map rebuild
would add another authority or repeated global work. **Confidence:** high.

#### SA5 — omit only rays proven unable to add visibility

A ray skips physical sampling only after proving every cell it could visit is
already in that side's visible union. Checking the farthest cells first changes
proof cost, not the answer. Eye order, shape and cadence remain unchanged.
Learning a visible building part still reveals all current live parts, and uses
immutable historical identities to remove far remembered replacements. Nearby
index discovery cannot turn an aggregate into independently learned pieces.
**Confidence:** high.

#### SA5 — reports distinguish work from scheduling delay

The city report can repeat its existing unit mix to a hundred units per side;
its original six-unit scenario remains unchanged. It reports process CPU time
beside wall time and retired instructions. A tick delayed by other work remains
a delayed wall-time tick; CPU time only helps attribute the delay. Reports are
measurement seams rather than production telemetry or new game settings.
**Confidence:** high.

#### SA6 — public terrain can prove disconnection, removable bodies cannot

Connected terrain row runs are built before stamping bodies. Different component
labels prove an impossible crossing; equal labels merely admit physical search.
Repeated identical row intervals share a query band. Long terrain probes and
route revalidation keep cursors across scheduler steps instead of scanning a
whole leg in one tick. A removable or unseen wreck cannot establish permanent
river disconnection. **Confidence:** high.

#### SA6 — road-access discovery belongs to each endpoint

If nearby road accesses are absent or all physically inaccessible, search arcs
incrementally for the nearest terrain-legal access at that endpoint. Retain the
existing access-radius band beyond the nearest candidate. Discovering a start
access cannot suppress a later goal search. Rejecting one connector leaves the
road arc usable by other journeys. Soldiers share the squad's global corridor
and test terrain for their local lanes, avoiding eight independent global
searches. **Confidence:** high.

#### SA6 — stalled followers may make room

Every stalled vehicle may try a local detour, and a newly committed route clears
an obsolete reversing manoeuvre. Straight followers reserve turning room; that
extra longitudinal reserve is removed while reversing, turning or approaching a
corner. Arc probes stop at the remaining heading error and may not create or
deepen hull overlap. Numeric unit priority alone could keep the rear vehicle
from yielding forever while its leader needs room to reverse. **Confidence:**
high on the defined physical moments; dense traffic still relies on local yielding.

#### C77 — conservative placement preserves routes

A body's bounding circle must fit inside forest/map boundaries and remain clear
of roads, water, trunks and other bodies. A diagonal log cannot poke onto a road
merely because its centre fits. A bounded local navigation check then preserves
each old component and its open boundary for the actual catalog movement
profiles. Rejected candidates roll back. One temporary shared grid is built only
for active floor density; an influence window over 4096 cells per fit pattern omits the candidate.
Sampling a few routes would not prove that no enclosed pocket was introduced.
Conservative rejection may reduce density. **Confidence:** high.

#### C77 — default activation waits for drawing

Missing optional floor fields mean zero density, preserving frozen inputs. The
playable village also keeps zero density until accepted log/boulder drawing
exists; explicit systems trials use five log and three boulder candidates per
hectare. Pending art resolves to no model, so default activation would create invisible blockers.
Dynamic felled trees retain the existing cleared-ground lifecycle; this does not
add a separate tree-to-log rule. **Confidence:** high.

#### C44/C77 — unfinished art is represented explicitly

New physical catalog rows carry `appearance.status: systems_only` and their
intended scenery name. A bench cannot silently borrow crate art and claim a
fitted source. Ordinary appearance checks remain strict, and pending rows must
have no accepted binding. C45/C78 supply and fit real art before removing that
status. **Confidence:** high.

#### C80 — the bound includes all geometry and runtime multipliers

The validator measures actual vertices at every LOD and composes both runtime
height maxima. Horizontal ribbon width permits only bake roundoff. Wheat scale is reduced to 0.65 and verge scale to one to meet the 0.9 m
effective field-height cap. Testing only LOD0 or
individual multipliers would miss a taller far tier or an excessive product.
This accepts a physical bound, while silhouette, shadow/fog cues and appearance
remain specialist work. **Confidence:** high.

#### C86 — tree lines consume the existing forest owner

A tree line is the shared Stroke forest shape with ordinary trunks, canopy and
foliage. A narrow strip can conceal distant identification while remaining
transparent at close range. Its canopy extends past the authored ground strip
using real trunk radii. A second always-opaque hedgerow mechanic would change
sight and duplicate the geometry contract. The village gains no tree lines.
**Confidence:** high.

### Delegated starting physical values — C44/C77

### Per-kind war-film audit (delegated values)

Every row uses ordinary body columns. Soldiers can walk around an object;
vehicles either shove a lighter object or route around it. A thin object does
not grant an invisible cover position merely because its footprint blocks feet.

| Kind | War-film moment and verdict |
|---|---|
| lamp | A truck knocks the pole aside; rifle fire passes the thin shaft and it grants no cover. |
| bench | Soldiers crouch behind a light destructible bench; vehicles shove it and rounds pass its slats. |
| bollard | A jeep cannot push a bollard; a tank can, and the narrow post stops intersecting rounds but shelters no soldier. |
| bins | Light bins offer weak concealment-like cover through the cover rule, but neither block sight nor stop rounds. |
| hydrant | A jeep stops at the anchored hydrant; a tank shoves it, and it is too narrow to shelter a soldier. |
| utility box | Soldiers shelter behind the metal box; a tank can shove or shoot it apart. |
| scooter | Vehicles push a scooter aside; its thin frame neither stops rounds nor provides cover. |
| planter | A solid filled planter is medium cover and a medium obstacle; tanks can push it. |
| parked car | A car shelters soldiers, is shoved by a tank, and becomes a lower car wreck under destructive fire. |
| car wreck | The lower shell retains medium cover and remains shoveable; further destruction removes it. |
| Jersey barrier | Concrete is heavy cover, stops fire and resists a tank's ordinary shove; aimed destructive fire clears it. |
| bus shelter | Glass and open framing block walking but not sight or rounds; it offers no dependable cover. |
| scaffold | The frame obstructs movement but neither hides enemies nor acts as a bulletproof screen. |
| Heras fence | Mesh blocks walking while eyes and rounds pass through; vehicles shove it, and fire can damage it. |
| skip bin | Soldiers shelter behind substantial steel; a tank shoves it while a jeep cannot. |
| pallet stack | Light destructible cover can be shoved; incidental rounds pass while wearing it down. |
| site cabin | A closed storage cabin is an immovable opaque obstacle and heavy exterior cover, with no interior fighting positions. |
| traffic cone | A small cone does not interrupt soldiers or vehicles and grants no cover; intersecting fire can remove it. |
| road barrier | A light open plastic barrier blocks movement but does not shelter or hide a soldier; vehicles shove it. |

The HP and weight values in the catalog are starting physical values, not a
balance claim. The closed storage cabin avoids turning a street prop into a
second garrison representation; a future occupied cabin must be a C01 aggregate.

A log is destructible medium cover that stops intersecting rounds; a tank can
shove it but a jeep cannot. A boulder is immovable heavy cover and routes traffic
around its footprint. Their initial dimensions/HP and street-row values are
physical starting values, not a final balance claim. Placement and accepted art
remain owned by their later slices.

#### C80 — keep the complete footprint oracle affordable

The grass-prop test still scans every raw footprint for every sampled point.
Its double-precision inverse rotations are prepared once, and one local vector
is reused by the query. This removes repeated temporary allocation and fixed
trigonometry without narrowing the sample set, copying production's spatial
index, or changing strict footprint edges. Reducing cases or raising the test
deadline would conceal the oracle's avoidable work. **Verdict: sound. Confidence:
high; this changes test execution only.**

## C59 encounter planner

The planner, its seam, rules and measurements are in the [C59 outcome](slices/C59-encounter-planner.md#outcome). Recipe revision `encounters-1`.

### Both sides have a column at their own edge, and the defender also holds the objective

**Choice:** A roster row names its post. `column` rows stand on the road in from their side's edge: blue's at the bottom, red's at the top. `garrison` and `overwatch` rows are the defender's only and start at the objective. The shipped `assault` recipe gives red three rifle squads in buildings, an AT team on overwatch, and a tank and a jeep in a column at the top edge.

**Gap:** M22 says the two sides start at the top and the bottom. C59 also asks for initial garrisons in real buildings, and the only opponent is the village's defender policy, which holds what it garrisons and never advances.

**Verdict:** provisional. With every red row in the column, its squads would walk three to five kilometres to their buildings while blue's vehicles took an empty town. A recipe can say that (posts are per row), and it is the meeting engagement; it needs an opponent that attacks. **Confidence:** medium. This is the call most worth a second look.

### The defender's column is ordered to the objective when the battle starts

**Choice:** `deployment.defender_advances` writes one scripted move order (`ScenarioDefinition.scripts`) for red's column, to the objective by the fastest route, at tick 0.

**Gap:** C59 says the planner derives defender waypoints from its anchors. The defender policy gives a unit no order unless it garrisons, sees a tank or is hurt.

**Verdict:** sound. Without it red's reserve sits at the map's edge for the whole battle. It is an ordinary order through the existing script path, recorded and replayed like any other. **Confidence:** high.

### A fair start is equal drives, and the farther column moves up its road

**Choice:** The measure is the time a jeep (`deployment.pace`) takes from the head of each column to the objective by navigation's fastest route. The two must be within 15 s (`max_route_difference_s`). The side with the longer drive starts further up its own road, 50 m at a time, up to 1,500 m; past that the placement is refused as `unfair_deployment`.

**Gap:** The task says the two deployments must be fair to each other. C59 names no measure, and L08 warns that equal coverage is not equal access.

**Verdict:** sound. Two of the nine cells needed it (blue moved up 550 m and 850 m). It is measured on the simulation's routes, not on the plan's `transit` estimate, so it is true of the map the battle runs on. The 15 s and the jeep are guesses. **Confidence:** medium.

### A column takes the edge road it drives soonest from

**Choice:** Every road with an end on the side's edge is a candidate, up to `attempts.roads` (4), and the column stands on the one whose drive to the objective is shortest.

**Gap:** M22 promises a road from each of the two edges to the centre without marking which road it is in the compiled map.

**Verdict:** sound. No road is named or special; the spine wins because it is fastest. On Open Large seed 1 blue's fastest road enters the town from the east, and the overwatch post follows it there. **Confidence:** high.

### Sites travel beside the map

**Choice:** `contract::encounter::EncounterSites` holds each settlement's id, centre, outline and districts, and the measured approaches. `MapPlan::sites` makes it, `GeneratedMap` carries it, and `mapgen generate-map` writes `sites.json`. The contract owns `Half` and `Approach`; mapgen re-exports them under its old names.

**Gap:** C59 says the planner reads compiled urban and plain geometry. The compiled map has no land regions yet, and the simulation may not read a plan.

**Verdict:** sound for now. It replaces the lab's second generator call. Generation and compile outcomes grew a field, so the `map-layout` and `map-compiler` parity records were re-blessed; map hashes did not change. When land regions reach the map, the planner can read those and this record can go. **Confidence:** medium.

### The planner is a new simulation module, beside the village's scenario builder

**Choice:** `sim::encounter`, with its tests in a new file and its report a new example. It returns the same scenario fields `sim::village::scenario` builds.

**Gap:** C59 says to extend the existing scenario builder. That builder reads the village fixture's coordinates, and its files belong to another lane this pass.

**Verdict:** sound. Folding the village's builder and this one under one name is a rename for whoever next owns `village`. **Confidence:** medium.

### A unit is reachable when navigation plans its route into the zone

**Choice:** For each unit type of a column, each garrison squad and each overwatch post, `navigation::plan` must return a route whose last point is inside the capture zone. Only the pace unit's route is timed.

**Gap:** C59 asks for required routes and objective reachability without saying how they are judged.

**Verdict:** sound. `NavGrid::route_time` returns infinity for infantry routes that thread between buildings (it samples straight segments; the search steps through sub-cell gaps), so it cannot be the test for squads. That is noted for the navigation owner, not changed here. **Confidence:** high.

### Garrison capacity is one question, and the seed chooses the buildings

**Choice:** `garrison_seats` is the number of perimeter slots `garrison::slots` gives a building; a squad is admitted when it has a seat for every soldier. Candidates are the buildings within 150 m of the centre (the zone's radius) that stand in one of the settlement's districts. The encounter seed shuffles them, and those on the side of the centre the attack comes from go first; each garrison row takes the first that has the seats, stands 60 m from those already taken, has legal ground 4 m out from a door and a route into the zone. At most 40 are tried per row.

**Gap:** C40's floor-band seats are being built in another lane. C59 says only that whole squads are admitted within capacity.

**Verdict:** sound. When seats become floor bands, `garrison_seats` is the one line that changes. The encounter seed draws nothing else, so the same map gives the same columns and posts under every encounter seed. A first version drew from every building within 200 m; an unprimed reviewer saw the garrison behind the objective and outside the zone. **Confidence:** medium.

### Overwatch watches the attacker's road first, then the open approaches on the attacker's side

**Choice:** The ways in are the road the attacker's column drives in by, where it crosses the settlement's outline, then each measured approach whose bearing points toward the attacker's edge, nearest the road first. Overwatch rows take them in turn. A post stands 5 m beyond the settlement's last ground on a line parallel to its way in; the places tried are 25 m apart across it (the first 25 m to the side of a road, never on it), and of up to 12 the post takes the one that sees farthest down its way, up to 1,000 m. `objective.open_approach` says whether the objective must have such an approach at all.

**Gap:** C59 wants the encounter to use an 1,800 m approach. The plan's `half` says which half a corridor lies in, which is not the side it is approached from once a settlement is off-centre.

**Verdict:** provisional. With the approach first, the AT team on Metro Small 3 and Open Large 1 watched a field while the column came in by another road. With sight as a hard rule, a road through a hamlet left the road unwatched. The shipped roster's one post therefore watches the road; an open approach is covered from the second overwatch row on. Nothing here judges cover: a post may stand in an open field. **Confidence:** medium.

### What stands where: the footprint rules

**Choice:** A hull is tested at its centre, corners and edge middles for ground, against every body that stops vehicles, and against navigation's clearance. A squad needs its middle clear and, on a lattice at the squad's spacing over its spread, one standing place per soldier reachable on foot from the middle. Footprints keep 1 m (`clearance_m`) apart.

**Gap:** C59 says footprints are clear of bodies, water and other units and names the queries, not the test.

**Verdict:** sound. Every question is the world's or navigation's own; the planner adds only the list of points asked. A battle draws each squad's own arrangement from its seed, so the lattice says there is room, not where each soldier stands. **Confidence:** medium.

### The recipe carries the opponent's thresholds and the hold times

**Choice:** `defender.*` and `objective.hold_s` / `max_assessment_s` are recipe fields. The planner sets both fallback points to the objective's centre.

**Gap:** The stand-in read them from the village fixture's `defender_policy` and `encounter` sections, which are the village encounter's own data and not part of `Rules`.

**Verdict:** sound. The shipped recipe holds the village's values. **Confidence:** high.

### A mission or variant is a recipe row

**Choice:** `fixtures/encounters.json` is a table of named recipes under one revision. There is no mission enum and no variant list: another mission is another row with other posts, sides or preferences.

**Gap:** C59 lists "mission/variant" among the recipe's contents.

**Verdict:** sound while there is one mission (hold a zone). **Confidence:** medium.

### Results are rounded, and bearings use one software evaluator

**Choice:** Positions are whole centimetres, headings millionths of a radian, times milliseconds. Headings and bearing vectors come from `libm` through `contract::encounter`. Distances the planner compares are square roots.

**Gap:** The simulation's own geometry uses the platform's `sin`, `cos` and `hypot`, which native and Wasm do not promise to round alike.

**Verdict:** sound: six records agree to the byte. A comparison that falls within a rounding of its threshold could still differ between targets; none was seen. **Confidence:** medium.

### Planning builds a world, and the battle builds its own

**Choice:** `plan_encounter_json`, and so the Wasm export and the preparation worker, builds the world, navigation grid and road graph for the map, plans, and drops them. `plan_encounter` itself borrows them.

**Gap:** C59 says to reuse the prepared geometry owner and not hold a second world. C33, which makes that owner, is not built.

**Verdict:** a known cost, not a design. The build is 5.6 to 26.6 G instructions natively against 0.16 to 0.48 G for planning, and the battle worker repeats it. C33 passes its one world to `MapQueries` and the cost goes. **Confidence:** high.

### Not done

The planner has not run on the C58 map, which does not exist yet. No authored-encounter checker was built: `legality::stands` and `apart` are public for the one C58 needs. The lab draws no placement overlay; the pictures come from `encounter_report`. Cover and concealment are not judged anywhere. A refusal tries no other encounter seed and no other recipe.


## C09/C60 saved-map cutover

The seam these decisions belong to is in the [C09](slices/C09-fetched-maps.md#outcome) and [C60](slices/C60-map-catalogue-data.md#outcome) outcomes.

### A map's id is its lab's name; routes stay separate from maps

**Choice:** The folders are `village`, `endurance` and the twelve lab maps under their old names without `-lab` (`geometry`, `movement`, `river`, `sensors`, `weapons`, `consequences`, `deployment`, `ambush`, `garrison`, `supply`, `ground`, `readouts`). A route names the map it plays; nothing lists routes per map.

**Gap:** C60 delegated each lab's id.

**Verdict:** sound: fourteen folders serve 29 routes with no second list. **Confidence:** high.

### An encounter is the half of a scenario that is not the map or the rules

**Choice:** `encounters/<name>.json` is `contract::scenario::EncounterDefinition`: `units`, `events`, `scripts`, `opponent` and `encounter`, the fields of `ScenarioDefinition` other than `map` and `rules`. `EncounterDefinition::on(map, rules)` makes the scenario. An unknown top-level field is refused. A file is named after the route that plays it (`geometry/encounters/authority.json`); the ambush lab's variants are `late`, `prompt` and `crossfire`.

**Gap:** C60 said each lab's scenario moves into an encounter and did not give its schema. C58 and C59 want "the existing encounter shape".

**Reach:** The battle's seed is not in the file: `Battle::new` takes it apart from the scenario, and C59 keeps map and encounter randomness as separate pinned inputs. Only the top level refuses unknown fields; a misspelt key inside a unit is still ignored, as it is in any scenario.

**Verdict:** sound: C58's authored encounter and C59's planner output are this type. **Confidence:** high.

### A lab's pinned rules stay with its route

**Choice:** The ambush lab's instant drive timing and the garrison and lean labs' all-but-unkillable soldiers are still set by the route, which passes its rules to the encounter loader. The encounter file holds no rules.

**Gap:** The labs' inline scenarios carried these experiment controls; the slice did not say where they go.

**Verdict:** sound for now. The values are controls of the lab's experiment, not of the forces on the map, and the soldier pin is a loop over the unit catalog that a patch in a file could not express simply. **Confidence:** medium: a generic map route (C61) will want an encounter to be playable without its lab's code.

### The village and endurance encounters stay factories

**Choice:** The village's two variants are still built by `sim::village::scenario` from `village.json`'s `spawn`, `variants`, `defender_policy` and `encounter` sections, and the endurance waves by `sim::endurance::scenario`. Only the village's `map` left `village.json`. The village map has one saved encounter, `lean`.

**Gap:** C60 names the labs' scenarios. The village's deployment is rules data other work is editing, its tests rewrite spawn rows, and endurance takes a seed.

**Reach:** `sim::fixtures::village()` composes the rules, the unit catalog and the resolved map under `map`, so no test or example that reads it changed. The browser composes the same value. Endurance gained an input, so it takes its field as an argument and a stale caller fails to compile.

**Verdict:** sound for a location-only cutover. C59's planner takes the map and the rules apart; the village factory should follow it then. **Confidence:** medium.

### The endurance field is saved as its code printed it, and its wrecks stay in code

**Choice:** `maps/endurance/map.json` is the canonical JSON of the map `sim::endurance` built before the cutover. Its `SOURCES.json` selects the one template it uses, so the catalogue hash it always had still resolves. The late state's 2,000 wrecks are appended to the resolved map in code, from the same stream, in the same order, with the same ids.

**Gap:** C09 named the wreck augmentation and left how the base is produced open.

**Verdict:** sound: the typed scenario is byte for byte the old one at seed 1, seed 1 late and seed 7. **Confidence:** high.

### Receipts name the files the maps came from

**Choice:** Each `SOURCES.json` has one `repository` receipt: the old path, the revision before the cutover (`f303c07c`) and the sha256 of that file there. A lab's is its `fixtures/<id>-lab.json`, the village's is `fixtures/village.json`, and endurance's is `crates/sim/src/endurance.rs`. The lab maps moved byte for byte and the village's map was lifted text for text.

**Gap:** The earlier choice fixed the receipt's shape, not which file each map names.

**Verdict:** sound. The receipt records provenance; the map's identity is its content hash. **Confidence:** high.

### One allowance for the saved catalogue, owned beside the resolver

**Choice:** `MapAdmission::CATALOGUE` (4,096 authored parts, 65,536 bay positions) is a constant of `contract::maps`. The native adapter and the Wasm export both pass it.

**Gap:** The resolver makes each caller state its allowance. Both catalogue adapters are the same caller, in two languages.

**Reach:** The largest saved map has 56 parts. C58's saved generated map will need a larger allowance, and raising it is then a decision about every reader's startup.

**Verdict:** sound: one number cannot differ between native and browser. **Confidence:** high.

### The adapters' refusals use the resolver's error

**Choice:** `ResolveCode` gains `invalid_id`, `missing_document` and `invalid_encounter`. Every refusal's location starts with the map's folder. An identity mismatch states the hash the resolver computed, so an authored map's `SOURCES.json` can be written from its first refusal.

**Gap:** The resolver does no IO, so it had no code for a missing file or a bad address.

**Verdict:** sound: native and JavaScript callers see one error shape. **Confidence:** high.

### `meta.json` has no id, and what it repeats is checked

**Choice:** The id is the folder's name. `size_m`, `tags`, `encounters` and, for a generated map, `source` and `seed` are compared with the map, the directory and `SOURCES.json` by `checkMapFolder`. `tags` are exactly the physical features the map has (`relief`, `river`, `road`, `bridge`, `forest`, `prop`, `building`). `character` is `open`, `mixed`, `metro`, `village` or `arena`; every lab and the endurance field is `arena`. `benchmarks` are the ids of the benchmark routes that run on the map, and the test checks each is a registered route.

**Gap:** C60 delegated the `character` and `tags` vocabulary. Q-G9 lists the fields without saying which are derived, or what `benchmarks` holds.

**Verdict:** sound for the derived fields: a listing cannot say something the map does not. `benchmarks` is provisional: C61 decides what the menu needs from it. **Confidence:** medium.

### Each document is its own served file

**Choice:** `web/src/maps/browser.ts` globs `map.json`, `SOURCES.json`, the encounters and the template library as URLs (`?url&no-inline`), so each is a hashed file fetched when a route first asks for it. `catalogue.ts` globs `meta.json` eagerly; nothing on the entry path imports it yet.

**Gap:** C09 said HTTP and no map in the entry script, and left how a document is served open.

**Reach:** The entry script shrank by 6.7 KB (1.3 KB gzipped). C61's menu will import the listing; if the metadata then weighs on the entry, the glob can turn lazy without changing `listMaps`'s callers much.

**Verdict:** sound. **Confidence:** high.

### Node has its own adapter

**Choice:** `web/src/maps/node.ts` reads the folders from disk and calls the built Wasm resolver, synchronously. Web tests and scenes use it; the catalogue test reads the directory with it.

**Gap:** C09 names a browser adapter and asset tools over the built Wasm. Vitest and the scene runner cannot fetch a Vite asset URL.

**Reach:** The asset CLI (`web/asset.mjs`) and `scene-assets`' fit authority read rules only (`physics`, `forests.rule`), so they did not need it. `scene.mjs` lists no maps yet; that is C61's.

**Verdict:** sound: three adapters, one resolver. **Confidence:** high.

### The camera and generated labs are not saved maps

**Choice:** `fixtures/camera-lab.json` (a plan the map compiler builds at run time, over the prototype templates) and `fixtures/generated-lab.json` (the generated route's limits, developer encounter and camera) stayed where they were.

**Gap:** Q-G9 says the labs move out of `fixtures/*-lab.json`. These two hold no `MapDefinition`, and a saved map of prototype templates needs the catalogue to name a second library, which C58 has to decide for its saved generated map.

**Verdict:** open. Both bypass `contract::maps::resolve` today: their maps come straight from the compiler. C55 gives the generated request its variant of `MapSource`, and the camera lab's plan can be saved as a compiled map once C58 settles the library question. **Confidence:** high that it does not belong in a location-only cutover.

### Not done

C33 (one simulation world) was not started. Its consumers call the main-thread world synchronously for picking, ground height and foliage clearing, and moving them needs a public query index and prepared geometry that `Battle` can reuse: a slice of its own. `specs/city-maps/assets/map-acquisition/README.md` still describes the core checkpoint; it is the frozen record and was left as written.

## Road ends

The user sent a close-up of a road that stopped in open ground in a perfect half-circle and asked that roads not end like that. Two things were wrong: how a stroke ends, and where the generator let roads end.

### A stroke ends square, by one rule for every stroke

**Choice:** A ground stroke's membership is the band within half its width of its rounded centreline, cut flat across its first and last point (`contract::ground::stretch_contains`). Bends stay round. A positive margin grows the flat end like any other edge, round at its two corners. A point behind the start that a later stretch reaches is still the stroke's.

**Gap:** C03 said "a union of closed capsules", which puts a half-disc of the stroke's half width on both ends of every road, track, sidewalk and tree line.

**Verdict:** sound. One rule, no per-kind exception and no schema field. Tree lines: no saved map and no generator output has a stroke forest today; a square-ended strip is what a hedgerow or shelter belt is. **Confidence:** high.

### Stretches close behind an end are cut too

**Choice:** A stroke is tested stretch by stretch between its rounded samples. The first stretch is cut at the stroke's first point and the last at its last; and every stretch that starts within half the width of the first point, measured along the line, is cut square at its own start, and likewise at the other end (`contract::ground::stretches`).

**Gap:** Cutting only the first and last stretch is not enough. The round joint between two stretches is a disc of half the width, so a bend, or just a second sample, within half a width of the end bulges out past the end's face. The GPU check found it on a ground whose samples are 2 m apart under roads 6 to 12 m wide: no end there read differently from a round cap.

**Verdict:** sound. Every saved and generated road's first bend is farther than half a width from its end, so only their first and last stretches are cut and no saved digest or oracle moved. Where a stroke does bend that close behind its end, the outside of the bend there has small wedges between stretches in place of a bulge. **Confidence:** medium.

### Rivers keep their round ends

**Choice:** A river is not a ground stroke. It has its own contract (`contract::river`), whose one distance also sets the height the bed and bank are carved to. That distance still clamps to its end points.

**Gap:** The brief listed river stretches among the strokes.

**Verdict:** sound for now. A generated river runs from the north edge to the south, so its ends are off the map. Two labs end a river inside the map (`geometry-lab`, `movement-lab`): a round end there is a pool's end. A flat cut would be a cliff across the bed, since the carved grade has nothing to run out along. **Confidence:** medium; revisit if a river is ever meant to stop at a weir or a culvert.

### A stroke's limits stay the capsule's box

**Choice:** `GroundShape::limits` is still the samples grown by half the width. The square ends lie inside it.

**Gap:** The tight box of a square-ended stroke is smaller at each end.

**Verdict:** sound. Limits are a conservative bound for buckets and broad phases. A forest's trunk lattice hangs off them, so tightening them would re-seat every trunk of every tree line for nothing. **Confidence:** high.

### The export says which ends of a stretch are cut

**Choice:** `surfaceStrokeFields` and `forestStrokeFields` gain a last column, `cuts` (stride 6 to 7), and the layout a `strokeCuts` entry naming its two bits. The renderer reads the stretch through one module (`terrain/strokes.ts`), on the CPU and in WGSL.

**Gap:** A stretch between two samples cannot know how near its stroke's end it is.

**Verdict:** sound. The alternative, a flag packed into the sign of the half width, hides a contract in an encoding. **Confidence:** high.

### Navigation's road net still joins by nearness

**Choice:** `navigation::roads` links two road pieces whose centrelines come within the sum of their half widths, as before.

**Gap:** With square ends two roads can be that near and not touch: one stops a metre short of the other's edge.

**Verdict:** provisional. The net says where roads lead and every journey is checked on the grid, so a link over a metre of grass costs a little speed, not a wrong path. The generator no longer produces such a gap. **Confidence:** medium.

### One pass closes every road end, in both generation steps

**Choice:** `mapgen::joints::close` takes the carriageways the layout laid, and again those with the parcel pass's streets before parcels are cut. It leaves each end part of a through road (ends of one kind and width that meet become one stroke through the point they shared), under the road it joins (just past that road's rounded middle), the outer edge of a corner (a wider road run over a narrower one's end), square on the map's edge, or cut back to the last block it serves.

**Gap:** Each emitter (roads, avenues, lattice streets, links, run-ons) relied on round caps to close its joints in its own way: an avenue ran to the far edge of the road it met, a street half a metre past a middle, an L-corner on overlapping caps.

**Verdict:** sound as a design: twelve kinds of joint across four emitters became one rule in one place, and the test judges the finished plan. **Confidence:** medium; see the residue below.

### Two roads that meet alone are one road; at a junction only the straight-through pair is

**Choice:** Ends of one kind and width are welded round any corner when only the two meet (a country road or track up to 110°). Where three or more meet, only ends within 30° of straight are welded; the rest each run on past the shared point until their centrelines cross.

**Gap:** Welding the straightest pair at every junction left a third road touching the rounded bend from outside, at a tangent. The plan's road graph joins roads where centrelines cross, and one settlement in a sweep lost its road.

**Verdict:** sound. `no_road_turns_back_on_itself` now allows 110° (it was 90°): a lane round a block's corner is one stroke now and blocks are a few degrees off square. **Confidence:** medium.

### The pass never loses a joint; it leaves the roads concerned as laid

**Choice:** Before and after, the pass lists the pairs of roads whose centrelines cross. If a pair crossed before and neither crosses after nor is one road, those roads are pinned as they were laid and the rest are closed round them, up to four rounds.

**Gap:** The road graph's test is an exact crossing, and any change to an end (a centimetre's rounding, a bend moved by a weld) can turn a touch into a miss. Chasing each case one seed at a time did not converge.

**Verdict:** sound: the sweep's road-graph refusals stopped with it. It costs two more crossing passes a plan. **Confidence:** high.

### Welded streets cost about a fifth more generation work

**Choice:** The cost is accepted. Generation and compilation retire 2.24 G instructions for a Mixed Small map against 1.84 G with the pass switched off, and 7.92 G against 6.52 G for Metro Large (medians of 12 seeds): 21 to 22% more.

**Gap:** No budget names generation's own cost; the startup budget covers it.

**Verdict:** provisional. Skipping only the weld gives the cost back (6.44 G for Metro Large): a street welded round a corner has a rounded bend of a dozen samples where two straight strokes had none, and everything downstream (the parcel pass's street index, `measure`, the compiler) reads every sample. The pass itself and its lost-joint check are about 0.3 G of the 1.4 G. The simulation and the surface field carry those stretches too, which has not been measured. A bend sampled more coarsely on streets is the lever if startup needs it. **Confidence:** medium.

### A road leaves the map square to its edge

**Choice:** A road that ends on the map's edge at a slant gets one more authored point, two widths in from the edge, so its last run is square to the edge and its flat end lies along it.

**Gap:** The compiler admits no authored point past the edge, so a road cannot simply run on out of the map; and a flat end at a slant leaves a wedge of grass inside the map.

**Verdict:** provisional. Nothing shows at the edge, at the price of a slight kink in the last 16 m of a country road. Letting strokes overhang the edge would be straighter and touches the compiler, the simulation's buckets and navigation. **Confidence:** medium.

### A road stops at the last block it serves

**Choice:** A road that ends in open country, by no other road and short of the map's edge, is cut back to the last district block it runs on or along (within its half width and a metre), or to the last road that joins it.

**Gap:** A settlement's main street was laid to the end of the ground the settlement might build on. The settlement often grew less far, and the street ran on 50 to 300 m into the fields and stopped: the picture the user sent. The settlement's `outline` is not the measure: it is one ring and can leave districts across a road outside it.

**Verdict:** sound. No road in the sweep stops in open ground. **Confidence:** high.

### A street runs to a street end that faces it

**Choice:** A dead-end street's run-on goes to the end of another street that faces it within a block and within that street's half width aside, where one is no farther than 10 m past the first carriageway it would cross.

**Gap:** Neighbouring districts' grids are offset by a few metres. A run-on that passed the facing street's end ran beside it for a few metres: one street drawn twice, with a step where the other stopped.

**Verdict:** provisional. It removed about half of those steps and made more streets carry through. **Confidence:** medium.

### What is left: 1.1 ends in a thousand

**Choice:** `tests/road_ends.rs` allows 2 ends in a thousand to be a bite or a step in a joint, and none to be a gap before a road, a face at the map's edge or a road stranded in open ground.

**Gap:** Two kinds of joint are not closed: three or more roads of one width that meet at a point at sharp angles, or whose ends stand a few metres apart round one junction; and two streets of one width laid side by side for a few metres. (Two roads that fork alone, too sharply to be one road, are closed: the first runs on over the second one's end. The unprimed critique found that fork as a stepped tip before this.) A wider road that ends on narrower ones shows its shoulders and is counted sound: the road narrows there.

**Verdict:** provisional. The bound is a measured count, to be lowered as those are closed and never raised. A second unprimed critique still reads the closed fork as "a tick mark with a blunt heel": the first track runs on about half its width past the point and ends flat. It is sound (no bite) and not pretty; a link that joins a main street's far end at a sharp angle is the layout's choice, and choosing a kinder join belongs to the road network, not here. **Confidence:** medium.

### The change to physical ground is named; the quick village report did not move

**Choice:** Square ends take up to half a disc of road (or tree line) off each stroke end on every saved map, which is a change to physical ground and so a named decision for every digest that depends on it.

**Gap:** C03 promised the village's digests through that cutover, and the standing gate asks for `village_report --quick --compare main`.

**Verdict:** sound. The six quick trials (flank and ambush, seeds 1 to 3) end in the same six digests with square ends as with round ones: no unit crosses the three free road ends of the village map in them. Main had no cached run after the simulation lane merged, so the baseline was the merged tree with the square cut switched off, which is main's ground exactly. No saved-map oracle needed a re-bless; the two that moved are outputs of unchanged inputs: `parity/ground/curve-strokes.json` (the stroke export gained a column) and `parity/map-layout/paired-records.json` (generator `layout-5`). A full report or a battle fought across a road end may still differ. **Confidence:** high for what was run.

### Saved maps are not edited here

**Choice:** No stroke of a saved map was changed.

**Gap:** `game.json`'s village map starts its two roads at one point at an angle (140, 780), which is now a bite on the outside of that joint, and its roads and the labs' start and stop in open ground 10 to 140 m from the map's edge, by design of the old round cap. Another lane was moving those maps while this ran.

**Verdict:** open. Listed for the map lane: make the village's two roads one stroke through (140, 780), and run the lab and village roads to the edge or to something. **Confidence:** high that they need it.

### A road changes width only where another road crosses it

**Choice:** `joints::carry` replaces the rule that let a wider road turn a corner and stop a width and a half down a narrower one. Where unlike ways meet end to end, one of them is carried through the corner along the other's line to the first carriageway that crosses that line and covers the wider way's whole flat end. The wider way ends there, a quarter of a metre past the crossing road's middle, and the narrower one starts on the same point. A stroke still has one width and one kind: the change is two strokes meeting under a third.

**Gap:** The old rule left a flat end with a square shoulder either side of the narrower road, in the open. An unprimed review ranked it the worst thing left about roads, and `tests/road_ends.rs` counted it sound (`Narrows`).

**Verdict:** sound. The test now counts a wider road's shoulders as a flaw (`Shoulders`). Over its 36 maps: 362 flawed ends of 25,014 before (340 of them shoulders), 23 of 24,636 after (3 shoulders), under the unchanged bound of 10 in ten thousand. **Confidence:** high for the count; medium for the look, which one critique judged.

### The shorter stretch is the one that changes, and a side road is not a crossing

**Choice:** Two stretches could change: the narrower way's, on from the corner, or the wider way's, back from it. The pass walks both and takes the shorter. A side road that only joins the line (a T) covers one shoulder and is passed over; the change is made under a road that runs on both sides, or at the line's far end.

**Gap:** The brief left the side open ("for example the shorter stretch, within a bound").

**Verdict:** sound. Of 334 corners in the 36 maps, the wider way carried on in 309 and the narrower reached back in 25; 80 stop under a crossing road and 254 run to the line's far end, where it joins a road, leaves the map or stops. **Confidence:** medium.

### No bound on the distance: the far cases are carried too, and counted

**Choice:** Where no road crosses within a block or two, the stretch changes all the way to its far end. There is no fallback shape.

**Gap:** The brief asked for the least bad honest shape where no junction lies within a sensible distance. Every other shape (the old tail, a flat end at the corner point, a change on the straight before the corner) shows the same step somewhere.

**Verdict:** provisional. One way that is one width from end to end is honest and reads as a longer avenue, not as a defect. Counted: 51 of the 334 corners carry farther than 300 m, 19 farther than 400 m and one farther than a kilometre (1,369 m of street that became avenue). A bound with a different shape past it would bring the shoulders back for those 51. **Confidence:** medium; a long avenue where a street was planned may want a second look on a metro map.

### A track no longer carries on from the end of a country road

**Choice:** In the layout, a road may end on another road's end only if the two are of one kind (`roads::Network::joins`). A hamlet's track or a link between settlements joins where a road passes, in a T. A link that has no point on a settlement's roads to join, and may not join its centre either, is not laid.

**Gap:** The layout let a road end on any road's end it carried straight on from. That made a paved road become a dirt track at a point in open country, a kilometre or more from any junction either way: about one such corner a map, and the pictures of a road turning onto a track.

**Verdict:** sound. This is the fix at the emitter the brief preferred: the change of kind was being created needlessly, and carrying it in the joint pass would have turned whole tracks into country roads or whole country roads into tracks. After it, none of the 36 maps has a country road meeting a track end to end, and all 900 maps of the sweep still generate. The link that used to fall back to a settlement's centre, however many roads met there, is now refused there by the same rule as anywhere else; one map's centre would otherwise have had five roads. **Confidence:** medium: the road network of a map with such a corner differs from the one `layout-6` drew.

### What is left: 9 ends in ten thousand

**Choice:** `FLAWS_PER_TEN_THOUSAND` stays at 10 with shoulders now inside it: 23 of 24,636 ends (15 bites, 5 heels, 3 shoulders).

**Gap:** Three corners of unlike roads are left as laid. In the one traced, mending it lost a joint, so the pass pinned its ways (the rule above that the pass never loses a joint); the other two were not traced.

**Verdict:** provisional. Heels went from 2 to 5 and bites from 20 to 15 as the roads moved; neither kind was worked on here. **Confidence:** medium.

### What the unprimed critique still saw

**Choice:** Left as they are, and listed. One unprimed critique of four close-ups of Mixed Medium seed 2 found no shoulders and no step at any of the four corners, and named two other things.

**Gap:** (1) Where a street's short link (20 m, between two turns of about 55°) had joined an avenue's end, the avenue's width now runs through that double bend, and a side street meets its outer point: the critique called the bend swollen and lumpy with a spike where the side street attaches, its worst finding. The double bend was there before at the street's width, with the old step beside it. (2) Where the track used to leave the end of the village's country road, the road now stops at its last block and a street that had met the track stops 55 m away: two flat dead ends in one view, both of the kind the pass has always left ("at the last block it serves").

**Verdict:** open. The first belongs to the parcel pass's links (a link to an avenue's end need not dogleg), the second to dead ends in general. **Confidence:** high that both are visible; neither is a width change.

### The generator is `layout-7`, and its cost did not move

**Choice:** `GENERATOR_VERSION` is `layout-7`. The map-layout and encounter parity records are re-blessed and Market Town is saved again.

**Verdict:** sound. Over a 100-seed sweep of every type and size, before and after, the median instructions retired by generation and compilation are within 2% either way in eight cells and 5% lower in Open Large (Metro Large 8.95 G before, 8.82 G after; Mixed Small 2.39 G and 2.41 G). **Confidence:** high.

## Reuse manifest removed

**When:** 2026-10-01, at the owner's request: delete it unless it can be justified.

### The manifest, its test and the asset validator's provenance checks are deleted

**Choice:** `reuse-manifest.json`, its test, the validator's provenance findings, the licence allow-list in the asset schema, the `asset provenance` command and every step that rewrote a hash in the manifest are gone. Nothing replaces them.

**Gap:** C10, L10 and slices C16 to C19, C67 and C74 planned to extend the manifest.

**Verdict:** sound. Its record of code taken from the owner's sibling project raised no licence question, about half of it said nothing was copied, and only a test of the manifest itself read it. Forty-one of its fifty-one art rows were our own generated sources, hashed and "accepted", so every rebake had to rewrite a hash that the bake's own check and git already cover. The origin of ported code stays where it always was, in each file's header comment. **Confidence:** high.

### Kept: the pack pins, and a plain note of third-party sources

**Choice:** `packages/scene-assets/blender/packs.json` holds each downloaded pack's zip name, URL, licence and sha256, and the sha256 of each file read from it; only `packs.py` reads it. `assets/README.md` lists every third-party art source with its URL and licence. The three.js-derived shader code keeps its licence file beside it.

**Gap:** A pack is downloaded, not committed, so git cannot notice a re-upload; a third-party asset still needs a known source and a licence that allows shipping.

**Verdict:** sound. The pin is the one check that did real work: it stops a build on changed pack bytes instead of silently changing the art. C10 is rewritten to this contract. **Confidence:** high.

### Dropped with it: two guards that did not depend on the manifest's rows

**Choice:** The test that no source file imports the sibling project by path, and C10's planned test refusing the two unknown-source interior atlases by hash, are not carried over.

**Gap:** Both lived in the manifest's test file.

**Verdict:** provisional. An import by a path outside the repo already fails in every worktree and any other checkout, and the atlases are excluded by Q-E and the README's firewalls. Either can come back as a small test of its own if the owner wants the guard. **Confidence:** medium.


## Data files pruned

**Choice:** Every frozen record that only one side read (terrain queries, the buildings oracle, the foliage and patch snapshots, the rotation corpus: 2.7 MB) is deleted with its test. A record stays only when a native test and a Wasm test both compare against it, and it stores hashes unless a reader needs the bytes. The fog oracle became `publication/stream.json` over a saved map and the live rules. Raw data dumps in two finished specs are deleted. Everything removed is at tag `data-files-before-prune-2026-10-01`.

**Verdict:** sound. The snapshots proved cutovers whose old implementations no longer exist; since then they only failed on deliberate changes and were re-recorded unread. One snapshot was the sole test of a rule (how cleared ground thins foliage in Wasm) and was replaced by a small behaviour test. **Confidence:** high.

**Open:** the stream record is the only native-against-Wasm check of the simulation itself, and it is a four-unit move with no firing; a short battle with combat would be a stronger pair at the same cost.

## Scale lane decisions

The scale lane's durable rationale and consolidated decisions live in
[city-map scale](../done/city-maps-scale/README.md) and its
[choices](../done/city-maps-scale/choices.md). Other lanes' contracts and
historical measurement evidence retain their parent owners.

## Buildings lane

The final ledger of the [buildings lane](buildings-lane.md), rewritten when the lane closed (2026-10-02) from the per-pass entries and checked against the code as shipped. It holds decisions only: what was chosen where the spec was silent, what was declined, and what follows. Measurements and evidence are in each slice's Outcome. An entry that a later pass replaced is not here; where a later pass changed one, the entry states the end state.

Terms used throughout. A **template** is one building design the map generator can place (a 35 × 11 m four-floor slab, a bungalow). Its **parts** are the solid boxes the simulation builds for it. A **kit** is a file of shared pieces (**modules**: a window, a balcony, a wall shell), and a template's art is a list of **rows**, each placing one module. A **set** is one script's output: a kit and its templates. A **tier** is a level of detail, 0 the finest and 3 the coarsest. The **catalogue** is the list of templates' physical shapes the generator reads; its hash is part of a saved map's identity.

### Read these first

**China's apartment blocks ship first, and the lane closed on one family.** *Decision:* a generated town's apartment blocks come from the vendored China building graph. New York and Paris are later families through the same exporter; they were not built. The alternative was Paris first, which sits more naturally beside a European village. *Gap:* the spec names three sources and one family per map, not which comes first or how many close the lane. *Rationale:* China is the only source whose blocks stand free with four facades at exactly 3 m a floor, which is what the generator places and the simulation seats; Paris has 3.2 m floors under a mansard and New York always has two blank sides. Its enclosed balconies are also the look you named as the reference. *Consequence:* every generated map today is family `china`. A second family is a new set and a line in `map-presets.json`, not new code.

**Our own houses, farms, towers and sheds are tagged `china` and do not look Chinese.** *Decision:* the four scripted sets carry the shipping family's name so a map has all six categories. They are styled plainly (plaster, brick, tile, precast concrete). The alternative, a styled variant of each per family before anything ships, triples the art before a town can be judged. *Gap:* "missing variants cannot fall back to unrelated regional art" does not say whether project-made buildings count. *Consequence:* a town is Chinese apartment blocks beside generic European-reading houses, and the three farms read to a fresh eye as a German, a French and an English farm in one hamlet. If towns should read as one place, the levers are one palette across the scripted sets or the generator picking one farm per hamlet; neither is built.

**A house's box is as tall as its roof's ridge.** *Decision:* every pitched-roof part's top is its ridge: 5.2 m for one floor, about 8.3 m for two, about 12 m for three (the old boxes were 4, 6.5 and 9.5 m). The alternative was a box to the eaves with the roof drawn above it as allowed overhang. *Gap:* the old boxes had no roofs. *Rationale:* from the game's camera a soldier behind a house is hidden by its roof, so the box agrees with the picture. Pitches are held to 25 to 35 degrees to keep the over-claim small. *Consequence:* the simulation stops rounds and sight in the empty air beside a roof, up to 3 m above the eaves at the wall line. This is a rule of the game, not only of the art.

**Some bays have no window, and a soldier seated there fires through a drawn wall.** *Decision:* barns, warehouses, a slab tower's gable corners and one column of the tallest tower keep blank bays (100 of 2,090 bay positions on the towers; most of a warehouse's back). Houses and apartments have an opening in every bay. The alternative, a window in every bay everywhere, makes a barn read as a house and a warehouse as an office. *Gap:* the format puts windows on the bay lattice; it does not say every bay needs one. The simulation seats a garrison at every bay. *Consequence:* nothing marks a blank bay to the simulation. If it looks wrong in play the fix is fewer seats there, which is a rules change nobody has taken.

**A destroyed building's art fills the box the simulation leaves, and nothing stands over it.** *Decision:* a collapsed building's art is held by the asset check to the remains the simulation builds (a quarter of the building's height, between 2 and 6 m, on the same plan), with at most 0.6 m of jagged wall above (none for the apartment blocks). A building of more than six floors does not collapse: it stands as a burnt shell at full height. The art reads the rule's numbers from the simulation's own building row; nothing copies them. *Gap:* the slices ask for ruin and gutted art, not how it is bound to the physics. *Consequence:* what stops a round is what is drawn. A ruin can never be a tall dramatic skeleton; it is a low heap with stumps.

### Known shortfalls, not choices

These are disclosed defects and unheld checkpoints. None is presented as a decision.

- **A shopfront's glass reads as an open hole.** Upper windows read as dark glass over a room; a large pane with no bars and a pale room behind does not. The lever is a recipe for the pane (dust, waviness), about 1.3 MiB a bundle. Not built.
- **A gutted tower, far off, reads as a darker intact tower.** Its smoke carries the read at mid range. A facade in shade is also darker, so darkness alone is ambiguous.
- **A town downloads more than the kit budget.** A page fetches only the kits its map draws, but a block of a generated town draws from four or five: 94 MB without a tower, 119 MB with (apartments 43.5, towers 24.7, industry 19.4, farmsteads 16.1, homes 15.4). The spec's budget for a shared kit download is 50 MiB. The facade passes grew the kits; nothing shrank them.
- **A kit that fails to download leaves the loading cover up for ever.** The failure is an error on the console naming the kit. The loading screen has never reported an asset failure; it is that screen's to carry.
- **Identical neighbours.** One template is one colourway, so a suburb of twenty houses shows the same five. More colourways are more templates over the same modules, or a tint a placement supplies; neither is built.
- **Tier boundaries still show on some art.** A cage at tier 1 is its sheets without its frame; a tower's curtains go from six cloths to two at 128 m; a cutout sheet shows faint bands at 25 m. None was judged in motion.
- **The village farmhouse's ruin draws more triangles than its farm**, behind one named opt-out (`damage_budget=False` in `village.py`). Rebuilding it with the shared ruin builder removes the flag.
- **Checkpoints that were not held with you.** The far tier (C23), cutouts, glass and rooms (C24 to C26) and ruins by knowledge (C27) each ask for the shots to be shown and a wait. Those passes ran unattended; each verdict stands on measurements and an unprimed critique.
- **Outside this lane, found on the way:** `props.py tooth` does not export the same bytes twice; every page still downloads about 240 MB that is not kits before it starts; the parcel pass places some buildings overlapping, which boxes hid and roofs do not.

### What a town is built from

**Attached homes are scripted by us, not taken from a graph.** The spec assumed the graphs covered low attached buildings. The export spike measured that no graph makes a two-storey house or a mid-terrace unit with one street face; a graph building squeezed to 7 m wide is a shop block with a water tower. Terraces, a townhouse and shop rows are in the houses' set.

**Graph-made sides are 3n + 2 metres with 3 m floors; our own are whole bays.** *Gap:* the spec forbids stretching art to a footprint and left the legal sizes to the spike. *Decision:* a graph-made side is 3n + 2 m (whole 3 m bays between two 1 m corner piers), towers the same, industry whole multiples of 3 m. The alternative kept the old box sizes and left part-bays of plain wall. *Consequence:* every old box changed size a little (36 × 12 became 35 × 11, the tallest tower 28 × 28 became 29 × 29), and the catalogue's hash moved once per set as it replaced its boxes. It should now hold still.

**A compound block is built from its outline, never from joined wings.** *Decision:* a template is boxes that abut. The exporter traces the outline of their union and dresses each straight run with one graph facade of that length. The alternative the spec named (build each wing, then hide the faces at a join) leaves a facade cut mid-bay with a window through the inner corner. *Consequence:* a join never exists, so there is no interior facade or corner module to manage. A run must be 3n or 3n + 2 m, which makes compounds 12 m deep and the court block 41 × 38 m round a 17 × 14 m yard, smaller than the 44 × 40 m it replaced.

**A slab's long side and its shops face the street.** The graph puts its one entrance on its Depth facade, so a slab's recipe has Depth as the long side and the template is turned a quarter. The other way puts a 35 m slab's only door in its 11 m gable. A compound has one entrance, shops on the street and east flank, and plain backs elsewhere; the boxes it replaced had two doors.

**Each tower is one part, with no podium.** A tower on a low podium needs two touching parts of unequal height, which the contract's supported join refuses. The four towers are 10, 12, 16 and 20 floors. Three differ by one built thing each (a terracotta top floor, ribbon windows, a slab form); the 20-floor one is the plain one.

**The works' hall and office are one building of two 8 m boxes.** A join needs equal tops and floor levels belong to the template, so the hall is 2 m lower than the box it replaced and is a two-floor part whose tall windows serve both floors. *Consequence:* a garrison's upper floor in the hall is a notional gantry; nothing is drawn for it. Whether that plays well is untested.

**Farm plans are the old boxes' plans; the small farm is 2 m wider** so a cart passes between house and barn. The cart shed is under 3 m tall so the upper floor level is above its box and it seats soldiers on the ground only. It is drawn open-fronted while its box is solid: a unit behind it is hidden in the simulation where a low camera shows daylight under the roof. Each barn's wagon door on the street side is an entrance. No yard wall or gate: only a part can hold art, and a yard is open ground.

**A loading dock is not an entrance.** Dock doors have sills at lorry-bed height. A warehouse's entrances are its side door and drive-in doors, two or three a building as the boxes it replaced had.

**The sawtooth roof's glazing faces along the street.** A template has no compass. A true north light with the street to the south would show the viewer nine plain slopes.

### The source format and the bake

**A set is two files, and the script that models a building is the one author of its shape.** *Decision:* a script writes `kit.glb` (modules) and `templates.json` (each template's physical descriptor and, per state, its rows). The catalogue the generator reads is copied from those descriptors by `asset catalogue` and checked against them. The alternative kept descriptors hand-written in `fixtures/` with art fitted afterwards. *Rationale:* it is the one arrangement where art and physics cannot drift. *Consequence:* a row can scale, turn about the vertical and move a module, and nothing else; a tilt or mirror is a module variant.

**A kit is an ordinary static bundle whose states are its modules.** The alternative was a new bundle kind with its own codec and loader. A module is exactly what a state already is (four tiers, bounds, shared materials), so the existing bake, codec and loader carry kits unchanged.

**The template art library is one runtime file of packed rows, 34 bytes a row.** The alternatives were quantised positions (about half the bytes) or a file per template. Quantising to a centimetre risks cracks between modules that meet on a lattice. The shipped library, 41 templates each standing and destroyed, is 0.56 MB, so the bytes were never a problem.

**The library covers two catalogues, and a set names which it dresses.** The generator's catalogue and the hand-authored maps' boxes (`fixtures/building-templates.json`) are both drawn from the one library; a set says `generated` or `authored`. The alternative, one merged catalogue, would have put the authored boxes into the generator's hash and moved every generated map. The authored boxes are held to the contract's plain validity, not to completeness: resolving their floors and doors would change what the simulation seats in a village house.

**Art identity is the library's own bytes.** `art_hash` is the hash of the library encoded without it, and the library names each kit's bundle hash, so anything that changes what is drawn changes it and nothing else does.

**Missing art is refused by name, never drawn another way.** A template with no rows (`template.missing`), a state with none (`state.missing`), a module its kit lacks (`module.missing`), a kit not installed for a map (`kit.missing`). A template must have exactly its own damage state (ruin for six floors or fewer, gutted above) and a row at every tier, or it does not bake. The earlier fallbacks (a tinted box for a fallen part, a silent skip for a template without art) are deleted. *Consequence:* a stale library stops a map drawing; it cannot quietly hide a building.

**Fit: every vertex inside some part grown by the set's allowance, and nothing below a part's base.** Each set states how far art may reach past a part's faces (0.5 m for houses, 0.6 farms, 1.2 industry, 1.5 towers and apartments) and above its top (0.9 to 4 m, for chimneys, lift rooms and aerials). *Consequence:* a plinth sunk into a slope is refused; foundations on sloped ground need the ground lane's pads or a new `below_m`. Two neighbours need the sum of their side allowances between their boxes.

**Triangle budgets per template are reported, not enforced:** 150,000, 50,000, 12,000 and 2,000 at tiers 0 to 3. The numbers were guesses the chunk pass was to amend; it measured frames well inside budget and left them. The bake prints each template against them and refuses nothing; no shipped template is over. What is enforced is the script helper's rule that a damage state draws no more than its intact state at any tier.

**The asset bake and check need the WebAssembly built.** Descriptors and the catalogue hash are judged by the contract's own code through one new export, not by a second implementation or a native helper. `asset bake` fails without `bun run build:wasm`.

**The empty prototype set is gone; one unit box remains for props with no model.** While categories were being built, undressed templates drew as tinted stand-in boxes from a "prototype" set. Every template now has art, so the set and its command were deleted. `asset stand-in` writes the one box a prop without a model is drawn as, which the street-furniture lane uses.

### Drawing a town at scale

**One owner of static chunks, shared by scenery, corpses and buildings.** *Decision:* the bookkeeping that buckets static things into 64 m chunks, culls them and chooses a level moved out of the scenery layer into `frame/staticChunks.ts`. Each population gives its own rule for a chunk's level and its own bounds function. The alternative generalised the scenery file in place and had the models layer import from scenery. *Consequence:* corpses and trees draw exactly as before (their boxes are bit-identical), and the owner knows nothing of any layer's presentation.

**The whole map is expanded at the coarsest tier only.** Tier 3 of every building is one static population (22,340 records for 9,909 buildings on the largest map). Tiers 0 to 2 are expanded into a bounded pool for chunks in view and near enough. The alternative, the whole map at tier 2, would have materialised most of a town of houses, whose tier 2 is 12 to 95 rows each. *Consequence:* a chunk the pool cannot hold falls back to tier 3; no view tried ever reached that.

**A building's tier follows pixels per metre of wall, not its height on screen.** Thresholds are 10, 4 and 1.2 px a metre: at the battle camera tier 0 to about 130 m, tier 1 to 320 m, tier 2 to about a kilometre. *Rationale:* what a tier drops is windows and trim, the same size on a bungalow and a slab. By projected height a slab would keep its 149,000 triangles four times as far out as a house keeps its 2,750. *Consequence:* a tower is at tier 1 or coarser in most frames, so its far tiers carry its character. The thresholds were looked at again when thin bars aliased and were left: moving tier 0 nearer costs houses their window frames at the default camera.

**One tier per chunk, and a building belongs to the chunk it stands in.** The alternative was a tier per building chosen by walking buildings on every camera move. A view change walks chunks only, and a building can never be drawn at two tiers. The cost is a 64 m step in where a tier changes.

**The pool is kept kind by kind in ranges that grow.** One draw per module per tier whatever the camera. A region per chunk would be chunks times modules (1,250 draws a pass at the 250 m camera, six passes a frame). The cost is a second copy on the CPU for packing and calling the pool full at about four fifths.

**Residency follows the view, with a budget per frame.** A chunk is expanded while it is in view and near; a camera cut expands the nearest chunks first within a row budget and the rest draw coarse until caught up. *Consequence:* a scene that shoots after a cut waits for the pool to settle. There is no hysteresis at a tier's edge; none was needed in the pans flown.

**Near buildings cast shadows from their own rows a tier coarser.** The alternative, a plain shell as the caster, puts every recessed window in shadow on a sunlit facade. The cost is the near buildings' triangles again, a tier down, in each of four shadow cascades.

**The far tier is each template's own coarsest tier; no tile builder was written.** *Gap:* the far-tier slice was written when towns were boxes and asks for a builder of merged tiles from descriptor geometry. *Decision:* the slice closed without one. The sets' scripts already fold each template into a shell of 12 to 1,766 triangles (239 a building on the largest map), and the whole frame there is 11 ms against 15. A merged tile would save about 30 to 60 draws and add a vertex copy of every building; a builder from descriptors would draw boxes with roofs, which is less than the shells keep.

**Nothing was built to make a town read from the whole-map view.** At that height a house is about a pixel. No building tier can fix that; a built-up tint under settlements would, and it is the ground lane's material.

**Buildings draw through the models layer's own pipelines.** A kit is an installed appearance and a row is an ordinary model record, so a unit behind a building is x-rayed like behind any model. Draws after the first of each buffer go to the raw GPU pass, because the typed wrapper re-sets about eight things per draw.

**What a side knows changes records in place.** `setBuildings({ placed, fallen })`: a new `fallen` hides or shows the buildings that changed and rebuilds only the fallen population. The alternative rebuilt the whole map's population on every change (24 ms on the largest map).

**Two labs are where buildings are judged, and both draw by the game's own path.** `/lab/city-lineup` stands every template on flat ground at any tier, state and station; a tier is forced by handing the frame thresholds no view passes, not by a renderer switch, so a forced picture cannot differ from the game's. `/lab/city-block` is a block of a generated town with no battle. The per-set Blender sheet scripts were deleted once these existed; `assemble.py` remains as a quick look at an export before it is baked.

### How the art is made

**A far building is one row.** At tiers 2 and 3 a script folds what is left of each fitting (a dark pane a window, a box a chimney) into the template's own shell, so the whole-map population stays small and openings and colours survive to the last tier. A terrace keeps one row a house, because the houses' colours differ.

**Wall colour is a row's tint; roofs and brick are their own colour.** A row has one tint, so walls are pale tint-masked surfaces and the shell's row carries the building's colour. *Consequence:* the library can recolour a body without a new export, but not accents (a stair stripe, balcony fronts), which are baked.

**A tower's wall is panel modules near and a texture far.** One-bay panels at tier 0; from tier 1 the shell draws each run of like bays as one face sampling a facade picture generated from the same table of openings, so near and far cannot drift apart. A wall's face is the part's face, with glass and loggias recessed into the box, so what hides a unit is the plane the eye reads as the wall.

**Thin things are cutout sheets or thicker bars, never thin geometry.** A 2 cm bar is under half a pixel at the default camera and stipples. Where three or more thin bars stand in one plane (a window cage, a railing) they become one cutout face; any other bar is drawn at least 5 cm wide at tier 0. The scripted sets use no cutouts at all: their balconies have solid fronts, and a ground-floor grille aliased in front of the one floor whose rooms the street camera sees.

**Large marks are placed once, never painted into a repeating texture.** Roof stains, rust patches, soot fans and mended sheets are vertex paint or faces of the shell; a texture carries only what is smaller than a few metres. Three attempts taught this: stains in a tiling texture read as camouflage, and a recoloured tile set chequered from the air. The China roof and plinth take dull colours of our own for the same reason.

**The exporter writes its own tangents.** One export in three differed by one float, because the glTF exporter averages a smooth vertex's tangent in thread order. The script rewrites every tangent from the first triangle that uses the vertex, so the same inputs write the same bytes.

**The village's and the labs' houses are templates of the same library.** The fitted house path is deleted, so there is one way to draw a building. The three village houses are built exactly as before; a lab's box borrows the nearest house's look a tier coarser so twelve farms fit one kit (37 MiB against 81). *Consequence:* up close a lab's house has no glazing bars or door planks. You said not to polish the labs; you did not ask for them to be coarser.

### Facades: coverage, cutouts, glass and rooms

**A material says how it is covered: opaque, cutout or blended.** *Decision:* coverage is the base colour's alpha (one number: a pane's opacity) times the normal texture's alpha (per texel: a grille's holes). The alternative places collide: vertex alpha is wear, albedo alpha is the wear threshold, and a fourth texture breaks the three-slot limit. *Consequence:* this is not glTF's convention, so a cutout authored the glTF way would draw solid; the validator refuses any cutout or glass whose coverage never crosses its threshold. Bundle format 4 makes coverage a required field, so an old material cannot pass for opaque. Blender's exporter is not trusted to derive the alpha mode; the helpers write it after export.

**A mesh is ordered by surface kind and each kind has its own pipelines.** The alternatives were a branch and discard in the opaque shader (which costs every model early depth) or a second mesh per kind. A mesh that is opaque throughout is untouched.

**A cutout thins to a veil with distance; it does not stay a hard pattern.** Coverage is read through ordinary mips and becomes how many of a pixel's four samples are covered, rounded where the pixel resolves an edge and dithered where it does not. The alternative, coverage-preserving mips with a hard cutoff, keeps a grille solid until its bars pass under a pixel and then shimmers. The depth prepass alone decides the silhouette and the colour pass draws at equal depth, so the two cannot disagree. A cutout's shadow is the same share of shade; individual bars cast none.

**Glass draws last in the world pass, unsorted, and casts no shadow.** It is blended over what is behind by its opacity and writes no depth. Panes are not sorted by depth: for panes of one glass the order changes the result by under 1 of 255, and sorting is CPU work every frame over otherwise static records. *Consequence:* overlays, fog culling and effects do not know a pane is there; smoke behind a window is drawn over it, not dimmed by it. A red pane before a blue one would be wrong; nothing authors that.

**A pane's mirror is bounded so it never reads as a boarded window.** A glossy pane mirrors the pale horizon at a grazing angle. Two numbers in `presentation.glass` cap that, and a pane stays 0.25 to 0.42 of its wall's brightness under four suns. *Consequence:* with no sky gradient or glint a pane reads by darkening alone, which is the shopfront shortfall above. Opacity is the same from every angle; glass does not touch the fog mask (a pane is not the thing seen or unseen; what stands behind it is).

**A room behind a window is one open box showing a picture from an atlas.** Ten apartment rooms and ten shops, rendered by our own script with a path tracer on the CPU for repeatable bytes; never photographs. Each window picks a room by hashing its position, so it is the same room from every camera. The picture is shown unlit, at the brightness a matte surface has in sun shadow: with the open sun as reference, rooms on a shaded facade read as lights left on. *Consequence:* the sheet's level is one compromise between "a black hole at 80 m in the sun" and "lit on a shaded facade". From the steep tactical camera a window shows mostly the room's floor, which reads as a dark coloured opening, not furniture. Nothing glows: upstream's gain, depth falloff and lamp colour are not carried over.

**Room sheets ride inside the bundles that show them** as an ordinary texture, not as a third kind of runtime file. Each kit bundle with rooms carries about 1.4 MiB a sheet; on the GPU a sheet is one layer however many kits show it.

**Rooms draw at tiers 0 and 1; a tower's at tier 0 only.** A room box a window at the far tiers would be hundreds of thousands of records for windows of two to six pixels. A tower's room is inside its panel module, and panels are tier 0; from tier 1 its facade picture paints each pane the tone a room behind glass has at that distance, so the boundary does not flip a window dark. A house's room is a row of its own.

**In the scripted sets a window is a hole cut in the wall at the two near tiers.** Their walls are one face with no thickness, so a room behind a window was behind the wall. The helper cuts the shell where a row stands a window. A house's tier 1 rises by 16% to 48% in triangles.

**At a corner one window keeps its room and the other draws a blind.** Two walls' rooms want the same corner, and two boxes sharing space fight for depth. Squeezing the second room showed its pale walls just behind the glass, which read as an error. *Consequence:* a quarter of the houses' windows have a blind (83 of 342), four of the bungalow's nine; one tower panel in eight.

**Some glazing stays opaque or dark on purpose:** a barn's window is glass over a dark recess (a room there would be a bedroom in a barn), and factory wired glass, stair lights, fanlights and frosted bathroom glass stay opaque. A shop window has a stall board and boxes behind its glass as a placeholder for goods.

**China's curtains are fewer and faded.** The graph hangs a curtain in three windows of four in flag colours; with rooms behind, few windows showed one. The probability is 0.35 and the colours are pulled half way to grey.

### Destruction

**Damage states are made from the intact export, not by the graph.** *Gap:* the damage slice says to evaluate patched damage inputs in each graph; the spike found none to patch. *Decision:* the exporter cuts the same walls, piles rubble of the same materials over the same plan and chars or throws down the same fittings. Writing breach logic in geometry nodes inside a vendored file was the alternative. *Consequence:* a destroyed building is the same building by construction, at no second source.

**A ruin's doorway is where the door was.** The scripted sets read each wall's openings from the intact rows and break the stump to each sill, so no second table has to be kept in step. A terrace falls house by house in each house's own colour, unevenly. Timber and steel fall their own way (charred posts, torn cladding over leaning frames); the heap under a steel shed still reads as masonry rubble to a fresh eye.

**Burnt is a change of hue with black marks that never repeat.** A gutted block's wall goes to an even smoked grey, with a soot fan above most openings, bays blown out to the slab, and holes in the roof with something 3 m down in them. Darker paint read as shade; soot in a tiling texture read as camouflage; a flat dark roof patch read as a tree's shadow. A gutted tower is its shell built again in burnt facade pictures and darkens with height.

**The destroyed state is read from what the simulation already publishes.** *Gap:* the slice says "via the knowledge path", not which published fact tells ruin from gutted. *Decision:* a side draws a building gutted when it knows one of its parts as the prop type the building row names as its gutted shell, and as a ruin when it knows a part replaced by anything else. No publication change was needed. The alternatives were guessing from the remains' height or reading a presentation label. One known part is the whole building's state: the simulation destroys every part in one call and a side that sees one part learns all.

**A destroyed building smokes for the side that knows it.** Dark smoke off a gutted roof for three minutes and thin for fifteen more; pale dust off a ruin for a minute and thin smoke for ten. No flame. It starts when the side first learns of it, as a wreck's does, and shares the one smoke budget. It is a cue, not the fix for the far read; the numbers were chosen by eye from one battle.

**The proof is a scripted shelling, not a fight.** `/lab/city-ruins` plays a saved encounter in which the scenario's own blast emitter wears a block and a tower down through the ordinary damage and publication path while one side watches and the other stands behind two slabs. Tanks firing would add minutes of flight to a scene that judges knowledge. Nobody fires a weapon in it.

### Loading

**A page fetches a kit only when its map draws from it.** *Decision:* the one loader takes everything that is not a kit with the catalog, and `withKits(names)` fetches what is missing and installs a new generation, every hash checked. The alternatives were a second catalog file listing kits per map, or every appearance lazy. Which kits a map needs has one owner, the library's `templateKits`. *Consequence:* the village fetches its own 38.8 MB and no town kit; a lab with no buildings fetches none. Kits stay once fetched and nothing evicts them. The town's total is the shortfall listed above.

## C55/C58 play a generated battle

### The battle's address is its request, and a page per battle

**Choice:** The menu's `Deploy` is a link to `/battle?type=&size=&seed=`, a full navigation like every other menu entry. The battle page prepares what its address says. Cancel and a refusal's way out are links back to `/?type=&size=&seed=`, which reopens the menu on the same choice.

**Gap:** The slice asks that replacing or cancelling a pending request never start a stale battle, and that the menu show the share identity, without saying whether the menu and the battle are one page.

**Verdict:** sound. One request per page makes a stale battle impossible across requests (leaving the page closes the worker), the address is the share identity for free, and reload replays the same request. Inside the page the client still silences a cancelled request, which a test holds. An in-page flow would need client-side routing the app does not have. **Confidence:** high.

### The player's request is pinned on the page; the worker is handed the documents

**Choice:** `map_source.request` is the whole `GenerationRequest` (generator version, preset revision, catalogue hash, limits), pinned on the page by `generationRequest` from the build's own generator before the worker starts. The build's documents (rules, presets, templates, recipes) travel beside the request in the worker's message and are not part of it.

**Gap:** The slice names the request's fields but not who fills the pins, or whether the rules are part of a request.

**Verdict:** sound. A pinned request is what a replay must store to be refused by another build, and the generator already refuses a stale pin. The rules stay outside because a lab may pin its own; their digest is in the replay. Pinning costs the page one parse of the 47 KB template list. **Confidence:** high.

### `recipe_id` names a saved encounter on a catalogue map

**Choice:** On a generated map `recipe_id` is a recipe the planner places with `encounter_seed`. On a catalogue map it is the map's saved encounter of that name, and `encounter_seed` is not read.

**Gap:** The request has one `recipe_id` for both sources, and saved maps have no sites for the planner to read.

**Verdict:** provisional. It is what C58's saved `encounters/assault.json` needs, and it lets a lab map play through the same path today. The unread seed is a wart: when a saved generated map carries its `sites.json`, a catalogue map could be planned too, and the saved encounter would become a cache of that plan. **Confidence:** medium.

### The village and the labs keep their own scenario builders

**Choice:** The village's variants (`village_scenario`, the simulation's factory) and the labs' `savedBattle` were not moved onto `prepare`. They already reach their maps through the one resolver (`loadMap`).

**Gap:** The pass asked for one preparation contract with the village and labs working through the same path.

**Verdict:** provisional. The village's encounter is built by a factory from rules, not a file or a recipe, so it has no `recipe_id` without a special case on the map's id; the labs pin their own rules and compose scenario text whose bytes every lab scene was proved against. Moving either is a change to prove with every scene, which this pass was told not to run. What is shared: the map owner, the battle view, the objective readout and the replay file module. **Confidence:** medium.

### A replay stores the request, not the compiled battle

**Choice:** `{ request, replay }`. Playback prepares the request again; a mismatch is refused by the generator's pins and by the replay's scenario and rules digests.

**Gap:** The slice says runtime replay data stores the exact compiled map, encounter and rules.

**Verdict:** provisional, and a named departure. The stored-scenario form is a 10 to 15 MB file per replay and overflows the browser's storage for "the last saved battle"; the request form is a few hundred bytes and is refused, never replayed wrong, when the map would differ. What it does not give: playback after the generator changes (the slice wanted no older generator to be needed), and a build identity for simulation-code changes. **Confidence:** medium.

### The menu shows names, not extents, and one seed

**Choice:** Type and size are shown by name with one line on the type; the size's kilometres are not shown. The seed field is the map's; the encounter seed and battle seed are address parameters with fixture defaults.

**Gap:** The slice asks for the two composition controls and the seed, and leaves wording to the implementer.

**Verdict:** provisional. The extents are a constant in Rust (M04) and are not exported; printing them in the menu would be a second copy. One seed is what a player shares; three would be a form. **Confidence:** medium.

### `/lab/generated` is deleted

**Choice:** The developer route is gone. Fixture `generated` is now the production route `/battle`, and its scene starts from the main menu.

**Gap:** The pass allowed deleting it or keeping a thin entry.

**Verdict:** sound. The address takes the same parameters the lab did, so nothing a developer could do there is lost. `fixtures/generated-lab.json` became `fixtures/generated-battle.json`. **Confidence:** high.

### No generated map was saved

**Choice:** C58's map was not committed: 9 to 15 MB for a Mixed Small map against a 2 MB limit for this pass. Seed 1 is named as the one to save.

**Gap:** The pass set the limit and said to stop and report.

**Verdict:** open. The owner decides between a larger budget (the spec's is 25 MB per saved map), a compact saved form (a building as its template id and frame, materialized by the resolver, which already checks exactly that), or saving the request and generating at load. The second also needs the catalogue's admission raised and the prototype library reachable by the adapters. **Confidence:** high on the sizes.


## C62 ground evidence rig

### The class mask is a frame view the terrain writes, not a readback or a CPU picture

**Choice:** `ground-classes` is a `FrameView`, a named alternative picture of the same frame. In it the terrain's own fragment writes three bytes a pixel in place of its lit colour: the distance outside the paving, the distance outside the water, and a packed byte (forest floor or its verge, the plot's kind, two hashed bits of the plot's index). The fog mask pass keeps only pixels that are wholly ground, and post passes the bytes through untouched, which cost a third mode of `post.encode`. No paired frame-cost run was made for the switch: the final view gains one uniform comparison per terrain fragment.

**Gap:** The slice says the mask is "written by the terrain material's own shading function" and delegates the encoding; it also says the mask view "adds 0 ms to the final view" without saying how that is shown.

**Verdict:** sound. A mask computed anywhere else could drift from what is drawn. This one is the same site lookup the colour uses, lined up with the shot pixel for pixel, so every later scene check ("the road's core", "the wood's floor", "this plot") reads the ground the renderer drew. **Confidence:** high.

### Distances in the mask are exact only within the look's own reach

**Choice:** The mask holds the distance the material itself reads, and the material reads no farther than `groundReach`: 4.3 m outside a road's edge (set by a crossing's bars, C30), about 6 m outside a forest's edge, and 5 m outside the water or the top of its bank if that is farther. Past that reach a pixel keeps its side (inside or outside) and may read farther than the truth. The reach was not widened for the mask alone.

**Gap:** The slice asks for "road signed-distance bands, river bands"; later slices name bands out to 6 and 8 m.

**Verdict:** sound. Widening the reach only for the mask would make every lookup in the final view visit more records. Each slice that drew a wider band widened the reach to draw it, and the mask became exact over that band with it; the road checks' farthest band (5 to 9 m out) asks only "outside, clear of the road", which holds past the reach. **Confidence:** medium: a check that needs an exact distance past 4.3 m from a road would read a wrong one without failing.

### Black means "not wholly ground", and grass and water are left out of the view

**Choice:** A mask pixel is black where any sample is a building, tree, unit or the backdrop. The view skips the grass and the water surface, so the ground under them is read; to read the floor under a wood the mask is shot with the trees off.

**Gap:** The slice does not say what a non-ground pixel holds.

**Verdict:** sound. A mixed pixel would decode to a wrong class, and the river's bed has to show to carry its distance. **Confidence:** high.

### Generated-map stations follow the map's own report, and the rig rebuilds the ground in the page

**Choice:** A station is a named camera pose the rig shoots. On the village and the river lab the stations are coordinates; on the generated map they are functions of the preparation report's anchors (the objective town, blue's start, the map's size). For the stations that stand on the renderer's own ground (the town's edge, a road's join) and for `drawnRoads`, `_groundStations.mjs` rebuilds the terrain surface in the page from the page's own generation request, because no probe of the lab reaches it.

**Gap:** "Lab maps add theirs"; the lane asks for a generated map at the tactical camera and close, and the lab is not this lane's to change.

**Verdict:** sound. The layout generator's version moves often, and fixed coordinates would land in a different field each time; the rebuild costs under a second a page. No station stands on a generated forest edge, river or tree line. **Confidence:** high.

### The forest stations look down at 0.6 rad

**Choice:** `forest-deep-25` and the two forest-floor stations are 25 m away at 0.6 rad, above the canopy, not at the ground view's 0.32 rad. No pose under the crowns was added.

**Gap:** The slice names the station, not its pitch.

**Verdict:** sound. At 0.32 rad the eye is 8 m up, inside a crown, and the frame is one leaf. C76 judged closure from the class mask at 65 m and 120 m and did not need a pose under the crowns. **Confidence:** medium.

## C70 river bank bands

### The bank is wet silt, then bare earth whose outer line wanders in

**Choice:** Two bands by the simulation's distance from the water's edge (`groundShore`, the biome's `shore` row): wet silt for 0.6 m, then bare earth out to 5 m at most. The earth's outer line wanders in toward the water by up to half that reach and never out past it, on two octaves of value noise laid on lattices turned from the map's axes and from each other. Grass stands nowhere on the wet bank and thickens all the way across the earth.

**Gap:** "Band widths, palettes, noise" are delegated; the slice names bed, wet bank, mud and grass.

**Verdict:** sound for the bands and the bounded reach: because the line only wanders inward, nothing reads the water's distance past 5 m. Provisional for the line. Unprimed critiques called it scalloped, then "regular lobes" at 250 m, and the bank is one width on both sides of every bend (no point bar, no cut bank), which reads as a stripe from high up. A line that is not value noise, or bank art that differs between the inside and outside of a bend, would settle it. **Confidence:** medium.

### The bank is at least 1.27 times as light as the field it lies on, in a grey silt and a pale grey-brown earth

**Choice:** Both bands are drawn at least `shore.lift` (1.27) times as light as the plot under them, in their own hue: the palette's colour is scaled up wherever it is darker than that. The plot's lightness is its own colour with its rows at their mean, eased to the verge's over 10 m toward the plot's edge. `palettes.shore` is silt `[0.3, 0.295, 0.285]` and earth `[0.325, 0.305, 0.28]`. The lift was 1.15 and the palette a tan until the fields went olive and tan themselves (C84) and a bank lay beside rapeseed; it was recoloured three times, the last after a fresh eye read a browner earth as "an orange outline round the river".

**Gap:** The slice says "at or above grass luminance, differing by hue" and does not say which grass: fields run from dark ploughed earth to stubble twice as light.

**Verdict:** provisional. A fixed palette light enough for wheat was a pale road beside every dark field; a floor against the pixel's own ground carried each furrow into the bank; a floor against the plot stepped at every plot edge the river meets, hence the easing. The river scene passes on every bank (no darker than the grass; 8 to 18 apart in hue against a bar of 6). The final colour has had no critique of its own and reads as a pale sandy shore; one unprimed look at the river stations would settle it. **Confidence:** medium.

### A bank's shading shows half its true slope

**Choice:** `shore.relief` (0.5) scales the slope the bank is shaded by (`groundBank`). The biome refuses a value over 1.

**Gap:** The rivers slice left "the bank reading as a slope rather than a smudge at a low sun" to the look.

**Verdict:** provisional. The lab's banks slope 14° under a sun 17° up: lit as cut, the bank facing away from the sun takes almost no direct light and reads as a dark band with nothing above it to cast it. At half the slope it keeps about three fifths of the flat ground's light. Critiques still call the bank a flat stripe at every height; more relief needs more lift on the far bank to keep the luminance rule. **Confidence:** medium.

### The water's light is drawn into its colour, in lanes that hold their distance from the bank

**Choice:** The surface's colour carries streaks of light (`waterSurface`): noise in the distance from the water's edge, in lanes 0.5 m wide broken along the stream into streaks, fading to its mean as a pixel outgrows a ripple. The surface clears to the bed over 0.35 m at its edge (the biome's `water` row). The ripples' normal is unchanged.

**Gap:** The rivers slice left "the water's own look from above" and "ripples and glints on every reach".

**Verdict:** provisional. From above, a reflection shows ripples only where the sun lies behind the water, so without drawn light a reach is a flat band. Lanes by distance run with the channel round every bend and need no along-stream coordinate. A critique read them at 250 m as lane markings and from straight above as paint smears; the water does not move, and a straight river still reads as a canal because its width is the simulation's. **Confidence:** medium.

### The bank's luminance rule is checked on rendered frames, a bank at a time

**Choice:** The `river` scene walks sections across each bank at two stations and compares the frame's luminance on the wet bank and on the earth with the grass on the same section, as medians per bank. The rule is "no darker than the grass, and apart from it in hue by more than 6 in CIELAB's a\*b\* plane". That bar of 6 is the one the road checks reuse (C31).

**Gap:** "Luminance per band ≥ grass" does not say where or how.

**Verdict:** sound. Pooled over both banks, the sunlit bank's surplus hid the bank facing away from the sun: the old dark shore read 1.04 times the grass pooled and 0.58 on the far bank alone. **Confidence:** high.

### The water's reach is the earth's

**Choice:** `groundReach`'s water term reads the earth's reach (`shore.mud_m`, 5 m) in place of the old shore width, or the bank's top where that is farther.

**Gap:** The brief asks to widen the reach to what the bands read.

**Verdict:** sound. The bank's shading already reads 12 m from the water in the lab and on generated maps, farther than the bands' 5 m, so the surface field lists the same records as before. **Confidence:** high.

## C71 river bank roundness

### The bank's shading normal is bounded, and its roundness is a scene check

**Choice:** The slope `groundBank` shades by is the bank's own times `shore.relief`, never past tan 40° (`MAX_SLOPE`, the bound scars already had). Its blend between neighbouring stretches is the rivers slice's. The `river` scene walks each bank and holds the shading's step from sample to sample under 8% of the flat ground's luminance.

**Gap:** The slice was written before the rivers slice landed the rounded shading as its own fallback.

**Verdict:** sound. Walking each bank the shading steps by at most 1.5%; with the bank's shading off the same frames step by 18.7%. No map the contract admits has a bank past 35°, so the bound cannot be reached in a test. The land past the bank's top is still lit as if flat. **Confidence:** high.

## C06 per-tick geometry bounds

### Cache derived bounds only while their authoritative positions are stable

**Choice:** Sensing computes each unit's footprint radius once per immutable
call. Movement gathers living infantry in unit order and keeps their derived
radii beside the existing crowd, refreshing them after squad motion or a shove.

**Gap:** The spec delegates measured cost reductions but does not choose a
cache owner or lifetime.

**Verdict:** sound. Recomputing a squad's radius for every vehicle or observer
repeats a scan of its soldiers. These short-lived bounds have explicit refresh
owners and preserve collision and sensing results. Fallen squads remain in the
battle and observations; they add no traffic work per vehicle. **Confidence:** high.

## C06 engagement-search bounds

### Reject only an engagement search proven empty by physical reach

**Choice:** Cover planning skips a fight search only when every target lies
beyond weapon range plus the squad-area radius and maximum lean distance.
Boundary cases retain the ordinary search with a micrometre of rounding slack.

**Gap:** The cost slice delegates reductions without choosing how to bound
failed cover searches.

**Verdict:** sound. Every candidate starts inside the area, and every lean is
bounded by the existing physical rule; three-dimensional distance is at least
its horizontal distance. This saves searching thousands of positions without
changing chosen cover or planning timing. **Confidence:** high.

### Sound — medium confidence: fixed-row reuse follows exact content, not entity identity

**When:** C07 sparse-row pass.

**Choice:** Index the exact float32 words of each retained fixed row, then
assemble the new group in its simulation-supplied order using source spans and
literal spans. For example, learning a new corpse between two hundred known
corpses sends that new row and instructions to copy the known neighbors. If a
known corpse changes place, its changed row is sent literally or through the
cheaper existing word-replacement arm. Identical duplicate rows may reuse the
same source because their complete words are equal. The alternative would key
rows by soldier/body ids, adding deletion and identity rules that different
kinds of observations do not all share.

**Gap:** Measurements showed group-local word replacements still resend sorted
retained tails. The spec did not prescribe how to find unchanged rows after
insertion or reordering.

**Reach:** This arm applies to every fixed-row group, preserves spatial corpse
ordering and needs no new simulation identities. Groups with variable sections
keep their existing word encoding. The temporary index adds bounded storage and
sorting work, which must pass the matched active cost gate.

**Verdict:** Sound: exact content comparison avoids hash collisions and hidden
identity assumptions, while choosing the minimum payload preserves sparse-field
savings. **Confidence:** Medium, because actual active instruction/byte gates
remain open.

### Sound — high confidence: copies reconstruct the whole group before commit

**When:** C07 sparse-row pass.

**Choice:** A source-copy operation reads an aligned span from the immutable
previous group; a literal operation supplies new row words. Operations fill the
new group sequentially, so reordering and removal require no mutable edit list
and a missing word is rejected. If a malformed generation copies beyond the
previous group, the decoder keeps its old baseline and accepts the corrected
same generation. New sides/epochs still require snapshots. The alternative,
editing the previous array in place, could both overwrite later copy sources and
change an observation the page already retained.

**Gap:** The new representation required source validation and failure rules;
existing generation/epoch checks remain the shared publication boundary.

**Reach:** The published metadata calls the selector an encoding and names its
three modes explicitly. Source indices, counts and copied words remain exact
inside the existing record allowance. Publisher index allocation and wire
admission are fallible before output/cursor commit.

**Verdict:** Sound: complete, immutable reconstruction retains atomic side
knowledge and the existing recovery contract. **Confidence:** High.
## C06 spatial fog invalidation

### Mutation stamps belong to the footprint index

**Choice:** When a tree falls far from an observer, the observer's cached solid
heights remain usable. The footprint index already divides the world into
32 m buckets to find nearby bodies. World alone enables a private last-change
number in each bucket. Insertion and deletion stamp affected buckets; movement
stamps both the old and new footprints. A removed body's bucket retains its
number even when empty. After any world change, a fog tile compares the largest
number among the same buckets its body query uses. If unchanged, its raster
(the solid height at each fog-cell centre) is still exact. The global revision
remains the first cheap check while no body changes at all.

**Gap:** The slice delegates dirty bookkeeping but does not specify storage or
a retained-history policy.

**Reach:** The additional arrays depend on physical extent, not battle history:
one number per world bucket and a second number per existing fog tile. They
add 6.25 MB at 20 × 20 km with 8 m fog cells; side-known indexes allocate no
stamp arrays. More precise footprint-shaped invalidation could save some
boundary rebuilds but would add bookkeeping beyond the index's bucket bounds.
This uses the existing
conservative body query without new sampling or changed sight rules.

**Verdict:** sound. Unrelated changes no longer force active eyes to sort and
raster nearby bodies again, and no changed-prefix scan can miss a later update.
**Confidence:** high; fresh-sweep and cost regressions remain the acceptance seams.
## Road joints and ends
## C13/C32 kits and the template art library

### A sharp joint is one road round a bend, never a cut heel

**Choice:** Two carriageways of one kind and width that meet end to end are one stroke round whatever corner they make, short of an exact reversal (within 5°). The 110° limit for country roads and tracks is gone, and so is the rule that ran one fork arm on over the other's end. At a junction of three or more, the straightest pair of alike ends is the road through it at any angle, not only within 30°.

**Gap:** The user called the closed fork (two tracks at about 40°, one running half a width past the point and ending flat) a weird corner. C65's rounded centreline can turn that sharply: the bend's tangent at its apex is the bisector of the two runs, only an exact reversal has none, and the outside of the bend is then the stroke's own round join, which the square-end contract leaves alone more than half a width from an end.

**Verdict:** sound. The same fork is now a switchback with a round outside (`throwaway/road-ends/forks-after/mixed-track-fork.png`). `no_road_turns_back_on_itself` now bounds a turn at 175°, the reversal the weld refuses. **Confidence:** high.

### Where unlike roads meet at a corner the wider one turns it and narrows

**Choice:** The widest end at a point with no road through it gains a last run along the narrower way that leaves most nearly straight on: one and a half of its widths long, at least its half width plus a metre, stopping that way's width short of its first turn. Its outer corner is its own round bend, the narrower way starts under it, and its square end lies across the narrower one with a shoulder either side. The last run ends up to 5 cm to the far side of the narrower way's line, so the two middles cross whatever rounding does. Any other end at that point joins the wider road as it would anywhere along it.

**Gap:** Two strokes of different widths cannot be one stroke, and whichever ended at the shared point showed a flat heel on the outside of the corner. The earlier rules (mitre, run-on, run-back) each left one.

**Verdict:** provisional. It reads as a road that turns and then narrows by a step; a taper would read better and a constant-width stroke cannot draw one. **Confidence:** medium.

### A branch that comes in at a slant curves round to meet the road square

**Choice:** An end that joins a carriageway more than 30° off square, and not within 20° of running alongside it, leaves its own line where the road's middle is still its half width plus a width and a half away, turns in two halves a width apart, and goes straight to the nearest point of the road's middle, a quarter of a metre past it. With less room it tries half that depth; a last run under four widths long swings whole instead. A wider end whose corners would show past a narrower road's far edge is treated the same way.

**Gap:** An acute fork is where a flat end shows: the end's corner stands out past the other road's edge on the open side. A real track swings round to meet a road it joins. A lane that peels off within 20° is left: squaring it moves the join far along the road and took a farm's lane away from the farm (two refusals in the one 900-map sweep, open medium 49 and one mixed medium).

**Verdict:** provisional. The join moves along the road by up to the depth over the tangent of the angle, about 15 m at 35°. **Confidence:** medium.

### Stubs, doubled streets and short links are tidied before joints are made

**Choice:** Three more steps. An end that stops within two of its widths past a road it crossed is cut back to it. A street drawn beside another (under their two half widths apart, running the same way) is cut back to the last road that crosses it within ten widths, or, with nothing between them, the two ends meet half way and weld. A turn within the widest road's half width plus a metre of an end is dropped, so the end's last run is long enough to move, meet or turn along; a street's link to the road it joins is a metre or two long, and blocked every one of those.

**Gap:** These were most of the bites: an avenue that ran 15 m past its last cross street beside the street it should have cornered with, the offset grids of neighbouring districts, and corners the pass could not make because one arm's last run was 1 to 3 m.

**Verdict:** sound for what the sweep counts. **Confidence:** medium.

### A joint is kept if both roads still join one third road there

**Choice:** The pass's own guard no longer asks that every pair of centrelines that crossed still cross. A pair is kept if they cross, are one road, or both cross a third road within 25 m of where they crossed each other.

**Gap:** Three roads that shared a point are one junction when two of them end on the third. The stricter guard read that as a lost joint between the two, pinned all three as laid and left the heel.

**Verdict:** sound: the plan's road graph still connects them through the third road. **Confidence:** medium.

### What is left: 9 ends in ten thousand, counted with heels

**Choice:** `tests/road_ends.rs` drops the `Corner` class it counted as sound (a free face with a road joining within two widths) and counts it as a `Heel`, with a wider road's end that narrower roads cover in part and none carries on from. Bites, steps and heels together may be 10 in ten thousand ends. Only carriageway strokes count as joining or covering an end; a yard in front of a building is paving and no road.

**Gap:** The old bound (2 in a thousand) counted bites only and called the fork's heel sound.

**Verdict:** provisional. By the new count the previous pass left 355 of 25,302 ends (140 in ten thousand; 328 heels, 27 bites); this one leaves 22 of 25,014 (8.8; 2 heels, 20 bites) over the same 36 maps. The rest are junctions where three ends stand a few metres apart without sharing a point, and a wider road that ends at a slant on a narrower one with no room to turn. **Confidence:** high for the count.

### The lab roads run to the map's edge; the village's two roads join in a T

**Choice:** The deployment, geometry and movement labs' roads run to the map's edge where they stopped 10 to 20 m short of it. The village's east-west road runs on to the west edge, and the road that shared its start at (140, 780) now curves to meet it square at (159, 780), as the generator would lay it. Each map's `SOURCES.json` hash follows. The village scene's road-edge oracle reads the road's longest run, which is the ground it read before.

**Gap:** Roads that stop in open ground short of the edge, and two road ends sharing a point at 52°, which showed a bite on the outside.

**Verdict:** sound, with a named move: the village edit changes physical ground, and the quick village report's three flank digests move (blue cost lost 1805 to 1792, rejoined 5 to 6, captures and tanks lost unchanged); the three ambush digests do not. The river lab is not edited: its road ends on the map are what `terrainSurface.test.ts` samples to hold the renderer's square ends to the simulation's. The movement lab's road to (40, 230) still stops in open ground, 170 m from any edge. **Confidence:** high.

### The unprimed critique: the step where a road narrows still reads as a flat end

**Choice:** Nothing was changed after the critique. Its findings are recorded here for the next pass.

**Gap:** One unprimed reviewer was given eleven close shots and ten 3x crops and asked whether any road corner or junction looks wrong. Its three worst, all high confidence, are the same thing: where a wider road turns a corner and narrows, the wide strip "ends in a flat, square-cut end and the narrow strip pokes out of its middle", with a square shoulder either side (`forks-after/fork-avenue-corner-2.png`, `fork-road-turns-onto-track.png`, `fork-avenue-corner.png`). At medium confidence it calls the switchback's outside "a bulbous cap wider than either track" with a flat facet (`mixed-track-fork.png`, `fork-street-sharp-bend.png`), and sees a concave notch where a squared branch leaves the outside of a curving road (`fork-slanted-branch.png`). It passes the plain T-junctions and the squared tracks.

**Verdict:** open. The step is the rule above working as written, and it is a flat end showing past a narrower road's edges on both sides. `tests/road_ends.rs` classes it `Narrows` and counts it sound: 340 of 25,014 ends, against 22 counted as flaws. So the count of 9 in ten thousand does not include the defect the reviewer ranks first. A stroke has one width, so the pass cannot taper it; removing the step needs either a width that varies along a stroke (a change to the shared stroke contract, in the simulation, the export and the renderer) or a layout that never asks a wide road to become a narrow one at a corner. The switchback's cap is the bend's own round joins and is the shape asked for; its facet is the bend's short stretches showing. **Confidence:** high that the step reads wrong; medium on the rest.

## Compact saved maps

The contract these decisions belong to is in the [C58 outcome](slices/C58-offline-encounter.md#outcome).

### A map's content hash is the hash of its resolved definition

**Choice:** `map_hash` stays what it was: the hash of the resolved `MapDefinition`, buildings materialized. The resolver materializes first and hashes after.

**Gap:** The pass said to store a building as its template and frame, and did not say what the identity then covers.

**Verdict:** sound. No authored map's `SOURCES.json` hash changed, which is itself the proof that the conversion moved nothing, and a generated map's saved identity is the generator's own. The hash now also covers the library's geometry: a template edit changes the hash of every map that uses it, which the old form caught as a template mismatch instead. **Confidence:** high.

### One map type, generic over what a building is

**Choice:** `MapDefinition<B = BuildingDefinition>`, with `SavedMap = MapDefinition<SavedBuilding>`. Every existing use of `MapDefinition` is the resolved map, unchanged.

**Gap:** The saved and resolved maps differ only in their buildings.

**Verdict:** sound. A second struct would repeat twelve fields and their number readers, and a new map field would have to be added twice. **Confidence:** high.

### A saved building keeps `kind`, `owner` and `parts`; category and family are the template's

**Choice:** `{ owner, kind, template_id, frame, parts }`. `category` and `regional_family` are no longer stored.

**Gap:** "Plus whatever is genuinely per-building, such as its ids."

**Verdict:** sound. The resolver already refused a building whose category or family differed from its template's, so they were never the building's own. `parts` could be derived when ids are dense and in template order, and was kept explicit: the village's props and buildings share one id space that the map authors. **Confidence:** high.

### `SOURCES.json` names the library by file name

**Choice:** `catalogue.library` is a file name in `fixtures/` (lowercase, `.json`, never a path). Every folder states it; there is no default. The adapters read the sources first and fetch that file.

**Gap:** The pass asked that the library be something the sources name and the adapters honour.

**Verdict:** sound. A name is only an address: the map's catalogue hash still decides whether the library is the right one, so a wrong name cannot admit a wrong library. JavaScript reads the name itself (one regular expression beside the resolver's check) instead of a second Wasm call. **Confidence:** high.

### The resolver reads a library as a catalogue or as a descriptor list

**Choice:** `building-templates.json` is a canonical catalogue (`{ hash, templates }`); `prototype-building-templates.json` is the descriptor list the generator reads. The resolver takes either.

**Gap:** The two library files have different shapes, and other work owns both files.

**Verdict:** provisional. Converting either file was out of this pass's reach. One shape for both is the cleaner end, and it is the template library's decision. **Confidence:** medium.

### The catalogue admits what the generator may make

**Choice:** `MapAdmission::CATALOGUE` is 60,000 parts and 600,000 bay positions, the limits of `fixtures/generated-battle.json`, with a test holding them equal.

**Gap:** "Set the admission from what a full generated map needs."

**Verdict:** sound. One number for "a map the game can make" and "a map the game can save". The constant is in Rust because both adapters must agree without reading a file; the test is what ties it to the fixture. **Confidence:** high.

### Preparation takes the resolver's text

**Choice:** `ResolvedMap` in JavaScript carries `json`, the definition as the resolver printed it, and the catalogue arm of preparation splices that into the scenario.

**Gap:** A saved map's definition used to be printed again by JavaScript, which loses the sign of a zero. The old test for it (print the definition and resolve it again) has no meaning once the saved form differs from the resolved one.

**Verdict:** sound. The generated arm already worked this way. The labs still print the parsed definition; a test now holds every authored map to surviving that. Seed 1's resolved map happens to hold no negative zero, so nothing moved either way. **Confidence:** high.

### The saved map is `market-town`, and the menu lists the catalogue

**Choice:** The folder is `market-town`, labelled "Market Town". The menu shows every released playable map that has the game's default encounter saved on it, as `Play <label>`, under `Play village`.

**Gap:** The pass asked for a menu entry beside the village and left the name and the rule.

**Verdict:** provisional. The listing rule means the next saved map needs no code. The village stays its own entry because its battle is a factory, not a saved encounter. The name is a placeholder a person can change in `meta.json`; the id is in the address. **Confidence:** medium.

### `sites.json` stays in the folder, and the report saves the encounter

**Choice:** The CLI's `sites.json` is committed beside the map, and `encounter_report --save` writes the planned encounter's setup as `encounters/<recipe>.json`.

**Gap:** The pass asked that the saved folder be reproducible, and the planner is the simulation's, not the map generator's.

**Verdict:** provisional. 22 KB buys planning the encounter again without generating the map, and is the input a planned catalogue map would need. Nothing checks it against the map. **Confidence:** medium.

### `mapgen request` prints the game's pinned request

**Choice:** A CLI command makes the request for a type, size and seed, pinned to the generator, the presets' revision and the catalogue's hash, under the game's limits.

**Gap:** Nothing outside the browser could make the request the menu makes.

**Verdict:** sound. Without it the saved map's request was hand-written JSON with a hash in it. **Confidence:** high.

### A far building is one row

**Choice:** At tiers 0 and 1 a kit mesh is a row. At tiers 2 and 3 what is left of it is folded into the template's shell, whose walls are then flat with each opening one dark quad on them, so a template is one row there. The alternative kept every window a row at every tier.

**Gap:** "A template's coarsest tier is little more than its shell" and the 2,000 triangle budget, with 336 windows on the largest template.

**Reach:** 16,000 far buildings are 16,000 instances, not millions. A shell stores its template's far triangles (7,300 and 1,500 on the largest), once per template.

**Verdict:** sound. **Confidence:** high.

### Tiers come from one rule, the smallest feature a tier keeps

**Choice:** `detail.py` simplifies a kit mesh island by island: features under 5 cm, 20 cm, 60 cm and 1.5 m go at tiers 0 to 3 (dropped, or replaced by a ribbon, a bar, a box or a clustered mesh). The sizes were tuned until the 53 x 14 m, 8-floor slab fits every budget, so the smaller templates sit well under theirs. The graph's own LOD input only switches families off and is used for nothing; `mesh_lods.py`'s decimation leaves any island of 32 triangles or fewer alone, which is most of a cage or an air conditioner, and its coarsest collapse is not reproducible.

**Gap:** "Use the graph's own LOD kit where it helps, decimation where it does not."

**Reach:** Tier 0 is not the source's full mesh: an air conditioner is about 290 triangles of 1,700, a cage 150 of 880. Nothing in it calls a Blender operator, so two runs write the same bytes.

**Verdict:** provisional. It reads right at 30 m and 80 m in the reassembly; the renderer's own tier distances will say whether tier 1 can afford more. **Confidence:** medium.

### The kit's textures are 256 px, nine recipes shared by 38 materials

**Choice:** Every texture shipped today is 256 px (321 images in 38 sources), so the kit's are too. Nine ambientCG sets are baked (plaster, concrete, stone tile, roof tile, metal, corrugated steel, plastic, fabric, wood floor); a material is a recipe at its own base colour, roughness and metalness, and the seventeen sets the graph also reads are stood in for that way (black steel is the metal recipe darkened; rust, brick and soil are tinted concrete). `textures.attach(worn=False)` keeps the factors and writes no wear colour.

**Gap:** C12 left the edge and the packing to the implementer.

**Reach:** 27 new 256 px layers, 1.8 MB of PNG in the source. No surface of the kit can wear: its vertex-colour alpha is zero and its materials name no wear colour (L9).

**Verdict:** sound for the edge; provisional for the stand-ins, which are judged at 30 m only. **Confidence:** medium.

### Opaque glass hides what is behind it, so it is not exported

**Choice:** Glass is opaque, dark and glossy until C25. Left out with it, and switched back on by `china.py`'s tables: room boxes and curtains behind every window, shop interiors behind glazed fronts, laundry and the door inside a glazed-in balcony. Rooms and interiors behind an open stall stay. Also left out until cutout (C24): the rain-streak decals and the pot plants (leaf cards on a pot). Frosted glass and the shops' PVC strip curtains are opaque flat colours.

**Gap:** The task said to make glass opaque and leave decals and leaf cards out; it did not say what to do with what opaque glass hides.

**Reach:** Windows are dark panes; the curtains that give the source much of its colour are gone until glass lands. About a third of the graph's instances are not exported.

**Verdict:** sound. Drawing them would spend a third of tier 0 on triangles no camera can see. **Confidence:** high.

### Grime is in the wall texture, and the roof is quieter than the source's

**Choice:** The stucco recipe covers 4.8 m (the set twice each way) with damp blotches and rain streaks burnt in, so streaks do not repeat bay to bay. The roof tile recipe covers 4.8 m too, darker and less saturated than the source's (base 0.8, 0.7, 0.66 against 1, 0.92, 0.88) with heavier dirt.

**Gap:** "Grime is burned into the texture"; the source's own grime is the decals left out above.

**Verdict:** provisional. An unprimed critique called the source-coloured roof "carpet or a toy" and the largest surface at the play camera. **Confidence:** medium.

### Sign text is the kit's own generic words

**Choice:** Shop signs keep the kit's text meshes: words for trades (tea, pharmacy, fast food, hotel). The one that names a real city (Lanzhou noodles) is swapped for another. The readme's rule now allows a generic word for a trade.

**Gap:** The readme asked for the project's invented-name list. There is none yet, and the file carries no CJK font to set new text with.

**Verdict:** provisional: the user may still want invented names. **Confidence:** medium.

### What of the graph is not a building of ours

**Choice:** Switched off by input: the sidewalk, street trees, lamps and props, the shops' pavement clutter and parked scooters (street props without bodies, up to 2 m in front of a door), the rooftop sign, lit rooms. Dropped from the shell: the overhead cables (8,500 triangles of 2 cm tube). Not restored: drainpipes. The vendored file's five Object Info nodes for the pipes and the scooter point at no object, so the graph as vendored makes none; relinking them is a five-line patch nobody has asked for.

**Verdict:** sound. **Confidence:** high.

### Wall colour is the shell row's tint

**Choice:** The graph's walls stay white and tint-masked; each template's shell row carries its colour (cream, pale yellow, grey-green, pink, blue-grey). Balconies and the stair house stay white, as the source has them.

**Reach:** The library can recolour a block without a new export.

**Verdict:** sound. **Confidence:** high.

### No module variants

**Choice:** The China graph only yaws and scales its instances, so the exporter refuses a row that tilts or mirrors instead of carrying an untested variant path. New York mirrors a few; its exporter adds the path.

**Verdict:** sound. **Confidence:** high.

## Surroundings for the existing maps: cut

**Choice:** C56 (reservations) and C34, C35, C36 (surroundings for the village, the labs and the benchmark fields) are removed from the spec, with the lane that was written for them. The old maps stay the small arenas they are.

**Why:** the owner's call (2026-10-01): the village and the lab maps are developer test arenas, nobody plays them, and the game is not finished. The need those slices answered, a full-size map to play, is met by generated battles from the menu and saved generated maps. Where another slice still says C56 or C34–C36, read "cut".

**Verdict:** sound. **Confidence:** high.

## C20 renderer fog at scale — startup implementation pass

### Use exact per-eye grid and angular candidates while the S4 verdict is absent

- **When:** C20 startup implementation pass.
- **The choice:** Keep the existing sharp ray–box intersections, and use the existing grid of building footprints to give each eye only nearby boxes. On a 10 km city, a squad beside one block tests that block's neighbours instead of every building at the far end of the city. A first measured grid-only arm still cost 2.61 ms, exceeding the 2 ms gate, so the same table now narrows each ray to a conservative angular sector. Bounding circles enclose turned boxes, eye-inside circles retain every direction, and an extra sector at each edge covers f32 angle rounding. Rasterising building tops would instead replace the exact corner geometry.
- **The gap:** C20 says to implement the technique S4 picked, but only S4's plan exists; its named verdict file is absent. Grid representation itself is delegated, but selecting the unrecorded arm is a spec gap.
- **The reach:** Future fog work keeps the same horizon precision and sharpness. If the paired city measurement misses the budget, this arm needs further work rather than a silently relaxed gate.
- **Verdict:** Sound within the startup contract. Exact intersections and whole-building visibility remain authoritative. Missing full S4/G0 envelope admission remains separate from this local-work design.
- **Confidence:** Medium.

### Put candidate lists in the existing rebuild table

- **When:** C20 startup implementation pass.
- **The choice:** The table saying which eyes rebuild also carries each eye's nearby building indices. Before rebuilding an eye, the CPU writes its eye number, its angular-sector table offset and count, then each sector's list offset/count and sorted building indices. Terrain reads the eye number; occluder rays read their sector. Both passes share this table. The alternative would add another storage buffer and binding solely for candidates.
- **The gap:** The spec does not define the CPU-to-GPU list layout.
- **The reach:** This keeps the horizon passes' existing GPU binding count. The whole-surface pass separately keeps a reachable-structure/eye-pair table because both lists are consumed by commands in the same submission and cannot overwrite one another. Its one additional binding stays within the guaranteed storage limit. The horizon table can grow when a denser nearby neighbourhood arrives, so probes must construct their sampling bind group after preparation; otherwise its dummy table binding can refer to a destroyed old buffer. The regression test covers a probe before the first rendered frame.
- **Verdict:** Sound. One owner carries the rebuild transaction and its variable data without spending another scarce storage binding.
- **Confidence:** High.

### Detect occluder changes by geometry, independent of row identity

- **When:** C20 startup implementation pass.
- **The choice:** Compare complete box geometry before and after a publication, rather than treating a moved row as a changed building. If a demolished house disappears from the first row, the remaining unchanged houses shift indices but keep every distant squad's map. Only eyes in reach of geometry that disappeared or appeared rebuild. Two identical overlapping boxes have the same effect as one, so removing one duplicate alone does not invalidate maps.
- **The gap:** Fog occluders have no stable identifiers in the existing input contract, and C20 does not prescribe change identity.
- **The reach:** Future callers may reorder or recreate equivalent records without starting a rebuild storm. All geometry fields participate, including height changes and moved boxes' old and new locations.
- **Verdict:** Sound. Map validity follows physical occlusion, while the separately rebuilt whole-structure table uses the new row indices.
- **Confidence:** High.

### Preserve whole-fog roof reach while making structure work local

- **When:** C20 whole-surface followup.
- **The choice:** A structure may become visible when its roof probe looks inward toward an eye even though its footprint is just outside that eye's reach. Keep the whole pass's existing broad bounding-circle test, rather than reusing the tighter horizon footprint query unchanged. Its grid query grows by the largest known box radius, then filters each box against the original eye-plus-box circle bound. Each retained structure lists only the eyes that can reach it; structures outside every eye's reach receive cleared flags.
- **The gap:** C20 names the horizon merge and invalidation sites but does not say how its cost contract applies to the later whole-surface visibility pass. That pass otherwise still samples every global box against every eye.
- **The reach:** The whole pass has one additional storage binding for structure/eye pairs, seven total across its two groups. Moving away from a previously visible building clears that building's whole-fog flag, and inward-looking roofs keep their earlier visibility rule.
- **Verdict:** Sound. The existing whole-fog semantics are retained while both dimensions of its sampled work follow nearby structure/eye pairs. It preserves the original roof reach while distant structures do no sampled work; flags must clear when the eyes leave.
- **Confidence:** High.

### Sound — medium confidence on broader art: accept the preserved corner look

**Choice:** After the non-blocking five-minute Preview checkpoint received no
feedback, retain the unchanged village corner appearance. Exact horizon and
whole-building visibility remain the authority; existing roof-trim ambiguity
and bright seams stay with building-art work rather than prompting an unrelated
fog redesign.

**Gap:** C20 requires a recorded decision when the human checkpoint is silent.

**Reach:** This accepts only fog edges at building corners. Distant forest and
unit/effect pixels differ slightly; complete dense-city appearance and the
missing S4/G0 envelope remain separate acceptance decisions.

**Verdict:** sound within the startup slice. The scoped comparison and unprimed
critique support preserving the look without claiming all frames identical.
**Confidence:** high for unchanged corners, medium for broader art.

## Startup lane: combat parity and named native math change

**When:** Startup step 5, 2026-10-01; the owner explicitly authorized the deterministic math correction and its native digest change.

### Combat has its own paired record, with the movement pair preserved

**Choice:** Keep the moving four-unit stream unchanged and add a stationary rifle duel to the same native/WebAssembly publication harness. Each tick compares the whole battle state digest, packed observation bytes and delivered fog, and the duel must actually publish fired shots and physical impacts. Moving and firing in one record would make a first mismatch harder to assign to either mechanism.

**Gap:** Step 5 asks for a short shooting battle but leaves the fixture, roster and comparison shape open.

**Reach:** The existing movement, side switches and resynchronization remain covered; later combat changes get a separate small reproduction on shipped rules, without test-only aim, damage or scatter overrides.

**Verdict:** sound. It adds a real combat path while retaining the prior proof. **Confidence:** high.

### Use the pinned Rust math implementation for state-bearing combat transcendental operations

**Choice:** A shot's direction and weapon elevation, the normal random samples that choose its spread, and conversion of angular spread to displacement now use the same pinned pure Rust math library in both targets. Native system math can round a result differently from WebAssembly by one final binary digit. That tiny difference can enter the battle's Float64 state even when its Float32 observation looks identical. Keeping system math and accepting an approximate digest would make a seed or replay mean different battles across targets.

**Gap:** The lane originally required unchanged battle digests. The new combat check exposed existing cross-target drift, so the owner authorized this exception as a named native math change.

**Reach:** Combat's native digest changes where system math had differed. The original movement record remains byte-for-byte unchanged. In the shooting sample, corrected native digests and publications agree with the pre-correction WebAssembly build at every tick; this preserves sampled browser behavior, not a claim that every unrelated simulation math path has been audited.

**Verdict:** sound. The correction removes platform rounding from the tested authority paths rather than weakening the parity check or finding a lucky fixture. **Confidence:** high.
### Sound — high confidence: native expectations follow named upstream contracts

**Choice:** Keep the combat scenario and seeds fixed while native expectations
follow shipped rule and publication changes. Main's grenade gravity changes the
shot history; its variable-span normalization changes packed representation.
Native remains the recording source and WebAssembly remains the independent
reader of those expectations.

**Gap:** The combat fixture was produced before those main changes and the lane
left integration of expected records unspecified.

**Reach:** A rule change can alter affected state expectations, while a codec
change alters only packed hashes. Neither allows approximate comparisons,
changing inputs to evade mismatches, or rewriting the original movement record.

**Verdict:** sound. One exact current contract replaces stale expectations rather
than comparing different builds' rules. **Confidence:** high.

## C33 preparation and public queries — startup lane

### Sound — medium confidence: keep public query arithmetic in Rust

Superseded by [C33 simplified](#c33-simplified): the page has no query index; it builds its own world.

**Choice:** After Deploy, the page imports a read-only Rust query index containing the already sampled ground, static surfaces and picking boxes. It also holds sparse static foliage needed to apply only the clearing this side learned. The battle's navigation and mutable world remain in its worker. A TypeScript query implementation would save a second Wasm instance but would also duplicate the simulation's interpolation, bridge, river and ray rules.

**Gap:** C33 delegates query-index internals but does not choose the language or transport precision.

**Reach:** The page still loads Wasm for public queries. The query payload preserves floating-point bits explicitly: decimal JSON parsing moved a terrain normal by one bit in the regression. Its temporary payload and resident index must be included in startup memory accounting; rendering's Float32 arrays cannot substitute for exact picking inputs.

**Verdict:** sound within the measured menu envelope. Exact shared arithmetic earns the import cost; full dense-city G0 admission remains separate. **Confidence:** medium.

### Sound — medium confidence on prominence: show an authority refusal without waiting for drawing

**Choice:** A replay refused by the simulation keeps the existing error HUD and menu available even when no world exports arrive. The viewport still requires meshes, and a failure hides the loading cover. For example, importing commands recorded on the ordinary village into the crossfire variant explains the scenario mismatch and lets the player open the menu to load another file or return home.

**Gap:** Independent review found that making world delivery part of authority startup also made the error HUD depend on successful initialization.

**Reach:** The normal battle and refusal share the same HUD, menu and error component. The prepared replay adapter forwards the simulation's existing error text, including main's new engine-build refusal. Development replay imports persist through the existing replay-file storage owner before reloading; a storage failure stays visible on the current page. Downloads may still proceed when persistence is unavailable.

**Verdict:** sound. Refusal navigation belongs to the existing HUD and must remain available before world construction. Normal status/clock is hidden on refusal to avoid implying playback is waiting. The existing compact HUD remains the owner rather than introducing another failure layout. **Confidence:** high on the failure contract, medium on first-glance prominence.

### Sound — high confidence: preparation's worker becomes the battle authority

**Choice:** The worker that lays the encounter keeps its world and becomes the worker that plays the battle. The page adopts its channel using the existing simulation client. Cancelling closes that worker, so an abandoned planner cannot publish a stale battle. Restart is a new battle: a fresh worker builds the same scenario once.

**Gap:** The lane requires world reuse but does not specify the worker handoff or restart ownership.

**Reach:** Both generated and saved battles use this handoff. Since [C33 simplified](#c33-simplified) no route exports geometry from a worker: every route's page builds its own `WorldView`.

**Verdict:** sound. It retains the existing command/publication authority and lets worker termination release all abandoned preparation allocations. **Confidence:** high.

### Sound — high confidence: measure browser memory with explicit bounds

**Choice:** Startup samples this Chromium instance's own process counters from Deploy, through the first playable view. It records cold and warm navigation separately, stage peaks, charged memory and retired instructions. Summed RSS can double-count shared pages; sampled instruction deltas can miss a process's final work. The sum of process lifetime memory high-water marks provides a conservative bound, not a simultaneous tab peak.

**Gap:** The lane asks for tab memory and retired instructions without prescribing a browser measurement API.

**Reach:** These measurements include the browser and GPU processes, not merely the JS heap. The harness cannot label a sampled maximum as an exact simultaneous high-water mark or a development-server start as a production download proof.

**Verdict:** sound. It uses kernel counters and states their limits. **Confidence:** high.

### Sound — high confidence: keep source-dependent Vite caches local to each checkout

**Choice:** Checkouts share installed dependencies while their Vite caches live under their own ignored scratch directory. Two simultaneous startup checks therefore cannot overwrite each other's compiled dependency metadata.

**Gap:** The shared-dependencies rule did not prescribe a Vite cache location; a worker integration run exposed repeated cache invalidation across checkouts.

**Reach:** Every local dev/verification server uses its checkout's cache. No new dependency or user-facing setting is added.

**Verdict:** sound. It applies the existing prohibition on sharing build output between different sources. **Confidence:** high.

### Sound — high confidence: preserve the developer stress scenario's actual map

**Choice:** Main's city-stress producer remains the owner of its full early/late
scenario. Preparation builds its retained authority world from that scenario,
including late wrecks, and forwards its living-force count and camera start.
Normal menu recipes continue planning on the retained world itself.

**Gap:** Main introduced the developer stress factory after the startup branch
was measured; its synthetic remains can change the map after generation.

**Reach:** The public index and battle must share the stress scenario's actual
map rather than the generator's earlier map. The developer factory still builds
its existing temporary placement world, so it is not included in the normal
menu's one-construction or frozen cost claim. Its broader scale admission stays
with the scale owner.

**Verdict:** sound for integration. It preserves main's authoritative fixture
and makes its static exports agree with the scenario that actually plays.
**Confidence:** high.

## Camera catalogue — startup lane

### Sound — high confidence: separate authored geometry from camera trajectories

**Choice:** The camera lab's physical plan is an offline source beside its saved catalogue map. The route loads that map by id; its trajectory fixture keeps only camera paths, framing and the public owner id of the building whose fall it demonstrates. An authored identity describes the hand-placed arena even though the compiler produces its saved physical document.

**Gap:** The lane requests catalogue resolution but does not specify where the lab's source plan or fall reference should live.

**Reach:** Changing a camera path does not rebuild geography. Changing the arena goes through the compiler and saved-map provenance, as other catalogue maps do. The catalogue cutover preserves framing and compiled geometry. Integration later adopts main's named China slab template for the wall and repins this authored map and its source receipts to that same input.

**Verdict:** sound. The resolver result exactly matches the former compiler output and the catalogue validation covers the new folder. **Confidence:** high.
## C54 pipeline tooling

### Measure a short advance without turning it into an arrival deadline

**When:** scale-lane pipeline tool checkpoint, 2026-10-01.

**Choice:** Each of the nine map type/size cells runs fixed map seeds 1–10;
the encounter and battle seeds stay fixed at the game's encounter seed and 1.
The attacker sends its planned column toward the objective by an ordinary group
move, while the defender keeps its existing scripts and policy. The default
sample lasts 30 simulated seconds. A rifle squad starting kilometres away may
move normally for all 30 s without arriving. Its row retains the remaining
distance and every movement state; coming within 10 m is reported as proximity,
not completion. The alternative was to call every unfinished short route a
failure, which would confuse normal transit with blocked navigation.

**Gap:** C54 delegates fixed seeds and reporting and gives no short-run duration
or arrival deadline. **Reach:** The tooling reveals refusals, route-blocked and
pending work across the matrix; a later playability verdict still needs a longer
battle and its own arrival/engagement contract. **Verdict:** sound — the sample
reports observed progress without inventing a completion requirement.
**Confidence:** medium.

### Keep the simulation dependency inside verification tooling

**When:** scale-lane pipeline tool checkpoint, 2026-10-01.

**Choice:** The runner is a `mapgen` example using its existing simulation test
 dependency. It calls the same generator and compiler that produce a player's
map, then the simulation's existing assault planner and Battle constructor.
When a seed is refused, its exact diagnostics stay in its row and the next
requested seed runs; nothing stands in for it. Costs and input hashes are saved
in that row before proceeding. The alternative was to add generation to the sim
runtime or parse several reports' prose, either adding a dependency cycle or a
second reporting oracle that could silently drift.

**Gap:** The spec names one runner but no crate or output format.
**Reach:** One streaming JSONL report can be inspected while a long matrix runs;
command refusals and unwinding panics remain explicit. This does not recover a
hung process or imply full C54/art acceptance. **Verdict:** sound — production
ownership stays unchanged and no new dependency is introduced.
**Confidence:** high.
## Replay engine build identity

**Choice:** Every simulation replay requires the automatically derived engine build fingerprint. `Battle::from_replay` checks it before scenario/rules identity or battle construction; the Wasm adapter forwards the same explicit refusal. Missing identity is malformed replay data. No manual version bump or legacy fallback is provided.

**Scope:** Hash normalized relative paths and exact bytes of simulation, contract and thin Wasm adapter Rust source, their manifests, workspace manifest, mapgen manifest, Cargo lock, rustc version, and semantic simulation cfg/features through the existing contract SHA256 owner. Dependency manifests/lock are conservative: even an unrelated dependency edit may refuse playback. Mapgen source is excluded because it prepares scenarios rather than executing the stored scenario; the separate generated replay request-versus-compiled-storage debt remains open. Assets, art pipeline, browser source, examples and test files outside runtime source are excluded. Source comments and inline test edits conservatively invalidate the engine.

**Portability:** This identifies the supported deterministic engine, not identical binary bytes. Native/Wasm target, platform, debug, panic, test, lint and overflow-instrumentation cfg differences are normalized; their supported battle outcomes already share the simulation contract. Current source has no other generated/include input, and current crates expose no dependency feature selection outside the hashed manifests. Adding such inputs or target-dependent simulation semantics requires extending this owner rather than silently retaining this scope.

**Cost:** Hashing occurs once at build time; replay recording copies one 64-character identity, and playback compares it once. No new tick work, codec dependency or battle digest input. The contract crate is reused as a build dependency. Native red/green checks expose the formerly accepted mismatched build through `Battle::from_replay`; same-build replay pins every tick digest. Scope tests prove art-only stability and source, dependency, compiler and semantic cfg invalidation. Worker refusal and actual native/Wasm fingerprint parity are checked on the rebuilt module.

**Verdict:** Sound within the current supported build inputs. Compiled scenario storage and broader scale admission gates remain separate unfinished work. **Confidence:** High.
## C06 infantry town corner routes

**Choice:** A connected coarse infantry cell is permission to walk through its
free sub-cells, not permission to cut straight between its entry and exit. The
route reconstructs a bounded local connector and certifies every emitted link
with the same sampled reader used by route timing and smoothing. A returned
illegal segment still has infinite cost; no failed route is made finite by
changing the timing rule. Vehicles keep their existing physical clearance.

**Why:** C59 exposed an infantry route accepted by coarse search but rejected by
the timing consumer. The public corner regression reproduced it without a city.
A squad bending around that corner is the expected physical behavior. The
alternative of rejecting connected cells would unnecessarily close passages
whose free half-metre path already exists. A whole-map fine search would add
storage and work unrelated to the local defect.

**Reach:** Infantry waypoints, costs, planning completion ticks and affected
battle digests intentionally change. Legal endpoints use their actual containing
cells; bounded nearest-fit selection remains, but returned first links must pass
the sampled reader. Local connection is an incremental planning phase, with no retained
full-extent fine data. Certified infantry links remain available to smoothing
until a longer link passes its reader, because even collinear merging changes
sample positions. This supersedes C59's historical infinity limitation; changing
the encounter planner to consume the corrected timings belongs to its owner.

**Verdict:** sound for the focused contracts. Public timing covers narrow foot
passage, blocked/disconnected and boundary cases. A small battle checks actual
soldier bodies and same-build replay under the smallest planning budget. NavGrid
certifies its sampled mask, while movement owns exact body legality. Generated
town admission and native city cost remain open in the scale lane. **Confidence:**
high for the regression and replay, medium across generated towns.


## C07: variable word-span copies

**Choice:** Generalize the existing copy operation through layout-owned per-group alignment: fixed rows preserve complete-row source/count checks; variable collections accept word spans. Retain one decoder and one flat baseline. Use exact eight-word sparse anchors, a same-position match preference, greedy forward extension and literals between retained spans. Choose the smallest of copies, replacement and snapshot after exact preflight.

**Why:** Measured own-group section shifts still resent tails after the corpse fix. A public route edit among 80 squads needed 53,568 B, now 376 B with complete float-bit reconstruction. Across 450 captured active transitions, own max falls 51,968→10,656 B and p95 27,200→8,556 B; isolated mean encoding instructions rise 0.288→1.840 M. The integration owner accepts this measured tradeoff against current whole-step costs; whole-record admission remains open. Bitmask/XOR estimates did not address the shifted retained tail adequately.

**Bound:** One u32 per eight old variable words; aggregate scratch remains ≤12.8 MiB because fixed groups' smallest row is five words and all groups partition the admitted baseline. Exact comparisons avoid identity/hashing assumptions and collision scans. Sorting and two greedy scans are O(words log anchors), with fixed eight-word comparisons and no retained operation list. Fallible index and complete record reservation precede output/cursor commit. Existing record buffers and fog ownership stay unchanged.

**Verdict:** Sound and independently reviewed. Supported bits/order, fixed alignment, malformed retry and retained observations are verified. Full active early/late bytes, decoder throughput, full packing cost and peak overlap remain open under the unchanged 19.8 KB and memory contracts. **Confidence:** High for reconstruction; admission pending integration.
## C06 corridor starts

### A squad corridor starts at a member the known grid admits

**Choice:** Select the closest living soldier whose position is standing room
in the side's navigation grid before falling back to the physically nearest
member. A squad's centroid is still never its corridor start. Road preference
measures the same physical nearest member as before.

**Why:** The old grid reconstruction tolerated an invalid first link, so a
physically clear soldier near a wall could seed a usable corridor despite being
rejected by the conservative sampled mask. Certifying links exposed that mismatch:
a small Battle that arrived before became blocked. Choosing another existing
standing member restores the corridor contract without weakening the mask or
granting a body-exit exemption. This intentionally changes corridor starts and
affected digests; movement's exact body checks still decide each soldier's steps.

**Verdict:** supported by the old/new Battle comparison; follow-up verification
is recorded in C06. A direct non-standing virtual anchor has no guaranteed legal
timing; nearest-fit selection is not permission to publish an invalid segment.

## C07: reuse unchanged decoded static groups

**Choice:** Bundle the existing word baseline with decoded corpse and known-prop arrays in `ObservationDecoder`. Buffer identity witnesses unchanged static words; validate the current header's exact fixed-row count before reusing arrays, rows and coordinates. Rebuild a whole static group when its words change. Commit the bundle after the complete frame validates, and clear it on side invalidation; fresh epochs bypass it.

**Why:** Sparse delivery already retains unchanged static words, but decoding recreated every row and coordinate on every own-unit update. Unchanged 20,000-corpse/2,000-prop delivery now constructs no per-static-row wrappers, closures, section objects, views or coordinates. Retaining the latest two view arrays adds references to the observation objects already returned, not a second word baseline or a complete retained observation. There is no per-row identity scheme or cache outside the decoder.

**Contract:** Returned observations are immutable; the static arrays are now readonly in TypeScript. The current consumers read these rows; pose reconciliation copies positions into its own state, and effects read aliased prop extents. Cache reuse still checks current counts. Malformed cached counts and failures later in fog/ground cannot advance either baseline; corrected same-generation retry works. Changed-group views and prior observations remain distinct, and stale side/epoch records do not populate the cache.

**Verdict:** Sound; identity and rollback proofs pass. Allocation claims describe skipped construction, not measured heap bytes or wall time. Producer layout/digests and all whole-record, peak and throughput gates remain unchanged. **Confidence:** High for reuse and reconstruction; runtime admission remains open.

## C15 interior atlas

### A cell is one whole room, 128 px square

**Choice:** Each sheet is 256 × 640 px: 2 × 5 square cells of 128 px. A cell is the picture of one room box 3 m wide, 3 m tall and 4.5 m deep, and a window shows the whole of it, optionally mirrored.

**Gap:** The slice says "the repo's 2×5 layout". Upstream's cells are strips about 3.6 times as wide as tall (11 m of wall for a 3.1 m room), and each window slides a 2.9 m room along its strip by a hashed offset.

**Verdict:** sound for the layout, provisional for the size. Square power-of-two cells keep every mip down to one texel a cell inside one room, and five rows under the 1024 px cap leave 128 as the largest power of two. That is 43 px a metre against upstream's 63, and it gives up the sliding offset: variety is ten rooms a sheet, times mirroring and the building's own dimming. If windows read as repeats in C26, the fix is more sheets or the strip layout, here. **Confidence:** medium.

### The room box is one bay and one floor, on every floor

**Choice:** 3 × 3 × 4.5 m, the pinhole 16 m out, for apartments and shops alike.

**Gap:** Upstream's box is 2.9 m wide and 3.12 m tall (4.09 m on the ground floor), as deep as half the building up to 4.6 m, with the same 16 m pinhole. Ours is unstated.

**Verdict:** sound. Our lattice is 3 m bays and 3 m floors, and a box of another size scales the lookup by its own width and height. A shallower box than 4.5 m shows the picture's middle larger than it was rendered; C26 can clamp depth as upstream does. **Confidence:** medium.

### Cycles on the CPU, not Workbench or EEVEE

**Choice:** The rooms are path-traced on the CPU with a fixed seed and sample count and no denoiser; the tone curve, the 4× box filter and the PNG are numpy.

**Gap:** The slice asks only for a deterministic recipe.

**Verdict:** sound. A room lit by nothing but its window is bounce light, which only a path tracer gives; the CPU path does not depend on the GPU or its driver. Two runs on this machine wrote the same bytes. Another CPU architecture is not tested; the committed sheets are the source of truth, as for the GLBs. **Confidence:** high on this machine.

### Daylight is an area light in the window opening, the same flux for every room

**Choice:** The sky is a rectangle of light filling the room's window opening in an unseen window wall. An apartment's opening is 1.3 × 1.5 m and a shopfront's 2.6 × 2.3 m, and both let the same total light in. No sun patch.

**Gap:** "No lamps", "daylight-only".

**Verdict:** sound. At the same sky a shopfront's room is three times as bright as an apartment's and read as lit. A sun patch would contradict the game's own sun direction on three facades out of four. **Confidence:** medium.

### The picture has a ceiling, and no shadow lift

**Choice:** One exposure for both sheets under a soft ceiling of 0.25 linear (0.54 sRGB). The depths are not lifted.

**Gap:** "Dim": how dim, and what stops a curtain in the window's full light from reading as lit.

**Verdict:** provisional. A first curve lifted the depths; the unprimed critique read it as a grey veil over the whole sheet. Whatever hangs at the window (curtains, a shutter) is the brightest thing in a cell however dark its cloth, so those are dark fabric and dark steel, and a pale pelmet that read as a strip light was removed. C26 owns the final level against the game's facade; change the tone here. **Confidence:** medium.

### Curtains are in the picture

**Choice:** Five apartment cells carry curtains, hung inside the window and part of the cell.

**Gap:** Upstream draws curtains as their own geometry in front of the room; the slice does not say which side of the line they fall.

**Verdict:** provisional. In the picture they sit right from in front and slide with the back wall from the side. If C26 adds curtain geometry they come out of these cells. **Confidence:** medium.

### The layout comparison against upstream's atlases was not run

**Choice:** The layout and projection were taken from upstream's code (`interiors.ts`, the interior shader in `materials.ts`), and its two atlases were never opened.

**Gap:** The slice's visual step 2 compares "against the repo's atlases for layout only".

**Verdict:** sound. Opening the files is the one way their content could leak into ours, and their layout is fully stated by the code that reads them. **Confidence:** high.

### From the tactical camera a window shows mostly the cell's floor

**Choice:** None; a finding for C26. The lookup maps the box's floor to the bottom ninth of a cell, and a ray looking down 50° through a window lands on the floor within 2.5 m of the wall. So at the tactical camera a window is the cell's floor strip stretched, with furniture feet smeared along it, and the back wall shows only from near street level.

**Gap:** The slice judges the sheet, not the read behind glass.

**Verdict:** the mechanism's own behaviour, the same for upstream's photographs. The mock-up shows dark, coloured, slightly streaked panes: not holes, not rooms. If the user wants furniture from above, the lookup has to change, which Q-E forbids, or the pinhole has to move, which is this recipe. **Confidence:** high that it happens, low on whether it matters.

### The sheets are toned for the shade the game shows them in

**Choice:** Exposure 1.0 under a ceiling of 0.15 linear (0.42 sRGB), light floors, strong wall hues. Cell means are 0.28 to 0.39 sRGB where they were 0.13 to 0.25. This supersedes "The picture has a ceiling, and no shadow lift" above: the ceiling stays, the level does not.

**Gap:** C15 judged the sheet as a picture and made it dim. C26 then showed a cell as a matte surface in sun shadow, which darkens it again: at 80 m every window was a near-black rectangle.

**Verdict:** sound on the facade scene's frames, with one limit the sheet cannot remove.

| Room against the sunlit wall | before | after |
|---|---|---|
| 30 m, no glass | 0.16 to 0.44 | 0.26 to 0.58 |
| 30 m, behind glass | 0.23 to 0.37 | 0.28 to 0.45 |
| 80 m, no glass | 0.25 to 0.39 | 0.49 to 0.58 |
| 80 m, behind glass | 0.25 to 0.33 | 0.39 to 0.44 |
| shops from the street, behind glass | 0.25 to 0.32 | 0.36 to 0.42 |

The scene's bound (a room at most 0.7 of its wall) was not moved. Two brighter tries were rejected on the pictures:

- Exposure 0.7 under a ceiling of 0.5 tripped the bound: a white room and a pale tiled floor reached 0.76 and 0.79 without glass, and the white room was the brightest thing on the facade. The bound was right.
- Exposure 0.9 under a ceiling of 0.2 passed every check, and an unprimed critic still read the pale shops as "a dim light left on" where the facade stands in its own shade. A room takes no sun, so it does not darken when its wall does: there a window's brightest twentieth reached 0.95 of the shaded wall. At the ceiling chosen it is 0.84 at most, and a window's mean 0.60 to 0.79 (0.47 to 0.60 before).

A second unprimed critic, on the chosen sheets' frames: no window reads as lit under any of the four suns (high confidence; the pale shop on the shaded facade is "the nearest thing to a lit look", read as a dim daylight room); none is a hole at 30 m or from the street; rooms are told apart at 30 m by wall colour and contents; two of the three shops read as shops, the third as the vacant unit it is. At 80 m every opening is still a flat dark pane, brown for flats and slate for shops, "not black, but functionally blank": the floor strip under the glass's veil, with no wall colour left.

That is the limit: the sheet trades "a hole at 80 m in the sun" against "lit on a shaded facade", and one level serves both only so far. If shaded facades still read as lit, the lever is the light a room is shown in (C26's `unlit`, which could follow the facade's shade), not the sheet. The scene checks one window under each sun and no shop; a check of every window against the shaded wall would hold this. The sheet on its own now looks like a lit doll's house; that is what a shaded surface's colour looks like before the shade. **Confidence:** medium: one block, one biome's light.
## C19: warehouses and light industry

### Sizes are whole bays, and the shed turns its gable to the street

**Choice:** The five industrial templates are 15 x 24 m (the prototype shed was 24 x 16, long side to the street), 48 x 24, 72 x 33 (72 x 32), a works of a 54 x 27 hall and an 18 x 9 office (54 x 26 and 18 x 10), and 90 x 39 (90 x 40). Every side is a multiple of the 3 m bay, so a row of bay-wide modules tiles a wall exactly. The shed is 15 wide and 24 deep with its vehicle door in the gable. The alternative kept the prototype sizes and left a part-bay of plain wall at each corner.

**Gap:** The brief gave sizes as "about", and the prototype sizes were chosen for boxes.

**Reach:** The catalogue's hash moves when these replace the prototypes. Heights are 6.2, 9.0, 10.0, 8.0 and 11.0 m against the prototypes' 6, 9, 10, 10 (hall) and 7 (office), and 11.

**Verdict:** sound. A workshop's door is in its gable, and a wall of whole bays needs no filler module. **Confidence:** high.

### The works' office is joined to the hall's street face, and both boxes are 8 m

**Choice:** The office block stands against the west end of the hall's street face, flush with the hall's west wall, joined by a supported join (`hall-south-0` to `office-north`). Both parts are 8.0 m: the hall's sawtooth ridges and the office's parapet. Floor datums are 0 and 3.6 m for the whole template, so the hall is a two-floor part whose tall windows (1.2 to 5.7 m) serve both. The prototype had a 10 m hall and a 7 m office standing 4 m apart with no join.

**Gap:** A join needs equal bases and tops, and floor datums belong to the template, not to a part.

**Reach:** The hall is 2 m lower than its prototype. A garrison's upper floor in the hall is a notional gantry level: nothing is drawn for it.

**Verdict:** provisional. It reads as one works on screen; whether a second floor of seats in an open hall plays well is the simulation's to judge. **Confidence:** medium.

### The sawtooth's glazing faces along the street, not away from it

**Choice:** The works' nine teeth run front to back, so the street elevation shows the zig-zag and the glazing faces the template's -X. A true north light, with the street to the south, would turn every pane away from the street and show the viewer nine plain slopes.

**Gap:** The brief asked for a sawtooth north-light roof and a door on the street side; a template has no compass.

**Verdict:** sound for the look. **Confidence:** high.

### Not every bay has an opening, and a dock door is not an entrance

**Choice:** Openings sit in bays, but a warehouse keeps blank bays: the dock warehouse's back and ends have a high window in every second bay, the distribution warehouse the same. Loading-dock doors have their sills at 1.1 m (lorry-bed height) over a floor datum of 0 and are not entrances; the template's entrances are its side door and its drive-in door. The depot draws eight vehicle doors and names two of them, with the side door, as entrances; the distribution warehouse's three side doors are drawn but only its three vehicle doors are entrances. The houses (C17) gave every bay a window on every floor.

**Gap:** The readme puts windows on the lattice; it does not say every bay needs one, nor that every drawn door is an entrance.

**Reach:** A garrison's seat in a blank bay fires through a wall the player sees as solid. Entrance counts follow the prototypes (two or three a building).

**Verdict:** provisional. A warehouse with a window in every bay stops reading as one. If blank-bay seats look wrong in play, the fix is fewer seats there, not more windows. **Confidence:** medium.

### The set's fit is 1.2 m to the side and 1.2 m above

**Choice:** Canopies reach 1.15 m, a vehicle door's bollards 0.9 m, a dock's leveller 0.6 m, gutters 0.41 m. Above a part's top: flue stacks to 1.1 m over a flat roof's parapet, ridge ventilators 0.46 m, turbine vents 0.7 m. The houses' fit is 0.5 and 0.9 m.

**Gap:** The brief said to choose both and keep them small.

**Verdict:** sound. A canopy shallower than a metre does not read as one. **Confidence:** high.

### Rows draw at the two fine tiers; the shell carries the rest

**Choice:** As the apartment set does, a row draws at tiers 0 and 1 only. At tiers 2 and 3 the script copies what each row's module still has at that tier into the template's shell, in the row's place and with its tint baked into an untinted material, so a far building is one row. Flat panels that meet (a band of colour, a run of high windows) become one face. Rooflights and mended sheets on a pitched roof are faces of the shell at every tier, because a row cannot tilt a module to a roof's pitch.

**Gap:** The brief asked for instancing on long walls and budgets per tier; the readme's rule that a far building is one row arrived with the apartment merge.

**Reach:** A shell's second tier holds the folded detail, so its first tier must be at least as heavy: three shells bake their paint on a finer grid than they otherwise need. Folded surfaces lose the recipe's own tint mask (rust weeping from a fixing takes the paint's colour).

**Verdict:** sound. **Confidence:** high.

### A roof's large marks are faces and vertex paint, never the recipe

**Choice:** The sheet and felt recipes carry only what is smaller than a few metres: ribs, laps, fixings, the tone of one sheet. Rust patches, damp and fading are vertex paint on a 4.4 m grid kept down to tier 2; mended sheets, rooflights and felt patches are faces of the shell. A first felt recipe with pools and patches in the texture tiled into a camouflage pattern across a 48 m roof.

**Gap:** The brief asked for stains and patches readable at coarse tiers "through the shell's own texture".

**Verdict:** sound. A recipe repeats every 6 to 8 m; anything a viewer can count across a roof must be placed once. **Confidence:** high.

### Wall colour is the shell's tint; roofs and brick take none

**Choice:** Cladding, blockwork, precast panels and render are pale tint-masked recipes and the shell's row tint is the building's paint. Roof sheet, felt and brick are untinted materials in their own colour, since a row has one tint. Bands, doors and a re-sheeted bay are rows with their own tints.

**Gap:** The same as the houses': a row has one tint.

**Reach:** One template is one colourway. More colours are more templates over the same modules, or a tint a placement supplies.

**Verdict:** provisional on the repeats, as for the houses. **Confidence:** medium.

## C21 material transport

### The coverage value is the base colour's alpha times the normal texture's alpha

**Choice:** A cutout or blended material's coverage is `base_color` alpha (one number for the surface: a pane's opacity) times the normal texture's alpha (texel by texel: a grille's holes), 1 where the material has no normal texture. An opaque material ignores both.

**Gap:** The slice says coverage must not ride albedo alpha or ORM alpha, and leaves where it does ride open: the factor's alpha, a dedicated texture channel, or the vertex colour.

**Alternatives:** The vertex colour's alpha is how worn the surface is (L9), so it collides. Albedo alpha is the wear threshold: reading it as coverage "when the material does not wear" is the overload the slice exists to end. A fourth texture breaks the three-slot limit. Of the twelve channels in three RGBA textures, the normal map's alpha was the only one unused (255 in every shipped texture), and the base colour's alpha was carried to the renderer and never read.

**Reach:** A cutout needs a normal texture, flat if it has no relief. In the source this is not glTF's convention (there, coverage is the base colour texture's alpha), exactly as our albedo alpha already is not. A cutout authored the glTF way is caught by name, not drawn solid (next choice). Mips average the alpha as stored; whether a cutout needs coverage-preserving mips is C24's.

**Verdict:** sound. **Confidence:** high.

### A coverage that never crosses its threshold is refused

**Choice:** `material.coverage_source`: a cutout whose coverage value is everywhere under its cutoff or everywhere at or over it, and a blended material whose value is 1 everywhere, do not bake. The check reads the finest level of the normal texture.

**Gap:** The brief asks to refuse "a cutout with no source of coverage". A missing normal texture is one way to have none; a normal texture whose alpha is 255 throughout, because the coverage was painted into the albedo, is the likelier one, and it would draw solid with no error.

**Verdict:** sound. It is the slice's own feedback line as a check: lost coverage reopens transport. **Confidence:** high.

### Blended cannot wear; a cutout can

**Choice:** `material.wear` refuses a blended material with a wear colour. A cutout with wear and a tint mask bakes.

**Gap:** The brief names "blended with wear" as unsupported and says nothing of cutout with wear.

**Verdict:** sound. Wear replaces the surface with an opaque worn one where the vertex alpha passes the albedo's threshold; on glass that patch has no opacity anyone chose. On a cutout the four channels are independent: a rusty grille is a legitimate surface.

**Reach:** `parts.export(worn=…)` is per file, so a blended material that samples a recipe has to be in an unworn export. A flat blended material (`flat_paint`) carries no wear. **Confidence:** high.

### Interior metadata is the sheet's name, on the room's own surfaces

**Choice:** `Material.interior` is `"rooms"` or `"shops"`, from the glTF material's extras, and nothing else. It marks the walls, floor and ceiling of the open box behind a window, not the pane in front. The box's UVs are the cell-local `u`, `v` of the atlas projection, written by whoever models the box, so the drawer needs no box size or pinhole per material.

**Gap:** "Enough for the later interior pass to know this surface is a window onto a room box and which atlas sheet it looks up."

**Alternatives:** Carrying the box's size and the pinhole per material repeats the atlas's constants in every bundle. Marking the pane as the interior needs a ray-marched box in the shader, which is more than the upstream mechanism (Q-E: room meshes plus a flat lookup). Which cell and which mirroring is per window, by position hash, so it cannot be material data.

**Reach:** A UV interpolated across a side wall is off the true projection by at most 0.7 % of a cell (under one texel of 128) for the 3 × 3 × 4.5 m box and the 16 m pinhole; a wall cut once in depth quarters it. If C26 would rather project in the shader from a box frame, the frame needs a home (a vertex attribute or per-module data) and this field stays as it is.

**Verdict:** superseded by [C26](#c26-interiors): the field stays as it is, and the UVs are the box unfolded, with the pinhole in the shader. The 0.7 % above is the error along a wall's edge; on a floor's diagonal it is 5.5 %. **Confidence:** high.

### A room is opaque, has no look of its own, and stands still

**Choice:** `material.interior` refuses an interior material that is a cutout or blended, has textures or a wear colour, or is on a skinned or articulated bundle. A tint mask is allowed: a building's row tint is how it dims its rooms.

**Gap:** The brief names only the skinned and articulated case.

**Verdict:** provisional. The atlas contract says a cell is a finished picture shown as it is, so the material's own textures and wear would be silently ignored, which is the loss this slice refuses elsewhere. If C26 finds a use for one (a normal map on a back wall), it deletes that branch. **Confidence:** medium.

### An opaque material's alphas are ignored, not refused

**Choice:** `alphaMode` absent or `OPAQUE` is opaque whatever the base colour's alpha, the normal map's alpha or a stray `alphaCutoff` say, as glTF defines it. `MASK` with no `alphaCutoff` cuts at glTF's default, 0.5. An alpha mode or cutoff that cannot be read is `material.coverage`.

**Gap:** Unstated.

**Verdict:** sound. All 310 materials in the 41 shipped sources name no alpha mode and have a base alpha of 1, so neither rule moved anything. **Confidence:** high.

### The packed representation is the material's JSON

**Choice:** `coverage` is `{"kind":"opaque"}`, `{"kind":"cutout","cutoff":0.5}` or `{"kind":"blended"}` in the bundle header's material, on every material; `interior` is the sheet's name, present only on a room. Format 4. The decoder refuses a material with no well-formed coverage or an unknown sheet; the combination rules are the validator's alone, so the workbench can still show a preview that has findings.

**Gap:** Delegated.

**Alternatives:** An optional field meaning opaque when absent would have let a format 3 material read as a valid format 4 one; required, an old or truncated material cannot pass for opaque. Flags in a typed array buy nothing: a bundle has tens of materials and megabytes of mesh.

**Verdict:** sound. **Confidence:** high.

### The Blender helpers write the alpha mode after export, not through Blender's material graph

**Choice:** `coverage=("cutout", cutoff)` or `("blended", opacity)` and `interior="rooms"` on `common.mat`, `parts.paint`, `parts.flat_paint` and `parts.textured` (coverage only: a room has no recipe). They record into one registry in `textures.py`, and `textures.attach`, which already rewrites the exported GLB's materials for textures, writes `alphaMode`, `alphaCutoff` and the base colour's alpha. `interior` is a custom property, exported as extras like `tint`. A recipe's coverage image is `Baked(coverage=…)`, in the normal PNG's alpha.

**Gap:** "The Blender export helpers can write both."

**Alternatives:** Blender's exporter derives `alphaMode` from the node graph feeding the Principled alpha (a Math node pattern for a mask). That ties the source bytes to exporter heuristics across Blender versions, for a value we already know.

**Reach:** Checked on Blender 5.2.1 with a scratch script through each helper family: the exported materials read back as cutout 0.5 with a 0-to-1 coverage image, blended 0.35, and rooms of both sheets. Existing exports are untouched: the crate, wall, fence and sandbags re-exported byte-identical to the committed sources. No infantry source was re-exported (Cycles bake); `common.mat` was exercised by the scratch script only.

**Verdict:** sound. **Confidence:** high.

### Found on the way: the dragon's tooth does not export the same bytes twice

**Choice:** None; a finding. `props.py tooth` writes a different index order for `tooth_LOD0` on each run (28 to 70 bytes of one buffer view; the JSON and every other view are identical), with and without this slice's changes, and none of four runs matched the committed file. Left alone: it is a model script, outside this slice, and the committed GLB is what bakes.

**Verdict:** a defect against "the same scripts write the same bytes". **Confidence:** high that it happens; the cause was not looked for.
## C80 grass presets

### Wind response rides the clump's vertex alpha

**Choice:** A grass kind's `wind` (0 to 1) is written into every vertex's colour alpha by the generator and read back when the field packs its shapes.

**Gap:** The slice says the spec carries a wind-response factor; the bake reads only the GLB, never the spec.

**Verdict:** sound. No bundle-format or loader change, and the response is in the art it belongs to. A model's alpha is its wear threshold only where a wear texture is bound, and grass has none. It cannot pass 1: a kind sways as the biome's wind says, or less. **Confidence:** high.

### "Clumping" is the spec's existing radius; the field's clumping is the biome's

**Choice:** No new clumping field on a grass kind's spec. `radius_m` is one clump's spread; how clumps gather across a field is the biome growth row's (`thin`, `drift`, C82).

**Gap:** The ground design lists clumping among a preset's properties.

**Verdict:** sound. One owner each: the clump is the art, where clumps stand is the field's. **Confidence:** medium.

### A headed blade's last pair sits on its head, in every tier

**Choice:** Strip vertices are evenly spaced up a blade except on a blade with a head (an ear, a flower), whose last pair sits where the head is widest.

**Gap:** The slice asks for blade shape; it does not say which tier must show it.

**Verdict:** sound. The play camera draws the two-segment tier, whose only pair sat at half height, below any head: ears and flowers vanished 16 m out and the tier boundary showed as a band. **Confidence:** high.

### The workbench ruler is a cage at 0.9 m

**Choice:** A grass kind's overlay in the workbench is a ring on the ground and one at 0.9 m (the tallest any grass or crop may be drawn, `GRASS_MAX_HEIGHT_M`) with four posts, and the views frame it.

**Gap:** "A workbench grass sheet with a 0.9 m ruler."

**Verdict:** provisional. It shows the cap beside one clump at its source height. A sheet of every kind side by side at its field scale, which would judge the species against each other, was not built; building it would settle this. **Confidence:** medium.

## C81 wild grass

### Rough ground and prairie are plot kinds

**Choice:** Two plot kinds, `rough` and `prairie`, carry tall rough grass and dry prairie; meadow keeps the short meadow and the verge takes the weeds.

**Gap:** "Biome growth rows point meadow, pasture, verge and rough ground at them": there was no rough ground.

**Verdict:** sound. A plot kind is how the biome says what a piece of ground is. Adding kinds reshuffles which plot is which kind on every map, the village too, which the ground design allows. **Confidence:** high.

### A clump's shading softens by its size on screen, as biome data

**Choice:** The grass row's `soften_m_per_px`, `soften`, `blade_facing` and `min_blade_px` (1.4) replace a constant share of the fade and a constant normal weight.

**Gap:** The slice delegates preset parameters; the failure at the play camera was in the field's shading, not in a preset.

**Verdict:** sound. At 65 m a blade is under a pixel, and root-to-tip contrast and per-blade facing alias into dark flecks whatever the preset. **Confidence:** high.

### The rig stands on plots by kind, and the grass pass can be retuned in the page

**Choice:** The rig has a station over the village's roomiest open plot of each kind, and `__lab.grass().retune(rules)` swaps the grass rules in a running page.

**Gap:** The slices name `field-65`; which crop grows there changes with every roster of plot kinds.

**Verdict:** sound. Fixed coordinates showed three different crops over one pass. The retune probe is the seam the checks drive the grass through, so they hold when the biome's numbers move. **Confidence:** high.

### Frame cost is the rig's paired run

**Choice:** A slice's frame-cost row is its feature on and off at two stations, alternated on one machine (`pairedCost` in the rig), not the village scene's own cost run. Every later ground slice measured the same way.

**Gap:** "Grass ms and clump counts; a frame-cost row."

**Verdict:** sound for the method, with a limit. Most rows were taken on a loaded machine and did not resolve finer than about half a millisecond (grass came out inside +0.4 ms). The lane's whole GPU cost was measured once at the gate: median +0.09 ms against a budget of +3 ms. **Confidence:** medium.

## C82 within field variation

### A grass among others differs by dryness, not by its own colours

**Choice:** A growth row's mix entry carries `dry`, a hue shift over the ground's colour; a clump's own colours stay relative to the ground it grows on.

**Gap:** The slice says a clump picks its species; it does not say how species differ in colour inside one plot.

**Verdict:** provisional. It keeps the rule that near grass and the painted ground beyond it agree, so a field does not change colour where the grass stops being drawn. The cost: a prairie tuft in a meadow is green with a straw cast, not straw. The owner's eye on a mixed meadow at 25 m would settle whether that is enough difference. **Confidence:** medium.

### A grass can gather in drifts

**Choice:** A mix entry's `drift` gathers that grass into patches of its own: at 1 it stands up to three times its share inside a drift and not at all outside.

**Gap:** "A clump picks its species by hash."

**Verdict:** sound. Four grasses mixed evenly read as noise; weeds and rough grass stand in drifts. **Confidence:** medium.

### Patches are a few metres across, and drying lightens a little

**Choice:** `grass.patch_m` is 3.5 to 6 m (thin 3.5, height 4, drift 5, dry 6); `dry_lift` is 0.08; a kind's sparse patches thin its clumps by its own `thin` (0.08 for a crop, up to 0.35 for prairie).

**Gap:** "Noise scales; amplitudes."

**Verdict:** provisional. At 11 to 17 m across, an unprimed eye read the field between lighter patches as cloud shadow: the design's ban on shadow-like blobs is about scale as much as sign. A hue shift at exactly the ground's luminance read as rust, hence the small lift. The gate's fresh eye still reads meadow as felt; a critique at the field stations would settle the amplitudes. **Confidence:** medium.

### Each clump's brightness varies, within a bound

**Choice:** `clump_value` is 0.16: half of it varies clump by clump and half by tussock (1.1 m).

**Gap:** "Colour varies in hue and saturation at bounded luminance."

**Verdict:** provisional. Without it a softened field is flat at 65 m. It is two-sided (lighter and darker), which the design avoids, but at one metre it is grain and not a patch; a scene check holds it to the amount asked. **Confidence:** medium.

### What sets a clump apart fades with it

**Choice:** A clump's dryness and grain scale by its fade with distance.

**Gap:** The design asks for a graceful fade; a field whose clumps are lighter than its ground would end in a visible band where the grass stops.

**Verdict:** sound. **Confidence:** high.

## C83 crops

### Crops keep to the terrain's rows

**Choice:** A growth row's `rows` (0 to 1) moves a clump toward the nearest painted row of its plot, never past the verge or into a bare margin: wheat and barley 0.25, rapeseed 0.2, stubble 0.5.

**Gap:** The slice asks for presets; a crop scattered like a meadow reads as recoloured lawn.

**Verdict:** sound in mechanism, provisional in numbers. Tighter rows showed bare ground between them that read as sand. The ground between rows has its own texture since C85 and the numbers were not tried again over it; one round of tighter rows at the wheat station would settle them. **Confidence:** medium.

### Hay stands

**Choice:** Hay is tall dry grass with seed heads, uncut; stubble is the cut field.

**Gap:** "Hay and stubble."

**Verdict:** provisional. Swaths and bales would be dressing (models with no body), not grass, and were not built. **Confidence:** medium.

### There is no generic crop

**Choice:** The `young_crop` plot kind and the `grass_crop` grass kind are removed. Every crop is its own plot kind (wheat, barley, rapeseed, hay, stubble), each with its own palette since C84.

**Gap:** The slice's list has no generic crop.

**Verdict:** sound. **Confidence:** high.

## C46 street placement

### Furniture stands back from the street, not at the kerb

**Choice:** No body stands nearer a carriageway's middle than the catalog's widest hull plus `lane_margin_m` (3.7 m: 7.3 m from the middle today, 3.9 m off a 7 m street's kerb).

**Gap:** The design put parked cars at the kerb, on the carriageway, with a clear lane down the middle. The simulation's vehicles keep right of the middle and check their lane on a 2 m grid; at the kerb, 8 of 17 swept maps lost their assault and 77 street drives detoured. With the cars on the carriageway's outer edge itself, the widest hull shoved a car on 235 of 443 street drives.

**Verdict:** forced by the route-survival rule; provisional as a picture. Terraced streets stay bare. It goes back to about 0.9 m when a road journey tolerates a body beside its lane. **Confidence:** high that the simulation needs it, low that it is what the streets should look like.

**Resolved by the kerbside integration:** the margin returns to 0.9 m after local actual-hull traversals, the generated access matrix and unchanged combined long-arrival gate pass. The provisional verge placement is superseded; rendered street-art and frame-cost acceptance remain C54's.

### The catalog is a fourth document of generation

**Choice:** `generate`, `generate-map` and the two Wasm exports take the unit and prop catalog's documents. The request does not pin it.

**Gap:** The slice's signature takes a catalog; the generation boundary had none, and a request pins only the generator, the presets and the template catalogue.

**Verdict:** sound for a build's own maps. A catalog change that moves the lane changes every generated map under an unchanged request; the paired records and the saved map's hash go red when it does. **Confidence:** medium.

### A door keeps 3 m clear either side

**Choice:** `door_clear_m` is 3, as designed, after a trial at 0.75 m.

**Gap:** At 3 m a terrace with a door every 6 m takes no car at all; 0.75 m would have let one stand between each pair of doors.

**Verdict:** sound. At 0.75 m a squad could no longer stand at 89 doors of the sweep: navigation's cell there had no room. **Confidence:** high.

### Which side parks is a width, not a lane count

**Choice:** A carriageway at least `both_sides_min_width_m` (10 m) wide parks along both sides, a narrower one along one side drawn for the whole street.

**Gap:** The design derived one side or two from the lane left on the carriageway. With the cars off the carriageway that sum no longer decides anything.

**Verdict:** provisional: it keeps the designed outcome as a number. **Confidence:** medium.

### A body's size is a preset row

**Choice:** `street_props.bodies` gives each placed kind its box and the room it keeps (a car is 4.2 by 1.8 by 1.5 m).

**Gap:** A catalog prop type has no size; a map prop carries its own.

**Verdict:** provisional until C45's models are fitted, when the boxes should be the models'. **Confidence:** medium.

### Street trees are a prop type of their own, not a forest strip

**Choice:** An avenue's trees are props of `street_tree`, a catalog row that extends the forests' `trunk` (the same body) with a binding of its own, one a spacing.

**Gap:** A tree line could also be a forest stroke (C86).

**Verdict:** sound: a forest strip along a verge would conceal the squads walking it and thin every sight line down the avenue. A lone trunk is cover and nothing more. **Confidence:** high.

### Densities were set by Metro Large's tick cost and its admission

**Choice:** Parking shares of 0.33 to 0.38 in centre, apartment and core districts, lamps every 60 to 70 m, trees every 24 to 26 m, small furniture one to a few hundred metres: 16 825 bodies on Metro Large seed 1, +0.6% of a 120 s battle's ticks on the merged build.

**Gap:** Densities were delegated.

**Verdict:** provisional. The first set (30 440 bodies) cost +8.8%; a middle set left the largest Metro Large of 100 seeds at 57 484 authored bodies of the 60 000 the game admits, and this one leaves it at 52 049. Furniture now counts against that allowance, which was sized for buildings. `city_report`'s crossing aggregate overstates the cost, because the report's own `Battle::load` walks every prop between ticks. **Confidence:** medium.

### A prop kind with no art is drawn as a stand-in box

**Choice:** A prop kind no appearance is fitted to is drawn as the prototype kit's unit box, stretched to the prop's box and tinted by its kind (`presentation.stand_ins.tints`), as an ordinary model instance from `PropAppearances`. The first version put these boxes in the massing layer; main deleted that layer the same day, and this one rides the path props with art take.

**Gap:** `systems_only` kinds resolved to no model, and nothing else drew them: placed, they were invisible bodies.

**Verdict:** sound as a stand-in; C45's models replace it kind by kind. **Confidence:** high.

## C73

**When:** 2026-10-01. Evidence and numbers: the Outcome in [C73](slices/C73-tree-skeleton.md).

### The far tier is the lobed volume of the tree's own clumps, and clumps run to tier 2

**Choice:** A tree has four tiers of detail, finest first. Tiers 0 to 2 draw one set of leaf clumps, each coarser than the last; tier 3 is a lobed volume whose lobes sit just inside those clumps. The old seeded crown now only gives a tree its size and its clumps their places. (A spruce's far tier is its own shape, C74.)

**Gap:** The slice kept the old lobed crown as the far tier and the shadow-casting tier, and the tree-cost spike's (SG1's) first fallback put it on tiers 2 and 3.

**Verdict:** sound. A tree's shadow is cast by the tier under the one drawn. A hull of another shape shadows every clump inside it: hard-edged dark shapes across the crowns and 3.3 times the old trees' shimmer. With one shape down the tiers it measured 1.05 times. **Confidence:** high.

### About 24 clumps a crown, not 50

**Choice:** A broadleaf crown is 20 outer clumps and 4 inner ones, 2.5 to 4 m across.

**Gap:** Clump size was delegated; the design asks for broken-up crowns.

**Verdict:** provisional. Fifty small clumps gave open crowns with limbs and sky showing, the closest to the reference, and cost +1.6 to +1.9 ms where a wood fills the frame, over the trees' 1.5 ms. Twenty-four cost +0.5 to +1.0 ms and read as lumped masses with a broken outline, which the gate's fresh eye called "lump crowns on sticks". Opening them again needs a cheaper tier 1 or a larger tree budget: the owner's trade of look against frame time. **Confidence:** medium.

### Branch generations: trunk, limb, branch

**Choice:** A limb to each of five or six clusters of clumps, and a branch from the limb to each clump. Tier 1 draws them all on fewer sides; tier 2 draws the trunk only.

**Gap:** Branch generations were delegated.

**Verdict:** sound. A third generation only made sense with twice the clumps. **Confidence:** high.

### The triangle budget is 10,000 / 2,500 / 500 / 80 and lives on the scenery kind's row

**Choice:** `SCENERY_KINDS.tree.tier_triangles` holds the most triangles each tier may draw, and the asset validator refuses a tree over it (`budget.tier_triangles`). The numbers are what the landed trees measured at, rounded up.

**Gap:** A budget-setting gate was to fix the numbers and never ran; the slice did not say where they live.

**Verdict:** sound for tiers 1 to 3. Tier 0 has room to 25,000 by the cost spike's bar, but nothing was measured above 19,000, so it stays near what passed. The row already holds what a kind's art must carry, so every tree appearance meets one budget with no extra plumbing. **Confidence:** medium.

### A tree stands inside the simulation's canopy on every tier

**Choice:** The validator's `fit.canopy` measures every tier's top and its reach from the trunk's axis against the forest rule's `canopy_height_m` and `canopy_radius_m` (12 m and 6.5 m), not only the finest tier's height.

**Gap:** The slice's verification names the radius; the validator checked only the finest tier's height.

**Verdict:** sound. It holds the art as built; C76 holds placement to it by never drawing a tree wider than its art. **Confidence:** high.

### A tree's size is its seeded crown's top and reach, and clumps past it are brought back

**Choice:** The clumps are set a little proud of the seeded crown; any vertex above its top or past its reach is moved onto that limit, and the build fails if the finest tier does not reach both.

**Gap:** "Unchanged sizes" did not say how a different shape keeps a size.

**Verdict:** sound. Scaling the whole crown until its one widest clump fitted squeezed the tall kind to 0.59 of its width. Placement reads a kind's size from the finest tier, so the widest and highest clumps reach the tree's size exactly and no kind places smaller than it is built. **Confidence:** high.

## Rejected scale hypotheses

The [scale rationale's dead ends](../done/city-maps-scale/README.md#dead-ends-worth-retaining)
retain the rejected cost hypotheses; numerical controls remain in C06.

## C66 road core

### The export's `kind` column names the area's kind; no column was added

**Choice:** A paved stretch's, triangle's and boundary edge's existing `kind` column holds the area's own kind, as an index into the layout's `surfaceAreaKinds` (road, country_road, dirt_track, sidewalk). Before, it held the surface a unit finds there (road or sidewalk), so every carriageway said "road". The layout also names which kinds are carriageways (`roadAreaKinds`), and the renderer refuses a layout whose lists differ from its own.

**Gap:** The road-kinds slice left "a distinct exported tag per road kind" open and did not say whether it is a new column or the old one's meaning.

**Verdict:** sound. The surface a unit finds is a function of the area's kind, so the old column held less; a second column would be two owners of one fact. No record's layout moved, and no rule or digest changed. **Confidence:** high.

### Kinds are painted one over another by layer

**Choice:** The material keeps a distance per paved kind and paints the kinds lowest layer first. A biome row may name its `layer`; without one a kind takes the simulation's order, the earlier kind on top, so a dirt track ends at the edge of the country road it joins. The summer street names 2.5: over a track, under a country road.

**Gap:** The slice asks for a look per kind and says nothing of where two kinds overlap.

**Verdict:** sound. The contract says "where kinds overlap, the earlier one here wins" for speed, so by default the drawing agrees with what a unit drives on; one "nearest kind" for the whole pixel left a line of grass colour across every junction. The street is the exception because a side street's stroke ends inside the road it joins, cut square across its own line: drawn on top it was a slanted slab of asphalt across half the country road at every mouth. A street and a country road drive at the same speed, so the order between them says nothing about movement. **Confidence:** high.

### Patches change hue only, and only toward the warmer colour

**Choice:** A road surface's patches go to its palette's second colour scaled to the first's luminance, cover about a quarter of it, and the second colour is the warmer of the two.

**Gap:** "Palettes and mottle" are delegated; the design bans blobs that are both lighter and darker.

**Verdict:** sound. A critique read half-and-half patches as stains and camouflage, and a cooler tone as shadow beside real shadows although no patch was darker. **Confidence:** medium: a second critique still reads the patches as faint stains.

### The road's luminance rule is a scene check, not a rule of the biome

**Choice:** "A road is not a dark band on its field" is checked on rendered frames at three stations (the core against the ground 5 to 9 m out), not refused by `validateBiome`. The rule's final form is C31's, "A road is judged against the field beside it".

**Gap:** The slice asks for a mask-based check and does not say whether the biome should refuse a dark road.

**Verdict:** sound. A palette rule would have to pick "the grass": held against every plot palette it would forbid any road beside a bright crop, which is not what the design's rule is about. **Confidence:** medium.

## C67 road shoulder

### The shoulder is a wash of hue with thinner grass, not a band of its own colour

**Choice:** Beside a road the ground goes 45% of the way to the shoulder's colour where the wear is whole (`shoulder.cover`), and the grass thins across the same wear. The wash is at most 3.5 m wide on the gravel road and 2.2 m on the track, its outer edge wandering inward. A shoulder painted fully in a sand colour was built first and withdrawn.

**Gap:** The design asks for "a worn shoulder 2–4 m wide"; the road-wear spike (SG3) was to say whether one reads as an outline, and names "shoulder by hue and grass thinning only" as its first fallback.

**Verdict:** sound. The unprimed critique called the sand band an outline in every frame from 25 m to 250 m and round every lawn of the generated town; painted in the road's own colour it read as torn paper. **Confidence:** medium: the wash all but vanishes beside a field as bright as itself (stubble, wheat), so one road can show a shoulder on one side only.

### "Never darker than the grass" is arithmetic, not a palette

**Choice:** The shoulder's colour is scaled up to the luminance of the ground it lies on wherever that ground is the brighter.

**Gap:** The design asks that the shoulder stay at or above the grass's luminance; fields range from ploughed earth to wheat.

**Verdict:** sound. No palette row can hold that against every field; the lift holds it against any, and a later field palette cannot break it. One measurement does not fit it and is unexplained: see "Ground lane integration". **Confidence:** medium.

### The shoulder fades out by the tactical camera

**Choice:** The wash is whole while a pixel is under 3% of the shoulder's width and gone at 8% (about 0.1 m and 0.28 m a pixel for the country road: whole at 65 m, gone by 250 m).

**Gap:** The design says fine detail fades with distance; the slice says nothing of the shoulder at 250 m.

**Verdict:** provisional. A critique called the few-pixel fringe at 120 m and 250 m a halo, and possibly a contact shadow under a raised road; a fringe that thin cannot show its ragged edge. The fade was looked at only by its author; an unprimed critique at 120 m and 250 m would settle it. **Confidence:** medium.

### One wear for the ground and the grass

**Choice:** `groundShoulder` returns one wear value, and both the ground's colour and the grass read it. The grass keeps `shoulder.grass` (0.15) of the field's clumps where the wear is whole, and none where it is over 0.9 (the foot of the shoulder and the road itself).

**Gap:** "Grass density ramps across it from the surface field" does not say whether grass and ground share an edge.

**Verdict:** sound. Thin grass over unworn ground, or worn ground under a wall of blades, is the outline again. **Confidence:** high.

### `setRoadWearShown` is a lab switch, not a frame view

**Choice:** The paired cost run and the paired frames turn the roads' wear off through `BattleFrame.setRoadWearShown` (the lab's `suppressRoadWear`), beside `setTreesShown`. Off, the roads draw plain: no patches, grain, shoulders, ruts, strip, curb, paint or slab joints.

**Gap:** The slice asks for a frame-cost row and names no switch.

**Verdict:** sound. A frame view replaces the frame's composition; this is the finished frame less one thing, as the trees' switch is. **Confidence:** high.

## SG3 road wear read

### Answered inside C67 and C68, on the real material

**Choice:** No scratch worktree and no spike report: the shoulder and the ruts were built in the terrain material, critiqued unprimed, and cut back to what passed. The verdict is in the slice file: each kind its own surface; the shoulder a wash, not a band; ruts and a centre strip on the dirt track only.

**Gap:** SG3 was written as a throwaway spike to run before C67.

**Verdict:** sound. The per-fragment road loop the spike was to paint through no longer exists, and the rig made each question a station shot. **Confidence:** high.

## C68 ruts and centre strip

### The gravel road has no ruts; the dirt track has them and its strip

**Choice:** `roads.default.ruts` is empty. `roads.dirt_track` has one pair of wheel ruts 0.8 m either side of the centreline and a grass strip 0.7 m wide between them, on tracks up to 5 m wide.

**Gap:** The design asks for two soft ruts inside the core; the spike's fallbacks allow cutting them.

**Verdict:** sound. On the 9 m and 12 m gravel roads the critique read ruts as "pencil lines" and "pinstripes down the lanes" at 65 m in both shapes tried. On the track it read "a grass strip growing between two wheel ruts", "the best result in the set". **Confidence:** medium: a wide road therefore has no structure across it, and at 250 m it is a plain ribbon.

### Ruts change no pixel once they fade

**Choice:** A rut darkens its own line and the rest of the surface lightens by the ruts' share of the road, so the road's mean is unchanged; the fade (whole under 0.2 of a rut's width a pixel, gone at 0.4) therefore ends on exactly the surface without ruts.

**Gap:** "They fade to the core's mean below about 2 px per rut, using the same footprint fade as the plot rows."

**Verdict:** sound. The plot rows' fade keeps a mean term; a rut covers a tenth of a road, so a mean term would darken the whole road. The window is 2.5 to 5 pixels a rut, where the rows' is 1.7 to 4: a rut shimmers sooner than a field of rows. **Confidence:** high.

### The strip is never broken along its length

**Choice:** The centre strip's edge wanders but the strip has no gaps.

**Gap:** The slice says only that grass grows in a centre strip.

**Verdict:** sound. Broken by noise, it read as "olive dashes" at 65 m: a dashed line painted down the track. **Confidence:** medium: a second critique still calls the continuous strip "a firm-edged painted stripe" at 65 m.

### The strip fades out with the ruts

**Choice:** The strip is whole while a pixel is under 0.2 of its width and gone at 0.4, as a rut is: gone by the tactical camera.

**Gap:** The slice gives the ruts a fade and the strip none.

**Verdict:** provisional. At 250 m a critique read the strip, a few pixels wide, as "a dashed centre line (road marking)", hence the fade. It is checked at the 250 m track station and was looked at only by its author; an unprimed critique there would settle it. **Confidence:** medium.

### The lane a point lies in rides with the paved distances

**Choice:** `groundPaved` returns a struct: the distance inside each kind's paving and, for the stroke the point lies deepest in, the direction away from its centreline, the distance from it, its half width, the distance along it and its kind. Ruts, the strip, a street's dashes and its walk's slabs are all functions of those.

**Gap:** The slice says "keyed from the field" and names no carrier.

**Verdict:** sound. The loop that finds the paved distance already has the closest point of each stretch; a second lookup for the lane would walk the cell's list twice. Only a stroke has a centreline: a paved polygon has no lanes. **Confidence:** high.

## SG4 palette vs shadow floor

### The spike ran inside C84, not in a scratch worktree

**Choice:** `fog-look` (the scene that checks seen ground stays apart from unseen ground under every fog style) was run on the starting commit and on two drafts of the palette, and the verdict written into the slice file; there is no spike report.

**Gap:** The slice asks for a throwaway worktree and a spike report; the lane says to answer it on the way.

**Verdict:** sound. **Confidence:** high.

### The lightness floor is on a plot's darkest albedo

**Choice:** `PLOT_MIN_LSTAR` is 24: the CIELAB L\* of a plot's darkest albedo (its palette at the low end of the per-plot jitter, its rows at their mean) may not be under it. `validateBiome` checks it on the albedo, not on rendered pixels.

**Gap:** "A biome L\* floor enforced as a test" names neither the number nor what is measured.

**Verdict:** provisional. The spike found `fog-look` is not about dark fields at all: it bounds how light and how grey the ground round the houses is. The floor guards the case no framing of that check holds, a sunlit field as dark as ordinary ground under fog. The number is main's own darkest ground, not a measured threshold; a frame pair that finds where a dark field starts to read as fogged would settle it. **Confidence:** medium.

## C84 field palette

### How far the palette is desaturated

**Choice:** Every plot kind has a palette of its own, at albedo chroma 7 to 17 (rapeseed 34), about half of what the kinds had; two rounds, the second at 0.7 of the first's chroma. (The settlement's ground is C31's `yard`.)

**Gap:** "Desaturated (olive, tan, brown)" and a palette gate that never ran give no numbers.

**Verdict:** provisional. The low sun adds about 10 of b\* on screen, so the first round still read mustard and orange. On screen the fields are still half again as chromatic as the reference frame, and the gate's fresh eye reads half the fields as "one muddy olive". The owner's eye on the patchwork station against the reference decides which way to move. **Confidence:** medium.

### Plot weights unchanged

**Choice:** The kinds' shares are as C83 left them (rapeseed 1.2 of 13.9).

**Gap:** None stated; a critique found the yellow pulls the eye.

**Verdict:** provisional. Unchanged weights keep the same kind on the same plot, so before and after show the same fields. Whether there is too much rapeseed is a taste call nobody has made. **Confidence:** medium.

## C85 field texture

### What "luminance-neutral" and "one-sided" were taken to mean

**Choice:** Grain and rows are as much lighter as darker, so a plot's mean stays its palette's (within 0.6% per kind). Every term that changes brightness is finer than about 5 m and fades to its mean under a pixel. Anything broader (the dry patches) shifts hue at the plot's own luminance, one way only. A scene check holds that no 9 m block is more than 3% darker for the texture.

**Gap:** The contract says "luminance-neutral and one-sided" of a texture that must still show clods and furrows.

**Verdict:** sound. **Confidence:** medium: the widest grain octave, 5 m on rough ground, is the nearest thing to a broad dark patch.

### Wheelings

**Choice:** A drilled crop has tramlines: two furrows left bare in every fifteen rows, darker by 20 to 30%, and the grass leaves them bare.

**Gap:** The slice lists furrows, clods and stubble.

**Verdict:** provisional. From 250 m no grass is drawn and a crop's rows are near a pixel; the wheelings are what still says "drilled field". They are thin dark lines, not a broad patch, and take about 1% off a crop's mean. The gate's fresh eye liked them close up and read crop fields from 250 m as corduroy; the owner has not judged them. **Confidence:** medium.

### The old fine mottle no longer shades plots

**Choice:** The 4 m value noise (`field_rules.mottle_m`, renamed from `mottle_scale_m`) still varies the verge, forest floor, shore and road; a plot takes its grain instead, and the broad scale is each kind's `patch_m`.

**Gap:** None stated: the grain replaces it.

**Verdict:** sound. **Confidence:** high.

### The texture switch rewrites the plot table

**Choice:** `setFieldTextureShown(false)` packs the plots again without grain, row breaks or wheelings, and the shader skips what a plot does not have. There is no uniform flag.

**Gap:** The slice asks for a frame-cost row and names no switch.

**Verdict:** sound. The shader pays for the texture only where a plot has it, so off is truly off for the cost run, and the road checks use the same switch to judge a road against plain ground. **Confidence:** high.

## Fields on generated maps

### One grain for the land, a tract's own once, a road's where there is one

**Choice:** `generatePlots` cuts the land on one heading (`orientation_deg`) down to tracts of about `tract_m` (600 m); each tract turns once by up to `orientation_jitter_deg`; land in a tract with a road in it, or within a plot's width of one, lies along the longest such stretch. Land above tract size is cut by a road only where the road runs nine tenths of the cut's chord.

**Gap:** "Find why the plots lay out that way on an 8 km map and make open country read as farmland cut along its roads."

**Verdict:** sound for the fan, provisional for the wedges. Before, the heading turned at every level above a tract, a random walk that reached 45° across 8 km, and each stretch of a bend cut the land on its own line. A road cut is still one whole line through its plot, so a curving road leaves wedge-shaped plots; only field polygons carried by the map would follow a curve, which is the map lane's to add. **Confidence:** medium.

### The other lane's guide test fixes one more rule

**Choice:** `web/tests/surfaces.test.ts` fixes `max_aspect` at 50 beside the field rules it already fixes.

**Gap:** That fixture pins field rules "so seeded cuts do not obscure the guide contract"; a strip between two road edges now lies along them, is five times as long as wide, and was being cut across.

**Verdict:** sound. **Confidence:** high.

## C74

**When:** 2026-10-02. Evidence and numbers: the Outcome in [C74](slices/C74-tree-species.md).

### The one size is 11 m tall with a 0.40 m bole, held by the validator

**Choice:** `SCENERY_KINDS.tree.size` names a top of 11 m and a bole 0.40 m in radius at 1.3 m up, each within 5%; the validator refuses a tree outside either (`fit.tree_size`). The bole's radius is that of a round trunk with the cross-section the bark encloses there. The spreading and tall broadleaves were rebuilt into the band (10.0 → 10.5 m; 11.8 → 11.5 m with its bole 0.32 → 0.40 m).

**Gap:** The slice asked for "±5% of one height and girth" and "the validator's size envelope" without naming the size, where it lives or how girth is measured.

**Verdict:** sound. The size is the common broadleaf's, so the kind that fills most woods did not change, and placement already drew every kind to one height. Placement draws a tree 0.9 to 1 of its width, which puts the drawn bole at 0.36 to 0.40 m beside the simulation's 0.35 m trunk. **Confidence:** medium: the simulation's trunk radius is not the number checked, only near it.

### A conifer is the same clumps in another arrangement, with its own far tier

**Choice:** A spruce's clumps are boughs in whorls up one leader under a spire; its far tier is the boughs' outline turned round the leader (78 triangles), not the lobed volume.

**Gap:** Branch and clump arrangements were delegated; C73 said the far tier is the lobed volume of the tree's own clumps.

**Verdict:** sound. A lobed volume is measured from one centre and misses a spire's tip. The turned outline stands inside the boughs as the lobed volume stands inside its clumps, so the shadow caster is still the tree's own shape, coarser. **Confidence:** high.

### The pine is a broadleaf-form crown, not a second conifer form

**Choice:** Ten flat pads on a seeded lobed crown, 6 m up a bare bole.

**Gap:** The design names spruce and pine as the conifers and gives one conifer profile to port.

**Verdict:** sound. A Scots pine's crown is an irregular lump on a pole; the whorled spire is the spruce's. It reads as a mushroom alone on the sheet and as a pine in a stand. **Confidence:** medium.

### A snag is a skeleton, drawn on every tier

**Choice:** A bare kind (a standing dead tree) draws bark alone: the whole skeleton on the two near tiers, the trunk and limbs on the third, the trunk and each limb as one segment on the far tier. Its height is its highest branch end.

**Gap:** The slice named a standing snag; the tiers and the far tier's crown assumed leaves.

**Verdict:** sound. It is 22 triangles at the far tier and nearly vanishes there, which is right for a bare pole. **Confidence:** high.

## C75

**When:** 2026-10-02. Evidence and numbers: the Outcome in [C75](slices/C75-forest-mix-and-colour.md).

### A stand is the cell nearest a seeded point, and its family is drawn by weight

**Choice:** The land is cut into cells about `trees.stands.size_m` (140 m) across, each one family's (broadleaf or conifer); a tree is of its stand's family `purity` (0.88) of the time and of any family otherwise. A species' weight is its share of all trees, whichever way it is chosen.

**Gap:** "Mostly one family per stand" did not say what a stand is.

**Verdict:** sound. A forest shape is not a stand: one big wood should hold several, and a small one is one. The cells follow the ground, so a wood and the copses past the map share them. Their edges are straight where two cells meet; at 0.88 the mixing hides it. **Confidence:** medium on the two numbers.

### Birch and snag have no family

**Choice:** A species with no `family` is the odd tree in any stand, at its weight's share: birch 6.6%, snag 2%.

**Gap:** "Plus the odd birch and snag" against "weights over all kinds".

**Verdict:** sound. One list of weights still says how often every kind is drawn. **Confidence:** high.

### "Not in the outer ring or in strips" is a row's `interior_m`

**Choice:** The snag's row carries `interior_m: 12`: it stands only that far inside its forest's edge, never in a strip-shaped forest and never past the map. A tree refused there is drawn as one of its stand's family.

**Gap:** The slice named the rule for snags; the renderer has no notion of a snag.

**Verdict:** sound. The rule is data on the row, not a named kind in the code. 12 m is past one trunk spacing (9 m) and its jitter. **Confidence:** high.

### Conifers differ from broadleaves in hue, not in brightness

**Choice:** Tints over the shared leaf colour: spruce `[0.74, 0.88, 1.25]`, pine `[0.86, 0.9, 1.1]`, birch `[1.1, 1.08, 0.85]`.

**Gap:** Tints were delegated; the slice's own critique question is whether a darker crown reads as cloud shadow.

**Verdict:** provisional. Stronger first tints kept the frames' mean luminance, but fresh eyes saw conifers' shaded sides go blue-black and birches read as autumn. Both were pulled toward the broadleaf green; the softened tints landed with C76 and were never critiqued on their own. One unprimed look at a mixed wood would settle them. **Confidence:** medium.

## C76

**When:** 2026-10-02. Evidence and numbers: the Outcome in [C76](slices/C76-canopy-closure.md).

### No crown passes the simulation's canopy radius, because no tree is drawn wider than its art

**Choice:** `trees.forest.girth` is a tree's drawn width as a share of its appearance's own, and the biome refuses a value over 1. The asset validator already holds every appearance inside the forest rule's `canopy_radius_m` (6.5 m), so no drawn crown passes it.

**Gap:** The slice bounds the crown by the simulation's canopy radius, but that number does not reach the renderer: the world layout exports a forest's canopy height only, and this lane may not change `crates/`.

**Verdict:** sound. The simulation's number keeps one owner and placement needs no copy of it; a test places the village's woods and checks every crown against the fixture's radius. Before this, crowns reached 6.95 m. The cost: closure is tuned in two places, the art's reach per species and the one `girth` range. **Confidence:** high.

### Crowns no longer fit inside the forest's shape

**Choice:** Placement used to narrow a crown to its room inside the forest's outline, with no floor, so a trunk near the edge drew a tree a few centimetres wide. That clamp is deleted: a tree at a wood's edge is as wide as one inside, and its crown overhangs the field.

**Gap:** C73 found the blade-thin edge trees and left them here; the tree-line slice's simulation half says foliage reaches a canopy radius past a strip's edge.

**Verdict:** sound. The drawn crown now follows the simulation's foliage, not the forest floor's outline. **Confidence:** high.

### Closure: girth 0.9 to 1, wider conifers

**Choice:** `girth: [0.9, 1]`, with the spruce built 5.3 m in reach (from 4.6) and the pine about 5.5 m.

**Gap:** Crown scale within the radius was delegated.

**Verdict:** provisional. At one girth for every species the broadleaf stands closed and the conifer stands stayed an orchard (a third of the floor seen); widening the conifers' art closed them to under a quarter. Narrow kinds (birch, the tall broadleaf) stay narrow: they are the gaps. It moves with the closure number below. **Confidence:** medium.

### "Mostly closes, floor still seen" is a band on the class mask: 0.12 to 0.40 of the floor seen

**Choice:** In the terrain's class mask, the share of the wood's floor pixels still seen with the trees drawn must lie between 0.12 and 0.40, at 65 m and 120 m over the village's west wood (`web/scenes/_canopy.mjs`). The landed closure is 0.23.

**Gap:** The slice names "the visible floor share from a models on/off pair" and no number.

**Verdict:** sound, accepted by the owner (2026-10-02). Below about a tenth the wood is a lid; above four tenths it read as an orchard (it measured 0.32 before). The band is wide on purpose: it is a guard, and pictures decide inside it. The owner saw 0.23 with the squad it leaves to the x-ray (0 of 8 torsos seen plainly under two crowns): "the 23% check is fine, and we can always tweak that number in the future". The slice's torso check is not in a scene; a unit under a closed crown is found by its x-ray. **Confidence:** high.

### The forest floor's drifts are turned noise, with their strengths in the biome

**Choice:** `forestFloor` mixes moss and humus over the leaf litter, each from two octaves on lattices turned against each other and the map's axes; `forest_floor.moss` (0.7) and `.humus` (0.4) say how far each takes the litter over, in patches 6 m across (from 4 m).

**Gap:** The brief asked for the floor's blockiness to be fixed inside that one function if the floor is to be seen through the canopy.

**Verdict:** sound. The bare floor's edge density fell from 0.12 to under 0.02 and its darkest twentieth rose from 69 to 83 of 255: no square patches, and no dark blob to read as shadow. Humus at 0.4 is a guess on the safe side. **Confidence:** medium.

## Ground lane integration

### Road checks judge roads against plain fields

**Choice:** After the fields passes (C84, C85) the road checks shoot with the fields' texture off, the road's first metre-wide band may hold 45% of the field's clumps (was 35%), and the dirt track's palette is a tenth lighter. The rule the checks hold is C31's, "A road is judged against the field beside it".

**Gap:** C66 and C67 state their rules against "the grass"; the fields beside the stations became pale crops with furrow-shaped rows, denser grass and different plots.

**Verdict:** provisional. A furrow or a wheeling lying along a road's edge puts its own stripe in a band of the check, so a road is judged against plain ground, its one variable. One measurement is unexplained: on bare ground the 5 m beside the village's gravel road reads about 6% darker than the field 5 to 9 m out, where before the fields passes it was lighter, although the shoulder's wash is lifted to the luminance of the ground under it. With grass drawn, the frame a player sees, the walk holds. Finding that cause would settle this entry; a track through stubble, the palest field, is not shot anywhere. **Confidence:** medium.

### The verge's grass is checked between plots, not beside the road

**Choice:** The grass check finds the verge where the class mask changes plot, clear of any road.

**Gap:** C67 deleted the verge keyed to the road; the wild-grass check sampled verge grass beside the road.

**Verdict:** sound. Beside a road the ground is its shoulder, which grows the plot's own grass, thinned. **Confidence:** high.

### The renderer skill keeps main's rewrite

**Choice:** The lessons the ground passes had added to `.agents/skills/renderer/` were dropped at merge in favour of main's principles-only rewrite; the lane's lessons are in its slice Outcomes.

**Gap:** `AGENTS.md` asks each pass to add its lessons to the owning skill; main rewrote that skill to hold architecture only while the lane was running.

**Verdict:** sound: it follows the owner's change to the skill. **Confidence:** high.

## C28 pavement

### A town's sidewalk is a walk beside each street, a look the map does not hold

**Choice:** A row of the biome's `roads` may name a `walk`: a band beside each stroke of its kind (2 m on a street), drawn by another row (the `sidewalk` row's slabs). The simulation has no sidewalk there: units find ground, as they do on a sidewalk a map does name.

**Gap:** The slice says "roadbed and sidewalks cover exactly the right ground", and no map holds a sidewalk: the generator writes streets as `road` strokes and yards as `road` polygons, never the `sidewalk` kind.

**Verdict:** provisional. It is the shoulder's place and the shoulder's contract (the surface is the rule, what lies beside it is its look), and a sidewalk moves as ground by the contract, so the walk misstates nothing a unit can feel. The class mask still says only the simulation's paving. If the generator writes sidewalks one day, they take the same row and `walk.width_m` goes to 0. **Confidence:** medium.

### A street's yards are drawn as paving, by the sidewalk's row

**Choice:** A row may name `area`: the row that draws its kind where it is laid as a polygon. A generated town's loading yards are `road` polygons and are drawn by the sidewalk's row, so asphalt is the carriageway and nothing else.

**Gap:** The roads pass left "a polygon union carries one distance for all its kinds, so street-to-sidewalk has no edge to feather".

**Verdict:** sound, and it routes round that problem rather than solving it: every generated street is a stroke, so the street's edge is the stroke's own, and the polygons beside it are all one look. Two kinds of polygon side by side with different looks would still have no edge between them. **Confidence:** medium: no station stands on a yard.

### What is painted and what is the rule are two distances

**Choice:** `groundPaved` returns `drawn` (how far inside each row's paving: strokes, areas and walks, by the row that draws them) and `rule` (how far inside the simulation's paving). The colour and the grass read `drawn`; the class mask reads `rule`.

**Gap:** The slice says pavement "reads the surface distance field" and bakes nothing.

**Verdict:** sound. Nothing is baked: the walk is one more `max` in the loop that already visits each stroke, and a check reading the mask still sees exactly what a unit drives on. **Confidence:** high.

### A map that names no kind but `road` keeps the country road's look

**Choice:** `roads.default` is the country road's gravel, and `country_road` takes it; `road` (asphalt), `dirt_track` and `sidewalk` have rows of their own. On a map whose paved areas are all kind `road` (the village, the geometry lab), `road` is drawn by the country road's row (`drawnKind` in `terrain/surfaces.ts`). On any map that names another kind, `road` is a town street.

**Gap:** The village's 12 m roads are country roads by the design, but its map says `road`; its map is not this lane's, and nothing the export carries says "urban".

**Verdict:** provisional. Every generated map has a country road or a track (a settlement stands on one), and every map drawn before roads had kinds has only `road`, so the rule is right on every map today, and the village's bare ground and class mask did not move by a byte. It is a guess from what a map happens to name: a town-only map would draw gravel streets. It is one function to delete when the village's map says `country_road`, which is the map lane's edit and the owner's call. **Confidence:** medium.

**Resolved by C64's authored cutover:** the rural maps now name `country_road` directly and the inference is deleted. An authored `road` selects the street row regardless of the other kinds present. The temporary compatibility choice is superseded; no new appearance policy replaces it.

### `join_m` is the lower road's own

**Choice:** Where one road runs under another, the lower road's `join_m` says how far its surface is carried onto the upper. Before, it was the upper road's number. A track's is 2 m; a street's is 3 m (C31; it was 0, ending on the country road's edge).

**Gap:** C66 wrote `join_m` on the road that is joined; with a street under a country road that blurred the street's mouth beside sidewalks that end on a ruled line, and a critique read the blur as a shadow's edge.

**Verdict:** sound for whose number it is. How the join looks is C31's entry, and is not good enough yet. **Confidence:** high.

### Asphalt is a neutral grey above the lawn's luminance

**Choice:** `road_asphalt` is `[0.35, 0.357, 0.375]`, a little cool, with paler patches at the same brightness and a roughness of 0.9; `paving` is `[0.55, 0.555, 0.56]`.

**Gap:** Palette is delegated; the design bans a road darker than the grass.

**Verdict:** provisional. On screen the roadbed is 1.7 times the lawn and 0.64 of the gravel road. A lighter grey came out as bright as the gravel, and a warm one sat on the tone of gravel in a building's shadow. The cool tint does not cure that: critiques still read shadowed gravel as asphalt where a gravel road runs between three-storey shadows. The reference's asphalt is 1.1 to 1.3 times its grass; ours cannot go that dark under the no-dark-road rule, so going darker is the owner's call on that rule. **Confidence:** medium.

## C29 curbs

### The curb is shading and a line of stones, not geometry

**Choice:** A row's `curb` is a band of kerbstones just outside a stroke's edge (0.3 m wide, a joint every metre) and a tilt of the shading normal across a narrow face just inside it. The ground is not moved and no mesh is added.

**Gap:** The slice says "presentation curb geometry" in `terrain/` "or the static chunks", and asks for joins, intersections, slopes and no cracks.

**Verdict:** sound. A 12 cm step has no parallax to see at 25 m and nothing to hide a vehicle behind; drawn in the material it follows the street's distance, so it has no joins to crack, turns every corner and lies on any slope. It is the ruts' mechanism. **Confidence:** medium: critiques read the kerb as a flush strip or a painted line from the ground camera, raised only on the side facing away from the sun.

### The face tilts 15°, and the curb is gone before it is a pixel wide

**Choice:** `tilt_deg` is 15; the curb fades out as a pixel grows from 0.25 to 0.6 of the stones' width (whole at 65 m, gone by 250 m).

**Gap:** "Curb profile as data" gives no numbers.

**Verdict:** sound. At 35° the face turned from the sun was "a near-black hairline on one side of the street only"; at 250 m a kerb under two pixels wide showed on streets running one way and not the other. **Confidence:** medium.

### No curb where another road covers the edge

**Choice:** The curb is left out wherever another carriageway's surface lies over the street's edge: across a street's mouth, and where two streets meet.

**Gap:** "Intersections" is named without a rule.

**Verdict:** sound. **Confidence:** high.

## C30 markings

### A dashed centre line, and a crossing's bars before every road that runs on across

**Choice:** Every stroke of a kind with `markings` has a dashed line down its middle, laid out by the distance along the stroke, and a crossing's bars where another carriageway's stroke crosses it and runs on past its far edge. A crossing is laid only beside the crossing stretch's own length, never round its end, so a side street has a crossing at its mouth and the street it joins has none there. Nothing is placed by a list: the lines are a function of the strokes.

**Gap:** "Deterministic, generic marking placements" names no layout.

**Verdict:** sound. Without the "beside its own length" rule a fan of bars was painted on the main street round the end of each side street. **Confidence:** medium: every arm of every junction has the same bars and nothing else (no stop line), a side street has a crossing where it meets a gravel road, and a crossing road's bend can leave a bar's width unpainted.

### A dash is whole or absent

**Choice:** A dash that would come within 0.3 m of a crossing's bars is left out whole: each point of it is judged at the dash's end nearer the road that crosses, extrapolated along the stroke.

**Gap:** Dashes are laid out by the distance along a stroke, so one can straddle the place the line must stop.

**Verdict:** sound. Cut at the stop, a dash left "a stub about a metre long" beside the crossing. The extrapolation is exact where the crossing road runs straight. **Confidence:** medium.

### The paved reach is set by a crossing's bars: 4.3 m

**Choice:** `groundReach` reads the paving as far as a dash that must stop short of a crossing's bars reaches: the bars' gap and length, 0.3 m, and a dash's length. The summer numbers come to 4.3 m on every map. Until this slice the paved reach was the widest shoulder's 3.5 m (C67), which lies inside it.

**Gap:** A crossing is placed from the edge of another road, which a point reads only within the field's reach.

**Verdict:** sound. The class mask's road distance is exact 0.8 m farther out than it was, and the bare ground did not change by a byte. The surface field's index grew by under 1% (651 to 652 KiB on the village, 9,834 to 9,920 KiB on the generated 8 km map). **Confidence:** high.

### Paint is dull, worn and fades with distance

**Choice:** Off-white `[0.88, 0.88, 0.86]` hiding 85% of the asphalt, worn in patches; a centre line 0.2 m wide in dashes of 1.5 m every 4.5 m. A line fades out as a pixel grows from 0.35 to 0.85 of its own width: the centre line is whole at 65 m and gone at 250 m, a crossing's bars (0.5 m) last to about 250 m.

**Gap:** "Legible at battle distance and never mistaken for tactical marks."

**Verdict:** provisional. Order paint is saturated yellow, glows, marches and keeps a width in pixels; road paint is lit like the ground, darkens in shadow, has no glow and a width in metres. Asked directly, two critiques put the crossings at no risk and the centre dashes at low risk against yellow marks and at low to medium against a white dashed mark along a street (the zone outline and the supply reach are white). A battle frame with a zone outline along a marked street would settle it. **Confidence:** medium.

## C33 simplified

### The page builds its own world; the public query export is deleted

**Choice:** The worker still builds the simulation's world once for the planner and the battle. The page builds its own plain world from the scenario's map with the simulation's `WorldView`, as it did before the startup lane, and answers picking, ground height, surface, learned foliage and camera clearance from it. `PublicWorld`, its lossless query payload, the worker-to-page transfer, the `PreparedWorld` copies of the world's exports and queries, and the tests that held the two query implementations in exact agreement are deleted: about 1,000 lines of production code and 170 of tests.

**Gap:** The lane's contract asked for a page that constructs no simulation world. The owner decided on 2026-10-01 that this half goes.

**Why:** Startup is 5 to 6 s against a budget of 30 s, and the lane's own measurement showed no startup gain from the export. Margin is spent on simplicity ([`AGENTS.md`](../../AGENTS.md), "One owner per concept"). The export was a second implementation of height, surface, ray and foliage queries that had to match the first to the bit; one code path for map queries is worth more than one fewer world build.

**Reach:** One battle builds the world twice (worker, page), not once. The page's build runs behind the loading screen. The page's world has no navigation and no battle state, so no query can read hidden destruction. Where the export had moved surface, terrain-ray, water and learned-crown arithmetic into free functions so two worlds could share them, that arithmetic is back inside `WorldGeometry`, its one caller. The worker handoff, cancellation, the prepared replay, the fog change, the camera lab's catalogue map, the combat parity pair and the startup harness are unchanged.

**Verdict:** sound. Battle digests and replays do not move; the generated and camera scenes pass unchanged; Mixed Small is playable 5 to 9 s after Deploy on a loaded machine, against 3 to 4 s for the export on a quieter one, with level retired instructions ([startup measurement](startup-lane.md#startup-measurement)). **Confidence:** high.

## C06: avoid geometry for a squad with no steering decision

**Choice:** A vehicle moves far from an idle squad. Each living soldier is already
at his holding post, if any, and the existing vehicle-dodge reader returns no
motion. Skip gathering local obstacle geometry for that squad. Share the exact
holding-post eligibility predicate with ordinary steering; do not create another
motion rule. Nearby traffic and unfinished cover posts retain normal movement.

**Gap:** The threat list described global moving vehicles, so its nonempty state
prevented the existing idle shortcut even when none could affect this squad.

**Reach:** Preserve velocity reset and garrison handling, the old empty-threat
return, and centroid settlement for the nonempty-threat no-steer tail. Crowd and
unit iteration order stay unchanged. This adds no cache, state or budget policy;
future changes to post motion inherit one eligibility owner.

**Verdict:** sound; high confidence. The omitted geometry cannot be consumed by a
steering decision in this branch. Paired measurements and exact battle evidence
live in C06; current full admission remains separate.

## C06: build cover exposure only for IDs its immutable knowledge can read

**When:** the untracked-cover-field pass, after the current move-certification
profile identified repeated ephemeral field work.

**The choice:** leave an empty position row for a unit neither side tracks,
while retaining its outer unit-ID slot. Imagine a late battle with thousands of
squads that were already fallen when it began. Movement repeatedly constructs
their exposed soldier positions, but its cover reader asks for an enemy track
before looking up that row. Because knowledge cannot change during this movement
call, those particular positions cannot be read. A squad either side does track
keeps all its member positions in order, including fallen members and remembered
sightings. Keeping only living/currently visible rows would change remembered
aiming; dropping the outer slots would shift every later unit's address.

**The gap:** C06 names repeated-work costs but does not prescribe this field's
read predicate. The measured parent task selected this single bounded candidate;
the pass verified the sole consumer and immutable lifetime before implementing it.

**The reach:** the field remains an ephemeral snapshot with the same one reader;
there is no extra cache or model state. A future reader that accesses untracked
rows must revisit the gather predicate. The paired complete observations prove
this current reader, not a speculative future use.

**Verdict — sound, high confidence:** the omission follows an unreachable read,
not a unit class, corpse count or hand-picked visibility rule. It earns the
predeclared whole-Orders gain and exact per-tick battle/observation proofs; the
smaller whole-step improvement is reported separately. Existing public cover,
last-seen, hidden and replay tests own the semantic contract, with tracked-row
omission falsified through the cover behavior rather than private vector shape.

The same unread-work principle applies to hull gathering. In a late battle,
fallen infantry can require a full member scan to establish liveness, but a unit
without a hull cannot emit vehicle geometry. Reject that row before checking
liveness. Vehicles keep the same health check, ordered geometry and pure mapping;
all consumers inherit the existing query rather than a cached hull projection.
Battle outcomes and serialized replay content remain exact apart from engine
identity. The bounded native evidence belongs to the [C06 outcome](slices/C06-sim-scale-passes.md#outcome--reject-non-hull-rows-before-member-liveness);
browser admission remains separate.

## C14: damage states of the scripted sets

### A ruin is held to the building's ruin height, not each part's

**Choice:** The fit rule gives every part of a collapsed template remains of one height: the rule's fraction of the building's height (its tallest part's top), between the rule's least and most. A set's `ruin` rows fit each part's plan at that height.

**Gap:** The brief for this pass said "25% of the part's height clamped 2 to 6 m"; C42 says "25% of the building's height".

**Reach:** Only templates whose parts differ in height, which here are the three farmsteads: the yard farm's 3 m cart shed leaves 2.1 m of remains (a quarter of its 8.4 m barn), where its own quarter would clamp to 2 m.

**Verdict:** sound. `Battle::destroy_prop` scales `geometry.height_m`, the materialized building's, for every part, and the art is made to what the simulation builds. **Confidence:** high.

### The rule's numbers are read from the building prop type, in both places

**Choice:** The bake reads the rule through `Authority.collapse` (the unit catalog's prop types that carry `destroyed.into.building`; differing rules are refused), and the Blender side reads the same row from the catalog's resolved view, `fixtures/catalog.json` (`city/collapse.py`; `village.py` now asks its template instead of keeping a copy of the formula). Neither holds a number.

**Gap:** "Read the ruin rule's numbers from where the simulation's rule lives ... rather than copying constants"; Blender scripts cannot call the unit catalog's resolver.

**Verdict:** sound. The resolved view is generated and a test fails while it is stale, so an `extends` on the building row reaches the scripts. **Confidence:** high.

### `fit.ruin_top_m` is optional and 0 when absent; 0.6 m here

**Choice:** A set says how far a ruin's broken walls may stand above the remains as `fit.ruin_top_m`. A set without it gets no allowance. `homes`, `farmsteads` and `industry` say 0.6 m; `towers` has no ruin and no number.

**Gap:** "a small top allowance for jagged wall tops, which you put in the set's `fit` as a named number".

**Reach:** The apartments' set must write the field to stand above its remains at all; without it its ruin is refused wherever a stump tops the box.

**Verdict:** sound. A default allowance would be a number nobody chose. 0.6 m is a quarter to a third of a 2 to 3 m ruin: stumps read as broken walls over the heap, and the box still describes what stops a round. **Confidence:** medium (the number was judged in pictures, not in play).

### A wrong or missing damage state is an error, so this branch's library does not bake alone

**Choice:** `templates.state` refuses a template without its damage state, with the other state, or with both, at error severity. With the apartments' states not yet written, `asset bake` refuses the library on this branch with seven findings, all `china_apartments`; the committed runtime was left as it was.

**Gap:** "the asset check refuses anything else by name" and "locally `asset bake` and `asset check` must pass" cannot both hold before the other half of the slice lands.

**Verdict:** provisional until the apartments merge. A warning, or a rule a set opts into, would have baked here and would not refuse a set that forgot its states. The 22 templates of this pass were proved by the strict bake naming none of them, and drawn in the line-up from a local bake with stand-in apartment states that is not committed. **Confidence:** high that strict is right; the integrator rebakes after both halves are in.

### A damage state may not draw more than intact, and the helper refuses it

**Choice:** `Kit.write` fails a set whose damage state draws more triangles than `intact` at any tier, or lacks a row at a tier, or has the wrong state for its floors.

**Gap:** The budget was stated in the brief; nothing said who holds it.

**Verdict:** sound. It moved several designs: ruins of small houses draw fewer heaps, long buildings coarser stumps at tier 2. The bake reports a damage state's triangles but does not gate on them. **Confidence:** high.

### A ruin's openings are read from the intact rows

**Choice:** `ruin_sides` finds, for each wall of a part, the ground-floor fittings the intact rows hang on it, and the stump is broken to each one's sill (a door's to the ground) with the wall cut at its jambs. Party walls stand; a stretch open to a neighbour part has no wall.

**Gap:** "ragged wall stumps with window openings broken to their sills".

**Verdict:** sound. The ruin's doorway is where the door was without a second table to keep in step. **Confidence:** high.

### A terrace's houses are two ruin modules and a level each

**Choice:** A row of houses falls as its intact art stands: one module a house (two variants, alternating) in the house's own tint, each scaled down to its own level (1.0 to 0.74), with the party wall and chimney breast on its west side and the row's east end wall a module of its own.

**Gap:** "A terrace's units break unevenly, so the row is not one flat line."

**Reach:** A terrace's ruin is one row a house and one for the end at the coarse tiers, as its intact state is; not one row.

**Verdict:** sound. One shell for the row could carry one tint, and the houses' colours are what say which terrace it was. **Confidence:** medium: the critique still found each house's heap and roof pieces alike.

### Timber and steel fall their own way; the heap under them is the same recipe

**Choice:** A boarded barn's stumps are thin lengths of board, burnt to the foot, between charred posts; a timber-framed barn keeps its plaster stumps and gains the posts. A steel shed's walls are its dado's stumps with torn cladding standing over them, its frame's legs leaning, and its roof's own sheets buckled over the heap. All of them stand on the one `rubble` recipe, recoloured (ash under timber and steel).

**Gap:** "An industrial shed collapses as buckled sheet and bent portal frames over rubble, not as masonry"; nothing was said of timber.

**Verdict:** provisional. The critique read the steel sheds' heaps as masonry rubble under the sheets. A recipe of ash and scrap is a later, cheap change. **Confidence:** medium.

### A gutted tower is the shell built twice, and its soot is in the recipes

**Choice:** `shell(..., burnt=True)` builds the same module in burnt facade recipes (one a kind of opening, plus a blown bay), and tier 0 places `gutted_<kind>_<quarter>` panels that show the quarter of the recipe the shell shows on that bay. Four to six bays a tower are blown out, by a list on the template. Balconies are scorched, broken or hanging by a fixed roll; aerials are gone.

**Gap:** "soot streaks and missing panels in the shell's own coarse-tier texture or vertex colour".

**Reach:** Seven more facade recipes in the towers' kit (about 2 MB of the source file), and four balcony recipes that carry their fronts' colours.

**Verdict:** sound. Soot as vertex colour alone read as a slightly darker wall; in the texture it has a shape at every tier. **Confidence:** high.

### The wall of a gutted tower darkens with height, and the panels' tints follow it

**Choice:** `smoked(z)` darkens a gutted shell's walls from 6% at the ground to 46% at 50 m, in vertex colour; a tier 0 panel's row tint is dimmed by the same function at its floor, and the burnt recipes' tint mask is the whole wall, so both dim the soot and the spall alike.

**Gap:** None in the slices: it answers "obviously dead ... from across the map".

**Verdict:** provisional. It is what separates a gutted tower from an intact one at tier 3, and the critique's worry stands: a facade in shade is also darker. The openings, blown bays and ash carry the rest. **Confidence:** medium.

### Roofs: one tile recipe without a quilt, a neutral slate, and stains in the roof's own paint

**Choice:** `roof_tile` is small staggered tiles within a few percent of one tone, with soft shallow courses and nothing wider than a tile; `roof_slate` is the same in neutral grey, for slate and stone roofs (a recolour of the clay recipe turned its variation into a teal and pink cast); a roof's material takes no per-vertex grain and no tile-scale lichen, and `weathered_roof` lays drift and moss in fields metres across.

**Gap:** The coordinator's note: no cross-hatch at 80 m or 250 m, one weathered colour family, variation in large soft patches.

**Reach:** Every shell of `homes` and `farmsteads` (the files that moved: both sets' `kit.glb` and `templates.json`).

**Verdict:** sound in the line-up at every tier. **Confidence:** high.

### The coarse tiers fold fittings into the shell, and a far house is one row

**Choice:** `kit.fold_far` copies what a fitting is from far off (panels and boxes, from a table a set keeps) into the template's shell at tiers 2 and 3, and the fitting's row stops at tier 1. `homes` now does at tier 2 what `farmsteads` did, and both do it at tier 3. A terrace folds the fittings of its second house into the one house module; its chimneys and shop signs stay rows at every tier, because they differ from house to house.

**Gap:** C23's list asks for the openings at tier 3 "as wall texture or vertex colour"; the kit readme says a far building is one row.

**Reach:** Tier 3 of a house rises from 12 to 46 triangles to 98 to 512. A terrace's doors are one colour from tier 2 out.

**Verdict:** sound: the validator requires a module's coarser tier to draw no more than its finer one, so the panels could not be tier 3's alone. **Confidence:** high.

### `assemble.py` picks tiers as the game does

**Choice:** The reassembly sheets choose a tier by pixels to the metre (`presentation.buildings.lod_px_per_m`), not by projected height, and `tower_sheets.py`'s tier sheet stands at those distances.

**Gap:** C18's outcome says a tower is at tier 0 out to 500 m; C22 chose pixels to the metre.

**Verdict:** sound. The old sheets showed a tower's tier 0 where the game draws tier 1. **Confidence:** high.

### Three towers differ by one built thing; the 20-floor tower was left

**Choice:** The 12-floor tower's top floor and parapet are terracotta under a roof slab that stands 0.55 m out (inside the side fit); the 16-floor tower's windows are ribbons the bay's width; the 20-floor tower is unchanged.

**Gap:** "vary one structural thing each ... if it is cheap". A setback top would have left the part's faces.

**Verdict:** sound for two; the third differs from them by being the plain one. **Confidence:** medium.

### The village's older ruin is held to the remains, and let past the budget

**Choice:** The village set (C37) says `ruin_top_m` 0.6 like the others, so its farmhouse ruin (walls to 0.35 m over the remains) passes the new fit; `Kit(..., damage_budget=False)` lets its ruin draw more than its farm (2,299 against 1,916 triangles at tier 1, 371 against 260 at tier 3 on the smallest box), and nothing else may.

**Gap:** The budget was set for the four sets of this pass; the village set arrived by merge with a ruin made before it.

**Verdict:** provisional. The opt-out is one named flag on one set. Rebuilding the farm's ruin with the shared `ruin_block` would remove it and is the better end. **Confidence:** medium.
## C45 street models

The models, their boxes and the sheets are in the [C45 outcome](slices/C45-street-models.md#outcome).

### Each street kind's box is the appearance's, since the catalog rows carry none

**Choice:** A street prop's row has no size; a placement gives each body its box. Each model is authored to one real-world box (`footprint_half_m` in `assets/catalog.json`) and the renderer fits a placed box from it per axis. A car placed at half its length draws a squashed car; generated maps place these props at the generator preset's boxes, not yet at the models'.

**Gap:** The slice asks whether each model "fits its body", and the street-bodies slice gave the bodies no dimensions.

**Verdict:** sound. One owner per number: the art's box is in the art catalog, the placed box in the map. **Confidence:** high.

### One scenery kind per street row; the car's wreck is its own row's art, 0.7 m tall

**Choice:** Every street row is drawn by its own scenery kind, each with one appearance and one `default` state. The parked car's destroyed state is the `car_wreck` row, drawn by its own appearance on the car's plan at the remains' 0.7 m: a burnt shell on its rims, the roof fallen into the cabin. A test holds any remains with art of their own to the destroyed body's plan and height. No other row has remains: the rest are removed.

**Gap:** "Each has LODs and its terminal state."

**Verdict:** sound for the binding, provisional for the height. A real burnt-out car is about 1.2 m; the 0.7 m is the simulation's remains height and the art follows it, so the shell is crushed flat. If it reads as too flat in a street, the fix is the row's `height_m`, which changes what the wreck hides and stops: a mechanics decision, the owner's. **Confidence:** medium on the look, high on the binding.

### A street tree is a tree body outside every forest, drawn by the scenery as a tree of its own

**Choice:** The `street_tree` row is drawn by `forest`, as the trunk it extends is, and is no longer `systems_only`. The scenery draws every tree body: a forest's trunks by the forest's rule, and a trunk no forest generated as a lone tree (`trees.lone` in the biome): one of the species named there (`tree_tall`, `tree_birch`), 0.85 to 1 of its own body's height, 0.7 to 0.85 of its appearance's width. No new art, no new scenery kind, no body value changed.

**Gap:** The slice lists "street trees from the one tree generator"; the street-placement slice made the street tree a prop type of its own and left it a stand-in box.

**Verdict:** sound. The street tree's box (0.35 m by 10 m) is the forest rule's trunk, so the one-size tree art (11 m, a 0.4 m bole) fits it as it fits a forest's: a smaller build or a kind of its own would have been a second size for the same body. The rule is by the body, not the id: a map's own `trunk` prop outside its forests, which nothing drew before, is a lone tree too. **Confidence:** high on the binding. Medium on the look: the crowns are narrowed by scale, so a bole comes out 0.28 to 0.34 m against the body's 0.35.

### A street tree's crown is drawn where the simulation has no foliage

**Choice:** Accepted. A lone tree's crown hides the ground under it from the camera; the simulation gives a tree outside a forest its trunk (cover, and the trunk row's own concealment) and no canopy.

**Gap:** Every forest crown is held inside the simulation's foliage; a street tree has none to be held in.

**Verdict:** provisional. It is one crown 5 to 6 m across with daylight under it, and a unit beneath is found by the x-ray as under any crown. If a street's trees should fade sight along it, that is a foliage rule for lone trunks, the simulation's. **Confidence:** medium.

### Ours for every kind, including the eight the vendored street kit was to supply

**Choice:** Lamp, bench, bollard, bins, hydrant, utility box, planter and scooter are project-owned models here, like the rest. The buildings design decided those eight would come from the vendored building pack's street kit, exported standalone by the buildings lane; that export has not happened.

**Gap:** The slice's "theirs, from the kit's standalone street kit" has no owner in this lane.

**Verdict:** provisional. If the buildings lane exports its kit, each is another appearance of the same scenery kind (the renderer picks by nearest box) or replaces ours; nothing else changes. **Confidence:** medium.

### Jersey barrier, Heras panel and scaffold bay repeat along their box

**Choice:** Those three rows' `appearance` carries `modular: true` (as the wall, fence and sandbags have), so a long placed box draws a run of 3 m barriers, 3.5 m panels or 2.5 m bays instead of one stretched module. No body column changed.

**Gap:** The rows had no `modular`; a barrier line is the common placement.

**Verdict:** sound. It is the appearance binding, not the body. **Confidence:** high.

### Glass and mesh are opaque geometry until model surfaces can blend

**Choice:** The bus shelter's panes are pale, glossy, opaque glass. The Heras panel's mesh is wires as geometry on the two near tiers, six uprights on the third and the bare frame on the far tier.

**Gap:** Scenery models cannot blend, and a one-pixel wire crawls at distance.

**Verdict:** provisional. The simulation sees and shoots through both, while the shelter draws as a pale screen that hides what stands behind it. Dark panes read as a black box. When scenery models can blend, the panes become a blended material in `street.py` (one argument). **Confidence:** medium.

### One car, one colour

**Choice:** A pale dusty grey hatchback, with no badge, plate or livery. The colour is neither side's tint, and reads as civilian beside olive vehicles.

**Gap:** "Model details within 'generic'."

**Verdict:** sound for one model. A street of identical grey cars will read as clones; more bodies or colours are more appearances of the same kind at slightly different boxes, or a per-instance tint the scenery path does not have. **Confidence:** medium.

## C06: restore contact by staging the existing stress fronts within weapon reach

**Choice:** Move the generated stress recipe's living starts 200 m closer to
contact using its existing legal-placement projection. Keep all commands,
physical rules, unit counts, full world and late remains. Incoming movement
certification refuses most scripted journeys; a combat benchmark should still
make both forces fight without changing that production policy.

**Gap:** The plan names active contact but does not prescribe how to preserve
that workload when incoming movement admission changes. This correction is a
new named benchmark input, never an outcome-preserving optimization.

**Reach:** Generated stress starts change; normal battles and saved stress starts
do not. Rising published shot counters guard contact during measurement in the generated browser
arms. Refused journeys remain a separate finding and cannot prove the historical
planning-delay cases resolved.

**Verdict:** sound, medium confidence. This makes combat representative of the
stated load while preserving production rules; it leaves marching cost dependent
on future movement admission. Full scale acceptance still requires fresh combined
receipts.

## C06: use pinned software trig for seeded soldier placement

**Choice:** Squad arrangement and nearest-free ring samples use the existing
pinned software math library, as combat already does. A shared seed must produce
the same authoritative positions in native and Wasm, even when the renderer's
float32 view cannot show their last-bit difference.

**Gap:** The prior arrangement used platform sine/cosine. The fresh full-world
parity check exposed a one-bit sine difference in one seeded draw, which changed
a member position and the squad centroid without changing the published words.

**Reach:** This intentionally corrects native placement's last-bit state; it is
not an outcome-preserving CPU optimization. Shared nearest-free callers include
cover, garrison and replacement-soldier placement; the correction applies to
their ring samples too. No rule value, spacing policy, random draw sequence, map
input, allowance or public ABI changes. The existing
engine fingerprint refuses older replays as designed. All paired publication
records now check initial state as well as stepped state; a one-squad case owns
the regression through the public battle interface.

**Verdict:** sound, high confidence. The observed sine difference reproduces the
first state mismatch exactly, and the public native/Wasm regression passes with
pinned math. Full-map parity is rechecked before performance admission resumes.
## C24 cutout

**When:** 2026-10-02, with C25 and C26, in the model layer. Evidence is the `facade` scene's (`throwaway/evidence/facade/`).

### A kind of surface is a range of the mesh, drawn by its own pipelines

**Choice:** At install a mesh's indices are ordered by how their material is drawn (`models/surfaceParts.ts`: cutout, opaque, room, blended), so every drawable and every kit module has one index range a kind, and each draw call names the kind it draws. The alternatives were a branch on the material in the opaque fragment stage, or a second mesh per kind.

**Gap:** The slice names the seam (`modelLayer.ts`) and says opaque wear must be unchanged; it does not say how one mesh with several kinds of material is split between pipelines.

**Verdict:** sound. A material is a per-vertex index here (L11), so one mesh mixes kinds freely, and a discard in the opaque stage would cost every model early depth. A mesh that is opaque throughout keeps its index order and its one range. Proved by frames of main and of this branch (all three slices in) from the same scenes: `city-block`'s differ on at most 689 pixels of a 1920 × 1080 frame by at most 14 of 255, which is what two runs of one tree differ by (676 pixels, 15), and its overview, wide and ground-class frames are identical; `village-watch`'s battle frames differ on a few thousand pixels between any two runs, on main as here (its presentation is not locked to a frame), and its mask frames are identical. **Confidence:** high.

### Mips are not coverage-preserved; the coverage becomes a sample count

**Choice:** A cutout's coverage is read through the texture's ordinary mip chain, scaled so that the material's cutoff is half, and becomes the number of the pixel's four samples it covers. Coverage-preserving mips at bake, with a hard cutoff, was the other option the brief names.

**Gap:** "Mips must not eat thin features: choose and say why."

**Alternatives:** Coverage-preserving mips keep a feature solid at every distance: a grille a quarter there stays a hard pattern, and when its bars pass under a pixel it aliases (it is the "alpha-tested cards shimmer under multisampling" trap). They also make a texture's mips depend on a material's cutoff, when a texture is shared by content.

**Verdict:** sound. A grille too far away to resolve is drawn as a veil a quarter there, the sheet as one seven tenths there, and neither vanishes (250 m: both still differ from the bare ground by 57 and 115 of 765). A cutout thinner than an eighth there would round to nothing at distance; none is authored. **Confidence:** medium on the far look, which was judged in stills only.

### The mask is the fragment stage's, not alpha-to-coverage, and only where depth is written

**Choice:** The prepass's cutout stage outputs a sample mask; the colour pass does not cut at all. It shades the cutout's triangles with the opaque fragment stage at depth *equal* (`battleWorldDepth("kept")`), so it draws exactly the samples the prepass kept.

**Gap:** "The prepass and colour pass must still agree on depth bit for bit; a cutout cannot go through a fragment-less prepass pipeline."

**Alternatives:** Alpha-to-coverage needs a colour target with alpha, and the prepass has none. Cutting in both passes with the same function leaves one sample a frame open to two compilers rounding differently, and a sample the prepass kept and the colour pass dropped shows the sky through a wall. The frame's usual greater-or-equal compare would fill every hole: the cut surface is nearer than what shows through it.

**Verdict:** sound. One stage decides the silhouette, by construction. A hole's pixel is the frame without the panel to the last bit (difference 0), with and without fog. **Confidence:** high.

### Rounded where the pixel resolves an edge, dithered where it does not

**Choice:** The sample count is the coverage times four, rounded where the coverage changes fast across the pixel and dithered (interleaved gradient noise) where it is an even part.

**Gap:** Unstated; found by the critique.

**Verdict:** provisional. Dither everywhere speckled bar edges seen close; rounding everywhere drew bands across the perforated sheet at 25 m and blotches in the far grille, where an average coverage crossed a step. With the mix a fresh critic still saw faint row bands on the sheet at 25 m and mottled panels at 250 m. Not judged in motion. **Confidence:** medium.

### A cutout's caster dithers the same coverage

**Choice:** In a cascade a cutout's texel is covered when its coverage beats the noise at that texel. Like every model's caster it is a tier coarser.

**Gap:** "Matching colour and shadow silhouettes."

**Verdict:** sound. A cascade's texel is wider than a grille's bar, so a hard cutoff casts nothing or a slab. Filtered, the dither is the panel's share of shade: the grille casts 0.29 of what a wall casts at the same spot and the sheet 0.71, for panels 0.26 and 0.72 there. Individual bars cast no shadow at any distance; close up the shade seen through the grille has soft waves where the bars beat against the cascade (left). **Confidence:** high on density, medium on the look.

### The fixture is a kit no template places

**Choice:** `city_kit_facade_lab` is a kit appearance with no set: `facade_lab.py` writes its GLB, the catalog lists it, and `/lab/facade` stands its modules as static models. The lab's map installs only the kits the template library places, so a battle never loads it onto the GPU.

**Gap:** "A small authored test kit ... drawn in a lab fixture route."

**Alternatives:** A set with a template would add a row to the physical catalogue and move its hash. A GLB validated in the page (as the workbench's drop zone does) would not go through the bake, which is where a room gets its sheet.

**Reach:** The building layer's draws (`buildingLayer.ts`) take the same ranges through the same pipelines, but no placed template has a cutout, glass or a room yet: that path is exercised for opaque surfaces only until the kits are restored. The bundle is 7 MiB in the runtime catalog, fetched by every page that loads the catalog.

**Verdict:** sound for the lab; the building path's three new kinds are unproven on screen. **Confidence:** medium.

### Units' cards and x-ray draw their opaque surfaces only

**Choice:** The impostor bake and the x-ray draw a model's opaque range.

**Gap:** Unstated.

**Verdict:** sound today: no unit has a cutout. A camouflage net as a cutout would be missing from its card. **Confidence:** medium.

### What the unprimed critique saw, and where it went

| Finding | Disposition |
|---|---|
| The perforated sheet read as raised studs up close (a lit rim round each hole, plate and holes one colour) | Fixed in the recipe: a flat, dark plate |
| The sheet is a solid dark slab at the default camera while its shadow says "mostly open" | As designed: it is seven tenths there. Every shadow is faint under this light's shadow floor (a wall dims the ground by 13 %) |
| The grille loses its bars at the default camera and is a blotchy patch at 250 m | The veil is the design; the blotches were the rounding, now dithered where unresolved |
| The grille's shadow has no bars; wavy bands in the shade seen through it; wide grainy edges; shadows detach at the foot | The cascades cannot resolve a 24 mm bar. The waves are the dither against the cascade, left. The rest is the sun shadow's known softness and bias, not this slice |
| Bar edges jog by a pixel and have a two-tone fringe at 4 m | Four samples a pixel: five steps of coverage. Left |
| Horizontal banding across the sheet at 25 m | The rounding again; dithered now |
| A shadow on a light checker reads as a dark checker | The lab's ground is one colour now |
| Cast shadow against the hatched unseen ground | Not confused: the critic answered no |

A second unprimed critic saw the final frames (after the dither and the repainted sheet) and added nothing to the table: the openings are clean, the bars are under a pixel at the default camera, the far panels are mottled.

**Not done:** the `preview-shots` checkpoint (this pass ran as a subagent with no one to show).

## C25 glass

### Glass draws at the end of the world pass, not in a pass of its own

**Choice:** A blended material's triangles draw last in the frame's existing world pass, after the water, through one more pipeline: blended over what is there by its coverage value, depth read and not written, four samples like the rest. It is in no prepass and no cascade.

**Gap:** "Alpha-blended model surfaces in the existing frame owner, with an explicit ordering and depth policy." The brief allows a new pass.

**Verdict:** sound. The water already draws this way in this pass, so the frame function is untouched and the phase order has nothing new in it. **Reach:** what reads the prepass's depth does not know glass is there. Overlays (order lines, rings) show through a pane as they show through air; the fog's tile cull and the x-ray ignore it; and an effect (smoke, a flash) behind a pane is drawn after it and over it, not dimmed by it. A pane is thin and mostly against a wall, so none of these showed in the lab. **Confidence:** medium.

### Glass is not sorted

**Choice:** Panes draw in the order the layer packs them (by mesh, then in list order; a town's in its chunks' order), never by depth.

**Gap:** "Sort granularity", delegated.

**Alternatives:** Back to front per instance is CPU work every frame over records that are otherwise static; per chunk or per building leaves the panes inside one unsorted anyway.

**Verdict:** sound for panes of one glass. Two panes blended in the wrong order differ from the right one by the product of their opacities times the difference of their two colours, and panes of one material under one light have nearly one colour. Proved on three panes each half across the next and a fourth seen from its back: the same modules handed over in the opposite order draw the same frame to within 1 of 255, on every pixel. It would not hold for a red pane before a blue one; nothing here authors that. **Confidence:** high for one glass.

### A pane is lit by the one shade function, with its mirror bounded

**Choice:** Glass is shaded like any surface (sun, sky, cast lights, haze) and laid over what is behind it at its opacity. Two numbers bound what it mirrors (`presentation.glass`): `turn` turns its shading normal toward the eye (0.5: seen edge on, it is shaded as if seen 63° off its normal), and `glint` caps its own light at that of a white matte surface in sun shadow (1).

**Gap:** "Glass must read as glass at the game camera: a dark, slightly reflective pane ..., not a hole and not an opaque plate." How it is lit is unstated. The coordinator added, from an unprimed critique of the first real town: today's opaque stand-in glass is charcoal on one face and pale khaki on the next, because a glossy surface mirrors the horizon at a grazing angle, and along a street the pale ones read as boarded windows.

**Alternatives:** Upstream's glass is a mirror weighted by Fresnel, nearly clear head on and a full mirror edge on: the pale plate again, from a grazing camera. A shading of its own beside the frame's one shade function would have had to repeat cast lights and haze.

**Verdict:** provisional. From the street, under the fixture's sun and three more (behind the camera, ahead of it, along the street), a window's pane is 0.25 to 0.42 of its wall's brightness, never near it. The cost is the unprimed critique's first finding: with its mirror bounded a pane has no sky gradient and no glint to say "glass", and reads as smoked film, or on a building as an open hole with a dim room in it. A stronger mirror veils the room, which is dimmer than any reflection of this sky: four hundredths of the sky's radiance is already several times a room's. The two numbers are the fixture's to tune; the cue that costs the room nothing is the pane's own material (waviness in its normal map, dust in its coverage), which a kit authors. **Confidence:** medium that the composition is right, low that the look is finished.

### Opacity is the coverage value and nothing else

**Choice:** A pane's alpha is its coverage value (the base colour's alpha times the normal texture's), the same from every angle. No Fresnel in the alpha.

**Gap:** Unstated.

**Verdict:** sound. One pane keeps 0.77 of the brightness behind it, two keep 0.59, and a pane seen from its back the same as from its front (0.77). An opaque frame in front of a pane is drawn exactly as without the glass. **Confidence:** high.

### Glass does not touch the fog mask

**Choice:** The glass pipeline writes colour only. The fog mask under a pane stays what the surface behind it wrote.

**Gap:** "Lit and fogged like the world."

**Alternatives:** Blending the mask by the pane's opacity, as the water does, was the first version. A seen pane over unseen ground then made the pixel half seen: the mask pass drew an unhatched plate inside each pane, as if the glass revealed the ground behind it, and broke the sight line's rim across it.

**Verdict:** sound. A pane is not the thing seen or unseen; what stands behind it is. A pane past the sight line is unseen with the ground it stands on. One case is wrong by a hair: an unseen pane in front of a seen surface would be drawn tinting it, which needs a seen surface farther from the eye than an unseen pane. **Confidence:** medium.

### Glass casts no shadow

**Choice:** None, at any opacity.

**Gap:** "Blended surfaces cast no shadow or a policy you state."

**Verdict:** sound for window glass. A dark, nearly opaque blended surface (a tarpaulin) would want the cutout's dithered caster; it is one line to add when something asks. **Confidence:** high.

### The comparison target is the lab's facade, not the China balcony

**Choice:** The Blender render compared against is the facade lab's block from the scene's own cameras.

**Gap:** The slice names "a Blender render of the China enclosed-balcony module at 30 m".

**Verdict:** a substitution. The China kit's glass is still the opaque stand-in (it is restored after these passes), so that module has no blended surface to draw. **Confidence:** high that it is the right order; the balcony comparison is owed by the pass that restores the kit's glass.

**Not done:** the `preview-shots` checkpoint.

## C26 interiors

### A room box's UVs are the box unfolded, and the shader does the pinhole

**Choice:** A room surface's UVs are its box unfolded round its back wall: the back wall is the unit square, and the floor, ceiling and side walls reach one unit out from its edges to the open face (`parts.room_box`). The fragment stage reads the point's place across the box and its depth into it off that UV and applies the atlas's pinhole itself. This replaces C21's provisional choice, that the UVs are the cell's projected `u`, `v`.

**Gap:** C21 left it "provisional until C26 draws one", and reckoned the error of interpolating projected UVs at 0.7 % of a cell.

**Alternatives:** Projected UVs are wrong between vertices, because the projection is not straight across a triangle: on the floor of the 3 × 3 × 4.5 m box the middle of the diagonal is 5.5 % of a cell off (7 texels of 128), which shows as a kink along it; cutting each wall eight times in depth would hide it at eight times the triangles. A third vertex attribute for depth has no free slot in the 48-byte vertex.

**Verdict:** sound. The unfolded position is straight in the box's space, so it interpolates exactly, the lookup is upstream's (which projects in its shader from a room-local position) at every pixel, and a box of any size shows the whole of its cell with no size in the material. **Confidence:** high.

### The sheets reach the runtime inside the bundles that show them

**Choice:** `assets/catalog.json` names each sheet's source picture under `interiors`. The bake lays a sheet's ten cells out again four to a row in a 512 px square, mips it and addresses it by content like any texture, and gives it to every room material as its albedo texture (`interior.ts`). A room whose sheet has no picture, or whose picture is not ten 128 px cells, does not bake.

**Gap:** "The atlas sheets need a way into the runtime ... through the existing texture path ..., not a side-loaded image."

**Alternatives:** A third kind of runtime file beside bundles and the template library needs a codec, a loader branch and a slot in what the renderer installs, for two pictures. Embedding the sheet in each kit's GLB from Blender copies it into every source and lets a kit ship a stale sheet.

**Reach:** Each kit bundle with rooms carries the sheets it shows (1.4 MiB a sheet with its mips); on the GPU they are one layer each however many kits show them. The source layout (2 × 5, 256 × 640 px) is not square, which is why the cells are laid out again; a cell is still whole at every mip down to one texel a cell (a test holds it), and the shader never reads a coarser one. The albedo array is as wide as its widest texture, so where units' 1024 px textures are installed a sheet is a 1024 px layer (5.6 MiB each). A source dropped on the workbench has no sheet and draws its rooms as their flat colour.

**Verdict:** sound. **Confidence:** high.

### The room is chosen by a hash of the model's position, read flat

**Choice:** The cell (of ten) and whether it is mirrored are bits of a PCG hash of the three float32 coordinates of the model's own position (the record's placement: the module's row on a building). The position reaches the fragment as a flat varying.

**Gap:** "Stable per-instance room choice by a hash of the instance's position (and mirroring)."

**Verdict:** sound. Interpolated, a constant comes back an ulp off and a hash of it flickers, so the models' `anchor` varying is flat now; nothing else read it but the corpses' fog, at the same point. In the lab a window shows the same room after the camera has been 250 m away and back and when the frame is handed the models in the opposite order (difference 0 on every window), and six windows show four different cells. **Reach:** rooms that share one module instance share one room, so a room box is a module (or part of one) that a row places per window; folded into a shell they would all match. **Confidence:** high.

### A room is shown at the scene's exposure, as a matte surface in sun shadow would be

**Choice:** The atlas's colour is multiplied by what a white matte surface facing the sky returns in sun shadow under this light (the sky's light and the shadow floor's share of the sun's), and put behind the air (`environment.unlit`). Nothing of the room's own surfaces enters: no facing, no shadow test, no cast light, no emission. A tint-masked room takes its row's tint.

**Gap:** "Unlit (... it takes fog and the scene's exposure, but no sun, no shadow, no emission)." What "the scene's exposure" is for a picture is unstated.

**Alternatives:** The lit path with the picture as albedo adds a rough dielectric's specular, a grey veil about as bright as a dim picture. A constant would not follow a dusk preset. A white surface in the open (full sun) was the first reference: on a facade in its own shade the rooms then came close to the wall's brightness, and the critique read one shop as nearest to "lights on".

**Verdict:** provisional on the level. The picture is the same under every sun azimuth (to within 1 of 765 at one elevation, while the wall beside it moves by 110 of 255), so it is unlit. Against the fixture's sunlit plaster the rooms sit at 0.16 to 0.44 of the wall's display brightness with no glass and 0.23 to 0.37 behind it, and stay under the wall on a shaded face. The slice says to tune the atlas, not the shader, if they should be darker or lighter; a building can also dim its own by tint. **Confidence:** medium.

### Rooms are solid to depth and shadow

**Choice:** A room's triangles are in the depth prepass and the sun's cascades with the opaque surfaces (one range, the fragment-less pipelines); only their colour has a stage of its own.

**Gap:** Unstated.

**Verdict:** sound. A building with windows on two sides and no room boxes lets the sun through it onto the ground in its own shadow. **Confidence:** high.

### Neighbouring rooms do not share a wall's plane

**Choice:** The lab's boxes are 2.9 m wide and tall in a 3 m bay and floor, as upstream's are.

**Gap:** C15 gives the box as 3 × 3 × 4.5 m, a whole bay.

**Verdict:** sound. Two boxes a bay apart would put two walls in one plane, each showing its own room, and they fight for depth where one is seen through the other's window. **Confidence:** high.

### O-1: rooms at tiers 0 and 1

**Choice:** A kit places its room boxes at tiers 0 and 1 (down to 4 px a metre, where a window is about 6 px), on every floor, and not at tiers 2 and 3.

**Gap:** "Record the O-1 verdict (all room tiers, or LOD0 only)."

**Verdict:** the shader allows every tier; the kit's budgets do not. The room stage is cheaper than the lit one (one atlas read and one sky read against the full shade), and the paired cost over a field of blocks is in the slice's outcome. But the whole map is expanded at the coarsest tier and a far building is one row there: a room box a window is ten triangles and a record each, hundreds of thousands of them on a large map, for windows of two to six pixels that a dark pane on the shell already draws. **Confidence:** medium: decided on the budgets and the pixel sizes; no real kit has rooms yet.

### From the tactical camera a window is its room's floor, and that reads

**Choice:** None; C15's open question, judged. At 80 m and the steep camera a window shows a dark, coloured pane with a hint of the floor and the far wall's foot; from 30 m and from the street the back wall, furniture and shop shelves read as rooms with depth.

**Verdict:** acceptable. Windows read as dark openings, differing a little in colour, which is what a facade wants from that height. **Confidence:** medium (one facade, one light).

### Nothing of upstream's lighting of the picture is kept

**Choice:** Upstream's interior shader multiplies the photograph by a gain, a depth falloff and, on some windows, a warm lamp colour. None of the three is here.

**Gap:** "Nothing beyond the repo's mechanism (Q-E)", and "unlit".

**Verdict:** sound. They exist to make night photographs glow; our cells are finished daylight pictures. **Confidence:** high.

**Open after the second critique:** in sun shadow's light the rooms no longer read as lit on a shaded facade, and at 80 m every window reads as a near-black hole (the four darkest already did). The reference is one line in `environment.unlit`; the atlas's floors are C15's.

**Not done:** the `preview-shots` checkpoint.
## C78

**When:** 2026-10-02. Evidence and numbers: the Outcome in [C78](slices/C78-forest-body-models.md).

### One script builds the floor's bodies and its dressing, from the tree generator's parts

**Choice:** `forest_floor.py` imports from `trees.py` (its bark tube, its leaf clumps, its export) and writes `assets/source/forest/`. `trees.py` builds its own kinds only when it is the script being run.

**Gap:** The slice says the log reuses the trees' bark and names no script.

**Verdict:** sound. The bark has one owner, and the trees' models were byte-identical after the change. **Confidence:** high.

### The models are built to the simulation's boxes: the log is 0.7 m thick

**Choice:** The log's model fills the forest rule's box, 4.4 × 0.7 × 0.7 m, and the boulder's 2 × 1.6 × 1.5 m. The art does not make either look bigger than the body the simulation has.

**Gap:** The slice asks for models that fit their bodies; the bodies' sizes are the forest-bodies slice's starting values.

**Verdict:** provisional. A fresh critique read the log as "a thin pole a tank would drive over and a man would step across", yet it is medium cover that stops vehicles. The art is right to follow the box; whether a log that stops a jeep should be nearer a metre thick is a mechanics decision (`forests.rule.log_half_extents_m`), the owner's. **Confidence:** low on the size, high that the art must follow it.

### A rock is a ball cut by seeded planes, with their edges rounded by a power mean

**Choice:** Along every direction the surface's distance is the power mean (power 9) of the distances to twelve seeded planes and the unit ball, so a face is flat and the edge between two is a curve. Its underside is squashed, so it sits on its widest section. The boulder is fitted to its box; the dressing's small rocks use the same generator.

**Gap:** "A new small rock generator (rounded, not pyramids)".

**Verdict:** provisional. Fresh eyes read the first build as an egg; the second is flatter-faced and sits lower, and was not judged again. One unprimed look at the boulder's sheet would settle it. **Confidence:** medium.

### The bodies are vertex-coloured like the trees, not textured like the village's props

**Choice:** Bark, wood and rock are plain materials with vertex colour.

**Gap:** The props pipeline bakes textured recipes; the slice asks for the trees' bark.

**Verdict:** provisional. The dressing's rocks and branches go through the scenery layer, which draws vertex colour only, and a boulder should match the stones beside it. Up close both bodies have less surface detail than a textured prop; the planned model pass is where that would be revisited. **Confidence:** medium.

### The bodies run a little below the ground

**Choice:** The log's underside is 5 cm under its origin and the boulder's 12 cm, each with its own `ground_m` tolerance in the asset catalog.

**Gap:** Fit is to the box above the ground; a box on sloping ground would float at one end.

**Verdict:** sound. **Confidence:** high.

### The log and boulder are no longer marked systems-only

**Choice:** `fixtures/props/forest/{log,boulder}.json` have no `status: "systems_only"`: wherever a map lays these bodies they draw as their models, not as stand-in boxes. Removing the marker alone changes no digest. Whether the forest rule lays any is C87's entry.

**Gap:** The forest-bodies slice left the marker until "accepted drawing".

**Verdict:** sound. **Confidence:** high.

## C79

**When:** 2026-10-02. Evidence and numbers: the Outcome in [C79](slices/C79-forest-dressing.md).

### The dressing is laid a cell of ground at a time and kept in a bounded pool, not expanded for the whole map

**Choice:** Dressing is what grows and lies under a forest's trees with no body of its own (ferns, bushes, saplings, small rocks, fallen branches). `DressingField.place(i, j)` lays one 64 m cell, seeded by where the cell is; the scenery layer keeps at most 128 cells in one instance buffer, lays a few a view nearest first, and gives up the cell wanted longest ago (`scenery/dressing.ts`).

**Gap:** The slice says "a third population in `placement.ts`, drawn by the existing scenery layer's chunks" and "seeded per forest". The cost spike asked whether whole-map residency holds.

**Verdict:** sound. It does not hold: the saved small generated map has 43,126 trunks and its dressing whole is 657,000 pieces, 31.5 MB and 0.42 s at load, and larger maps grow from there. The pool is 8 MB at most whatever the map. Because a cell is seeded by its place, a forest's dressing does not depend on which forest it is. **Confidence:** high.

### A cell draws whole at one tier; a piece shrinks to nothing on the GPU

**Choice:** No piece is touched on the CPU after its cell is laid. A cell's tier comes from its distance and the tallest piece any cell can hold. Each piece is scaled about its foot in the vertex stage by its own projected height: whole above `fade_px` (6 px), gone at half of it; a cell whose tallest piece would be gone is not drawn or laid.

**Gap:** "Far chunks leave it out, and it fades by pixel size within the budget gate's radius"; that gate gave no radius.

**Verdict:** sound. Nothing blends, so the dressing stays in the opaque passes. The piece's height over `fade_px` rides in the instance record's last float, which trees use as a noise seed. **Confidence:** medium on the 6 px.

### The dressing casts no shadow and is fogged like a tree

**Choice:** It is drawn in the depth prepass and the world pass by the forest's own fragment stage (shadowed by the trees, fogged whole at its own heart) and not into the sun's shadow cascades. It is drawn from both sides, so a frond is one sheet of triangles.

**Gap:** Not stated.

**Verdict:** provisional. Four cascades of it would cost more than the dressing itself. Fresh eyes said the pieces "sit on the floor like stickers"; nothing cheaper than the cascades was tried. **Confidence:** medium.

### `dressing` is a scenery kind with a height and a triangle budget

**Choice:** `SCENERY_KINDS.dressing`: no body, at most 0.9 m tall (`fit.dressing`), tiers of at most 600 / 200 / 60 / 24 triangles. The biome may not scale a piece above 1. The small rock is 0.33 m by construction; no rule holds rocks to 0.5 m.

**Gap:** "Everything ≤0.9 m; rocks ≤0.5 m" named no owner.

**Verdict:** sound for the height: under a man's waist, so nothing drawn looks like it hides what the forest does not. The rocks' own limit is unenforced. **Confidence:** high.

### Density, mix and drifts

**Choice:** 3,200 candidate pieces a hectare: ferns 68%, branches 14%, rocks 7%, bushes 7%, saplings 5%. Each kind has its own smooth noise field; between its drifts a kind thins to `1 - drift` of its pieces (ferns to a tenth). About 1,900 a hectare stand.

**Gap:** Delegated.

**Verdict:** provisional. 900 a hectare read as a bare floor with a few plants; 1,800 and 3,200 were shot side by side and 3,200 kept. The gate's fresh eye still listed "a bare floor" among the woods' faults. **Confidence:** medium.

### A piece keeps 3 m inside its forest's edge

**Choice:** `forest_floor.dressing.edge_m` is 3. The drawn floor reaches out to a verge that wanders about the forest's edge, and a piece at the edge stood on field-coloured ground.

**Gap:** "A test that nothing is placed outside forests" says nothing of the drawn floor.

**Verdict:** sound. A tree line 10 m wide keeps a 4 m band of dressing. **Confidence:** medium.

### Web aliases for `math/noise`

**Choice:** `web/tsconfig.json` and `web/vite.config.ts` gain the `math/noise` path beside the package's other entries.

**Gap:** The drifts need a smooth seeded field, and the repo's rule is the `math` package before anything hand-written.

**Verdict:** sound. Two lines in shared config. **Confidence:** high.

### The root lines on the forest floor are fainter

**Choice:** `forest_floor.roots` 0.25 → 0.1.

**Gap:** Not this slice's rule, but its critique's finding: the painted arcs read as ruts and as shadows of things that are not there, and were louder than the real branches now lying on the floor.

**Verdict:** provisional: a guess at how faint, not critiqued again. **Confidence:** medium.

## C86

**When:** 2026-10-02. Evidence: the Outcome in [C86](slices/C86-tree-lines.md).

### Fields are cut along a strip's long stretches, read from the forest's own export

**Choice:** A tree line is a forest drawn as a stroke (a strip). `forestStripRuns` makes every stretch of a strip forest longer than the strip is wide a guide edge for the plots, beside the roads' and rivers' runs, so a tree line stands on the boundary between two fields.

**Gap:** The slice says "plots cut along strip control runs"; the simulation exports control runs for roads and rivers and none for forests, and this lane may not add one.

**Verdict:** sound. A bend is exported as chords under 2 m, so the rule keeps the straights and drops the bends, which is what a control run is for. **Confidence:** medium: a strip narrower than its bend's chords would be cut along them.

### The drawn half is proven by tests alone

**Choice:** No picture, no hedgerow comparison, no critique and no 40-strip cost row were taken for a tree line.

**Gap:** The slice's `farmland` lab is the map lane's and does not exist; when the slice landed no map had a strip forest.

**Verdict:** provisional. Generated maps now hold tree lines (the open-country pass writes them), so the evidence can be taken: a rig station on one, an unprimed critique and one paired cost run would settle it. Until then the rules hold on an inline test map, and nobody has seen how a tree line looks. **Confidence:** high that the rules hold; none on the look.

## C31 urban and plain ground composition

**When:** 2026-10-02. Evidence and numbers: the Outcome in [C31](slices/C31-city-biome.md).

### Urban ground is where buildings stand, by the plot rule; no region reaches the renderer

**Choice:** The settlement's ground is the plots whose centre lies within `field_rules.yard_m` (40 m) of a building, the rule `terrain/plots.ts` already owned at a wider reach. Nothing was exported from `crates/`.

**Gap:** The slice says composition comes from the map's surfaces and `land_regions`. `land_regions` is not a field of the map definition; the compiler refuses a plan that names one. What the generator knows of a town (each settlement's outline and blocks) is handed to the encounter planner and dropped: it is not in the map, a saved map has none, and it reaches neither the page nor the renderer.

**Verdict:** sound. Exporting a town's outline means a new field of the map contract, its saved form and its identity: the map lane's decision. The buildings are in every map, saved or generated. Against the generator's own blocks the rule calls 85% of block ground yard and 0.3% of it a drilled crop. **Confidence:** medium: a yard is a whole plot, so its edge is a plot's straight edge, and a wide lot round a detached house is no yard.

### Two reaches: the yard, and the surround where no crop is drilled

**Choice:** A plot within `yard_m` (40 m) of a building is the `settlement_kind` (`yard`); any other within `settlement_m` (110 m) is the `surround_kind` (`meadow`). Neither is drawn from the weighted mix of plot kinds.

**Gap:** "Pavement and urban yards suppress farm plots" names no reach; the one old rule made every plot within 110 m the settlement's lawn, a block of one green round every town.

**Verdict:** sound. Keeping the old 110 m for the pair leaves every other plot on every map the kind and colour it was (checked plot by plot on the village and the generated map). **Confidence:** high.

### The yard is the old settlement green at its own lightness, two thirds of its chroma

**Choice:** The settlement's plot kind is `yard` (weight 0, never drawn by chance). `palettes.yard` is the old settlement green's three colours at their own L\* and hue, chroma 17 (from 27); its grass is the meadow's mix at full height. The kind was `green`, on main's meadow palette unchanged, from C84 until this slice.

**Gap:** "Urban yard and park appearance; grass density" are delegated, and the look must hold `fog-look`.

**Verdict:** provisional. The fog style `grey-veil` at one station separates seen from unseen ground by about 1 unit; a desaturated or mown lawn there fails the check (a smooth lawn has no dark blades, so unseen yard came out lighter than the darkest seen ground). The check holds only while the yard is as dark and as rough as it was; its chroma is free. Lawn with trodden courts and packed earth were tried and came out rust orange under the low sun. Fresh eyes still read "a mown lawn from wall to wall" and the darker green under the whole town as a shadow. Widening that fog style's margin, its owner's work, is what frees the yard. **Confidence:** medium.

### A country road is a street where yards lie on both sides of it

**Choice:** `roads.<kind>.town` names the kind a road is drawn as where a yard lies within `beside_m` (5 m) of its edge on both sides, carried on across gaps under `gap_m` (80 m). The summer country road's is `road`, the asphalt street. `drawnStrokes` splits the exported stretches where such a run starts and ends, and the surface field is built from the result (`TerrainSurface.strokes`). A stretch is retagged only when the row's town kind differs from its own, so on the village, whose roads are `road` already, the strokes are the exported array itself. No edge of any road moves and no rule changes.

**Gap:** "Never a second sim rule or a renderer-only guess about roads"; the generated map names the road through a town `country_road`.

**Verdict:** provisional until the map says which roads are streets. The road follows the yards, the one owner of where the town is, so asphalt never runs on past them; with "a yard on either side" a street with walks ran beside a wheat field wherever a yard lay across the road. A hamlet's road between wide lots stays gravel while its side streets, `road` by the map, are asphalt. **Confidence:** medium.

### The join: the street runs on under the gravel, which starts in drifts

**Choice:** A run of street is cut square `join_m` (3 m) past each end; the country road stops at the run's end, round. Where one road is carried onto another, the line the upper surface starts from wanders by the carry in drifts 2.5 m across (`JOIN_DRIFT_M`). This is every carried join's: a track's earth onto a road too.

**Gap:** "A join that is not a ruled line."

**Verdict:** provisional, and not good enough. Cut square at the run's end the asphalt was a ruled line with gravel sprayed over its middle; with drifts a critique still reads the join as a glitch (a lobed blob of gravel on one side, a ruled cut on the other), and the gate's fresh eye called it a smudge. The carried blend was made for a road that crosses another; an end across a road wants a term of its own. **Confidence:** low.

### A road is judged against the field beside it as "lighter, or apart in hue", and its walk as "no dark outline"

**Choice:** The two road checks are these, in `web/scenes/_roads.mjs`. A road's core may be darker than the ground 5 to 9 m beside it only when it is within 15% of it and their colours are more than 6 apart in CIELAB's a\*b\* plane (the river bank's bar). Walking out from the core to the field in half-metre bands, no band is more than 12% darker than the darker of the core and the field. The first forms, "core at or above the grass" and "only gets darker, band by band" (with bars moved once at integration to 4% and 10%), are gone.

**Gap:** C66 and C67 wrote "core luminance at or above the adjacent grass band's" and "monotone or flat from core to grass" when every field beside a station was green. After the field palettes the river lab's track runs through rapeseed, lighter than any earth.

**Verdict:** provisional. It is the form the spec uses for seen against unseen ground ("lighter than, or apart in hue"), and what the design guards against, a dark band beside the road, is still refused. The measured case: track 0.211 against rapeseed 0.225, 10.2 apart in hue; the bare half metre beside it is the field's own soil at 0.203. The band-by-band rule could not hold beside a field as light as the road, and its small steps were furrow phase. The 15% and the 6 are the bank's numbers, not measured for roads by a fresh eye; the owner may want them tighter. **Confidence:** medium.

## Open country

The open-country pass (M24, M25): `crates/mapgen/src/open_country/`, its rows in `fixtures/map-presets.json`, its tests in `crates/mapgen/tests/open_country.rs` and its measure in `crates/mapgen/examples/sight_report.rs`. Numbers below are from that report and `layout_sweep`.

### "An approach corridor" is the middle of a settlement's widest approach in each half

**Choice:** The layout records an approach as a fan: every bearing from `from_rad` to `to_rad` whose 400 m by 1,800 m corridor is open. The pass keeps, for each settlement and half, the widest fan's middle corridor (the bearings measured either side of the middle, and the middle itself, which is the line the encounter planner posts overwatch on) clear of homes and trees by 25 m. Everything else the layout measured as open is furnished like any other country. After the pass, `approaches` holds the kept approaches only, each as wide as its ground still measures round its middle.

**Gap:** The brief says "each measured approach corridor (400 m wide, 1,800 m deep)" stays a long view. The plan holds fans, not corridors: on the owner's map six of them, up to 43° wide, 1.3 km across at the far end. Keeping whole fans clear leaves ground more than 600 m from anything, which the sight rule forbids. Keeping the middle of every fan (11.6 corridors a map on average) left corridors lying side by side with no room between them for anything to cut a circle.

**Reach:** The encounter planner sees at most two approaches a settlement where it saw several fans; the street-furniture pass's `approach_corridors` reads the narrower list. Run this pass before it.

**Verdict:** sound for the main settlement, which the brief's rule (M19) is checked on and whose corridors never give way. Provisional for the others: see the next entry. **Confidence:** medium.

### A minor settlement's corridor gives way where nothing else can break a circle

**Choice:** Bare ground is filled in three tries: random places near it, then every place within reach in turn, then every place again with only the main settlement's corridors kept. An approach whose corridor took a copse that way drops out of `approaches`.

**Gap:** The brief asks for both rules without saying which wins where they cannot both hold: no unbroken circle anywhere, and nothing that blocks sight in a corridor.

**Verdict:** sound. Over 54 maps 473 of 479 kept approaches survive; five maps lose one. The alternative is a place where a unit sees a perfect circle, which is the thing the owner asked to be rid of. **Confidence:** medium.

### What stands in the open country ends an approach as a disc

**Choice:** `measure::approaches` judges a corridor along five lines 100 m apart, which is enough for a wood or a settlement and misses a yard or a copse between two lines. A yard, a copse, a single tree and each half-width of a tree line are asked as discs against the whole corridor instead. Forests under the layout's smallest wood (π × 60² m²) are the small ones.

**Gap:** Nothing but woods, settlements and water ended an approach before; the pass adds things smaller than the measure's resolution.

**Reach:** The layout's own approaches are unchanged (its tests and records agree). A wood's lobe can still reach between two measuring lines: the test found a tree of a layout wood inside a recorded corridor on Mixed Small seed 1, before the pass runs. Not fixed here.

**Verdict:** sound. **Confidence:** high.

### The sight circle is judged by the simulation's two sight queries

**Choice:** A bearing is open when the line from an infantry eye (the rifle squad's 600 m, eye at 1.6 m) to the fog's ground target (1 m) at full range has no occluding body or ground on it (`sight_clear`) and no foliage at all (`foliage_depth == 0`). Bearings are one fog cell apart at full range, as the fog's sweep casts them: 472. A bearing that leaves the map is judged to the map's edge. Samples are a square grid over open ground: dry, outside every settlement's outline, off forest ground, with no body on the spot. Column positions are the units of each side's column as `plan_encounter` places the shipped assault.

**Gap:** The brief asks for "the simulation's own sight query" and "the standard infantry sight range" without naming them. `sees_point` alone passes a ray that foliage only shortens when the map's edge is nearer than the shortened reach, so a place beside the edge read as unbroken with a copse 100 m away.

**Reach:** A single tree counts: its crown shortens the ray by about 6%. That is a notch at the rim, not a wedge of fog; the wedges come from buildings, which occlude outright.

**Verdict:** sound. **Confidence:** high.

### Keep visibility distribution diagnostic while finishing playable maps

**Choice:** Use the user's 50% floor for median sampled openness and keep the fraction of views with fewer than half their directions reaching full range as a reported diagnostic. Remove the inherited six-point enclosure-rise tuning gate rather than starting another placement optimization. Global interrupted sight, physical legality and clear main approaches remain mandatory.

**Gap:** The user selected the median floor and asked to stop overoptimizing before play, without prescribing every diagnostic's admission role. The former enclosure-rise threshold was an implementation choice rather than a user requirement.

**Reach:** A map can have more locally enclosed positions while its typical open-ground view still meets the floor. The report exposes that distribution for actual play review; it must not hide it behind the median or claim every position has half its directions open.

**Verdict:** sound as a bounded playability-first interpretation; final pictures and play still judge whether the fields remain useful. **Confidence:** medium.

### Bare ground is filled; densities are small

**Choice:** `sight.reach_m` 480 on 100 m cells: a place is at most 71 m from its cell's middle, so something stands within about 550 m of it, inside the 600 m sight range. A cell is filled by a copse (4 in 5) or a tree line (1 in 5), 300 to 1,200 m² and 60 to 130 m. On top of that: a home group for each 2 km of country road and track, 0.1 tree lines and 0.1 copses a km² of open ground at least, 1.2 single trees and 1.2 clusters of low cover a km².

**Gap:** The brief's first figures were denser (a home every few hundred metres of road, hedged fields); the owner's correction asked for a light touch, tree lines short and occasional, house groups sparse.

**Verdict:** provisional: numbers to tune by looking. Fill does nearly all the work: with the copse and tree-line densities at zero the same maps hold nearly the same copses. A Small map gains 8 to 44 buildings, 270 to 700 trees and 85 to 130 loose bodies; a Large one 26 to 72, 800 to 1,800 and 240 to 330. Open maps gain the most homes, Metro the fewest. The 120 s battle of `city_report` costs 0.4% more instructions on Mixed Small seed 1 (79.1 to 79.4 G) and 1.9% more on Metro Large seed 1 (134.9 to 137.4 G); its memory goes from 222 to 227 MiB and from 629 to 646 MiB. **Confidence:** medium.

### Trees are forests, whatever their size

**Choice:** A tree line is a stroke forest 12 m wide in stretches of 45 to 90 m with 12 m gaps, 8 m back from a road's edge where it follows one. A copse is a ring. A single tree is a 9 m square plot, square to the map, on which the one forest rule stands exactly one trunk; a yard's clump is a 12 to 16 m plot behind it.

**Gap:** The catalog has a `trunk` row, but the simulation gives a crown only to a tree a forest stood.

**Reach:** Forest ground conceals: a soldier under a single tree is concealed as in a wood. `forest_share` and the layout's forest fairness count the rings (under 0.1% of a map).

**Verdict:** sound. **Confidence:** high.

### Fair halves are counted, not weighed by area

**Choice:** `|top − bottom| ≤ max(25% of the total, a least amount)`, for buildings (3), metres of tree line (250), copses (2), trees (3) and loose bodies (6). Each kind is placed in the half that holds less.

**Gap:** The brief asks for "the same measure the generator already uses", which compares areas against the playable area. These things have no area worth comparing.

**Verdict:** sound: all five are even on 899 of the 900 swept maps. Open Small seed 38 holds 13 homes in the top half and 7 in the bottom, whose roads had no more room; Mixed Small seed 15 has no home at all. **Confidence:** high.

### Low cover is placed although nothing draws it here

**Choice:** Field cover uses `boulder`, `log`, `car_wreck` and `pallet_stack`; yards use `parked_car`, `pallet_stack` and `crate`. Their boxes are rows of the presets.

**Gap:** All but `crate` are `systems_only` rows: on this branch nothing draws them, so a boulder is cover and an obstacle the player cannot see. The street-furniture branch draws an artless prop as its own box (`f487502f`), on a renderer this branch's base has since replaced.

**Reach:** Until a drawing lands, `field_cover.per_km2` and `homesteads.body_chance` at zero take them out without touching code. The sight rule does not depend on them: none occludes.

**Verdict:** provisional. **Confidence:** high that it must not ship undrawn.

### The versions are the street-furniture branch's

**Choice:** `layout-8` and `layout-presets-7`, the strings the street-furniture branch also took, so the two merge without a conflict on those lines. The merged generator is a third thing and needs its own bump and re-record.

**Verdict:** sound for the merge; the records in this branch describe this branch alone. **Confidence:** high.

### What the unprimed look at the pictures found

**Choice:** None taken from it in this pass; recorded for the next. One fresh critique of the owner's map after the pass (the opening view, straight down over blue's column with fog on, a house group, a tree line, and the plan's picture) against the Broken Arrow references.

**Found:** The country still reads as empty: at the opening view the jeep is the only upright thing in frame, and the eye has few landmarks. The fog over blue's column is cut on the lower left, the right and the bottom by copses and tree lines, and is one clean arc across the top: broken, and mostly open, which is what the owner asked for, and less than the references' wooded country. Tree lines and copses ignore the field pattern, because the fields are the renderer's and the generator does not know where their edges are. The fog behind a tree line pulls in some way past it with nothing at the tip of the teeth: that is the one forest rule shortening sight, not a missing model. A house group is houses on grass: no yard, track to the door or outbuilding is drawn, two houses of one group are the same model side by side, and no car stands beside them because nothing draws one.

**Verdict:** provisional. The rule (M25) is met and measured; the look is the light touch that was asked for and is sparser than the references. Fitting tree lines to the drawn fields' edges needs the two to share one field geometry. **Confidence:** medium.

## C07: warm cover-facing parity

**Choice:** Use pinned `libm::atan2` in the existing cover-bearing calculation,
without adding a scalar math wrapper or migrating unrelated trigonometry.

**Evidence:** A generated battle's first divergent tick had only one squad's
yaw and copied sight bearing differ by one float64 ULP. The two-rifle public
paired fixture reproduces the same `(17, 100)` bearing and first fails at tick
30, while float32 publication hides it. The existing math module has no shared
bearing owner. This deliberately changes Native's last bits to the portable
result; gameplay rules and the transport contract are unchanged.

**Verdict:** sound; high confidence. Scope follows an observed field and public
red proof, rather than assuming all standard math needs replacement. Remaining
generated-battle parity must still be measured after integration.

## C07: ricochet numerical primitive

**Choice:** Canonicalize the whole existing ricochet scatter primitive's sine
and cosine calls with the pinned `libm`, keeping its RNG consumption and rules.

**Evidence:** Direct paired captures establish equal impact inputs and scatter
basis before the first divergent azimuth cosine. Persistent-state equality
alone would not have established that cause. A public damage-decision test uses
fixed captured inputs and explicit rules; the passing two-unit battle candidate
is not retained as a regression. Actual generated Native/Wasm playback validates
the consumer without adding a diagnostic API or durable format.

**Verdict:** sound; high confidence. This corrects named Native last bits at a
measured primitive rather than migrating an inventory of unrelated math calls.

## C07: the existing rotation owner

**Choice:** Pin the shared cached rotation evaluator with `libm::sincos`, rather
than patching suppression, near-miss distances, particular angles or individual
rotation consumers. Keep its matrix arithmetic and cached pair unchanged.

**Evidence:** Exact prop poses and authoritative pre-event state did not imply
identical transient normals. Event capture identified the first differing sine
component at the same yaw, then its downstream hit-time/distance/suppression
changes. A minimal public wall-ray query reproduces the float64 fault on Native
and checks the actual Wasm query directly. Review selected the existing combined
software evaluator to share argument reduction rather than call sine and cosine
separately. No measured performance win is claimed.

**Verdict:** sound; high confidence. This generalizes within an observed math
owner while leaving unrelated, unproven numerical hypotheses alone.
## Kits on request

Every page fetched every kit before it could start: about 116 MB of kit bundles, whatever it drew. A page now fetches a kit only if something it draws needs it (the [scene-assets readme](../../packages/scene-assets/README.md), "Fetched when drawn").

### The one loader fetches kits when asked, and everything else with the catalog

**Choice:** `AppearanceLibrary.load` takes every appearance that is not a kit, and the template art library; `withKits(names)` fetches the kits it lacks and installs the next generation with them, or returns the installed one when it has them all. The alternatives were a second catalog file listing kits per map, and making every appearance lazy.

**Gap:** The spec's download budget counts a "shared kit" and says nothing about when it is fetched.

**Reach:** One catalog file and one fetch path remain. A request for kits is a generation like a load: every hash checked, installed whole, and a failure names the kit and leaves the installed generation. Two askers for one kit share one fetch. Kits stay in the page's generation once fetched, so the next map that draws from one does not fetch it again; nothing evicts them, and a reload of the catalog (the workbench, after a re-bake) starts without them. The models layer still installs only the kits the map on screen draws from, so the GPU holds what it held before.

**Verdict:** sound. Measured from each page's network log, kits fetched fell from 116.3 MB on every page to: 38.8 MB on the village battle (its own kit), 58.6 MB on a block of a generated town (the four town kits its buildings use: apartments, homes, farmsteads, industry; no tower stands in that block, so no tower kit), 7.2 MB in the facade lab, and none in a lab with no buildings. The 50 MB budget for a shared kit download holds for the village and not yet for a town, whose apartment kit alone is 27.6 MB. **Confidence:** high.

### Which kits a map needs has one owner, the drawer's

**Choice:** `buildingKits` in the model layer says which kits a map's buildings draw from, and it is what a map asks the loader for and what its models layer installs. The loader takes names and knows nothing of maps. The alternative was a loader method that takes template ids.

**Gap:** `templateKits` (the library's) and `buildingKits` (the drawer's) both existed; the brief asked for one owner.

**Reach:** A template's kits are the library's fact, and `templateKits` stays its owner. What a map draws beyond them is the drawer's: a fallen part with no art for its state is the prototype kit's box, so a map with buildings asks for that kit too, and a map with none asks for nothing. A battle also draws props that have no art as that box, so every battle asks for it (4.7 KB) whether or not its map has a building. Every route that draws buildings takes its appearances from `useMapAppearances`; three labs (geometry, ballistics, camera) had been handing the model layer the whole catalog and now ask for their map's kits like the rest.

**Verdict:** sound. **Confidence:** high.

### A kit that was not fetched is a named state, and a map without its kit is refused

**Choice:** A generation lists every kit the catalog names (`kits`); a library module whose kit is not installed is bound to no state (`state: null`), which the type makes every reader handle; and `mapAppearances`, which says what the models layer installs for a map, refuses by name a map whose kit is not installed (`kit.missing`). The alternative was to leave absence to the model layer, which waits without a word for art that covers its buildings.

**Gap:** The library was bound to its kits at load, so an absent kit failed the load. With kits on request the library is installed first, and its rows name kits that may never be fetched on this page.

**Reach:** The model layer's wait stays: buildings and their art reach it apart, in either order, so an uncovered moment there is not an error. The refusal is where the two are put together for a map. The library is still held to its kits at load, without fetching them: the catalog's hash for each kit must be the one the library was packed against. A module its kit lacks is found when the kit is installed, and refuses that request.

**Verdict:** sound. **Confidence:** medium: the refusal is a thrown error in the page, as a template without art already was; neither reaches the loading screen.

### A failed fetch is an error on the console, as a failed catalog load was

**Choice:** A kit that fails to arrive is reported as a failed catalog load is: an error on the console naming the kit, and the world never draws, so the battle's loading cover stays up. Not built: the failure on the loading screen with a way back.

**Gap:** The loading screen reports the preparation worker's refusals; it has never reported an asset failure.

**Verdict:** gap. Scenes fail on the console error, so a broken bake cannot pass; a player on a bad connection sees a cover that never lifts. It is the loading screen's to carry, for the catalog and the kits together. **Confidence:** high.

### Not in this pass: the catalog's own download

**Choice:** Only kits moved. What a page fetches before it knows its map is now about 240 MB that no kit is part of: the soldiers' bundles (108 MB), scenery (104 MB, two wrecks 31 MB of it) and vehicles (28 MB).

**Gap:** The brief and the lane's status put the whole download at "about 100 MB of kits".

**Verdict:** gap, for whoever owns the download budget next. The same rule would cover it: a battle asks for the unit types its scenario fields, a map for the scenery its props take, through the request path kits now use. **Confidence:** high on the numbers, which are the files' sizes.
## C27: ruin and gutted art drawn by what a side knows

### The state is the published prop's type, read against the building row

**Choice:** `fallenBuildings(index, known, shells)` draws a building `gutted` when a side knows one of its parts as a prop of a type the catalog's building rows name as their gutted shell (`destroyed.into.building.gutted_prop`, read by `buildingRemains` in scene-assets), and a `ruin` when it knows a part replaced by anything else, or destroyed with nothing left. No publication change was needed: a known prop already carries its kind, the part it replaces and its box.

**Gap:** The slice says "drawn on the published collapse or gutting, via the knowledge path", not which published fact tells the two apart. The alternatives were the remains' height against the part's (a 2 m shed's remains are 2 m: a guess), and the appearance binding (`drawn_by: "gutted"`, which is a presentation label the simulation lane may still rename).

**Reach:** Every route that draws a battle (`useBattleSession`) and the two labs that show a destruction without one. The catalog's shipped view is where the rule is read, as the bake reads the collapse rule there.

**Verdict:** sound. Played in the built WebAssembly on the camera lab's map (`buildingKnowledge.test.ts`): a five-floor block shelled down is published as three `ruin` props at 4.075 m, a twenty-floor tower as one `gutted` prop at its 61 m, to the side that watched and not to the other. **Confidence:** high.

### One known part is the whole building's state

**Choice:** A building is drawn destroyed once the side knows any part of it replaced, in the one state that part says; were parts ever known both ways, a ruin.

**Gap:** The contract asks what a partly known compound is.

**Reach:** None in play. The simulation destroys every part of a building in one call and by one rule (`Battle::destroy_prop`), and a side that sees any part's footprint learns every part (`revealed`, in the sight pass). The test stands a squad where one wing of the U block is out of its sight and finds all three parts published.

**Verdict:** sound: the art is one building's, so half a building cannot be drawn in another state, and the boxes the camera and the fog are handed are per part from the same knowledge, which is whole. **Confidence:** high.

### No fallback: a template without rows for the state is refused by name

**Choice:** The box fallback is deleted, with `BuildingScene.box`, `buildingKits` (the kits a map draws from are `templateKits` again, the library's own fact), the prototype kit a map with buildings used to ask for, and `presentation.buildings.ruin_tint`. `setFallenBuildings` throws `state.missing` and leaves the scene as it was.

**Gap:** The contract says to delete it "if nothing can reach it".

**Reach:** Two things reached it, both labs inventing a fall: the city block's switch (remains of 1.5 m whatever the building) and the camera lab's, which brought a twenty-floor tower down to 4 m. A tower stands, gutted. Both now ask `seenDestroyed` what the simulation would have published (the template's own damage state in the library, the catalog's building row, `ruinHeight`), and the camera lab's falling building is its courtyard block (`fixtures/camera-lab.json` `collapses.owner`), whose remains its idle camera recovers over. Street-furniture stand-ins still ask for the prototype kit themselves.

**Verdict:** sound. **Confidence:** high. A map whose simulation ends a building one way and whose library has the other state would stop drawing; the bake refuses such a library.

### The proof is a shelling by the lab emitter, on the camera lab's map

**Choice:** A saved encounter, `camera-lab/shelling`: `burst` events (the scenario's own emitter of a weapon's blast) wear the U block and the tower down; a blue squad watches, a red squad stands behind two slabs and is scripted to walk out at tick 600. `/lab/city-ruins` plays it with a side switch.

**Gap:** The brief offered the generated battle, the saved town with scripted fire, or a lab fixture.

**Reach:** The bursts go through `structures.damage`, `destroy_prop` and the ordinary publication: nothing is injected. Tanks firing would have added minutes of flight and aim to a scene that judges knowledge, and a generated town has no place a side provably cannot see from. The map already holds a compound, a tower and slabs to hide behind.

**Verdict:** sound for what it proves. It is not a fight: nobody fires a weapon. **Confidence:** high.

### The far stations stand at 1.3 times the last tier boundary, and look down

**Choice:** The scene's far stations are 1,384 m out at pitch 0.8, and the tower's near station at pitch 0.9.

**Gap:** "The tier 3 distance" is not one distance for two buildings 220 m apart.

**Reach:** From 1.1 times the boundary, the other subject is nearer than the boundary and draws at tier 2. And the ground-classes view is black past the map's edge, which lies 105 m behind the tower: a flatter camera puts the edge behind its roof. At the whole-map view (2,000 m, the rig's own pitch) the edge is within reach of the roof, so the tower is judged there by its height and brightness only.

**Verdict:** sound. **Confidence:** high.

### A destroyed building smokes for the side that knows it

**Choice:** Each part a side knows as remains or as a gutted shell is a smoke source (`effectFeed.ts`), with a look of its own in `presentation.effects.smoke`: `gutted`, thick dark smoke off the roof for three minutes and thin smoke for fifteen more; `ruin`, pale dust for a minute and thin smoke for ten. No flame.

**Gap:** The slice delegates the burnt treatment. The unprimed critic could not tell the gutted tower from a dark-fronted or shaded one when it stood alone, and lost the ruin at range.

**Reach:** The same source machinery wrecks use: it burns from when the side first knows of it, so a side that learns late sees it start then, as a wreck does. Every map: a village house that falls smokes too. Sources share the one smoke budget.

**Verdict:** a cue, not the fix. The silhouette and soot are the art's (C14). **Confidence:** medium: the look numbers were chosen by eye from one battle.

### Not done here

- The `gutted` prop type still says `appearance.status: "systems_only"` and `drawn_by: "gutted"`. Its art exists and is drawn; the row is the simulation lane's.
- The preview checkpoint: this pass ran as one of several agents, so no Preview window was opened. The frames are listed in the slice's Outcome.

## C86 tree lines (drawn half)

**When:** 2026-10-02. Evidence: "Outcome — drawn half, first picture" in [C86](slices/C86-tree-lines.md).

### The rig's tree line is the longest straight strip between open ground, seen across

**Choice:** The `tree-line` stations take their candidates from the renderer's own forest strokes and keep one only where the simulation's ground agrees: wooded along its middle, open ground 4 m and 30 m beside it on both sides. Of those 80 m or longer and 400 m inside the map they stand on the longest, with the camera on its southern side looking across it.

**Gap:** The gate asked for a station on a tree line and named no line, no framing and no way to find one.

**Verdict:** sound. A scan of an 8 km map by the ground alone would cost a million probes a page load; the strokes name eleven candidates and the ground still decides. Across is the framing that asks the slice's question (does it block the view). The inset keeps the map's edge out of the 250 m frame. **Confidence:** medium: the stations move to another line when the generator's layout changes, as the rig's other generated stations do.

### A tree line carries a hedge under its trees; a wood does not

**Choice:** Every strip of forest is drawn with rows of the hedgerow shrub (2.8 to 4.1 m tall) along its length, under its trees: `trees.understorey` in the biome (spacing, rows, sway, gaps, length and height ranges). A wood keeps the forest floor's rule: nothing with no body stands over 0.9 m.

**Gap:** The first picture showed bare ground between the boles of a strip the simulation hides a far squad behind. The dressing's rule (nothing drawn may look like it hides a soldier) is written for a wood's floor, where sight fades with depth and a soldier near the edge is seen.

**Verdict:** sound. The two rules say the same thing: draw what the simulation does. Across a strip the simulation already fades sight to nothing at range, so a hedge there tells the truth and bare boles lie; inside a wood a soldier a few metres in is still seen, so a shrub that hid him would lie. **Confidence:** high on the rule; medium on the look (one hedge model, repeated).

### A shrub keeps its whole reach on the strip's ground and under its foliage

**Choice:** A shrub is placed only where the circle of its reach lies on the strip's own forest ground, every point of it in a fog cell with foliage or beside one, clear of paving and water and of every body but a tree. It is drawn smaller to fit, and not at all where it would be smaller than the biome's range. A strip under about 5.5 m wide therefore carries none.

**Gap:** "Never wider than the canopy the simulation blocks sight with" names no measure. The crowns' test holds a crown's rim within one fog cell of the foliage; the simulation's foliage does not cover every cell of a strip (a cell whose centre no canopy reaches is open).

**Verdict:** sound. The strip's ground is the stricter bound and needs no number from the simulation; the foliage test is the crowns' own, so a stretch of strip with no trunks (and no foliage) gets no hedge either. **Confidence:** high. A generated tree line is 12 m wide, which holds two rows.

### The ground under a tree line is the plots' verge with its grass, drawn in to a point at each end

**Choice:** A strip has no forest floor. Under it lies a band of the plot verge's colour with the verge's grass growing on it: the strip itself, its edge wandering by `forest_floor.tree_line.warp_m`, its half width falling evenly to nothing over the last `taper_m` (14 m) before each end the strip is cut square at. The hedge's rows draw together over the same length. Past the band, inside the strip's square corners, the field's own crop grows. Woods (rectangles and polygons) are untouched.

**Gap:** The floor under a strip read as "a brown smudge or scorch mark" and ended as a blunt blob; the brief left the look between the verge and the floor's moss.

**Verdict:** sound for the look, provisional in two details. A tree line stands on a plot boundary, where the verge already runs, so the band is that verge grown wide: it joins the field margins at both ends without a new colour, and light reaches the ground under one row of crowns, so grass is right there and bare earth was not. The details: grass now grows on ground the simulation calls forest (in the band and in the corners), which no rule reads; and the taper is per exported stretch, so a strip that begins with a bend narrows over its first chord only. **Confidence:** medium.

### The shrubs are a population of the scenery layer, drawn and shadowed as the forest is

**Choice:** `SceneryPlacement.understorey` is laid whole at load (a map holds a few hundred shrubs) and drawn through the static chunk owner beside the forest: same tiers, same fragment stage, casting into the cascades, gone with the trees' switch and where the side has seen its ground cleared. `BattleFrame.setUnderstoreyShown` (the lab's `suppressUnderstorey`) switches it alone for the cost measure.

**Gap:** "Through that same static chunk path" could mean rows appended to the forest's own list. That list is one tree per trunk, which the scenes and tests count.

**Verdict:** sound. No pass, no per-frame CPU work and no second chunk owner; the forest's count stays the trunks'. **Confidence:** high.

## C87 ground composition gate

### Forest floor bodies are off by default

**Choice:** `forests.rule` lays 0 logs and 0 boulders a hectare. They were turned on at 5 and 3 candidates a hectare once the models existed (`4bdfb76f`) and reverted at the gate (`b0d5b19a`). The models, the undergrowth and the rig's floor stations stay; the models draw wherever a map's own rules lay the bodies, as generated battles do.

**Gap:** The forest-bodies slice planned the activation once models exist. The quick village report (three seeds) moved every digest and no outcome, which read as safe.

**Verdict:** provisional: off is the reversible call, and when to turn them on is the owner's. With them on, the full village report (ten seeds) panics: the flank script's order is refused on seed 34, because cover now stands in a wood the script walks through. With them off it completes. Turning them on for the village needs its scripts and labs to stand cover appearing in their woods, the simulation's and the map's work; it is one rule in `fixtures/game.json` and a named digest change. **Confidence:** high that off is right today.

### The generated scene counts boulders with the static world

**Choice:** The check "every body on the map is drawn" is: structures plus boulders equals the map's props.

**Gap:** The buildings lane's check counted one stand-in per prop. A boulder never changes, so it draws with the static world, not among the structures the side's knowledge redraws.

**Verdict:** sound: 3,763 structures and 37 boulders are the map's 3,800 props. **Confidence:** high.

### The town's edge needs some drilled field in frame, not 15%

**Choice:** `_town.mjs` asks for over 3% of the town-edge frame to be drilled crop, of at least one kind.

**Gap:** The check was written when the frame held 20%; main's generator moved the fields, and the meadow that surrounds a town fills most of the frame.

**Verdict:** provisional. The rule it holds (yards on one side, crops on the other, in one frame) still fails if either is missing, but 3% is whatever today's generator leaves in frame; a station that follows the nearest drilled field would not need the number. **Confidence:** medium.

## Town look, second pass

Four things earlier unprimed reviews still found, taken in the owner's order: street warts left by the parcel pass's links, a city that is a disc round one crossroads, a large town that is a slab along one road, and suburb streets that read as wood grain. The model as it stands is in the [C52](slices/C52-procedural-generator.md#outcome) and [C53](slices/C53-parcels-and-buildings.md#outcome) outcomes and the [crate guide](../../crates/mapgen/README.md). Generator `layout-10`, presets `layout-presets-9`. Counts below are over the 36 maps of `tests/street_warts.rs` and `tests/road_ends.rs` (every type and size, seeds 1, 2, 3 and 2⁶⁴−1) unless a sweep is named.

### A district's grid is fitted between its edges, and each line runs to the road ahead of it

**Choice:** The parcel pass no longer lays a grid at a random offset and patches it afterwards with links and run-ons. A district's grid is fitted to it: a whole block from a carriageway on an edge to the first street, half a block from an edge that faces the fields to the last one. Each line of the grid then runs on from its last node to the first carriageway ahead of it, within `parcels.run_on_m` (60 m) past the district's edge, and ends just past that carriageway's middle. A line that meets nothing ends a verge inside the district's edge if it is a long street, and on the last long street if it is a cross street.

**Gap:** The brief asked for the warts to be fixed where the streets are laid: a street runs from a junction to a junction, or ends at the last lot it serves.

**Verdict:** sound. Streets that stop within 40 m of a carriageway ahead of them went from 120 to 4, and a map has about a seventh fewer road ends, because a street is one stroke from junction to junction where it was a grid line, a link and a run-on. **Confidence:** high for the count; medium for the half-block rule, which is one reading of "the last lot it serves".

### A street lands where a street already meets the road from the far side

**Choice:** `towns.align_m` (30 m). Where a line of a grid, a link or a run-on comes to a carriageway within that distance of a point where a street already ends on it from the other side, it is moved to that point: a bent street's line is tilted through it, a straight one's turned. The layout does the same for the cuts between blocks, so avenues either side of a road line up. A junction is taken once from each side: a third street there would make a fork, which `no_point_of_a_settlement_has_district_edges_fanning_out_from_it` caught when the rule first allowed it.

**Gap:** "Neighbouring districts' streets do not line up across the avenue between them."

**Verdict:** sound. Pairs of streets that meet one road from opposite sides 2 to 20 m apart went from 1,538 to 57. The 57 are mostly 15 to 20 m apart, where the nearer line of the grid was already taken or the move would have been more than 0.4 of the way to the next street. **Confidence:** medium: 30 m is a guess, and a line moved 25 m shows as a slight slant in a straight grid.

### Only streets of one width share an end; an unlike street lands opposite

**Choice:** A street that lands on a junction made by a street of its own width ends on that street's very end point, and the joint pass welds the two into one street through the junction. One of another width (a street opposite an avenue's end) ends opposite it, just past the road's middle from its own side. A street never carries on end to end from the open end of a wider avenue: it meets the avenue just short of that end, as a side street.

**Gap:** None in the brief. Found by the road-end test: sharing the point with an unlike way made a corner the joint pass had to carry a width through, and where that lost a joint it pinned the ways as laid, with a bite at every corner of them.

**Verdict:** sound. `tests/road_ends.rs` counts 6 flawed ends of 22,294 (under 3 in ten thousand; it was 23 of 24,636) and its bound is lowered from 10 to 3. `joints` is unchanged. **Confidence:** high.

### A street meets a road square, from its last crossing

**Choice:** A line of a grid that would come to the carriageway ahead at less than 45° to it turns at its last node and runs square to that carriageway, when the node is at least two street widths from it. Nearer than that it runs straight on, and the joint pass bends its last few metres as before.

**Gap:** "A street meets the road it joins, square."

**Verdict:** provisional. Side roads cut a town's blocks at a slant, so this case is commoner than it was. A try at turning one node sooner when the node was too near made more streets run beside the road, and was taken out. **Confidence:** medium.

### A link is a street of the grid carried on, and never a stub

**Choice:** A piece of grid no road crosses is joined by running one of its own streets on along the grid to the first carriageway ahead, or to a node of the same grid already joined. A street that turns off another to do so is at least `parcels.street_step_m` (45 m) long. A line whose whole length would be one run shorter than two widths is not laid. Two streets of one grid do not come to a road within three widths of each other: the second stops at its last crossing.

**Gap:** "A link shorter than a couple of street widths should not exist."

**Verdict:** sound. No street shorter than two widths shows on any of the 36 maps (one piece half a metre long lies wholly under a crossing road, where the joint pass changes a road's width; the test does not count what cannot be seen). Double bends went from 11 to 1, and streets lying beside another on the same ground from 115 to 8. **Confidence:** high.

### A short tail past the last junction is cut back

**Choice:** `parcels.tail_min_m` (35 m). After every street is laid, a street or avenue that runs on past its last junction for less than that and stops in the open is cut back to end just past that junction. A country road is left to the layout.

**Gap:** The unprimed critique's first finding: an avenue that ran the whole edge of its block overshot the last street by 25 m and stopped at the town's edge with a flat end. "A street runs from a junction to a junction, or ends at the last lot it serves": 25 m serves no lot.

**Verdict:** sound. The two then meet end to end and the joint pass makes them one street round the corner. Flawed road ends went from 9 to 6 with it. **Confidence:** medium: 35 m is a guess between one lot's front and two.

### What is still counted

**Choice:** `tests/street_warts.rs` holds each wart to what is left: 4 streets that stop short, 8 pairs side by side, 57 staggered junctions, 1 double bend, no stub, among 39,532 ends and junctions. The numbers are to be lowered, never raised.

**Gap:** The brief asked for "at or near zero".

**Verdict:** provisional. What is left is where a side road cuts a block at a slant and a grid meets it at an awkward angle. A country road and a track side by side (a lane peeling off its road) is counted apart and not held: 38, from 34; that is the road network's.

### A large town and a city have secondary roads, as a row of their class

**Choice:** `classes.<class>.side_roads`: how many (a large town 1 or 2, a city 2 to 4), where one leaves a road through the centre as a share of the way to the edge of the settlement's ground (a large town 0.1 to 0.35, a city 0.25 to 0.6), how far it turns off (55° to 100°) and how far apart two leave one road. Each turns into the widest sector no road runs out through yet and runs straight to a gate past the settlement's ground. It is an ordinary country road: the ground is cut along it, blocks grow along it by the class's `ribbon`, later links may join its gate, and industry may stand where it leaves town. No other class has the row.

**Gap:** "Give cities and large towns secondary roads out of the centre (not all through the one junction) and let growth follow them", and "make sure a large town always has one".

**Verdict:** sound for the roads. Every city in the tests has at least one road that leaves its main roads away from the central junction and the typical one two or more; every large town has two roads at 40° or more to each other. A city's outline now has arms along those roads and bays between, and its core is one group of blocks near the middle. A large town is still a compact mass: its second road crosses it, and its blocks still fill the ground between. **Confidence:** medium. The shares and angles are guesses, and a side road that no later road joins stops at the town's last block.

### `roads.centre_roads` counts roads, not the hair an end runs past a middle

**Choice:** `measure` no longer counts as a road out of a junction the quarter metre by which a joining road runs past the middle of the road it joins.

**Gap:** With a side road within 500 m of the centre on nearly every Mixed map, the T it makes counted as four roads meeting, and `main_roads_meet_in_a_crossroads_on_some_maps_and_fork_on_others` saw a crossroads on 29 maps of 30.

**Verdict:** sound. The number is now what its name says. The sweep's "four roads meet at the centre" share is lower than C52 recorded for the same reason. **Confidence:** high.

### Suburb and village blocks are shorter

**Choice:** `garden_suburb` streets have a cross street every 140 m (190) and skip 15% of them (30%); `village` 160 m (200) and 20% (30%).

**Gap:** "Add the occasional cross street so blocks are blocks."

**Verdict:** sound. With the grid fitted between the district's edges a cross street also runs the whole way across, so a suburb is blocks of four to eight lots a side. **Confidence:** medium on the numbers.

### Three tests were changed, and why

**Choice:** (1) `road_ends.rs` and `street_warts.rs` count an end that lies under its own road's paving, six widths or more along it, as joined: a street that comes back round to itself. (2) `towns.rs` follows a ray from an apex edge by edge, with a gap of 40 m at most, where it used to take any edge on the line no farther than twice its own length. (3) The control at the end of `open_country.rs` asks only that the owner's seed still has open ground with an unbroken circle before the pass; it no longer asks for a column that starts with one.

**Gap:** (1) The joint pass can weld a street into a loop that ends on itself; the test saw paving ahead of an end and called it a gap. (2) A cross cut 150 m from a village's centre lined up with it by chance once a fourth road met there. (3) That seed is another map under this generator, and its columns start in sight of a house.

**Verdict:** (1) and (2) sound: each makes the test say what its comment says. (3) open: the control was a fact about one map of `layout-9`; whoever owns the open country may want another seed for it. **Confidence:** medium.

### What the unprimed look at the pictures found

**Choice:** One fresh critique of the six plan drawings and three in-game close-ups (Metro Large seed 1, Mixed Medium seed 2), against the Broken Arrow references, asked "does any town read as a diagram, and does any street or junction look wrong?". One finding was fixed in this pass and the rest are recorded.

**Found, streets:** its worst picture was the first named corner, where an avenue overshot its last junction by 25 m and stopped flat at the town's edge (fixed: the tail rule above; the avenue and the street now turn the corner as one). Still there: in that corner a secondary road and the avenue beside it converge at a shallow angle with one row of houses between; a suburb street ends in a hook where it turns to meet a slanting road; a short diagonal street leaves the country road and ends after 100 m; suburb streets all take the same bend at the same height ("one combed wave"); where a secondary road leaves the main road at a slant exactly where a street crosses, the junction has five arms; a branch peels off a suburb street at a shallow angle and leaves a sliver of grass between two carriageways; in the city's core the north road arrives 200 m from where the south road leaves. Streets that end at the last lots by the town's edge read to it as dead ends, the wider avenue among them.

**Found, towns:** "Diagram." Outlines are stacks of rectangles with stair-step corners; a large town's centre is one carpet of identical long blocks at one tilt with no square or park; a city's districts are rectangular slabs, each with its own grid, butted along hard seams; a city has a detached block no road reaches in the picture; towns are one filled polygon with almost no industry and no open ground inside, where the references are loose estates among woods and fields; in game the buildings stand on bare lawn.

**Verdict:** open. The street findings are the residue the wart test counts, seen: nearly all of them lie along a secondary road, which cuts blocks at a slant. A secondary road that follows a block edge, or blocks cut square to it, is the next thing to try. The town findings are C52's standing list (block shapes, zoning, no open ground, blank country) and were not this pass's four items; the secondary roads gave a city arms and did not change how its blocks are cut. **Confidence:** high that these are visible.

## Town look, third pass

The second pass's unprimed review found nearly all its street faults along the secondary roads, which crossed the street grid at a slant, and no open ground inside a town. This pass makes a secondary road part of the block structure, holds junctions to four arms and near-square meetings, gives towns parks and open blocks, and brings the top and bottom roads to one junction. The model as it stands is in the [C52](slices/C52-procedural-generator.md#outcome) and [C53](slices/C53-parcels-and-buildings.md#outcome) outcomes and the [crate guide](../../crates/mapgen/README.md). Generator `layout-11`, presets `layout-presets-10`. Counts are over the 36 maps of the street tests (every type and size, seeds 1, 2, 3 and 2⁶⁴−1) unless the 900-map sweep is named.

### A secondary road runs beside another road or square off its own

**Choice:** `side_roads.turn_deg` is no longer a range a turn is drawn from. A secondary road takes the line of the road (or earlier secondary road) that best follows its sector, where that leaves its own road at 60° to 120°, and otherwise leaves square. It starts 100 m or more from any road's end and is not laid where it would cross a road at less than 45°; a settlement tries four places for each one it is due.

**Gap:** The brief asked that blocks either side of a slanting road be cut along it with streets square to it, and that two roads never meet at a shallow angle. It did not say whether the slant itself should stay.

**Verdict:** sound for the faults, a loss of variety. Rows of blocks were already cut along every road; what made slivers, converging avenues and slanted streets was two roads 0° to 35° apart with one grid between them. Roads that run side by side leave a strip that both rows fit. A city's roads are now a loose mesh of parallelograms, set by the angles its main roads happen to meet at, and no longer fan out. **Confidence:** medium. A fan with blocks fitted to each wedge would look more like a grown town and is a larger change to how ground is cut.

### A junction is a T or a crossroads; a street meets a road square, bends to, or stops short

**Choice:** One landing rule for every street, link and run-on (`Network::landing`): on its own line within 20° of square; turned at its last crossing to run square, up to 28°; not at all beyond that, or within three widths of a junction it makes no crossroads of, or within 60 m of a junction of the layout's roads. A crossroads needs a street on the far side in line within 25° at a junction with nothing else by. A street that may not land goes back to its last crossing, or to 60 m short of the carriageway. The layout's cuts keep the same distances by choosing among the places a block may be cut at, and an avenue more than 22° off square to a road stops at the last block corner before it.

**Gap:** "Never a five-arm junction", "a fork's arms at least about 45° apart" and "streets meet it square (or run parallel to it)" were the brief's; the numbers and what a refused street does were not.

**Verdict:** sound against the counts: five-arm places with a street 33 to 0, forks under 45° 81 to 2, streets more than 25° off square to a country road 189 to 4, hooks 33 to 0, among 26,238 places before and 22,758 after. The cost is dead ends: a street that may not land stops a block short, and a grid all of whose lines are refused is an island joined by one link. **Confidence:** medium. 20°, 28° and 60 m are guesses; the first try refused every landing within 45 m of any junction and broke grids into rings, so only the roads' own junctions keep that distance.

### A district's grid takes the line its edges agree on

**Choice:** A district's long streets run with the edge that most of its other edges run along or square to (within 20°), by length, a road's counting double and an open edge's a quarter. The main street's way only breaks ties.

**Gap:** The second pass chose the edge nearest the main street's way, so that neighbouring grids agree. Beside a road at another angle that put every street at a slant to the road.

**Verdict:** sound. A district beside a road has streets beside it and square to it, and a wedge-shaped district meets most of its edges square. Neighbouring grids can now run crosswise to each other. **Confidence:** medium.

### A bend is one bow, different for every street

**Choice:** A district's `bend` is a wave 1.2 to 2.4 times the district's length (never shorter than the preset wavelength), reaching no farther than 4% of that length, with a reach that changes linearly across the district from one drawn value to another, either sign.

**Gap:** "Vary the grid's bend by block, or straighten it; whichever reads as streets rather than a pattern."

**Verdict:** sound. The combed wave is gone: streets near one side bow one way, those near the other the other way or less, and those between run straight. The suburb reads as a grid of nearly straight streets now, and the review called it a plat. **Confidence:** medium.

### Parks are built round and then left out; open blocks are never built

**Choice:** `classes.<class>.parks` and `open_blocks`. A park is chosen after growth from the built blocks: of the eight nearest the centre, the smallest with built blocks on every side and 0.3 ha left for trees 25 m in from its edges. It is taken out of the built ground, keeps an avenue all round, and gets a wood on that inner ground whatever the halves hold (the rest of the woods balance round it). 6% of blocks away from the centre are marked unbuildable before growth, and the built share is lowered by 6%.

**Gap:** "A square or small park on a block near the centre, and an unbuilt block here and there at the edge", from existing shapes only.

**Verdict:** provisional. Choosing parks before growth left them at the town's edge half the time, where their trees stood in a kept approach and one Mixed Small seed was refused; choosing among built blocks makes them enclosed by construction. Open blocks are not held to the edge: one inside a town is a square of bare ground, which the brief's "open ground inside a town" also asks for. The review read the park as a plantation and the open blocks as holes. **Confidence:** medium that this is air; low that it reads as a park.

### The top and bottom roads meet at one junction

**Choice:** In the skeleton, the later of the two timed arms ends where the earlier did: at the hub, or on the far side of the arm the earlier joined, leaving its edge where a straight line through that junction lands if its edge allows. `roads.junction_apart_m` (700 m) replaces the roads' bend step (300 m) as the least distance between two junctions of the edge roads.

**Gap:** "The two edge roads meet at one place in a city's core, or pass through one junction, not 200 m apart, where the map's roads allow it." M22 had each arm fork on its own.

**Verdict:** sound. The road from bottom to top passes through one junction wherever the journey time allows the second arm that way. Four roads meet at the centre on 13% to 43% of the sweep's maps by type and size (it was 8% to 29%); `main_roads_meet_in_a_crossroads_on_some_maps_and_fork_on_others` still holds. **Confidence:** medium: a map loses the case where the two main roads join a cross road at two well-separated places.

### An avenue that carries a road on is that road

**Choice:** A cut's stretch that starts on a road's end and runs on along its line is laid at the road's kind and width, not the avenue's.

**Gap:** Found by `tests/road_ends.rs`: a 10 m avenue end to end with an 8 m road showed shoulders the joint pass could not mend once fewer cuts crossed there.

**Verdict:** sound. M26 already says a road keeps one kind and width from junction to junction. **Confidence:** high.

### Two fallbacks keep every map generating

**Choice:** (1) A settlement whose ground, cut clear of its junctions, holds no block a parcel fits on is cut again with cuts where they were drawn. (2) The parcel pass cuts back avenues that stop short of a carriageway; where that leaves a district no parcel, it runs again without cutting avenues.

**Gap:** Three seeds of the sweep and of the river tests were refused on hamlets: two round a fork whose blocks came out too small, one whose only lane was cut back.

**Verdict:** sound as a guarantee, a patch as design: both are a second attempt under looser rules, not a rule that cannot fail. The 900-map sweep generates 900. **Confidence:** medium.

### Tests added and changed, and why

**Choice:** (1) `tests/town_junctions.rs` is new: it clusters joints into places, counts the carriageways that leave each at 30 m, and holds five-arm places, forks under 45°, slants over 25° and hooks at 0, 2, 4 and 0. Two roads of the network leaving a place side by side count as one way out there, and are reported apart (64 such forks are the road network's own). (2) `street_warts.rs` is lowered to what is left: 3 stops short (4), 1 pair side by side (8), 19 staggered pairs (57). (3) `layout.rs`: a district's stated area may differ from its ring's by half a metre and a rounding error; a ring on the centimetre grid can measure exactly half a square metre over. (4) `parcels.rs`: the Small-against-Large density comparison adds two standard errors of the smaller building count to its 25%; industry is some forty buildings at either size and came out 1.00 against 1.34 per hectare on one run and 1.19 against 0.88 on another. (5) The encounter record "a start that cannot be made fair" asks for routes within 0.1 s where it asked for 1 s: on its map the top and bottom roads now meet at one junction and the two drives differ by less than a second.

**Gap:** None in the brief for (3) to (5); each failed on this pass's maps for a reason that was the test's.

**Verdict:** (1), (2), (3) sound. (4) sound as arithmetic, and it weakens the check for rare kinds to what their counts can show. (5) keeps the record's purpose (a refusal), on a tighter number. **Confidence:** medium.

### What the unprimed look at the pictures found

**Choice:** One fresh critique of six plan drawings and three in-game close-ups (Metro Large seed 1, Mixed Medium seed 2) against the Broken Arrow references, asked "does any town read as a diagram, and does any street or junction look wrong?". Nothing was changed after it; one trial (aligning streets across a road within 45 m, not 30) more than tripled the flawed road ends and was dropped.

**Found, towns:** "Diagram", both, the large town more than the city. The large town is a slanted slab filled edge to edge with one grid, its east edge a ruler line for 800 m; the city's outline is a staircase of rectangular slabs with a seam at each change of grid and of building type; block interiors in game are empty lawn; the garden suburb is a full street grid with five or six houses to a block; the park is a rectangle of evenly spaced trees with no path or clearing, and the open blocks read as holes; in game the through road and the side streets look alike.

**Found, streets:** none of the second pass's secondary-road faults (no five-arm junction, shallow fork, hooked end, slanting meeting or streets bending in unison). Still there: streets either side of a main road meeting it about 30 m apart, at most crossings along it, so a road has many T-junctions and few crossroads (1,719 pairs stand 2 to 45 m apart on the test maps; the test holds only those under 20 m); suburb streets that run 20 to 30 m past their last cross street and stop at the field; dead ends inside blocks where a street stopped short of a road; square street ends showing at some T-junctions; one skewed crossroads where an avenue meets a main road at a bend; small jogs across a main road; a 25 m loop of street at the town's edge; secondary roads that begin at the built edge; country roads with visible corners.

**Verdict:** open. The junction faults this pass set out to close are closed and counted. The commonest street fault left is the staggered pair 20 to 45 m apart: lines move at most 30 m and 0.4 of their spacing to meet, and refusing the rest breaks grids up. The town findings are the block model's (a block is built whole, each district keeps one grid, nothing stands between buildings); air helped the city's plan and not the verdict. **Confidence:** high that these are visible.

## Road-side yards, C31 pickup

### A plot's centre determines which building may claim its yard

**Choice:** For a field plot near a house, draw a line from the plot centre to the house centre. A road crossing that line prevents the house from turning the plot into a yard. This keeps an isolated farmstead from making the country road look like a town street. The entire plot still has one ground kind; its edges are not clipped to the road.

**Gap:** The cross-road bug needed a road-side ownership rule, but the plan did not choose between whole-plot classification and cutting yard polygons.

**Reach:** Future frontage work must distinguish plot membership from the visible boundary of a yard. This check does not promise every part of the polygon remains on one side.

**Verdict:** sound for the present plot contract. **Confidence:** medium.

### Native road geometry separates yards; the surrounding meadow remains radial

**Choice:** A stroked road blocks the ownership line at its centreline. Polygon paving blocks it at any exported triangle edge, including edges hidden inside the combined road surface. Both are the map's native geometry. The broader meadow around buildings remains based on distance alone, so a road does not turn an unbuilt town block or park into crops.

**Gap:** Combined paving boundary edges omit some road/sidewalk joins and cannot alone identify a carriageway crossing. The plan did not specify which presentation export supplies this test.

**Reach:** Tagged road aprons participate; stroke and polygon touching cases have different geometric boundaries. Changing that policy requires explicit frontage evidence.

**Verdict:** sound. **Confidence:** medium for the representation boundary, high for preserving undrilled commons.

### Yard ownership scans static surface records at initialization

**Choice:** When a nearby building might claim a plot, scan the map's surface records and reject distant road edges by their bounding boxes before testing intersections. There is no new spatial index and no per-frame work. An index would add another preparation structure and its memory/lifetime owner.

**Gap:** The plan did not select an indexing strategy for this presentation-only classification.

**Reach:** Full-map preparation cost must remain within the existing startup budget; if this scan is material there, use the existing surface owner rather than create a parallel geometry model.

**Verdict:** sound as the simpler provisional choice, with full-map cost still to be measured at integration. **Confidence:** medium.

## Long-move admission pickup

### Long routes use physical checkpoints instead of driving every empty metre

**Choice:** When a player orders a long move, preview runs on a private copy of the side's known world. It drives away from the starting position, places that private unit farther along its planned route, and drives the final approach. It also drives approaches to stationary hulls and intersecting props. Only this rehearsal copy moves by that jump; the real unit still travels every metre. The alternative is to run the whole journey before showing a destination marker.

**Gap:** The WIP chose bounded departure/arrival rehearsal without the owner's answer; the original physical-marker contract did not choose this tradeoff or a rehearsal distance.

**Reach:** Preview's evidence is now checkpoints plus route geometry. It does not independently replay every steering turn in empty stretches. The shipped 60 m distance is fixture data, and full rehearsal can be restored by removing the carry policy. C54 must continue to check actual arrivals through current-layout bottlenecks.

**Verdict:** needs-user for the evidence/cost tradeoff. Recommended provisional call: retain the checkpointed policy while completing the broader physical gate; reverse it if an accepted route fails actual arrival. The known skipped-shove failures were corrected, rather than accepted as part of that tradeoff. **Confidence:** low.

### Rehearsal skips no demonstrated physical shove

**Choice:** A crate may be light enough for a truck to push, yet be trapped against another crate. Route search permits the lighter body; preview must drive the encounter to discover that the game's no-chain-shove rule prevents passage. A straight hull sweep selects encountered props, and the existing segment clip conservatively locates first contact. This also handles the near end of a long diagonal body whose centre lies well beyond the crossing. Roadside props outside the hull sweep do not force this rehearsal.

**Gap:** The WIP only checkpointed stationary vehicles; it treated intermediate props as a route-search concern.

**Reach:** Movement remains the owner of shoves and their effects. The sweep/clip selects where to rehearse; it does not invent another shove solver or certify the footprint of a wheeled turn.

**Verdict:** sound for the demonstrated straight-stretch contact contract. **Confidence:** high.

### A leg becomes long at activation; actual displacement earns each jump

**Choice:** A 500 m destination reached by a long detour still gets full rehearsal. A destination farther than the existing long-leg threshold is classified before planning and keeps that decision across replans. Before each private jump, the unit must move away from its previous rehearsal origin by the configured distance. Shortening a route through replanning, shuffling locally, or taking the private jump cannot earn that distance.

**Gap:** The WIP used remaining-route reduction and classified by total route length, while the live long-road policy classifies endpoint distance.

**Reach:** Preview and live long-leg classification agree; queued legs are reconsidered when they activate. The private origin is temporary bookkeeping, not a new persistent movement state.

**Verdict:** sound. **Confidence:** high.

### A blocked member has its own repeated-planning share

**Choice:** In a mixed group, a vehicle blocked by a stationary hull can repeatedly search while an unobstructed squad is ready to walk. Initial long routes use the live planner's separate search bound; shorter initial routes spend the existing rehearsal allowance. Repeated searches get one share per mover and reserve one share for movement. A member that spends its share stops in the rehearsal; the remaining members keep their chance to prove arrival.

**Gap:** Charging every search to one pot let a permanently blocked member consume the whole group's proof. Exempting every initial route changed previously admitted short village orders.

**Reach:** Admission reads per-request accounting from the existing route planner. Scheduling and persistent digests stay unchanged; the accounting is discarded each tick. This policy may refuse a member that a larger allowance could prove, as the existing bounded admission contract already permits.

**Verdict:** sound, with the equal-share allocation a provisional cost policy. **Confidence:** medium.

## C61 catalogue listings

### Keep fixed saved sources separate from generated and synthetic worlds

**When:** C61. **Choice:** A fixture declares the catalogue id of its default saved world, or `map: null` when it has no fixed saved world. Opening the movement lab therefore resolves the registry's saved map. Opening a generated battle still asks the existing map source to generate or acquire the selected world; opening a model tool still constructs its synthetic ground. Null does not mean no drawn geometry. **Gap:** C61's original roster assumed every drawn world was a saved map, while M29 preserves generated battles and developer tools. **Reach:** Future fixtures name their source once, without inventing catalogue folders for worlds owned by a generator or a tool. **Verdict:** sound; it preserves existing acquisition and makes saved-route ownership explicit. **Confidence:** high.

### Inspect saved maps through the existing geometry fixture

**When:** C61. **Choice:** A developer opening a newly saved map uses `/lab/geometry?map=<id>`: the existing geometry viewer resolves that map, names it from metadata and frames its extent. It retains the same picking and rendering controls as the default geometry lab. The developer index lists every nonretired map, including drafts; the player menu continues to offer only released playable maps with its configured encounter. **Gap:** C61 requires generic inspection but does not choose whether it shares the existing geometry viewer or introduces a second viewer. **Reach:** Saving another map adds inspection automatically, without a new route, scene identity or duplicate rendering path. **Verdict:** sound; one viewer owns saved-map inspection and preserves existing default scene coverage. **Confidence:** high.

### Keep large resolved maps out of traced component props

**When:** C61. **Choice:** Loading still returns the same resolved map, but the loading/refusal renderer is a plain function and the geometry component receives a reader that captures the map. For a large saved battlefield, React's development profiler therefore sees a function reference instead of copying thousands of terrain and prop entries into each changed-prop record. **Gap:** Reusing the small geometry lab for full-sized saved maps exposes a data boundary that the small fixture could not stress. **Reach:** Saved-map acquisition and diagnostics stay under their existing owners, while the scene runner keeps its profiler-size guard. **Verdict:** sound; it removes the unnecessary data-bearing loading component and avoids a second acquisition or rendering path. **Confidence:** high.

## Kerbside integration and individual corridor recovery

### Sample free road lines before searching around a blockage

**Choice:** A vehicle meeting parked bodies tries its normal lane plus six evenly spaced lateral lines. It prefers a gentle merge 32 m ahead, then 16 m and 8 m, holding its chosen line through a row before returning. A larger gentle shift can therefore win over a smaller sharp one. **Gap:** C46 did not select a lane-search algorithm. **Reach:** This bounded deterministic search retains the existing grid-search fallback; it is not a proof that every possible narrow gap is found. **Verdict:** sound within demonstrated traversal scope; finer or continuous interval search is unnecessary until a real usable gap is missed. **Confidence:** low for the exact sampling/reach numbers, medium for the policy.

### Refine vehicle segments against the side's existing known bodies

**Choice:** A coarse cell touched by a parked car may still hold a clear vehicle line. Segment checks consult the navigation owner's actual known footprints, including that side's moved/removed beliefs, while destinations and cell searches retain conservative admission. Terrain and map edges still constrain the whole navigation width. **Gap:** The placement plan did not choose how to resolve coarse false blockage. **Reach:** A physically clear destination may remain conservatively unavailable, and navigation still plans width rather than all rectangular-hull orientations; movement proves actual turning/collision. No second world oracle or hidden-state cache is introduced. **Verdict:** sound for this boundary. **Confidence:** high.

### Traffic detours do not move roadside cover

**Choice:** A tank going around another live vehicle looks for clear width or another route without pushing static bodies, even though its ordinary journey may push a light car when no clear line works. **Gap:** C46 did not choose whether traffic avoidance may rearrange parked cover. **Reach:** Traffic yielding can prefer waiting/detouring over a mechanically possible shove. **Verdict:** needs-user for this gameplay tradeoff; provisionally retain the rule because passing a colleague should not casually rearrange the street. It is reversible by restoring ordinary pushing capability to traffic replans, with the traffic/body gates rerun. **Confidence:** medium.

### Yield dense queries under the existing work counter

**Choice:** Each road-line candidate and exact-body query keeps its cursor between advances. Body queries read eight raw slots per charged work unit; segments stop near the existing 64-work quantum. All retained progress enters the battle digest. **Gap:** The old synchronous query could exceed the existing planning overrun limit; C46 did not prescribe a work weighting for the new refinement. **Reach:** The job completes incrementally without increasing allowances or hiding scans. The weighting establishes deterministic accounting, not instruction-cost equivalence. **Verdict:** sound for bounded work; instruction calibration remains part of the cost gate. **Confidence:** medium for weighting, high for yielding and state identity. The enlarged driving state is boxed once per road journey; it adds no public schema or dependency.

### Keep clearance waypoints until reached or passed abeam

**Choice:** Wheeled steering retains a nearby route corner instead of dropping it early to begin a smooth turn. A stopped colleague may require that lateral waypoint for clearance. **Gap:** The earlier shortcut did not distinguish a useful clearance corner from a disposable fillet. **Reach:** The movement owner still steers at its turning radius; some open turns may start later or look less smooth. **Verdict:** sound; deleting the unsafe shortcut is simpler than another turn-clearance oracle or traffic-only exception. Broad visual smoothness remains unmeasured. **Confidence:** medium.

### Reserve the largest current facade fit in centre parcels

**Choice:** Both affected centre presets reserve 1.5 m side space even when a selected house's art allowance is smaller. Two adjacent parcels can therefore hold the largest currently accepted facade overhang without the parcel generator importing appearance metadata. **Gap:** C53 did not choose template-specific art setbacks. **Reach:** Current kits fit conservatively, at a density cost; Mixed Small seed 1 loses 145 buildings. A future larger fit needs a new explicit preset/geometry decision. **Verdict:** sound for current art, with the uniform density tradeoff kept reversible. **Confidence:** medium.

### A stranded soldier recovers independently of his advancing squad

**Choice:** If one soldier makes no physical progress, he uses the existing local fine planner and nearby live soldier discs to reach his own path endpoint or a standing rejoin point on the squad's corridor. Seven advancing squadmates cannot hide his jam. The existing one-second attempt throttle covers unsuccessful target selection too. **Gap:** Earlier recovery applied only on the final approach; aggregate squad progress masked a member stranded beside idle infantry. **Reach:** No new timer, radius, state field or collision exception is added. This repairs demonstrated stationary jams; slow or moving congestion remains distinct. **Verdict:** sound; the individual physical-progress owner should decide his recovery. **Confidence:** high.

## Global country coverage scope

### Preserve clear approaches while enforcing the circular-sight requirement everywhere

**When:** 2026-10-02 maintenance and global-coverage pickup. **Choice:** At every playable location, nearby features must interrupt circular ground-level sight, using actual circular observers rather than the old standardized infantry range. A jeep's shorter circle therefore counts. Narrow directional vehicle sight is still reported, but an obstruction at every heading is not promised: a supply truck looking along the middle of a required 400 m-wide clear firing lane can have its whole forward view inside that lane. **Gap:** The user explicitly required interesting surroundings everywhere and no empty circular fog, but did not specify every heading of directional observers. That stronger interpretation conflicts with the accepted clear-lane geometry. **Reach:** Global location coverage stays binding; a future every-heading requirement would need an explicit change to clear lanes or physical cover inside them. **Verdict:** needs-user for interpretation; provisionally retain the literal circular-sight scope and the accepted clear approaches. It is reversible by changing those contracts together rather than quietly claiming the narrower lobes pass. **Confidence:** medium.

### Compose the existing reports for C54

**When:** 2026-10-02 maintenance. **Choice:** A release review combines the generator's admission/identity report, layout composition measurement and the simulation's placement, geometry, movement and sight reports. No new validation CLI or parallel geometry module is required. For example, a road-time estimate answers a layout question; an executed battle answers whether the vehicle actually arrives. Keeping those outputs distinct prevents the first being treated as proof of the second. **Gap:** The original gate named a nonexistent validation command without specifying why another owner was needed. **Reach:** Existing owners retain their contracts and command schemas; C54 owns the retained evidence matrix and final verdict. **Verdict:** sound; composition supplies the required evidence without a second generator or physical oracle. **Confidence:** high.

## Shared physical generation inputs

### Pin the complete selected physical input sections

**When:** global coverage prerequisite, 2026-10-02. **Choice:** A generated result identifies the complete resolved unit/prop catalog, forest rules and ground eye/target heights alongside its plan. Even a name or combat column inside that catalog changes the configuration identity; unrelated top-level weapon/balance sections do not. The geometry hash still identifies geometry alone. **Gap:** The earlier configuration identity covered only the plan, although forest placement and observer ranges depend on additional inputs. **Reach:** The same geometric map can have a new input identity when its rules change. Keeping a minimal catalog projection would require a second list of which columns count as physical; the whole existing catalog avoids that drift. **Verdict:** sound; conservative provenance is preferable to an unowned projection, with its broader invalidation scope explicit. **Confidence:** medium.

### Share the actual forest placement and random sequence

**When:** global coverage prerequisite, 2026-10-02. **Choice:** The contract owns the existing forest seed, jittered candidates and exclusion order. The simulation supplies its indexed road, water, body and ground queries; generation can consume the same candidate owner for its certificate. Both use one deterministic random-number implementation, while the generator retains its named seed separation. **Gap:** Authored woodland outlines did not prove which trunks actually stood, and copying the simulation's placement would create another authority. **Reach:** Future forest placement changes have one owner. No simulation import, second tree algorithm or compatibility shim remains. **Verdict:** sound; shared construction allows physical witnesses while preserving the established sequence. **Confidence:** high.

### Require existing mechanics explicitly at generation entry points

**When:** global coverage prerequisite, 2026-10-02. **Choice:** The native generator receives the existing game rules and resolved catalog files explicitly; the browser supplies the resolved rules it already prepares the battle with. A caller supplying only catalog rows is refused rather than receiving hidden forest/eye defaults. **Gap:** The old catalog-only input could not establish sight coverage for the battle's actual physical rules. **Reach:** Native and browser callers agree on those inputs without a second settings file or an implicit sibling-file lookup. Direct authored-plan compilation retains its independent API. **Verdict:** sound; the caller names the physical environment it intends to play. **Confidence:** high.

### Retain each route at its own planning checkpoint

**When:** movement integration triage, 2026-10-02. **Choice:** The gap scene waits for each commanded squad to finish planning and keeps the route returned at that moment. The first squad may already travel while the second still plans; reading both routes afterward could miss the route it has consumed. The scene separately checks that both squads actually reach their own admitted destinations. **Gap:** The scene assumed admission and route publication happened together. **Reach:** This changes verification timing only; movement admission, routes and arrival rules remain the simulation's. **Verdict:** sound; the existing bounded wait observes the required state without relaxing the gap traversal. **Confidence:** high.

### Bind controlled movement checks to the admitted actors and markers

**When:** movement integration triage, 2026-10-02. **Choice:** Both checkpoints require a nonempty command acknowledgement, exactly its actor count, every admitted actor ID and that actor's goal or final position. Coordinates use the float32 precision the observation publishes. Otherwise, a missing squad could make an all-items check pass, or reaching some point beyond the wall could count as arrival at the marker. **Gap:** The old check trusted the kind-filtered actor list and did not bind arrival to each acknowledged marker. **Reach:** This tightens one controlled fixture without inventing a general squad-centroid arrival tolerance. **Verdict:** sound; the check proves the commanded actors completed the commanded moves. **Confidence:** high.

### Bound the transport comparison to its controlled workload

**When:** lossless kit transport, 2026-10-02. **Choice:** Compare raw and compressed downloads once with fresh browsers on the same engine, map and art. After concurrent weapon changes, run a fresh functional startup without treating it as the second half of that pair. **Gap:** The transport pass needed evidence of decoding cost while unrelated work continued. **Reach:** The measured instruction and footprint differences belong to that controlled pair; final whole-system startup and memory admission remain open. **Verdict:** sound; matched inputs isolate transport, while a functional check proves current integration. **Confidence:** medium; one sampled pair establishes a limited comparison, not a general performance bound.

### Give compressed bytes their own address while retaining art identity

**When:** lossless kit transport, 2026-10-02. **Choice:** A kit or template library has an encoded file hash and its original decoded content hash. A browser requests the encoded address, verifies it, inflates it and verifies the original art before installation. Otherwise, reusing an old raw URL could serve a cached representation with different bytes. All kit/library consumers migrate together, and the old raw runtime copies are removed; identified unit, scenery and skeleton readers stay raw. **Gap:** The spec required smaller shared downloads without choosing transport or content-address semantics. **Reach:** Changing compression can change encoded addresses without changing art, templates or physical map identity. **Verdict:** sound; separate addresses preserve immutable URLs and exact art identity with one retained representation per object. **Confidence:** high.

### Require transport records only for consumers that need them

**When:** lossless kit transport, 2026-10-02. **Choice:** The appearance catalog may omit its gzip table for an identified non-kit preview or bake. Every requested kit and template library requires its own record; absence is a refusal, never a raw fallback. **Gap:** Non-kit-only producers legitimately publish raw appearances without city art. **Reach:** Those producers keep their small contract, while city consumers cannot silently bypass the new transport. **Verdict:** sound; optional table ownership follows actual consumers rather than preserving a second loading path. **Confidence:** high.

### Use platform compression with one bounded verification owner

**When:** lossless kit transport, 2026-10-02. **Choice:** The baker, browser and native tool share the platform gzip stream contract. Decoding allocates the declared raw length, cancels on an overflowing chunk, and verifies exact length and both hashes before the original codec reads it. Platform encoders can produce different encoded bytes across build runtimes, which receive different addresses. **Gap:** Neither a compression dependency nor an inflation policy was specified. **Reach:** There is one transport owner and no second art codec. Per-kit decoded admission stays binding; no arbitrary library or GPU cap is substituted for their separate budget work. **Verdict:** sound; existing platform support supplies lossless encoding while shared verification makes failures explicit. **Confidence:** high.

### Admit the complete selected download before payload IO

**When:** lossless kit transport, 2026-10-02. **Choice:** The browser and native map-download tool count the library plus each unique selected kit hash before requesting kits or returning a cached result. The library alone is checked before it is read. Otherwise, cached entries or an initial library download could evade the aggregate gate. The native tool also decodes the actual selection without requiring a full art rebake. **Gap:** The budget named downloads but left runtime enforcement and targeted native verification unspecified. **Reach:** Encoded aggregate bytes, decoded per-kit bytes and resident/GPU budgets remain distinct contracts; every selection uses the same count owner. **Verdict:** sound; admission happens before the work it limits and can be verified through real consumers. **Confidence:** high.

### Retry the full kit request after catalog replacement

**When:** lossless kit transport, 2026-10-02. **Choice:** If one requested kit is already held and another is loading when the catalog is replaced, retry both names against the new catalog. Retrying only the previously absent name would lose the first kit and return an incomplete selection. **Gap:** The existing retry path treated the old cache's missing subset as the original request. **Reach:** Concurrent reload fulfills the same complete request; it adds no new loading protocol. **Verdict:** sound; the request survives a change of source, as the deterministic two-kit regression demonstrates. **Confidence:** high.

### Judge neutral boundary paint by colour and actual contrast

**When:** boundary integration triage, 2026-10-02. **Choice:** Check the stored paint's neutral colour at each boundary sample, then require that it changes the finished frame there. Grey over a yellow field can lower red and raise blue; checking only positive red/blue rises incorrectly rejected a visible dim white line. The paired-frame helper retains both finished images for signed contrast. **Gap:** The scene specified a visible neutral border but assumed neutral paint brightened every channel. **Reach:** This corrects verification without changing border style, width, geometry or rendering; absent paint and unchanged finished frames still fail. **Verdict:** sound; stored colour and composited contrast establish the two required properties independently. **Confidence:** high.

### Stage diagnostic flight against current physical rules

**When:** lab integration triage, 2026-10-02. **Choice:** A low grenade arc now starts nearer the existing ridge, and moving-body starts use the owner's nominal solved flight time while retaining their speeds and shapes. Spread changes the projectile without moving the diagnostic actors. Otherwise, the old coordinates would test a crest the round clears and a tank that has already left its path. **Gap:** Fixture coordinates encoded obsolete flight timing. **Reach:** Diagnostic staging changes; weapon coefficients and gameplay flight do not. **Verdict:** sound; the fixtures again demonstrate the original blocked-arc and swept-collision contracts. **Confidence:** high.

### Give the hidden weapons diagnostic a reachable physical screen

**When:** lab integration triage, 2026-10-02. **Choice:** A nearby firing emitter sits behind a low wall, and the blue observer stands where the identified tank is within both weapon mounts' reach. A distant hidden report cannot demonstrate a 150 m grenade's area fire, and rifle reach alone cannot establish the identified control's eligibility for every mount. **Gap:** The fixture needed nearby hidden evidence after weapon tuning. **Reach:** The authored diagnostic map and encounter identity change; no targeting or concealment rules change. **Verdict:** sound; range and source controls prevent an ineligible target from making the experiment pass. **Confidence:** medium; the wall is a chosen diagnostic arrangement rather than a product requirement.

### Keep diagnostic feeds reachable inside the viewport

**When:** lab-panel integration, 2026-10-02. **Choice:** The existing lab panel uses its own bounded scrolling; its feed retains one pixel below the last row so fractional scroll endpoints cannot clip it. The scene scrolls every actual feed row into view at desktop and smaller viewports, requiring full containment. Otherwise, a valid simulation event can remain unreadable below the panel. **Gap:** The lab layout did not specify overflow behavior. **Reach:** All consumers of the existing lab-panel style inherit its viewport bound; no new panel or navigation model is introduced. **Verdict:** sound; the existing surface remains readable with strict checks and no tolerance relaxation. **Confidence:** high.

### Capture crossing interaction from authoritative event times

**When:** lab-panel integration, 2026-10-02. **Choice:** The scene runs the diagnostic once to obtain the actual near-miss and impact ticks, resets it, then captures those moments. Otherwise, fixed historical ticks show bodies before the current slower projectile reaches them. **Gap:** Capture timing was tied to old tuning. **Reach:** The scene consumes the flight owner's events rather than adding a second trajectory estimate; all original collision assertions remain. **Verdict:** sound; screenshots show the interaction the test judges. **Confidence:** high.

### Admit a contact-capture approach before waiting for evidence

**When:** village-watch integration, 2026-10-02. **Choice:** The controlled tour approaches from open ground west of town and checks that the acknowledgement admits some observers before waiting for reports. The former centre destination refused every actor, leaving them at their starts; waiting longer could not repair that rejected command. Five of nine actors are admitted by the new approach, which is enough for this report/anchor fixture; it is not an all-actor arrival claim. **Gap:** The tour assumed a destination in town was admitted. **Reach:** Verification staging changes only, with the existing report preference, knowledge and anchoring contracts retained. **Verdict:** sound; it observes actual contacts rather than hiding refusal or inventing reports. **Confidence:** high.

### Measure central city contact on the complete generated world

**When:** C05 browser benchmark, 2026-10-02. **Choice:** The named city workload uses Metro Large seed 4 and the existing early city arena with 100 units per side. It starts recording after the first seeded movement wave at tick 150. The village keeps its own comparison commander; city uses the scenario's two-sided orders. Otherwise, a village script would be applied to the wrong encounter or player infantry transit would delay the contact measurement. **Gap:** The advertised city address had no distinct browser preset. **Reach:** The workload measures explicitly synthetic local combat on a full map; player transit and late corpse/wreck load remain separate controls. **Verdict:** sound; the existing workload authority is reused without changing player behavior. **Confidence:** high.

### Resolve a benchmark camera policy from the prepared map

**When:** C05 browser benchmark, 2026-10-02. **Choice:** A generated preset names its tour policy; after preparation, the actual returned extent supplies the city centre and overview. Reports carry that resolved tour and the checked request/map identity. Otherwise, hardcoded coordinates could silently measure an obsolete extent, or village coordinates could show the wrong battlefield. **Gap:** Generated and saved benchmarks did not have distinct preparation and camera contracts. **Reach:** The existing recorder owns all reports, and source/tour combinations are constrained together; there is no second generator or recording loop. **Verdict:** sound; extent has one authority. **Confidence:** medium; the fixed tour is a chosen representative composition whose full visual gate remains binding.

### Reuse and cancel the preparation worker at the existing session boundary

**When:** C05 browser benchmark and generated endurance, 2026-10-02. **Choice:** Prepared generated workloads pass their existing connection into BattleView, and leaving preparation cancels the same worker. Otherwise, endurance starts a second worker and abandons the first, or a late preparation answer can start an obsolete run. Ordinary saved workloads keep their existing new-worker path. **Gap:** A generated route omitted the prepared connection, while the new city route needed the same lifetime ownership. **Reach:** One worker owns each authoritative generated battle; no new worker manager or lifecycle state is added. **Verdict:** sound; real route/session and cancellation regressions demonstrate ownership. **Confidence:** high.

### Preserve the existing secondary benchmark-link convention

**When:** C05 menu review, 2026-10-02. **Choice:** The footer link inherits the footer colour and keeps its name on one line. Its small secondary target remains, measured at about 62 × 15 CSS pixels; the menu has no stated minimum and its developer navigation uses similarly small text. Otherwise, an unrelated target-size policy would widen this functional benchmark pass. **Gap:** Fresh review found an unreadable browser-default link and a minor tap-area concern. **Reach:** Readability is corrected at the existing menu owner; touch ergonomics remain a disclosed limitation. **Verdict:** sound for this bounded pass. **Confidence:** medium; the retained small target is a product convention rather than an accessibility acceptance claim.

### Keep one opaque report owner across visual retirement

**When:** current contact amendment, 2026-10-02. **Choice:** Each side lazily gives an observed firing/sighting cause one opaque report slot. Its existing collection distinguishes live evidence from an inactive handle; the same cause's fresh evidence updates that slot. Presentation can therefore reuse its existing three-second fade and overwrite a returning owner instead of drawing an old fading area beside a new area. A separate cause-to-ID map or authority copy of the presentation timer is unnecessary. **Gap:** The user requires one report per unit, consistent labels/borders and the existing fade, without selecting identity storage. **Reach:** A fresh report can remain correlated with the same uncertain cause even after old evidence expires; no actual enemy ID, unseen type or hidden position is published. Removal ends old targeting/return-fire intent, so a reused handle cannot resurrect it. **Verdict:** sound; actual lifecycle, pending intent, positive aim and independent identification-grace regressions pass, complete Native/Wasm state agrees, and deletion review retains one owner. **Confidence:** medium; enduring opaque cause continuity is a consequence of choosing one owner without a second retirement timer.

### Resolve the shader warning without changing the grass algorithm

**When:** approved grass shading checkpoint, 2026-10-02. **Choice:** Rephrase six raw WGSL comments containing an ordinary English token that TypeGPU mistakes for a named builtin external. The same route goes from six warnings to zero without changing shader math. Otherwise, accepting the pictures would leave an unexplained shader warning, while a parser or renderer replacement would widen a one-setting pass. **Gap:** The research captures exposed a dependency scanner defect. **Reach:** This narrowly removes false warnings in the affected current shaders; no new workaround layer or resources are introduced. **Verdict:** sound; actual warning red/green and unchanged shader logic support the smallest correction. **Confidence:** high.

### Verify village play through the current presentation and command contracts

**When:** focused village play triage, 2026-10-02. **Choice:** At far zoom, the tour requires visible compact cards clear of the HUD, allowing their documented depth-ordered overlap. For the sound approach it waits for that command's acknowledgement and requires an admitted destination before advancing toward heard cues. Restart waits through the route's temporary absence while it rebuilds. Otherwise, the tour rejects intentional card behavior, mistakes an earlier move's acknowledgement for the new approach, or fails during a valid remount. **Gap:** The historical tour assumed different card behavior and synchronous command/restart settlement. **Reach:** These are verification corrections; player behavior, planning budgets and the original sound/replay/pause checks are preserved. **Verdict:** sound; the existing contract and actual settled commands justify the corrections, with falsifying guard controls retained. **Confidence:** high.

### Reject ground detail that adds code without visible continuity

**When:** C85 bounded ground research, 2026-10-02. **Choice:** Keep the approved grass lighting and restore the ground shader after two finest-grain leaf-mark trials. The final matched candidate changes pixels, but enlarged gaps remain smooth and gameplay grass still reads as a mottled surface; a fresh critic sees no convincing improvement. Otherwise, the renderer would inherit extra profile/filter logic without solving the requested blending. **Gap:** The user requested independent investigation, without selecting a ground-texture technique. **Reach:** The source and complete evidence remain available for research, while production acquires no new texture owner, resource or shader machinery. The next hypothesis must address visible ground structure, retaining the liked yellow variation. **Verdict:** sound; the bounded comparison rejects an ineffective mechanism without declaring the larger visual problem solved. **Confidence:** high.

### End expired contact intent before accepting new evidence

**When:** contact consolidation, 2026-10-02. **Choice:** Expiry ends queued area attacks and acquired aim at tick start, then the existing attacker-pruning rule checks whether another valid track still grants return-fire permission. If a fresh shot renews the same opaque report later that tick, it can earn new aim but cannot revive the old order. Otherwise, stable picture identity would also preserve old combat intent. **Gap:** The required one-report/shared-fade contract did not select the tick phase for retirement. **Reach:** Report identity remains presentation continuity; commands and engagement permission still follow current knowledge, including independent identification grace. **Verdict:** sound; real expired-tick firing, positive old aim and short-lifetime/grace tests distinguish the retained consumers from redundant cleanup. **Confidence:** high.

### Reuse the player's contact presentation in its lab

**When:** contact consolidation, 2026-10-02. **Choice:** The contacts lab runs its original saved scenario, seed and camera through BattleView, using the player's labels and removal fade. Its separate legend, report list and view/session path are deleted. Otherwise, the lab would need a second implementation of the same visual contract. **Gap:** The lab's raw diagnostic rendering did not show the required labels or fade. **Reach:** The lab also gains the normal playable-border paint; its paused suppression pair confirms that visible difference. Actual raw report callers in other diagnostics remain an explicit next pass. **Verdict:** sound; existing components provide the contract with less code, and the original lab behavior checks remain green. **Confidence:** high.


### Describe wider landscape through one map-owned margin

**When:** bounded surroundings integration, 2026-10-02. **Choice:** One optional visual margin derives common playable, physical and rendered rectangles. Physical bounds describe the existing rounded height grid; they do not silently change to nominal bounds. Existing plot and backdrop-tree placement consume the rendered rectangle, while the generic horizon backdrop retains its actual mesh cutout. **Gap:** The user requires wider rendered surroundings without choosing metadata shape or a physical outer ring. **Reach:** A margin adds display scenery only, and zero preserves authored arenas. The initial 1,500 m player preset is a reasonable default for later tuning. Changing the plot region also changes decorative fields inside the map, requiring a new map identity. No additional geometry grid, renderer pass or tree generator is owned. **Verdict:** sound; matched production views and authored-arena pixel/geometry differentials support the seam. Far overview still reveals a finite detailed-field rectangle, so this is contract acceptance rather than an ideal landscape claim. **Confidence:** medium for the initial visual margin; high for common extent authority.


### Make actual report callers consume one presented contact collection

**When:** contact consumer closeout, 2026-10-02. **Choice:** The battle session owns the existing contact presentation, and the player view plus weapons/fog-look diagnostics receive its presented collection directly. Their shared glyph and existing info labels use the same report life; a rim’s opacity is independent of decorative hatch ink, and the visible title inherits the existing black text halo. **Gap:** The required common label/border/fade did not choose the owner shared by the actual diagnostic callers. **Reach:** No fallback from raw observations, diagnostic fade timer or new label component remains. Static material specimens still inspect geometry, not evidence lifecycle. **Verdict:** sound; actual route component regressions and complete real-worker before/after lifecycle frames prove the consumer boundary and clear endpoints. **Confidence:** high.


### Judge the timed city benchmark by current playability and late contact

**When:** final browser gate correction, 2026-10-02. **Choice:** A requested full benchmark must record its complete five-minute window at at least 30 average FPS. Its existing polling also requires advancing own-unit shot counters in the opening and final minute, using the endurance check’s ten-unit threshold. Startup is measured to the existing playable endpoint under the user-selected 30 seconds. **Gap:** The scene previously printed frame cost and accepted any earlier round in flight, which could admit a slow run or a fight that had already ended. **Reach:** This changes verification only, with one existing polling owner and no simulation, renderer, timer or resource changes. Short runs remain scoped diagnostics. **Verdict:** sound; actual scene entry-point negative controls reject incomplete/slow runs and opening-only contact. Live measurements on final maps remain required. **Confidence:** medium for the borrowed multi-unit contact threshold; high for the explicitly selected time/frame contracts.


### Preserve the selected approach dimensions without retuning weapons

**When:** final playability evidence reconciliation, 2026-10-02. **Choice:** Keep the user-selected 1,800 × 400 m clear approach and prove executed combat with the current production weapon ranges. Their maximum is 900 m. The later pickup prompt’s additional demand for an actual 1,800 m engagement is corrected as a plan assumption, rather than changing game rules to meet it. **Gap:** The map brief’s rationale assumed weapons reached 1,800 m; its selected contract specifies clear geometry, while M04 preserves weapon ranges and the user now defers numeric tuning. **Reach:** No approach, physical cover, weapon range or elevation changes. The later workbench owns generation tuning; weapon tuning remains with the existing mechanics owner. **Verdict:** sound; the original generation contract and unchanged production rules settle the apparent contradiction. **Confidence:** high for preserving both existing owners.


### Reject solid-hidden sight before integrating leaves

**When:** bounded playable closeout cost pass, 2026-10-02. **Choice:** Check existing solid line of sight before foliage depth in the same sensing query. A wall-blocked ray returns the same false answer without integrating irrelevant leaves behind it. **Gap:** The full city workload misses the playable frame gate; the user permits obvious corrections while deferring broad optimization. **Reach:** No rule number, query owner, cache or additional state changes. The isolated public-query cost regression and paired 1,800-tick battle preserve answers and digest. **Verdict:** sound; instruction reduction is measured separately from still-pending browser performance. **Confidence:** high.


### Fly the declared benchmark tour clear of buildings

**When:** current city benchmark correction, 2026-10-02. **Choice:** Raise the five near city orbit distances so the requested trajectory stays outside roof clearance, and name city tour/preset v3. Targets, headings, pitches and phase purposes remain. **Gap:** The former camera data requested a path through real generated towers; the safe camera’s correction made its drawn view differ from the report’s declared trajectory. **Reach:** Close/ground framing materially changes, so older v2 frame cost cannot be treated as a matching comparison. No player camera limit or drift gate is relaxed. **Verdict:** sound for collision-free trajectory; actual generated-world tests reject the old paths at both run lengths/aspects. Browser subject readability and timed performance remain separate admission. **Confidence:** medium for the initial framing distances, deliberately not tuned for an ideal shot.


### Preserve rear roles when concentrating synthetic city contact

**When:** bounded city stress staging correction, 2026-10-02. **Choice:** Apply the existing inward concentration only to the initial fighting line in the recipe owner. Reserves and logistics retain their rear deployment, supply goals and later wave orders. Name the changed workload `city-arena-2` / benchmark preset v4; the exact camera tour v3 and generated-map identity remain. **Gap:** Concentrating every living start contradicted the recipe’s separate rear roles. **Reach:** No gameplay number, weapon rule, roster, RNG draw or order cadence changes; default endurance passes zero inward displacement. The native and actual Wasm consumer regressions reject the former reserve placement. **Verdict:** sound staging correction, not sustained-contact admission: paired current-rule seed-4 native runs on the same complete Large map still produce only two blue firing units in the final minute versus one before. Trucks retain unused stock and reserves remain alive, so this correction does not make the heavy-contact gate pass. **Confidence:** high for preserving the declared roles; no performance claim.


### Apply authored formation orders to surviving own actors

**When:** bounded stress command diagnosis, 2026-10-02. **Choice:** At a scripted order's application tick, remove only known fallen own actors before the existing command preparation. Unknown and foreign ids remain for strict refusal; an all-fallen group is an ordinary empty no-op. Player and network commands still validate the exact submitted selection. **Gap:** Fixed script groups included fallen members, causing one casualty to refuse every survivor and later reserve in that group. **Reach:** This is an intentional engine and scripted-battle digest change across authored scenarios, including ordinary endurance, rather than a city-only policy. There is no new state, schema, actor list, rule number or order owner. **Verdict:** sound; a survivor physically arrives after the formerly refused group move, every live/replay tick agrees, invalid-actor negative controls reject permissive filtering, and public destroyed-actor validation remains. On the pinned current-rule city workload, actual accepted movement and turnover recover, but the final simulated minute has eight blue firing units, below the existing ten-unit check. No sustained-contact or frame-cost admission is claimed. **Confidence:** high for command correctness.
