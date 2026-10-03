# Contextual game cursor and building orders

The cursor describes the action a right-click can carry out using the ordering side's knowledge. A building click is one combined intent: the squad with the shortest reachable entry route enters; the other selected squads and vehicles gather outside. Useful parts execute even when other parts fail.

## Next Agent Prompt

Status: implementing; updated 2026-10-03. Building authority's reviewed checkpoint is integrated; follow-up native privacy, finite-queue and replay proofs remain. Cursor presentation is complete. Real WASM/authority/client preview purity and error recovery pass, and typecheck is green. Next complete [pointer integration](slices/03-pointer-integration.md) across viewport/session, held/accepted preview and gameplay scenes while the native follow-up runs. Use implement-spec: committed passes, review and choices audit, then continue until every decision and gate below is fulfilled. Update this prompt before ending each pass.

- [ ] Authoritative combined building order, preview and native proof.
- [x] Approved cursor presentation and isolated browser proof.
- [ ] Shared hover/held/release intent, wire integration and played browser proof.
- [ ] Whole-feature review, one full check/verify closeout, consolidated choices and close-spec.

The graph is `01 || 02 → 03 → closeout`. The simulation's bounded route search and joint certification are the first risk to resolve. Scratch evidence belongs in ignored `throwaway/`; the approved concept below is a lasting requirement.

Evidence: 37 native garrison tests and focused contract/sim/WASM lint passed at the checkpoint; multipart nomination is invariant, bounded search is fair, failed-held reproof is stable, and forest clearing no longer invalidates scratch navigation. Browser authority/client tests pass (19), including mixed preview-to-admission digest and replay equality, query-local failure recovery, and no command/publication side effects. Input/gesture tests (30) and isolated cursor/component/fixture proofs passed earlier with fresh visual critique. The CLI second-opinion tool cannot run with its configured model/account; independent collaborator reviews supply the code opinion without overriding user model settings. Use the scene runner's regular Chromium channel for GPU captures, not Playwright's default headless-shell variant.

## Known knowns — grounded territory

The simulation is the authority and presentation observes one side. [Player input](../../web/src/battle/input/useUnitControl.ts) chooses existing command precedence; [command reach](../../web/src/battle/input/commandReach.ts) owns capability filtering. The current [garrison validation](../../crates/sim/src/garrison.rs) admits one squad whole, and [its regression](../../crates/sim/tests/garrison.rs) rejects an order naming several squads. The feature changes the player intent rather than building capacity.

[Formation placement](../../crates/sim/src/formation.rs) already spaces footprints behind a front-center anchor. [Battle movement preview](../../crates/sim/src/battle.rs) and [movement certification](../../crates/sim/src/movement/certify.rs) own physical travel, side masking, earlier pending commands and queued prefixes. Reuse these owners; a browser route approximation would be a second authority.

[Pointer paint](../../apps/battle-lab/src/pointerPaint.ts) already coalesces one in-flight query and guards changed gestures. [Session picking](../../apps/battle-lab/src/useBattleSession.ts) merges drawn bodies and readout hits. Extend those owners rather than creating cursor-only picking or a second async manager.

## Known unknowns — decision ledger

| Decision | Answer | Attribution / reason |
| --- | --- | --- |
| What the cursor promises | Accepted action plus restrictions known to this side | User: choice A; avoid predictable refusal and information leaks |
| Multiple squads at a building | Choose one entrant; move every other selected unit nearby, vehicles included | User correction; preserve the useful positioning intent |
| Entrant ranking | Shortest reachable entry route, stable unit-ID tie | User: route choice B; tie answered by agent on user's behalf for predictability |
| Garrison unavailable | Gather all selected units outside and show ordinary movement | User: fallback A |
| Gathering location | Group's approach side, entry path clear | User: placement A |
| Partial failure | Execute valid parts; failed replacement movers hold, failed queued moves preserve prior queue | User: partial-success A; existing move contract supplies queue semantics |
| Default move appearance | Plain arrow, no attached move icon or word | User correction |
| Visual direction | Arrow + action | User explicitly selected the rendered concept |
| Remaining routine decisions | Agent may apply useful intent, actual reachability and partial success; disclose each call | User explicitly opted in; visual taste and conflicting tradeoffs remain outside delegation |

Agent answers already disclosed: Shift queues both entry and gathering; a vehicle-only selection gathers with the plain arrow; equal route lengths break by unit ID. Additional delegated answers are recorded in [choices](choices.md) and disclosed during implementation. No compatibility shims or data migrations are requested; keep the existing direct one-squad garrison command because authored scripts and encounter commands still use that distinct operation.

### Building intent contract

Add `Order::OccupyBuilding { units, building, gesture, facing }` and a read-only `BuildingPreviewRequest { units, building, facing, queued }`. Queueing for committed intent remains in the existing command envelope. `BuildingPlacement { building, entrant: Option<BuildingEntry>, destinations, unproven }` reports the chosen unit and exact entry approach, plus every gatherer's `MoveDestination` including failed placement. The optional `unproven` flag distinguishes unfinished bounded proof from a known restriction when no part succeeds. `BuildingEntry { unit, approach }` is the prepared entry. The acknowledgement retains its ordinary `placement` for gathering confirmation and adds the building result. WASM exposes `preview_building`; the browser's client/authority transport mirrors this typed contract.

Admission resolves the complete intent once, retains its outcome until application and records intent for replay. The browser must never send a winner computed during hover as authoritative. Direct one-squad `Garrison` remains a real simulation operation; both use the same geometry and validation owner. The group plan lowers to existing per-unit garrison/move queues, with no new persistent battle state or browser command fan-out.

Use only side-known geometry and own occupancy/reservations. Enumerate exposed spans at the navigation owner's resolution, including endpoints and projected nearest points; a blocked nearest projection must not exclude the rest of a facade. Rank completed shortest-policy routes by physical path length, then unit ID, then stable geometry order. Shortest means the game's navigation route, not travel time or straight-line distance. Share finite counted work across the selection and candidates; never call the unbounded tools/test planner in production. Budget exhaustion is unproven availability, not proof of no possible route.

Assess queued actions after executable finite predecessors using the existing movement owner, including garrison exit and earlier same-tick own commands. An indefinite attack has no promised endpoint and cannot prove queued entry. Querying cannot change caches, navigation charges, digest, orders, or later replay outcomes.

The group centroid-to-building direction defines the approach side, using predicted origins for queued planning. If the centroid is degenerate, use the chosen entry's exterior direction; without an entrant, use the first unit in stable ID order. Anchor outside the exposed facade using physical footprint clearance. Reuse formation's rear-side placement and reserve the entrant's final physical approach corridor. No surround or threat-dependent tactical flank is implied. Jointly certify entry and all gathering travel; units whose movement fails still occupy their holding positions. If the chosen entry is blocked by a holding member, try the next ranked feasible entrant; if none works, fall back to gathering all.

If a selected squad already holds or is entering that building, keep its entry/hold and gather the others instead of evicting it. A friendly nonselected holder or own reservation makes entry unavailable. Hidden enemy occupancy is not an initial failure: preserve discovery on arrival and the existing abandonment behavior. Known destroyed/ungarrisonable building geometry falls back to nearby movement using known geometry, never the secretly changed world.

### Pointer and cursor contract

One pure resolver owns the existing precedence: Ctrl attack-move; identified enemy; armed contact attack; contextual building; armed ground command; automatic single-vehicle reverse; normal movement. Hover does not mint gesture tokens, reset armed mode or issue orders. Capability filtering stays in commandReach and double-click recognition stays in MoveGestures. Explicit garrison mode uses the same combined building intent as a contextual click.

One merged semantic pick feeds hover, captured right-press, held preview and release. Press captures target and modifiers; dragging changes only facing about that target. Ctrl, Shift, mode or selection changes before a new press update hover immediately. Building-facing drag rotates gathering, without changing the target building. Ctrl over a building stays attack-move; hostile body/contact precedence stays unchanged. No move preview may appear for an attack or entry intent.

The cursor actions are plain/default, attack, attack-move, attack-ground, garrison, fast-move, reverse-move and blocked. No selection and selectable own-unit hover use the plain arrow. Unarmed enemy-target intent that cannot issue an attack is blocked; an unarmed contact falls back to movement as today. Blocked applies only when no part of the resolved action can execute. Partial success shows the successful action; existing refusal text reports failed portions. A pending or unproven query uses the plain arrow, not a fabricated validity badge.

At most one authority preview query is in flight. Identity includes side/client epoch, complete selection and mode, modifiers, press generation, target and relevant side-known geometry/eligibility. Changed identity immediately clears obsolete badges and marks. Intermediate updates coalesce. Same-intent latest results remain visible while publication ticks refresh; do not invalidate every tick and starve a running battle. Admission resolves current-state races. Matching acknowledgement owns release confirmation until its applied publication, including fallback movement and per-unit failures.

Use one screen-space DOM cursor overlay with a fixed arrow-tip hotspot, updated through the viewport's current pointer rather than per-mousemove React state. This makes actual production cursor captures possible without a new renderer pass. Hide the native pointer only on battle interaction surfaces; menus retain their own controls. Pointer exit/cancel, camera drag, reset, input-disabled replay/script and disposal clear action feedback. A stationary pointer re-resolves when camera, selection, modes or known geometry changes.

## Unknown knowns — visual taste and environment

The user chose a stable arrow with a small lower-right action badge over reticle, changing silhouette and word variants. The user's default-move correction removes redundant action decoration. The game runs in the existing desktop browser battle and shared playable labs; the consumer is a player commanding mixed selections, and completion means the displayed action agrees with actual orders.

![Approved Arrow + action concept](assets/arrow-action-approved.png)

This is the user's approved illustrative concept, not a gameplay screenshot. Its schematic scene and Lucide icons are not requirements. Preserve the arrow silhouette, precise stable tip, small lower-right badge, thin bright strokes and dark edge, plus readability over grass, road and fog. Production uses the existing generated game icons and fixture HUD colors; no Lucide dependency. Base arrow is approximately 32 CSS pixels, badge 19, offset 22 right / 19 down, with complete composite bounds and a fixed tip hotspot. Scale remains in screen pixels at default and far camera zoom.

## Unknown unknowns — landmines and closure

| Evidence | Why it bites | Resolution |
| --- | --- | --- |
| `garrison::approach` chooses geometric-nearest exposed projection | A nearer obstructed approach can defeat reachable entry elsewhere | Decided: navigation-resolution candidates and actual shortest-route ranking |
| `BattleView` held preview always builds a move in normal mode | Preview can disagree with attack/garrison on release | Decided: shared semantic intent for all consumers |
| `buildingUnderRay` uses authored static map | Known destroyed buildings can nominate stale entry | Decided: treat pick as nomination; authority resolves remembered state and aggregate owner |
| `ViewportPointer` lacks hover Ctrl and captured full intent | Modifier changes can show the wrong action | Decided: extend pointer snapshot and preserve press semantics |
| Independent entry and gathering proofs | Gatherers or failed members can block entry | Decided: reserved corridor and joint physical certification |
| Same-tick commands and unfinished queues | Current positions are insufficient planning origins | Decided: reuse pending/queued projection; indefinite predecessors stay unproven |
| Hidden enemy occupants and unseen collapse | Availability can accidentally reveal the opposing side | Sharp edge: existing knowledge/discovery contracts must remain intact |
| Tick-only async refresh | Dropping every old-tick result can starve continuously advancing play | Decided: invalidate semantic changes; refresh same-intent certificates |

The sweep covered the command contract, garrison geometry/validation/application, navigation search, formation/certification, WASM, worker/client protocol, input/capability/gesture owners, shared picking, viewport, battle view, pointer paint, publication and icon presentation. Next-slice implementation must confirm bounded candidate search and queued-origin behavior with red/green tests before depending on them. No user-only question remains open.

## Proof and closeout

Invoke write-tests before behavior edits; one red/green tracer at a time. Existing commands retain digests/replays; the new explicit combined order is the named outcome change. Native proof owns entry, placement, hidden-state independence and replay; browser proof owns real hover/press/release, modes, stale replies and displayed/issued agreement.

Every visual slice compares actual cursor crops to the approved reference and a same-route prior frame, uses compare-screenshots, gets unprimed screenshot-critique as its last visual acceptance check, and opens the relevant shots using preview-shots. Production icons may differ from illustrative symbols; do not exempt silhouette, hotspot, badge spacing or legibility. Human review is nonblocking: show evidence, continue independent work, and decide reversible details from evidence after a reasonable response window.

Use narrow tests and the affected garrison/movement scenes during passes. Run the complete check and all browser scenes once at finished implementation; share the machine's GPU lock. A generic balance report cannot exercise a new player command absent from its scripts: native played command fixtures are the relevant rule sample; existing battle digest parity demonstrates no change to prior scripts. No weapons or unit tuning belongs in this feature.

Copyable kickoff: “Implement specs/game-cursor with implement-spec, preserving the approved Arrow + action cursor and the complete building-order contract. Complete every slice, review, verify and close the spec.” Implementation is already authorized by the user's latest message.
