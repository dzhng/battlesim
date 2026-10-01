# Counted immutable road-core proof

A standalone experiment for the first cut of
[the integration proposal](../../../slices/SA2-counted-route-integration.md). It activates
no production planner, Battle state, observation schema or speed change. The core is
invoked explicitly, so it does not prove automatic mode selection in Battle. Its sources
(the candidate core, `intent.rs` for the accepted automatic-road boundary, and the
derivation script) were deleted from the tree and live only in the evidence tag.
Raw evidence: tag `city-maps-evidence-2026-09-30`.

## What it proves

Loading prepares one immutable junction topology. Requests share it while keeping their
own endpoint projections, search labels and physical context. Endpoint discovery, coarse
search, parent tracing, failed-arc retry and physical refinement spend credits through
resumable cursors. Running out of credits returns Pending; exhausting the road graph
returns CorridorDeclined, which does not mean the destination is unreachable. There is no
synchronous escape to the original planner.

A one-credit negative run showed that wrapping a complete planner in a counter does not
bound work. The green run exercises the real cursor path, directed destination admission,
concurrent requests and replay. A 5 km threshold was rejected in favour of the accepted
below/at/above 2 km contract. Replans keep activation intent; queued legs evaluate from
their actual activation pose.

Native and Wasm produce 19 identical records (16 physical contexts, two concurrent
contexts, one intent record; 257,949 JSON bytes) with costs and coordinates as raw f64
bits and a work/status/state digest per poll. The corpus uses shipped rifle/Jeep footprint,
push and material-cost data on small maps: clear ground, a rotated wall, water with and
without a deck. Every returned segment matches the unchanged physical evaluator bit for
bit. Coarse costs rank corridors only; the proof does not claim the original fine-grid
optimum, parent chain or commit tick.

## Limits that block Battle integration

- Credits are not an instruction or frame-time bound. Tree operations are logarithmic
  atoms whose depth and allocation need admission against real topology.
- Loading preparation is synchronous; its allocation, cancellation and startup cost are open.
- The per-poll reference scan, output serialization and proof ABI run outside the budget
  and must not become tick work.
- Cancel, final request destruction and the last topology release are not counted, and a
  completed request still retains labels, queue entries and endpoint overlays. The
  [cleanup handoff](../../../slices/SA2-counted-route-integration.md#stopping-point-and-next-cleanup-owner)
  names the owner and bound this needs.
- Side-known snapshot capture, fog learning and invalidation are not implemented;
  concurrent beliefs are distinct read-only grids.
- Polygon-road connectivity and a resumable general-region fallback are missing.

No full-size, startup, active-battle, arrival or G0 claim follows from this small proof.
