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

## Outcome — startup lane

One battle builds the simulation's world twice, where it built it three times:
once in the preparation worker, for the encounter planner and the battle, and
once on the page, for drawing and map queries. The page-side half of the
contract above (a transferred query export in place of a page world) was built
and then removed; [C33 simplified](../choices.md#preparations-worker-keeps-its-world-and-becomes-the-battle-authority-the-page-builds-a-plain-query-world) has the reason.

**The worker.** Preparation builds one `PreparedMap` (the world, its road net
and its navigation base). The planner borrows it to place the encounter, and
the battle then takes it (`Battle::from_prepared`, `from_prepared_replay`): the
worker that prepared the battle becomes its authority, for a live battle and
for a replay alike. Replacing or cancelling a pending battle closes that
worker, which releases everything it held and silences a late answer. A
restart is a new battle in a fresh worker, which builds the same scenario once.

**The page.** The page builds its own plain world from the scenario's map with
the simulation's `WorldView` (the world's geometry: no navigation, no battle
state) and answers picking, ground height, surface, learned foliage and camera
clearance from it. There is one implementation of those queries, the
simulation's. A side still learns damage and destruction only through its own
observations: the page's world is the static map, which is public.

Proofs, all narrow. The reused native preparation matches direct construction
through 180 combat ticks and a replay. A browser test with the GPU disabled
runs the real worker handoff: preparation, adoption by the simulation client,
three ticks and the replay's digest. The generated scene passes from the menu
through a battle, a cancelled load, a saved replay that prepares the same map
and reaches the same digest, the engine-build refusal and the saved
battlefield; the camera scene passes with its clearance and matched-pose
checks unchanged.

A battle the simulation refuses to start (a replay of another scenario or
another build) shows its refusal in the existing error HUD with the menu
available, hides the clock and the loading cover, and draws no battlefield
under it. A browser regression holds that.

Startup stays far inside its budget; the numbers and their conditions are in
[the lane](../startup-lane.md#startup-measurement). Full G0 admission remains
with its separate envelope gate.

The ground browser scene also passes: only the observed side's learned marks
are drawn; the opposite side's unseen crater remains absent, and a view switch
begins a full snapshot in a new epoch before deltas resume.
