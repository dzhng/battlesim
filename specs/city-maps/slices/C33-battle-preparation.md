# C33: shared battle preparation and public geometry

**Depends on:** C09, C01 and G0's world/export representation. **Kind:** slice.

## Question
Can battle preparation expose public geometry for drawing/picking without constructing another simulation world on the main thread?

## Contract it unlocks
The existing preparation worker owns resolved scenario/world construction. Its result transfers typed public static exports and the geometry needed by the existing drawing, ground/building picking and C57 clearance consumers. Prepared physical geometry is reused by Battle initialization rather than rebuilt after encounter planning.

Cut over `useStaticWorld.ts`: remove its main-thread `new WorldView(map, rules)` when its consumers move to the prepared export/query contract. Count exported arrays and any compact main-thread query index explicitly in G0's peak budget. A public ray/clearance query index is allowed; a second terrain/nav/foliage simulation world is not. Preserve ground and PropId picking with matched rays.

Static map geometry is public. Learned replacement/damage geometry comes from the side's observation; no ray/camera query can inspect hidden live destruction in the worker. Map replacement releases the previous prepared world and resident arrays. Developer unit probes may still instantiate WorldView directly; production route consumers cannot.

## API seam
Existing wasm world/Battle construction → worker preparation result → transferred public geometry/query inputs → lab hooks and renderer. Reuse the current world export types; extend them only for G0's selected representation and consumer queries.

## What the human can run or see
A resolved-map route with unchanged ground/building picks, public geometry and an allocation report showing one simulation-world construction.

## Verification
- Matched original/replacement route rays preserve ground positions and PropIds.
- Shared physical queries yield the same prepared scenario and Battle state as direct initialization.
- Public/side-known geometry holds across unseen collapse, side switch and resubscription.
- Cold start and replacement stay within G0's memory/time limits; obsolete preparation cannot publish a world.
- Compare query overlays to the current route via compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Transfer ownership and compact query-index internals within G0's chosen contract. Hidden live-state access and a second full world are not delegated.

## Must stay green
One simulation geometry construction, public picking/clearance, route outcomes and bounded startup.

## Feedback that would change this slice
A missing public query or transfer peak reopens G0's export representation.

## Outcome — startup implementation checkpoint

Preparation retains its worker and consumes one `PreparedMap` for live, scripted
or replay startup. Public typed exports transfer to the page; its `PublicWorld`
imports a lossless query payload and builds only the static picking/surface index.
Surface, terrain/water ray and learned crown arithmetic have shared Rust owners.
Production battle hooks build no WorldView. Standalone camera geometry is prepared
off the page thread; geometry/flight probes retain their explicit developer view.

Narrow proofs pass: reused native preparation matches direct construction through
180 combat ticks and replay; public grid/ray/PropId/foliage queries match exactly;
26 focused web contracts pass. A browser test with GPU disabled exercises actual
preparation-worker adoption, three ticks, transferred geometry and replay. Against
the frozen original Wasm (`4efec25b`), 1,323 point queries and 5,292 rays over the
geometry, village and camera maps match exactly, including learned foliage.

Startup/time/memory admission remains pending the shared GPU queue. Its final
numbers and cold/warm cache conditions belong to [the lane](../startup-lane.md).
