# Contextual game cursor and building orders

The arrow describes the useful pointer action (left-click confirms armed attack ground; right-click cancels it), using the ordering side's knowledge. Ordinary movement has a plain arrow. Other actions add a small badge beside the same fixed arrow tip, following the user's approved **Arrow + action** direction.

A building click expresses one intent for the complete selection: a fresh entrant is the squad with the shortest reachable navigation route, and its selected companions gather on the group's approach side, including cars and tanks. Compatible existing holds and entries are considered before fresh candidates, subject to earlier queued work and bounded proof. An unavailable entry becomes nearby movement. Useful parts execute when others fail; failed replacement movers hold, while failed queued movers keep their earlier work.

## Why the simulation chooses the whole group

Distance to a wall can conceal a long detour. Choosing a squad in the browser, then sending separate entry and movement commands, would let the first command change the conditions used by the second. Entry and gathering therefore share one admission, one physical proof and one acknowledgement. The browser submits intent and observes its result.

“Shortest” uses the game's existing navigation resolution and exposed-facade samples. It is not a continuous-space shortest-path solver. An unfinished route whose lower bound could beat the completed winner, an unknown finite predecessor endpoint or unfinished physical entry proof cannot justify choosing a different entrant. The planner preserves useful gathering and reports uncertainty when the entry ranking cannot be proved within its finite work allowance.

The durable seams are [OccupyBuilding and BuildingPlacement](../../../crates/contract/src/command.rs), [Battle::preview_building and admission](../../../crates/sim/src/battle.rs), and [garrison::occupy](../../../crates/sim/src/garrison/occupy.rs). [Movement certification](../../../crates/sim/src/movement/certify.rs) owns actual travel; [formation](../../../crates/sim/src/formation.rs) owns footprint spacing. Authored single-squad `Garrison` remains a separate real operation used by scripts and labs.

## Promises that must survive later changes

Preview is read-only. It cannot issue a command, advance a tick, consume command gesture tokens or change later battle/replay outcomes. The planner uses known geometry and own occupancy/reservations. Hidden enemy occupants and unseen collapse retain the existing discovery-on-arrival behavior. A static map hit nominates a building aggregate; it does not decide whether that building admits entry.

Fresh entry commands movement to the exact proved approach before invoking the existing garrison operation; ordinary squad spread, cover and arrival tolerance still govern physical positions. A direct garrison order may begin near another wall; relying on it alone would skip the route that earned the new entry promise. Entry and gathering are certified together. Failed replacement movers hold; failed queued new moves leave earlier work intact, and their continuing finite prefixes remain in the proof. The failed set must stabilize before surviving destinations become promises.

A replacement gives a selected squad physically inside or entering the requested building priority and reasserts its hold when proved, superseding an old departure. An outside deferred entry reservation competes afresh. Shift respects earlier executable work, including a departure before re-entry; compatible armed holding work stays intact. The prepared private entry decision travels from admission to application so application cannot reinterpret the queue differently.

Combined building planning uses the tick before application, consistently for live commands, authored scripts and replay. A readonly preview uses its current tick. This matters because finishing a queued garrison exit can depend on deterministic tick-seeded placement.

[Native group-order tests](../../../crates/sim/tests/garrison/occupy.rs) pin physical arrival, queue compatibility, uncertainty and per-tick replay. [Building tests](../../../crates/sim/tests/buildings.rs) pin aggregate identity and hidden-state independence. [Authority tests](../../../web/tests/authority.test.ts) exercise the real WASM query without command or publication side effects.

## Why one pointer intent matters

Hover, right-press, held preview and release share [pointerIntent](../../../web/src/battle/input/pointerIntent.ts). Hover describes an order without consuming it. The captured press retains its target, selection and modifiers; dragging adjusts facing. Observed casualties are pruned, expired contacts are refused and a client reset cancels the old press.

[The shared battle session](../../../apps/battle-lab/src/useBattleSession.ts) owns merged picking and pointer lifetime. [PointerPaint](../../../apps/battle-lab/src/pointerPaint.ts) owns one coalesced preview request. A changed semantic intent clears old feedback; publication ticks refresh the same intent without making a moving battle perpetually pending. Known geometry includes both remembered structures and the separate cleared-ground stream: felling trees can change navigation without changing remembered props. Clearing count and epoch belong to preview identity; an epoch replacement can change cleared cells even when its count stays equal. Release retains the requested geometry until acknowledgement, then the authoritative result owns confirmation until its applied publication. A different hover cannot inherit its cursor classification.

Initial pending state and wholly unsuccessful unproven results use the plain arrow; proved successful parts take priority over uncertainty. A successful entry earns the garrison badge even when some companions fail; fallback movement stays plain. Known zero-success building results and known capability refusals earn the rejection badge. The older movement-array query lacks a certainty diagnostic, so an all-unplaced hover result stays conservative rather than claiming a proved restriction; an actual admission refusal can still show blocked confirmation.

Held building previews show companions' gathering destinations, while the entry result drives the garrison badge. Attack intents show no held movement marks. [The game cursor](../../../web/src/battle/present/gameCursor.tsx) composes the arrow and the existing generated action glyphs into one image per action, and the system draws it as the pointer. It therefore moves independently of battle frames, so rendering and targeting work never postpone its position; action classification still follows the frame's observed intent. The game arrow is the app's only pointer: every page and element shows it, never the browser's own arrow or hand. [AppCursor](../../../web/src/battle/present/gameCursor.tsx), mounted once by the [app shell](../../../apps/battle-lab/src/AppShell.tsx), owns which image is shown; pages only choose its action. [LabViewport](../../../apps/battle-lab/src/LabViewport.tsx) reports the battle's action over the canvas and readout cards, and the plain arrow everywhere else, including HUD controls, menus, camera drag, cancellation and disposal. Setting the cursor on every element, rather than per surface, means no element boundary can flash a hand.

## Rejected approaches

Treating any future garrison order as an already-entering squad could preserve unreachable reservations and defeat a replacement click. Treating only the queue's front, or stopping at an attack, as compatible could miss a later departure: contact attacks can expire and expose that departure. Retention follows physical state and compatible queued work instead.

Returning a longer completed entry after a shorter proof exhausts its work would make uncertainty look like a shortest-route answer. Entry uncertainty must survive fallback and veto an unjustified winner. Separately proving an entrant and companions would allow a failed tank to invalidate entry; joint stable proof prevents that dependency.

## Visual provenance

[The approved concept](assets/arrow-action-approved.png) is the user's selected illustrative mockup. It drives the stable tip, bright thin arrow, dark edge and small lower-right action badge. Its schematic terrain and illustrative icon family are not gameplay requirements; production uses the game's generated icons and HUD colors.

The current size requirement is 70% of the original complete cursor, retaining its tip and action layout. The captures below preserve the original presentation scale; current browser scenes verify the reduced arrow, badge and callout clearance.

The frozen [before](assets/before-garrison.png) and [after](assets/after-garrison.png) images use the real playable garrison route at matched tick, camera, selection and battle digest. The before image is a historical baseline; the approved concept is the design target. [The action matrix](assets/native-matrix.png) and [narrow matrix](assets/narrow-matrix.png) preserve native-scale contrast comparisons, while [partial refusal](assets/partial-callout.png) and [narrow refusal](assets/narrow-callout.png) preserve the cursor/text separation requirement.

The narrow gameplay images also expose the pre-existing command bar overflowing the right edge; its layout and styles are unchanged by this feature. The narrow acceptance here concerns the complete cursor and refusal text, not responsive command-bar support.

Agent decisions and their confidence live in [choices](choices.md). The implementation adds no dependency, game tuning knob or persistent unit state.
