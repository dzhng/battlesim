# city-maps choices ledger

Implementation decisions not already settled by the user or plan. Each entry names its gap, consequence, verdict and confidence; delegated experiment limits are recorded in the owning evidence instead.

## Sound

### Systems proofs may unlock their production contract before complete art acceptance

**When:** S0 / systems-scope checkpoint, 2026-09-30.

**Choice:** A physical/resource contract can enter production after its own frozen proof passes while complete G0 and released appearance remain open. For example, a new terrain representation must reproduce the original heights/rays and fit full-size memory limits before simulation consumers use it; missing tower art does not block that code proof. The alternative would wait for finished models before any nonvisual implementation, so the requested specialist handoff could never be reached.

**Gap:** The original G0 bundled physical feasibility with source exports and visual frame approval. The user then explicitly deferred 3D design/modeling until systems implementation finishes.

**Reach:** Every mixed slice carries two evidence responsibilities: implemented systems and pending specialist acceptance. This does not mark the whole slice/release complete, waive resource failures or certify prototype art.

**Verdict:** Sound — respects the new execution order while keeping each original final requirement and a concrete parity/resource gate.

**Confidence:** High.

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

### Keep scratch admission and accepted parent writes under one entry

- **When:** SA2 measured repeated-work pass.
- **Choice:** when a neighbor offers a better route cost, hold its existing scratch-table entry through the cost check, optional pruning bound and parent write. The alternative was another retained per-cell cache; the measured heuristic cache saved too little work and increased memory and latency, so it was removed.
- **Gap:** exact lookup reuse was delegated without choosing how to combine reads and writes.
- **Reach:** one scratch owner retains its original arrays/generation, strict tie behavior and public route interface. The matched arm reduces instructions without extra payload; it does not establish the still-failed full-size planning budget.
- **Verdict:** sound — the source proof and complete original-output checks support the measured correction, while resource failures stay explicit.
- **Confidence:** high.
