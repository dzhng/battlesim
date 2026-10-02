# Decisions

All entries describe the final implementation. The least certain choice is the amount of work spent checking a move; it is deliberately allowed to refuse an unproven move. No unresolved product decision requires approval.

## Sound — medium confidence

### Bound checking work, and refuse an unproven destination

**When:** movement admission. **Gap:** the request did not specify how long a preview may search.

When a player clicks across a large map, the simulation runs the proposed movement on a private copy. Each simulated tick charges for the living units being checked, and route searches charge their counted work against the same allowance. If the units have not arrived before it runs out, those destinations receive no markers. This can refuse a physically possible but expensive move; displaying a point whose route has not been demonstrated would break the requested contract. There is no fixed travel-time limit, so a long unobstructed journey can still pass.

**Reach:** the navigation configuration owns this allowance. Larger battles or difficult routes may need tuning. The rejection says the move is unavailable, rather than claiming mathematical impossibility. **Verdict:** sound because finite work and truthful markers are both required; **confidence:** medium because the allowance is a tuning decision.

### Keep scratch forest ground conservative

**When:** side-known world snapshots. **Gap:** live forest-clearing history is shared by the physical world and cannot simply be copied as the ordering side's knowledge.

A hidden vehicle may have flattened a lane through trees. A move preview must not reveal that clearance by suddenly accepting a route. The private world therefore starts its forest-ground mask uncleared, while remembered tree bodies remain governed by side knowledge. This can make a long trip through previously cleared forest harder to certify, even when live travel is possible.

**Reach:** admission favors refusing an unproven move over using hidden clearing. A future improvement can maintain an explicit side-known clearing mask; it must preserve query neutrality. **Verdict:** sound for the marker and fog contracts, with a conservative admission-rate trade-off; **confidence:** medium because this may need refinement for long forest journeys.

## Sound — high confidence

### Check the existing movement rules instead of building another solver

**When:** movement admission. **Gap:** how to establish that standing room is actually reachable.

For a truck behind a tank, room to park does not prove that its approach works. The check runs the same steering, reversing, collision, infantry yielding and route search that the live battle uses. For a selected group, all accepted members move together during that check. If one replacement move fails, the others are checked again with that unit holding still, so they cannot rely on a rejected unit moving away.

**Reach:** future movement changes automatically affect admission. No persistent second route plan or new dependency is introduced. **Verdict:** sound because one movement authority avoids conflicting definitions of a valid move; **confidence:** high.

### Project pending orders and check finite queued journeys

**When:** queue and upgrade integration. **Gap:** previews must describe the order that will actually be applied, including commands already accepted this tick.

Suppose a truck is already ordered to leave, then receives Stop, and a jeep is ordered through its position. The check first applies those pending commands to its copy, so the jeep cannot depend on the stopped truck vacating. A queued move is checked after its existing finite waypoints and automatic garrison exit. An indefinite attack has no known completion, so a destination queued behind it cannot be promised. Attack-move legs are checked for physical travel; combat may still pause them later.

**Reach:** command application remains the shared owner of replacement, deployment and queue semantics. Changing a route policy must validate its changed queues before replacing them. **Verdict:** sound because order timing is part of reachability; **confidence:** high.

### Reject without leaving a misleading marker

**When:** command and presentation integration. **Gap:** the old failure behavior did not distinguish a failed standing slot from an unreachable journey.

A failed replacement move holds that unit and produces no destination marker. A failed queued waypoint keeps earlier waypoints and adds no new marker. A failed route-policy upgrade keeps the previous accepted policy. Partial groups retain markers only for members whose moves pass.

**Reach:** the command contract adds `NoValidDestination`; preview requests include route policy and queued intent with defaults. The browser filters failed placements in held previews, confirmations and committed geometry. **Verdict:** sound because every visible destination has the same admission meaning; **confidence:** high.

### Use only the ordering side's knowledge, without advancing the battle

**When:** isolated planning state. **Gap:** checking against full live state could reveal hidden units or mutate the actual battle.

If an unseen enemy truck sits at the clicked point, a preview must look the same as a preview without that truck. The copy uses remembered hull locations and currently observed enemy soldiers, retaining their original member indices. It excludes private enemy orders and unobserved bodies. Observed opponents cannot generate new cover positions from their private spotting knowledge: the movement check retains its own knowledge owner and supplies an empty opposing owner. Mutable props, forest bookkeeping and navigation work are isolated; immutable terrain is shared. Known ground damage still affects travel. Only the ordering side’s deployment and garrison timers count as timed progress; an unseen enemy’s setup cannot spend the retry allowance and suppress another group member’s marker.

**Reach:** preview calls do not change future digests or replay scheduling. Newly discovered obstructions can affect execution after issuance, just as later orders can change traffic. Forest journal cursors must be rebased when their private history is reset. **Verdict:** sound because side knowledge and deterministic query neutrality are existing contracts; **confidence:** high.

### Keep realistic rolling turns while separating parking clearance

**When:** physical movement. **Gap:** turning radius was also acting as a stopping gap.

The supply truck now uses the same generous wheeled turning radius as the jeep. It can reverse to make room and turn while rolling; it does not pivot in place like a tracked vehicle. Parking behind a tank is judged by hull clearance, not by reserving a full turning circle between the vehicles. Predicted turn arcs and actual collision use the same hull rule. Small wheel movements without progress still trigger traffic recovery.

**Reach:** vehicle tuning stays in the catalog; hull collision remains authoritative. Infantry yielding retains its existing behavior and needs no move order. **Verdict:** sound because these are physical properties rather than special cases for a truck or tank; **confidence:** high.

### Identify every asynchronous preview by its complete intent

**When:** browser integration. **Gap:** a result for a previous cursor, heading or queue mode could remain visible.

When a player changes the cursor or begins a Shift-queued gesture, a reply for the previous request cannot paint the new intent. Gesture identity includes destination, facing, direction, route and queue mode; the battle tick requests a refreshed check. Only a resolved result for that intent controls destination markers. Release checks admission again against current conditions; unavailable moves use the existing rejection feedback.

**Reach:** presentation cannot invent a valid point while waiting for authority. Existing visual language is retained. **Verdict:** sound because stale validity is misleading validity; **confidence:** high.

### Keep build caches local to each checkout

**When:** browser verification. **Gap:** shared installed dependencies also shared Vite's default writable cache.

Two checkouts may use the same installed packages, but their sources differ. The lab's Vite cache now lives in each checkout's ignored scratch directory, preventing one checkout's output from overwriting another's. **Reach:** dependencies remain shared while build output is isolated. **Verdict:** sound under the repository's worktree contract; **confidence:** high.

### Change development contracts directly

**When:** wire and configuration integration. **Gap:** compatibility requirements were unspecified.

The preview request gains optional route and queued fields, with shortest-route and replacement defaults. The navigation rules gain a positive movement-validation work allowance, and admission gains a new error variant. Existing authored rules are updated directly. No saved-state reset, compatibility wrapper, new dependency or persistent battle-state field is added.

**Reach:** callers migrate together through the Rust, Wasm and browser boundary. Rule changes may change battle digests; previews themselves may not. **Verdict:** sound for this unshipped development contract; **confidence:** high.

### Share the marker's approach heading with publication

**When:** rendered preview review. **Gap:** physical arrival does not imply that a squad's body angle equals the ordered route heading.

A squad walking diagonally can finish with a slightly different body angle because its soldiers settle around their assigned positions. Showing that final body angle in the preview and the route's approach angle after release makes the arrow jump despite the same order. Both surfaces now use the existing movement owner's heading calculation. The private check remembers the first planned approach of the final queued leg, while completion still requires physical arrival.

**Reach:** no new persistent facing field or independent heading formula is introduced. Normal routing and combat can refine facing later. **Verdict:** sound because presentation must give the same meaning to an order before and after release; **confidence:** high.

### Preserve the requested endpoint through a traffic detour

**When:** simultaneous bridge groups. **Gap:** the route search could replace a temporarily occupied endpoint with a nearby grid point.

A rear jeep can need a detour while a teammate is still moving through its future parking area. Previously, finishing that detour could empty the original order three metres away from its marker. A traffic detour now retains the requested endpoint as its final waypoint. Collision still controls the approach: the jeep waits or replans while blocked instead of declaring arrival elsewhere.

**Reach:** this is a general movement rule, independent of unit names or group size. The simultaneous opposing-column regression keeps eight starts, the original arrival distance and time limit, and physical separation/water/deck checks. Its two group orders use clear destination areas and retain each column's arrangement; the old individually swapped destinations were occupied at issuance. **Verdict:** sound because routing around traffic must not change the meaning of completing an order; **confidence:** high.
