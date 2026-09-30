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
