# Counted immutable road-core proof

This is a frozen standalone experiment, with no activated production planner, Battle
state, observation schema or speed change. It implements the first cut in
[the integration proposal](../../../slices/SA2-counted-route-integration.md).
Historical route and transit receipts remain unchanged. The accepted automatic-road
boundary is captured by [intent.rs](intent.rs); the core is invoked explicitly in this
experiment, so these records do not prove automatic mode selection in Battle.

## Contract and evidence

Loading prepares one immutable junction topology. Requests share it while retaining
independent endpoint projections, search labels and physical context. Endpoint discovery,
coarse search, parent tracing, failed-arc retry and existing physical refinement consume
credits through resumable cursors. Exhausting credits returns Pending; exhausting this
road graph returns CorridorDeclined, which is not destination infeasibility. There is no
synchronous original-planner escape inside the actor.

The [one-credit negative record](red-result.log) demonstrates that wrapping a complete
planner with a counter does not establish bounded work. The [tiny green record](tiny-green.log)
exercises the actual cursor path, directed destination admission, independent concurrent
requests and replay. The [5 km negative record](intent-red.log) rejects the superseded
threshold against the accepted below/at/above 2 km contract; the rejected source is retained
separately, and the green source is restored exactly. Replans preserve activation intent;
queued legs evaluate from their actual activation pose.

[Native](native.json) and [Wasm](wasm.json) contain 19 complete records: 16 actual physical
contexts, two concurrent contexts and one intent record. All 257,949 JSON bytes agree after
removing only Native stdout's trailing newline. Costs and coordinates are encoded as raw
f64 bits, and every poll records work, status and semantic-state digest. A second Native
execution agrees completely. The corpus uses shipped rifle/Jeep footprint, push and
material-cost data on small maps with clear ground, a rotated wall, water without a deck
and water with a deck. The directed toy validator is a regression instrument, not a new
one-way game rule. Concurrent belief proxies are distinct readonly grids; actual side-known
snapshot capture, fog learning and invalidation remain unimplemented.

Every returned segment is checked against the unchanged physical evaluator, including
the final sum's operation order and bits. Coarse estimated costs rank corridors separately;
this candidate does not claim the original fine-grid optimum, parent chain or commit tick.
There is no trial speed tuning in these records. [summary.json](summary.json) owns measured
work/preparation counts and retained collection sizes; [identity.json](identity.json) pins
source, configuration, inputs, binaries, records, toolchain and normalization.
The [review record](review.json) pins the reviewed source boundary; the
[cleanup inventory](cleanup-inventory.json) identifies disposable lane probes and their
durable receipt owners.

## Ownership and bounded-work limits

The topology owns geographic connectivity; a request owns only its virtual endpoint
splices and mutable labels. Fixed-node BTree maps/sets avoid copying an expanding flat
array during one poll. A tree operation remains a logarithmic atom whose maximum depth,
allocation cost and latency require admission against actual topology. Credits alone are
not an instruction or frame-time acceptance. Loading preparation is synchronous and
separately counted, including index traversal, candidate sorting and junction insertion;
its allocation, cancellation and startup resource gate are still open.

The physical stage owns one capped private clearance page and its partial transform.
Request/page fingerprints update with mutations; each poll's cached digest is compared
with a canonical fold over retained state. That reference scan, route collection, complete
output serialization and the synchronous proof ABI run outside the actor's budget and
must not become active-tick work by accident. Fingerprints are ordinary deterministic
digest values, not collision-free state serialization or a cryptographic identity.

Retry drains queue and traced path incrementally and generation-stamps labels. Cancel,
final request/output destruction and the last shared snapshot/topology release are **not
counted**. A completed request still retains labels, queue entries and endpoint overlays.
The [future cleanup handoff](../../../slices/SA2-counted-route-integration.md#stopping-point-and-next-cleanup-owner)
names the required owner, resource bound, digest and falsifications. This is an admission
blocker before Battle integration, alongside polygon-road connectivity and a resumable
general-region fallback. No full-size, startup, active-battle, arrival or G0 claim follows
from this small proof.

## Reproduction and review boundary

Stage this directory's sources and records under `throwaway/navigation-counted/` in the
pinned checkout and rename `candidate.rs.snapshot` to `candidate.rs`. The manifest pins
the checkout paths, while Cargo.lock pins external dependencies. `derive.py` can regenerate
the candidate only from the exact production navigation SHA it asserts; it appends the
resumable physical evaluator and verification-only segment-cost access. It must run from
that staging location, not this archive. The rejected graph/index are used only by the
negative wrapper. Tiny tests use the recorded rustc test configuration; paired output
uses the release binary and cdylib plus `parity.mjs`. Keep the worktree's isolated target
directory and shared node_modules recipe. The archive stores source/records and binary
hashes rather than large compiled binaries.

Shape, diff and documentation review retain separate topology, request, physical and
rule-data owners, with zero production changes. The configured independent Codex CLI
cannot analyze this diff: its ChatGPT-account endpoint rejects `gpt-6.1-sol` before review.
That is an unavailable review, not a clean verdict. Root independently read the frozen
core, topology, index, physical evaluator and intent source and reported no remaining
finding within this proof-only cut, retaining the loading/destruction/fallback gaps.
Any new review finding or source correction needs a new receipt rather than
rewriting these records. The user requested this stopping point; future integration is
not authorization to start another feature pass.
