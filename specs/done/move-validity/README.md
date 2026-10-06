# Valid move destinations

A destination marker promises a place to stand that its unit can reach on the ground its side knows: its footprint fits there, and neither water, a cliff nor a pocket its side knows to be closed off parts it from where the unit stands. The way there is found on the move. A unit replans when its side learns of a body on its route or when it stalls, and gives the move up, holding where it is (`RouteBlocked`), when no way is left or its way has not got shorter in 30 s. It tries again when its side learns something new of the ground.

## Why not prove the journey

Admission used to rehearse every order: it ran the movement owner on a copy of the battle until the units arrived. That made each marker a proof, and it does not scale. Every rehearsal step advanced every unit in the battle, and the allowance counted them all. In a 200-unit city battle most orders were refused, even a truck's drive to its own rear (the allowance ran out before arrival, not the route). Each refusal cost about a second of simulation natively, and the city-contact benchmark's contact tick stalled for 42 s. A proof is also impossible to keep: an enemy, or a body nobody has seen, can close a route after the click. So admission checks what can be known at the click, and execution owns the rest.

Superseded (2026-10-05): the rehearsal of move orders and route upgrades. Garrison entry still rehearses (`movement::certify_orders`, with its carry and allowance): whether a squad can get into a building is a question a route does not answer.

## The promise and its limits

- Every displayed destination has standing room its unit can reach by terrain and known pockets. Failed placements are filtered from held, confirmation and committed markers. Bodies on the way are found on the way: a squad sent into a walled yard walks to it and stops outside.
- A queued move behind an indefinite attack is not placed: an attack has no known end to set off from.
- Placement uses side knowledge only, and a query leaves live state, navigation charges, future digests and replay scheduling unchanged.
- Orders are rate limited at `rules.commands.orders_per_s` (30) a side, a second's worth at most at once; one past it is dropped (`RateLimited`) and never shown. No player reaches it; it bounds a script's flood or a stuck input.
- Conditions change after issuance: a newly discovered obstacle, a later Stop or combat changes execution.

## Owners and proof

[Battle admission](../../../crates/sim/src/battle.rs) owns pending command projection, the shared preview/admission boundary and the order rate. [Formation](../../../crates/sim/src/formation.rs) owns candidate standing positions; the [navigation grid](../../../crates/sim/src/navigation.rs) owns standing room and reach (`destination_point`, `can_reach`, terrain components and pockets). The [movement owner](../../../crates/sim/src/movement/mod.rs) owns replanning and giving a move up. The [command contract](../../../crates/contract/src/command.rs) is the browser boundary.

[Admission tests](../../../crates/sim/tests/move_admission.rs) pin reach across rivers and pockets, pending commands, visibility and query neutrality, and the pathfinding the rehearsal used to stand in for: a truck giving up on a lane its shove cannot clear and on a corner too tight to turn, a vehicle boxed by a parked one, a squad ringed by standing soldiers, and a squad sent into a closed yard. [Movement tests](../../../crates/sim/tests/movement.rs), [road journeys](../../../crates/sim/tests/road_journeys.rs) and [movement scenarios](../../../crates/sim/tests/movement_scenarios.rs) pin physical arrival and collision. Browser intent and rejection checks live in [web tests](../../../web/tests/); rendered interaction checks live in [browser scenes](../../../web/scenes/).

[Pointer paint](../../../apps/battle-lab/src/pointerPaint.ts) identifies each asynchronous result by the complete current intent. Preview and publication share the movement owner's approach-heading calculation; a squad's final settled body angle is not a second definition of its ordered heading. [BattleView](../../../apps/battle-lab/src/BattleView.tsx) and the [order overlay](../../../packages/battle-renderer/src/orderOverlay.ts) consume accepted destinations rather than reconstructing validity.

## Visual provenance

The user's [waiting screenshot](assets/reference/user-waiting.png) is the historical failure reference: a truck behind a tank with an apparent destination. The user reported that it made no progress. Production browser captures reproduce [close parking](assets/result/close-parking-valid-held.png) and [actual arrival](assets/result/close-parking-arrived.png) at the same image size and a comparable diagonal road view. The [group departure](assets/result/group-departure-complete.png) frame uses wider framing to keep every origin and destination visible; the [arrival](assets/result/group-arrival.png) frame returns to a closer road view.

The matched [previous-filter frame](assets/result/baseline-invalid-held.png) and [corrected-filter frame](assets/result/candidate-invalid-held.png) use the same current simulation, camera and inputs. They isolate marker filtering rather than claiming a historical simulation comparison: the unavailable destination disappears. [Release feedback](assets/result/invalid-released-refusal.png) confirms the refused command. These retain the existing marker style; ordinary held and committed infantry overlays differ, so they are not a promise of identical meshes before and after release.
