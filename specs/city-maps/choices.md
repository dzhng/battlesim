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

### Preparation has a worker of its own, closed after its one answer

**Choice:** `/lab/generated` asks a preparation worker for a battle; the worker generates the map, lays the encounter on it, returns the scenario JSON and is closed. The battle then starts in the usual battle worker from that scenario.

**Gap:** C55 says "the pre-battle preparation worker"; C33 says "the existing preparation worker". No preparation worker existed, only the battle authority's.

**Verdict:** Sound. Closing the worker frees everything generation allocated (93 MiB of Wasm memory on Metro Large) and is the cancel: a request that is no longer wanted cannot start a battle. Restart reuses the scenario, so it never generates again. The cost is one copy of the scenario text (27 MB on Metro Large) to the page and one to the battle worker. **Confidence:** High.

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

## C05 — scale-lane measurement choices

### Sound — medium confidence: separate contact density from transit

**When:** scale-lane measurement pass. **Choice:** reuse the endurance stress
recipe inside a central 3 × 2 km arena of the generated map, with the full map
loaded. On a 10 km world the original recipe would send each side toward its own
rear, leaving them kilometres apart; the new load brings both into the same city
area, with the fronts within weapon reach. Starting units move to nearby ground the production navigation permits.
The existing saved-map endurance recipe keeps its coordinates and identity.
**Gap:** the lane required generated contact without specifying its fight script.
**Reach:** this proves local street cost and complete-world allocation; the separate
crossing load proves long transit, and neither proves encounter placement quality.
**Verdict:** sound; workload scopes stay explicit rather than using a quiet journey
as fighting evidence.

### Sound — high confidence: profile the existing tick with external counters

**When:** scale-lane measurement pass. **Choice:** reports receive callbacks when
each system finishes and read the OS instruction counter there. Ordinary play
calls that same tick path with a no-op callback, so it never reads clocks or
counters. If an eye sweeps fog and then learns terrain, those costs are reported
separately; building the observation and packing it for delivery are separate too.
**Gap:** existing reports counted complete ticks and could not select the expensive
owner. **Reach:** future native reports share the production path without a second
tick implementation. **Verdict:** sound; measurements remain outside battle state
and replay identity.

## C06 — scale-lane termination pass

### Sound — high confidence: exhaust the corridor, then use the existing fallback

**When:** C06 termination pass. **Choice:** stop a soldier's rejoin search once
it has sampled the remaining route. If parked bodies block a nearby route end,
the former sampler kept returning that same blocked place forever; the revised
search returns no rejoin point and lets the existing personal-spot fallback run.
The limit comes from route length and sampling distance, with one endpoint
rounding sample, rather than an arbitrary retry cap. **Gap:** the plan required
bounded scale work but did not name this infinite iteration. **Reach:** all
soldiers retain the same successful candidate order and can exhaust an obstructed
route without freezing a tick. **Verdict:** sound; the search cannot create
progress after it reaches the end, and no valid candidate is discarded.

The boolean-only walkability call is a local ownership simplification, not a new
physical rule or a measured performance gain.

## C07: exact observation group delivery

### Sound — medium confidence: retain one canonical non-map baseline

**When:** C07 reconstruction pass.

**Choice:** Compare float words within each existing observation group, rather
than introducing entity-keyed stores. When a squad loses a soldier, its member
rows become shorter; that can change the squad group's remaining addresses, but
cannot move the independently addressed corpse or known-prop group. The page
receives the new group length and exact replacement ranges, reconstructing the
same ordered arrays it would receive in a complete record. An entity-keyed
alternative would add identity/deletion rules for every kind of row; this work
keeps those decisions with the simulation's existing logical observation.

**Gap:** G0 selected fog/known-prop delivery but did not choose a representation
for the measured 60 KB own rows and 522 KB late corpse tail. C07 explicitly
reslices that arm while preserving the original byte target.

**Reach:** A shrinking group can still incur within-group replacement bytes.
Full-size active measurements decide whether this representation is sufficient;
this checkpoint makes no budget-success claim.

**Verdict:** Sound: one existing logical owner, exact ordering, no new entity
identity scheme. **Confidence:** Medium, because the active byte gate remains
open.

### Sound — high confidence: choose snapshots by encoded size and reserve before commit

**When:** C07 reconstruction pass.

**Choice:** If changes are dense, send the group's full words when that costs no
more than replacement ranges. For sparse changes, scan once to count the encoded
bytes and again to emit them, without retaining a list of ranges. Admit and
reserve the complete wire record and next non-map baseline before replacing the
published output or advancing its cursor. A rejected record therefore leaves the
last successful side/generation available for recovery. The baseline uses one
flat allocation, so a large group on one side followed by another large group on
the other cannot accumulate separate group-sized allocations.

**Gap:** The representation needed a snapshot threshold and bounded scratch
lifecycle; a per-range list or per-group high-water cache would make peak storage
larger than the current logical observation.

**Reach:** Producer staging, current wire, pending wire and the non-map baseline
each have an exact bounded reservation. Those four capacities and the existing
fog cursor must still be included in snapshot/peak admission measurements.

**Verdict:** Sound: admission precedes transport commit and scratch space does
not grow with the number of changed ranges. **Confidence:** High.

### Sound — high confidence: share the existing side/epoch revision boundary

**When:** C07 reconstruction pass.

**Choice:** A side change or resync starts every group with a complete snapshot.
The existing fog generation also names the group baseline. If a page misses a
publication, the next generation fails that baseline check before applying any
group, fog or ground state. A valid resync then reconstructs the side's full
known state. The alternative would introduce an independent group revision that
could disagree with fog or ground and permit a mixed observation.

**Gap:** C07 needed recovery rules for the added representation; the existing
publication cursor already supplied exactly the ordered stream boundary.

**Reach:** Producer/decoder cut over together. The published layout describes
delivery metadata separately from the canonical logical row schema, and the
complete serializer is named `pack_logical` so native oracle/report consumers
cannot mistake it for the transport record.

**Verdict:** Sound: one atomic publication baseline, exact float32 words, and no
codec dependency or new simulation state. **Confidence:** High.

## Buildings lane

### China's family ships first

**When:** S2 verdict, 2026-10-01.

**Choice:** A generated map's apartment blocks come from the China graph first. New York and Paris follow later through the same exporter as further families. The alternative was Paris first, whose Haussmann blocks sit more naturally beside a European village.

**Gap:** The spec says one regional family per map and names three sources, but not which is built first or how many are needed to close the lane.

**Reach:** The first maps have Chinese apartment blocks beside our own scripted houses, farms, towers and sheds. Shop signs use our invented names. Adding a family is a new source set, not new code.

**Verdict:** provisional. China is the only source whose blocks are free-standing with four facades at exactly 3 m a floor, which is what the generator places, and its enclosed balconies are the look the user named as the reference. Paris has fixed 3.2 m floors under a mansard and New York always has two blank sides. If the user would rather open with a European town, the order changes and nothing else does. **Confidence:** medium.

### Attached homes are our own source

**Choice:** Terraced and semi-detached houses are scripted by us, in the same source as detached houses (C17). The spec assumed the graphs would cover low attached buildings.

**Gap:** S2 measured that no graph makes a two-storey house or a mid-terrace unit with one street face.

**Verdict:** sound. A graph building squeezed to 7 m wide is a shop block with a water tower, not a house. **Confidence:** high.

### Our own sources serve every family until a family has its own

**Choice:** The scripted houses, farmsteads, towers and industrial buildings are tagged with the shipping family, so a map of that family has all six categories. The alternative, three styled variants of each before anything ships, would triple the art before the first town can be judged.

**Gap:** "Missing variants cannot fall back to unrelated regional art" does not say whether project-made buildings count as unrelated.

**Verdict:** provisional. They are styled plainly (plaster, tile, concrete) so they do not read as another region's. A second family needs its own pass over them, or an explicit decision to share. **Confidence:** medium.

### Stretched primitives are folded into each template's own mesh

**Choice:** New York's walls and trim are unit cubes stretched up to 24 times per instance, textured in the source by world-space projection. The exporter realizes rows of such generated primitives into one mesh per template with UVs in metres; real kit meshes stay shared instances. The alternatives were a world-space projection path in our shader, or one module per distinct stretch.

**Gap:** Q-H′ asked for box-projected baked UVs and L1 for instanced modules; on stretched cubes the two conflict.

**Verdict:** sound. A cube is twelve triangles, so folding costs little storage, and the shader keeps one material path. **Confidence:** high.

### Graph-made templates are 3n + 2 metres a side with 3 m floors

**Choice:** A graph-made template takes its size from a recipe whose sides are 3n + 2 metres, which gives exactly 3 m bays in all three graphs. The prototype sizes they replace (36 × 12 becomes 35 × 11) change, and the catalogue's hash with them.

**Gap:** The spec forbids stretching art to a footprint but left the legal sizes to S2.

**Verdict:** sound. The simulation's seats and eyes already assume 3 m bays and floors. **Confidence:** high.

### The source format: one kit and one template file per set

**Choice:** A building script writes a folder with `kit.glb` (modules as named roots with the usual four tiers) and `templates.json` (each template's physical descriptor and, per state, rows of module, position, yaw, per-axis scale, tier mask and tint). The physical catalogue the generator reads is derived from those descriptors and checked against them. The alternative kept descriptors hand-written in `fixtures/` with art fitted afterwards.

**Gap:** C13 and C32 named the contents (modules, descriptors, placement rows) and delegated grouping and encoding.

**Reach:** A template's shape has one author, the script that models it. A row cannot tilt or mirror a module; the exporter bakes those into module variants.

**Verdict:** sound. It is the one arrangement where art and physics cannot drift, and it matches how the renderer already instances (yaw and per-axis scale). **Confidence:** high.

### Triangle budgets per template are provisional

**Choice:** 150,000, 50,000, 12,000 and 2,000 triangles drawn at tiers 0 to 3. They are starting numbers for the scripts; the placement-chunk pass (C22) measures real frames and amends them.

**Verdict:** provisional. **Confidence:** low.

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

### The mask is a frame view the terrain writes, not a readback or a CPU picture

**Choice:** `ground-classes` is a `FrameView`: the terrain's own fragment writes three class bytes, the fog mask pass keeps only pixels that are wholly ground, and post passes them through untouched.

**Gap:** The slice says the mask is "written by the terrain material's own shading function" and delegates the encoding.

**Verdict:** sound. A mask computed anywhere else could drift from what is drawn; this one is the same site lookup the colour uses, lined up with the shot pixel for pixel. It cost a third post mode, because the two existing mask views survive the tone map only by being black or white. **Confidence:** high.

### Distances in the mask are exact only within the look's own reach

**Choice:** The mask holds the distance the material reads. Past `groundReach` (about 1 to 2 m from a road edge at a play-camera pixel, 4.5 m from a forest, 3 m from water) it keeps its side and may read farther than the truth.

**Gap:** The slice asks for "road signed-distance bands, river bands"; later slices name bands out to 6 and 8 m.

**Verdict:** provisional. Widening the reach for the mask alone would cost the final view (more records a lookup), against "the mask view adds 0 ms". A slice that draws a wider band must widen the reach to draw it, and the mask is then exact over that band. Until C67 lands, "the grass beside the road" is "outside, within a few pixels", not "SD in 1 to 3 m". **Confidence:** medium.

### Black means "not wholly ground", and grass and water are left out of the view

**Choice:** A mask pixel is black where any sample is a building, tree, unit or the backdrop; the view skips the grass and the water surface so the ground under them is read.

**Gap:** The slice does not say what a non-ground pixel holds.

**Verdict:** sound. A mixed pixel would decode to a wrong class; the river's bed has to show to carry its distance. To read the floor under a wood, shoot the mask with `trees: false`. **Confidence:** high.

### Generated-map stations follow the map's own report

**Choice:** The generated map's stations are functions of the preparation report's anchors (the objective town, blue's start, the map's size), not coordinates.

**Gap:** "Lab maps add theirs"; the lane asks for a generated map at the tactical camera and close.

**Verdict:** sound. The layout generator's version moves often; fixed coordinates would land in a different field each time. **Confidence:** high. They show a town street and a country road between fields; a station on a generated forest edge or river waits until a slice needs one.

### `forest-deep-25` looks down at 0.6 rad

**Choice:** The deep-forest station is 25 m away at 0.6 rad, above the canopy, not at the ground view's 0.32 rad.

**Gap:** The slice names the station, not its pitch.

**Verdict:** provisional. At 0.32 rad the eye is 8 m up, inside a crown, and the frame is one leaf. C76 (see the floor through the canopy) may want a second pose under the crowns once trunks are bare below them. **Confidence:** medium.

### The final view's cost is argued, not measured

**Choice:** No paired frame-cost run for the view's switch.

**Gap:** "The mask view adds 0 ms to the final view."

**Verdict:** sound. The final view gains one uniform comparison per terrain fragment; the machine's run-to-run noise is hundreds of times that. **Confidence:** high.

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
## C13/C32 kits and the template art library

### A kit is a static bundle whose states are its modules

**Choice:** A kit is an appearance of its own unit (`kit`), baked into the existing static bundle with one state per module. The alternative was a new bundle kind.

**Gap:** C32 says "existing scene-assets bake/schema/loader" without saying how a module is stored.

**Reach:** The bundle format, codec and loader are unchanged, so every existing bundle keeps its hash. Anything that hands every installed appearance to the model layer now hands it the kits too; the battle already filters to what it draws.

**Verdict:** sound. A module is exactly what a state already is: four tiers, a bounds, shared materials and textures. **Confidence:** high.

### A module's frame is its empty's

**Choice:** A module's geometry is read in its root empty's own frame, so a script may lay modules out side by side in the file. The alternative required every empty at the file's origin.

**Gap:** The source readme said "in the module's own frame" without saying whether the empty's position counts.

**Verdict:** sound. It costs nothing, and a kit file that can be opened and looked at is worth having. **Confidence:** high.

### Rows are columns, 34 bytes each, in the bundle container

**Choice:** The library is one file in the bundles' own container under its own magic: a header naming kits, modules and templates, and four columns over every row (module index, seven floats, tier mask, tint). A template's state is a range of them. The alternatives were quantised positions (about half the bytes) or one file per template.

**Gap:** C32 delegated "codec grouping/packing".

**Reach:** At 100 templates of 6,000 rows it is about 20 MB before transport compression; real sets should be a few megabytes. Resolving a building copies only its transforms.

**Verdict:** provisional. Quantising to a centimetre risks cracks between modules that meet on a lattice, for bytes nobody has yet measured as a problem. C22 measures a real library and may amend this. **Confidence:** medium.

### Art identity is the library's own bytes without the hash

**Choice:** `art_hash` is the sha256 of the library encoded without it. The library names each kit's bundle hash, so the hash covers modules, materials and tiers as well as rows, tier masks, tints, status and the catalogue hash it covers.

**Verdict:** sound. Anything that changes what is drawn changes it, and nothing else does. **Confidence:** high.

### Missing art is refused in three places, by one name each

**Choice:** A template or state with no rows is refused by the resolver (`template.missing`, `state.missing`). A module its kit lacks is refused when the sets are packed and again when the loader binds the library to the installed kits (`module.missing`, `kit.missing`), which fails the whole load.

**Gap:** C32 asks for explicit failure on missing templates, modules and state rows; the resolver's four arguments cannot see the kits.

**Verdict:** sound. A missing module is a broken catalog generation, not a per-building event, so it belongs to the atomic install. **Confidence:** high.

### Fit is every vertex inside some grown part, and nothing below a part's base

**Choice:** A state fits when each vertex its rows draw, at each tier a row draws at, lies inside at least one part grown by the set's `side_m` and `top_m`. Below a part's base only the catalog's ground tolerance is allowed. A millimetre of slack covers single-precision transforms.

**Gap:** The readme gives `side_m` and `top_m` and is silent on the underside and on what "inside the union" means for a mesh spanning two parts.

**Reach:** A plinth sunk into a slope is refused today. A triangle whose corners are in two parts and whose middle is in neither is accepted.

**Verdict:** provisional. Foundations on sloped ground are a real need the ground lane's pads may answer first; if not, `fit` gains a `below_m`. **Confidence:** medium.

### The catalogue is derived, and the prototype set is derived from it

**Choice:** `asset catalogue` rewrites the physical catalogue's rows from every set's descriptors, keeping an unchanged row as written and where it is. `asset prototypes` then gives stand-in rows to every catalogue template no other set dresses. A descriptor equals its catalogue row when the contract's canonical forms are equal.

**Gap:** "The physical catalogue the generator reads is derived from those descriptors and checked against them" did not say which way the prototypes flow, since they have no script.

**Reach:** To retire a stand-in, delete its catalogue row and regenerate the prototype set. The catalogue file's text is the command's, so its hand formatting is now fixed.

**Verdict:** sound. Run on today's catalogue it changes no byte, so nothing moved in this pass. **Confidence:** high.

### Prototype rows carry the category's tint, not each building's jitter

**Choice:** A stand-in row's tint is its category's massing tint. The massing boxes also varied each building's value; a template's rows are shared by every building placed from it, so that variation is the renderer's per instance, or gone.

**Verdict:** provisional, for C22 to settle when the boxes are replaced. **Confidence:** medium.

### Triangle budgets are reported, not enforced

**Choice:** The bake prints what each template draws at each tier and marks one over budget. It refuses nothing.

**Gap:** The readme lists the budgets among the rules a set keeps; the ledger above calls them provisional.

**Verdict:** sound until C22 measures frames. A gate on guessed numbers would refuse good art. **Confidence:** medium.

### The asset bake and check need the WebAssembly

**Choice:** Descriptors and the catalogue's hash are judged by `contract::templates` through one new export (`complete_template_catalogue_json`), loaded only when the catalog has city sets. The alternative was a native helper binary.

**Reach:** `asset bake` and `asset check` now fail without `bun run build:wasm`. The dev server's rebake already runs after it.

**Verdict:** sound. It is the route the web tests and the map adapters already take. **Confidence:** high.
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

## C17: our own houses

### A house's box is as tall as its ridge

**Choice:** Every house part's top is its roof's ridge: a one-floor house is a 5.2 m box (the prototype was 4 m), two floors 8.2 to 8.45 m (6.5 m), three floors 11.75 to 12.85 m (9.5 and 10.5 m). Floor datums stay at 0, 3 and 6 m (0, 4 and 7 m over a shop), and walls reach 2.85 m above the top datum. The alternative was a box to the eaves with the roof above it as `fit.top_m`.

**Gap:** The prototypes were flat boxes with no roof, and "a storey is 2.5 to 12 m of box" does not say where a pitched roof goes.

**Reach:** The simulation stops rounds and sight through the empty air beside a roof, up to 3 m above the eaves at the wall line. Plans did not change, except the L's wing (below). The catalogue's hash moves when these replace the prototypes.

**Verdict:** provisional. A soldier behind a house is hidden by its roof on screen, so the box agrees with the picture from the game's camera; pitches are held to 25 to 35 degrees to keep the over-claim small. **Confidence:** medium.

### The L-shaped house's wing is 6 x 6 m, set a metre in from the gable

**Choice:** The wing is 6 x 6 m (the prototype's was 5 x 6) and stands from x = -2 to 4 behind a 10 x 8 house, not flush with its east gable. Its box is as tall as the house's, because a supported join needs equal tops, while its ridge is 0.6 m lower, at the house's own pitch.

**Gap:** Flush with the gable, the wing's eave ran along the gable wall as a stray board under the verge. A 5 m wing under the house's ridge height needs a 43 degree roof.

**Verdict:** sound for the look; the wing's box over-claims 0.6 m more than the others. **Confidence:** medium.

### Bays are centred on each wall, and a door takes a bay or stands between two

**Choice:** Each exposed edge's 3 m lattice is phased so its bays are symmetric about the wall's middle and at least 1 m from a corner: three on a 9 to 11 m wall, two on 6 to 8 m, four on 12 to 13 m. Every bay has a window on every floor, on gable ends too. A front door either replaces the ground-floor window of a bay (terraces, the town house, the bungalow) or stands at the middle between two bays. A shop's door and display window are a bay each.

**Gap:** The readme says windows sit on the lattice; it does not say whether every bay needs one or where a door goes.

**Verdict:** sound. A garrison's seat is always at an opening. **Confidence:** high.

### A terrace is one unit module repeated, each unit tinted by its row

**Choice:** A terrace's unit (front and back wall, its stretch of roof) is one module placed once per part, with an end-wall module turned to face each way. Detached houses are one shell module each (walls and roof together, so the eaves shade the wall in the baked occlusion). Wall colour is the row's tint on a pale tint-masked plaster or brick; roof colour is the material's own, since a row has one tint.

**Gap:** "Each template's wall shell and roof is its own module" does not say whether a repeated unit counts.

**Reach:** A template's colours are fixed in its rows, so two placements of one template are the same colours. Five detached templates repeat visibly in a suburb of twenty (`suburb-250m.png`). More colourways are more templates over the same modules, or a tint a placement supplies; neither is done here.

**Verdict:** provisional on the repeats. **Confidence:** medium.

### The set's fit is 0.5 m to the side and 0.9 m above

**Choice:** Eaves overhang 0.35 m with a gutter to 0.46 m, steps and door canopies 0.4 to 0.45 m; chimney pots stand 0.77 m over the ridge and capping tiles 0.07 m.
## C11/C12/C13 the China apartment kit

### A recipe's Width is the template's short side

**Choice:** The graph puts its one entrance on its Depth facade. A slab's recipe therefore has Depth as the long side (`Width 11, Depth 35`), and the template's frame turns the graph a quarter so the long side runs along X with the door on -Y, as the prototypes had it. The alternative, Width as the long side, puts a 35 m slab's only door in its 11 m gable.

**Gap:** The readme's example recipe reads `Width 35, Depth 11`; nothing says which facade carries the door.

**Reach:** The parcel pass turns the first entrance toward the street, so slabs front the street with their long side and their shops.

**Verdict:** sound. **Confidence:** high.
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

### The camera lab was not moved

**Choice:** `fixtures/camera-lab.json` is still compiled at run time.

**Gap:** "If it falls out cheaply."

**Verdict:** sound for now. Its plan is already template ids and frames, so the move is mechanical, but a saved map pins the prototype library's hash, and the lab would stop loading whenever a prototype template changed. **Confidence:** medium.

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
## C22 static chunk owner

**When:** C22's first half (the owner, and corpses on it), 2026-10-01.

### The owner is `frame/staticChunks.ts`; `scenery/lod.ts` keeps scenery's record and tier rule

**Choice:** The chunk bookkeeping moved out of `scenery/lod.ts` into `frame/staticChunks.ts`. What stayed in `lod.ts` is scenery's own: its 12-float record, `treeInstances`, and the rule that a far chunk draws at the last tier. The alternative was to generalise `lod.ts` in place and have the models layer import from `scenery/`.

**Gap:** The slice says "promoted from the scenery layer's existing chunk path" and names no home.

**Verdict:** Sound. The owner serves two layers, so it sits beside the other things both read (`frame/detailView.ts`); scenery's files, which the ground lane owns, changed only where they called the old functions. **Confidence:** High.

### A population gives each instance's bounds as a function, not as arrays

**Choice:** The owner's input (`ChunkSource`) is records of any stride, a kind and a size per instance, and `bound(i, box)`, which grows a chunk's box by one instance. The alternative was the scenery path's two arrays (height and reach), extended for corpses.

**Gap:** "Record packing" is delegated; the shape of an instance's bounds is not mentioned.

**Verdict:** Sound. A tree stands on its foot (its reach about it, its height above); a corpse reaches its length and the shadow margin every way, computed from a 32-bit size in double precision. Arrays would have needed a third and fourth per instance or moved a corpse chunk's box by a rounding step, and a box that moves can flip a chunk between cards and meshes at the threshold. The function keeps both populations' boxes bit-identical to what they were, and runs only when a list is rebuilt. **Confidence:** High.

### A chunk's level is the population's rule, and a level is whatever its layer draws

**Choice:** `selectChunks` asks the population's `ChunkLevel` for each chunk in view: a level (the chunk draws whole, as merged ranges kept per kind and level) or `NEAR`. A population declares how many levels it has. Scenery has four and answers the last for a far chunk; corpses have five, the fifth their impostor card, and answer it only for a chunk whose every corpse has a card. The alternative was one built-in "far" level decided by a pixel threshold the owner holds.

**Gap:** The slice asks for one owner of "tiering"; it does not say who decides a chunk's level, or that a card is a level.

**Verdict:** Sound. The two rules differ in more than a number (a corpse chunk also needs every card baked), and kit modules and far-tier tiles bring their own; the owner stays free of each layer's presentation. **Confidence:** High.

### Near chunks are listed; staging per level is a separate piece only scenery uses

**Choice:** The owner lists the chunks whose instances choose their own level (`near`). `stageNear` copies those instances into a list per level, which scenery uploads per tier. The models layer does not use it: it walks the near chunks itself, asks the models' own detail rule per corpse (culled, a tier, or a card), and packs the result with the frame's other models. The alternative was to stage corpses through the owner too and draw them from buffers of their own.

**Gap:** "Corpse chunks move onto it" does not say how far the per-corpse work moves.

**Verdict:** Sound for this pass. A near corpse shares its mesh draws with the units (one records buffer, one run per mesh and fog class) and is culled by its own sphere with the shadow margin; staging it elsewhere would have changed the draws, the stats the scenes read, and added a second detail rule. What moved is everything that was duplicated: bucketing, chunk culling, the far decision's distance, the merged ranges. **Confidence:** High.

### Corpses are one kind, with the placed order kept beside the records

**Choice:** All corpses are one kind in the owner, whatever their appearance: one static buffer, so a far stretch of mixed corpses is still one card draw (the card's atlas layer is in the record). The owner keeps, per kind, which placed instance each sorted record is (`order`), and the models layer uses it to keep each record's appearance. The alternative was a kind per appearance.

**Gap:** Not covered.

**Verdict:** Sound. A kind per appearance would split every card range by appearance and raise the draw count the scenes read. **Confidence:** High.

### Whether a population casts is the caller's argument, not the population's field

**Choice:** `selectChunks` takes the sun's shadow (fall and reach) or null. Scenery passes it for the forest and massing and null for the backdrop; corpses pass null, since a card casts nothing and a near corpse casts through the models layer's own caster draws. Cast ranges stay one list per kind, drawn at the coarsest mesh.

**Gap:** "Casters" are the owner's, with no detail.

**Verdict:** Sound; this is what the scenery path did, with the flag moved to where the shadow is known. **Confidence:** High.

### The corpse comparison is a scratch capture, not a registered scene

**Choice:** "Corpse scenes unchanged" was judged on frames of the endurance lab's late state (the newest thousand of 20,000 fallen) at eight fixed cameras and one tick, with grass, effects and cast lights off, taken from the base commit and from the change under one hold of the GPU lock, twice each. No registered scene frames static corpses at a fixed camera; the village's fallen are mid-death when it shoots them.

**Gap:** The slice names no corpse scene.

**Verdict:** Provisional as a method: the script lives in the gitignored scratch folder. If corpse drawing changes again, a station for the fallen belongs in a scene. **Confidence:** Medium.

## C16: our own farmsteads

### A farm is one shell module, and its buildings' colours are in its materials

**Choice:** Each farmstead's walls and roofs, for all its buildings, are one module in the template's frame, placed by one untinted row. A building's wall colour is baked into its own material (the same recipe under another vertex colour), which stays tint-masked. The alternative was the houses' way: a shell module per building, coloured by its row's tint.

**Gap:** The brief asked for the houses' structure; the readme gained "a far building is one row" while this was being written.

**Reach:** A farm is one row at tiers 2 and 3 (two or three with a shell per building). A row's tint, if a placement ever supplies one, shifts the whole farm, not one building. A shell is its template's alone, so nothing is shared between farms but the fittings.

**Verdict:** sound. **Confidence:** medium-high.

### Fittings are rows at tiers 0 and 1, and flat panels in the shell at tier 2

**Choice:** A window, a door, a chimney, a stack of bales is a row at the two fine tiers. At tier 2 the shell carries what is left of each: one dark quad per window, one quad in the door's paint per door, a box per chimney and bale stack, and the barn's framing as flat boards. At tier 3 the shell is walls and roofs only. `Farm.place` does the folding, so the houses' row helpers (`glaze`, `front_door`, `stack`) are used unchanged. The China set keeps the dark quads at tier 3 too; here that is 73 quads against a budget of about 100 triangles, on a farm under 24 px tall. The barn's framing loses its braces at tier 2: a 0.2 m timber is a pixel wide from 250 m, and a diagonal one crawls.

**Gap:** "A template's coarsest tier is little more than its shell."

**Verdict:** sound. **Confidence:** high.

### The plans are the prototypes'; every box is as tall as its ridge

**Choice:** The yard and long farms' plans are the prototype rows' (the same parts, centres and sizes). The small farm's house and barn stand 5.5 m apart, not 3.5 m, so a cart passes between them: the farm is 26.5 m wide where the prototype was 24.5 m. Heights follow C17's rule, a part's top is its roof's ridge: the yard farm's house 8.35 m (6.5), barn 8.4 m (7.5) and shed 2.94 m (3.5); the long farm's house 5.2 m (4) and byre 7.5 m (8); the small farm's house 8.15 m (6.5) and barn 7.8 m (6). Roof pitches are 29 to 33 degrees, the cart shed's single slope 8.

**Reach:** The catalogue's hash moves at the cutover. The longest sides are 34, 27 and 26.5 m, inside the category's 20 to 45 m.

**Verdict:** sound for the look; the boxes over-claim the air beside each roof, as the houses' do. **Confidence:** medium.

### The cart shed stays under the farm's upper floor datum

**Choice:** The yard and small farms have two floors (0 and 3 m), the long farm one. A barn in a two-floor farm has its loft openings on the 3 m datum (sills 0.6 m above it, under the eaves); the long farm's byre, 7.5 m to the ridge, has openings on the ground only. The yard farm's cart shed is 2.94 m tall (the prototype's was 3.5 m), so the 3 m datum is above its box.

**Gap:** `floor_heights_m` is one list per template; nothing says what a datum means in a part too low for a second floor.

**Reach:** The simulation seats a garrison on every datum below a part's top (`seat_plan`, `crates/sim/src/garrison.rs`), so a 3.5 m shed in a two-floor template has soldiers seated half a metre under its roof. The prototype row has that. A shed under 3 m has ground seats only.

**Verdict:** sound for this set; the rule it leans on (a datum counts where a part reaches it) is the simulation's, read from the code, not tested here. **Confidence:** medium.

### A barn keeps few openings; a bay without one is a blank wall

**Choice:** Every bay of a house has a window on every floor, as the houses have. A barn has a door or a small window in most ground bays and in a few loft bays; the cart shed's two end walls are blank. Each opening is in a bay and on a datum. The cart shed's open front is its three south bays, between the posts.

**Gap:** "A barn's few small high windows still sit on bays" allows bays without windows; C17 chose a window in every bay "so a garrison's seat is always at an opening".

**Reach:** A soldier garrisoned in a barn's blank bay fires through a wall that shows no opening.

**Verdict:** provisional. A barn with a window every 3 m on both floors reads as a house. **Confidence:** medium.

### The cart shed is drawn open; its box is solid

**Choice:** The shed's south side is three open bays with a cart, a woodpile and straw inside. Its part is a whole box, as every part is.

**Gap:** "Every outer wall stands on a face of a part" assumes a wall on every face.

**Reach:** A unit behind the shed is hidden in the simulation where the picture shows daylight under the roof from a low camera; rounds stop at the open front. At the game's camera the roof covers most of the opening.

**Verdict:** provisional; the brief names an open-fronted shed. **Confidence:** medium.

### Each barn's wagon door on the street side is an entrance

**Choice:** Every farm has two entrances on the south: the house's front door and the barn's wagon door. On the long farm the byre stands behind the house, so its wagon door is in the end bay (offset 10.5 m), outside the house's width, with the clear 60 m the map generator asks for. The prototype long farm had the house's door only.

**Verdict:** sound. **Confidence:** high.

### No yard wall or gate; what stands in a yard hugs a wall

**Choice:** The village courtyard farm's yard wall, gate, cart and woodpile are not carried over as yard furniture: only a part can hold art, and the yard is open ground. Of `house.py`, the barn doors and the gable framing are lifted into `masonry.py` as builders (`barn_doors`, `timber_frame`). A woodpile, a trough, a rain barrel and straw bales stand against walls within the set's side fit, and the cart stands inside the shed's box.

**Verdict:** sound. A walled yard would need the wall as a prop the generator places, which is not this lane's. **Confidence:** high.

### The set's fit is 0.6 m to the side and 0.9 m above

**Choice:** Eaves and gutters reach 0.46 m, a loft hoist 0.55 m, a woodpile 0.49 m, a trough 0.53 m, a rain barrel 0.57 m; chimney pots stand 0.77 m over a ridge. The houses' set has 0.5 m.

**Verdict:** sound. **Confidence:** high.

### A new recipe for stone walls; the houses' code is shared, not their modules

**Choice:** `rubble_stone` (random rubble in lime mortar) is a new texture recipe, because `field_stone` is one stone's surface with no joints. The farm's windows, doors, chimneys and gutters are its own modules built by the same `masonry.py` builders as the houses'; the houses' shell and row helpers moved to `masonry.py` (`house_shell`) and `kit.py` (`glaze`, `front_door`, `rainwater`, `stack`, the sill and overhang conventions). `homes.py` writes the same bytes as before.

**Verdict:** sound. **Confidence:** high.

### The three farms differ in material and form, and read as three regions

**Choice:** Variety was asked for, so each farm has its own wall materials, roof colour, roof forms and barn type: cream plaster under clay tiles with a half-timbered barn; whitewashed roughcast and rubble stone under slate; red brick and tarred boards under brown and clay tiles. The alternative was one material family in three sizes.

**Reach:** An unprimed reviewer read them as a German, a French and an English farm, and a hamlet of all three as "a sampler of regional styles, not one village". None reads as Chinese; the family name is the first shipping family's, as it is for the houses.

**Verdict:** provisional. If a map's hamlets should look like one place, the generator needs to pick one farm per hamlet, or the farms need one shared palette; neither is this slice's. **Confidence:** medium.
## Surroundings for the existing maps: cut

**Choice:** C56 (reservations) and C34, C35, C36 (surroundings for the village, the labs and the benchmark fields) are removed from the spec, with the lane that was written for them. The old maps stay the small arenas they are.

**Why:** the owner's call (2026-10-01): the village and the lab maps are developer test arenas, nobody plays them, and the game is not finished. The need those slices answered, a full-size map to play, is met by generated battles from the menu and saved generated maps. Where another slice still says C56 or C34–C36, read "cut".

**Verdict:** sound. **Confidence:** high.

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


## C06/C07: full generated browser stress ownership

**Choice:** Keep the saved endurance scene as the default and add a fixed current
Metro Large seed-4 arm. The existing preparation worker resolves its generated
MapSource and calls a thin Wasm forwarder to the simulation's city stress factory.
The selector belongs to the worker message, alongside the unchanged normal
preparation request. No second generator, placement rule or persistent large
fixture enters the browser. The prepared report identifies the full compiled
world separately from the synthetic contact and late-state inputs.

**Reach:** This adds a repeatable full-extent browser admission input, including
worker cancellation when the view changes. It does not prove startup memory,
GPU cost, image quality or real-time throughput. Those gates remain coordinated
integration work. **Verdict:** sound for the preparation seam; browser evidence
pending. **Confidence:** high for fixture identity, medium until the scene runs.
