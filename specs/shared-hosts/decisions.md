# Shared hosts: decisions ledger

Settled decisions, with why and what lost. Sources: the 2026-11-16 unknowns
walk with the user ([map](unknowns-map.html)) and four independent plan drafts
(fewest slices, risk first, seam quality, determinism) synthesised into the
[README](README.md). Add new entries at the bottom as implementation decides
things the plan left open.

## From the walk

- **Several squads share a host; overflow ends up outside** (user, Q1). The
  same for buildings and transports: the user widened the question from
  buildings to both hosts.
- **Seats are taken when a squad starts stepping in, never at the command**
  (Q2; agent on the user's behalf, then locked by the user's relief remark).
  First to arrive gets in. At the command only a squad bigger than the whole
  host is refused. *Why:* the user's own example, "I can have one squad walk
  over, then tell the other squad to leave, and by the time the other squad
  reaches it'll be empty. This is GOOD, it's a feature." Lost: promising seats
  at the command, which would refuse that manoeuvre.
- **A squad arriving to no room ends its order with `CapacityFull`, never
  waits** (user, Q3). The player hears why. Lost: a waiting phase (deleted in
  September; stays deleted).
- **One building order, `OccupyBuilding`** (territory, Q4). `Order::Garrison`,
  `OneSquadPerBuilding`, `BuildingOccupied` and companion gathering are
  deleted; the defender sends `OccupyBuilding`.
- **Window swaps span every squad in the building** (user, Q5). The AT gunner
  takes the window facing the tank whoever's rifleman held it. Lost: per-squad
  windows split at entry.
- **The start check ignores squads stepping out; the end check counts
  everyone** (user, L1). Equal in and out timers per host mean a squad that
  began leaving before you arrived is gone before you finish stepping in.
  Lost: leavers keeping their seats (makes the relief manoeuvre a timing
  trick).
- **One ring per house** (user, U1): the same ring whichever occupants are
  selected; it means "in this building", not a count.
- **The IN BUILDING row shows seats used/total** (user, U2), like the carrier
  Seats row.
- **No full-host cursor warning** (user, U3). It would discourage the relief
  manoeuvre.
- **The relief manoeuvre is a required end-to-end test** (user).
- **Labs show the new rule** (user, U4): two squads share, a third arrives to
  "no room", in the garrison lab and the transport controls fixture.
- **No backward compatibility, no migrations** (user). Hard cutover.
- **Reinforcement needs a free seat in the host**, both hosts (shown, card 2).

## From the drafts

- **Entries use the old `Garrison` lowering; the proof machinery is deleted.**
  Each listed squad's approach is `garrison::approach` from its queue's end (or
  where it stands), with the same reach the old order used; applying the order
  pushes `UnitOrder::Garrison { building, approach }` exactly as the old order
  did. A squad is kept if it is within entry reach or the side's grid reaches
  its approach. `movement/certify.rs`, the candidate search and `unproven` go.
  *Why:* `Board` has no proof, so this is the consistent rule; the defender's
  entries stay byte-identical by construction (the menu reel); and the old
  proof replays the battle and requires the entrant to end up inside, which
  the relief manoeuvre would fail. *Alternative (three of four drafts):* keep
  the bounded proof, route each squad, and change its success to "reached the
  building". Lost because it keeps 25 KB of machinery built for the companion
  plan and risks changing the defender's lowering. If grid reachability gives
  wrong answers for buildings behind rows, stop and reslice slice 01.
- **Window-change time stays per window, on the shared plan** (three drafts;
  corrects the map's card 3). Moving it onto soldiers changes the digest of
  every scene with a window swap, including empty windows' history.
- **The shared plan lives on `Battle`, keyed by the building's structure
  owner**, created on the first admission and dropped when no living squad
  holds the building. The digest folds it inside each occupant's hold digest
  exactly where `digest_place` folds it today. *Alternative:* an `Arc` shared
  between holds (one draft), lost because the per-window change time needs
  mutation.
- **No new selection interaction** (determinism draft). Overlapping rings keep
  today's nearest-centre pick; soldiers and deck cards still select single
  squads. *Alternative (three drafts):* clicking a held house selects every
  own squad inside. Lost to the scope firewall; revisit only if the user asks.
- **Collect refuses at the command only a squad bigger than every listed
  carrier.** Its overflow is decided on arrival. This also fixes a latent bug:
  an overflowing Collect today refuses whichever squad finishes its timer
  first, because later squads' reservations already count.
- **The building's end-of-timer failure now publishes `CapacityFull`** (it
  drops silently today). An enemy inside stays a silent give-up.
- **Riders are never reinforced** (`Unit::on_field` excludes them), so
  reinforcement only touches buildings and squads stepping in beside a
  carrier. At full capacity, supply spends neither stock nor a soldier id.
- **The building seat total is published on `HostRef::Building`; used is
  summed client-side** with the same phase rule as the carrier Seats row.
  Enemy panels gain no capacity.
- **`OccupyBuilding` goes only to infantry**, through the existing
  `commandReach` predicate, so a mixed selection doesn't refuse the whole
  order.

## Slice 00

- **The relief test sends `OccupyBuilding` in its wire form** (JSON, no
  `facing`), so it compiles unchanged through slice 01's contract cut. Today
  that order is accepted and B is parked as a companion; the slice's "refused
  with `BuildingOccupied`" holds only inside `garrison::occupy`'s entrant
  search, not as a published refusal (slice 00, Done).
- **The test sets its own bound**: `buildings.capacity_soldiers` is set to
  two squads less one soldier, read from the squad's size, and the seat total
  comes from `seats_for_state`. It asserts the scene seats either squad but
  not both before anything else.
- **"Seats taken never exceed the total" is asserted two ways each tick**:
  soldiers of squads stepping in or inside (the start check's count), and
  soldiers inside or stepping out (those at windows).
- **The probe plays every shipped encounter**, not only the defender ones the
  slice lists, because `UNCHANGED_OTHER_SCENES` means every encounter but the
  contested two. Rules follow the map's category (test maps: the game's set
  plus the test units, as labs run; capture maps: the capture set), seed 1,
  a fixed tick count. The menu scenes therefore run seed 1, not the reel's
  seeds; the reel's own event-log tests cover those.
- **Single-host building cases mirror existing tests** (entry and exit with
  re-entry, `an_atgm_gunner_trades_windows…`, the garrisoned reinforcement in
  `supply.rs`, the `shelled` collapse); `transport/board` (`Board`, `Collect`)
  and `transport/heli` are the single-host carrier cases.
- **A single-squad entry is sent as `garrison` while that order exists, else
  as `occupy_building`.** Slice 01's claim is that the second lowers exactly as
  the first, and `UNCHANGED_SINGLE_HOSTS` checks it. Each run's `INDEX` says
  which form ran.
- **Contested demos**: garrison lab, rifle squads ordered in at ticks 30 and
  330, scouts at 900; transport controls, both squads `Board` the APC at
  tick 1. Both scenes need reshaping in slice 07 (slice 00, Confirm first).
- **The commit key ignores changes under `crates/*/tests/`**: tests never move
  an outcome, so a run with only test edits keys as the clean commit.

## Slice 01

- **Grid reachability answers for a building behind a row.**
  `NavGrid::reaches` (terrain connection plus the side's known pockets) on a
  copy of the side's grid keeps the row-of-buildings test green; the planner
  was not resliced. It calls open ground reachable even from a boxed-in start
  ("the route search has the last word"), so a walled-in squad still gets an
  entry and finds out on the way, as a `Board` would.
- **Entries come out in unit order, each listed squad once,** so a preview
  doesn't depend on selection order.
- **A queued order sets off from the queue's last movement end** (a move's
  destination or an entry's approach), with the side's orders accepted this
  tick applied first (`Battle::move_source`), as move placement does. A
  replacing order sets off from where the squad stands; that is the old
  `Garrison` lowering exactly, so the defender's entries are unchanged.
- **The clicked building resolves as the old occupy did:** the clicked part
  as the side knows it, else another remembered part of the same building,
  owned by its remembered structure owner.
- **A vehicle, or a squad bigger than the whole building, refuses the whole
  order** at the command (`NotInfantry`, `CapacityFull`), as `validate` did
  for one squad. The client only sends squads (`commandReach`), so a mixed
  selection never meets it; slice 03 turns the size check into per-squad
  refusals.
- **A squad with no entry is left as it was** (the old arm's `continue`); an
  order where no squad gets an entry is acknowledged `NoValidDestination` and
  applies as a no-op, like a move nobody can make.
- **`gesture` stays on the wire but the simulation reads nothing from it:**
  `UnitOrder::Garrison` carries none, so `UpgradeMove` never touches a
  building order.
- **The planning-snapshot machinery went with `certify.rs`:**
  `WorldGeometry::planning_snapshot`, the `PropStore` overlay,
  `SideGeometry::beliefs`/`planning_world`, the forest's clearance reset, the
  route planner's per-request charges and `Battle::movement_context` had no
  other reader. The probe stayed identical through their removal.
- **`navigation.move_validation_work` and `move_rehearsal_m` now have no
  reader.** Removing them changes the rules identity and re-records every
  fixture that embeds the rules, which this slice's digest gates forbid; they
  go in slice 07.
- **A selection with no squad over a building moves to the ground point**
  (a helicopter included). The sim test of a helicopter hovering by the
  entrance went with companion gathering.
- **The preview draws each entering squad's destination ring at its
  approach, pointing the way it walks;** the accepted order then draws one
  line per squad. Seen in the lab shot (three squads on one side of the
  house): they share one approach, so the rings coincide (two radii for three
  squads), sit half under the house, and the three order lines chain through
  each other. That is `garrison::approach`'s nearest point, kept by the
  binding lowering decision; an unprimed critique judged the shot clearer
  than the old companion rings in the field. Not fixed here; revisit with
  slice 06's marks if it reads badly in play.
- **The publication vectors changed only in the refusal-name table.**
  `codec-vectors.json` was re-blessed and its diff is `layout.orderErrors`
  alone. `fog-sight-vectors.json` was edited by hand to drop the two names
  from `streetLayout`: its test already fails at slice 00's commit
  (paris-corner frame 0, the eyes upload), and its bless would also
  re-record every run and the street stream and layout's drifted `groups`
  and `propKinds`. That failure is open and unrelated to this plan.

## Slice 02

- **Confirmed first:** nothing recomputes a plan after entry. `seats_for_state`
  is otherwise called only for capacity counts (`validate`, encounter posts).
  The readers were the listed ten plus `battle/flown.rs` (a `DamageContext`)
  and `battle/state_digest.rs`; `carriage` reads only `entry`.
- **Readers are methods on `Windows`** (`seat`, `body_position`, `free_seat`,
  `faces`, `changing_window`, `facade_eyes`, `allocate`, `digest_place`), not
  free functions taking `&Windows`. Each finds a squad's plan through its
  hold's building, and only while the squad is inside or leaving, as the old
  per-squad copy was empty while it stepped in. `seat_evenly` takes the plan
  itself because it runs before the plan opens.
- **`&Windows` rides on `FireContext` and `DamageContext`.** Elsewhere it is
  a parameter: `sensing::{eyes, evaluate, side_sees_round}`,
  `SideKnowledge::identified`, `supply::service`, `Unit::digest`,
  `hold::{digest, advance}`. `Battle::windows()` exposes it to tests. The fire
  step builds its context twice (a local macro), because allocation changes
  windows between choosing them and firing.
- **Plans close once a tick, at the end of `hold::advance`,** not at every
  place a hold ends (death, collapse, retirement, a crash). A plan whose last
  holder died waits for the next advance; nothing reads it meanwhile, because
  every reader goes through a squad inside or leaving. Opening a plan replaces
  any left over, so a squad seated then starts with fresh window history, as
  before. Slice 03 must keep the replace only for the first squad seated.
- **The digest gates the plan on the hold's phase:** a squad stepping in folds
  an empty plan even when another squad's plan is open for the building.
- **`battle.rs` made room by a split:** each side's sensing pass (`sense`,
  the fog sweep, `footprint_seen`) moved to `battle/seeing.rs`, changed only
  by the windows it now passes. It shares the refactor's commit: the size
  ratchet compares with the previous commit, and `battle.rs` stays over budget
  after the move, so a separate move would leave the refactor's growth
  refused.

## Slice 01a

- **"A squad's spacing" is the soldiers' ground, not the drawn ring.** Two
  entries stand at least the two squads' half spreads
  (`arrangement::spread` of each squad's living soldiers) plus
  `infantry_movement.spacing_m` apart, so their soldiers don't mingle.
  *Alternative:* the formation's spacing (`formation::member_radius`, the
  cover area, plus `formation.spacing_m`), which keeps destination rings
  clear of each other as a group move does. Lost because around a lab-sized
  house no two points are that far apart, so every later squad would fall
  back to the shared nearest point. The drawn rings (cover area at
  `area_draw_scale`) therefore still overlap each other and the house; they
  no longer coincide.
- **A later squad's point must also be reachable.** Its candidate must be
  apart from earlier entries *and* pass the entry's own reach test (within
  entry reach, or the side's grid reaches it), so a spread point on a walled
  courtyard facade never costs a squad the entry the nearest point would have
  given it. The first squad (an empty list of earlier entries) skips both,
  which keeps it on exactly today's path.
- **Sampling** (delegated): the nearest-point candidates `garrison::approach`
  always had, plus points every `APPROACH_STEP_M` along each side-known
  exposed edge's span, standing off it like the edge candidates; points inside
  any part are dropped; the nearest clear one wins, ties by facade order. The
  samples are only built when the nearest point is not clear.
- **Earlier entries are those actually given in this order.** A listed squad
  that gets no entry reserves nothing.
- **Gates.** The probe is identical to slice 02's commit in every group,
  `CONTESTED` included: the garrison lab and transport demos send one squad
  per order. Against `BASELINE` only `contested/garrison/garrison` differs
  (tick 30), as since slice 01.
- **The three-squad shot** (`throwaway/shared-hosts/shots/spread-*`): three
  distinct rings (west, north-east, south-east of the house) and three order
  lines ending at them, where slice 01 had two concentric rings and one
  chained line. The unprimed critique still reads the lines as a chain near
  the squads, because the lab's three squads stand in a row toward the house
  and each line passes the squads ahead of it; that is the staging, not the
  approach. Its other findings (no ring-to-squad link, overlapping rings, no
  markers while walking, identical labels, the panel over a squad, hover
  feedback) predate this slice and belong to slices 06–07.

## Slice 03

- **Confirmed first:** commands apply, then `carriage::advance`, then
  `hold::advance`, then movement; `Host::timing` is one timer for both
  directions. A squad ordered back into the building it is leaving does *not*
  turn back: an entry order is not "wants to stay", so it finishes stepping
  out and walks in again through admission. Only an order that keeps it in
  place (a Stop) cancels its exit, and the tests turn a leaver back that way.
- **The relief test was already green when this slice began.** Squad B walks
  150 m, long after A has gone, and slice 01 had deleted the companion
  parking; the test is un-ignored as a guard. This slice's red-to-green proof
  is the both-hosts test, red at its first command before the code.
- **One function, `hold::admit`, starts every hold**, called from the building
  path in `hold::advance` and both `carriage::advance` boarding sites; the end
  check is `hold`'s `seated`. Both read `hold::Seats::of`. The building path
  stops the squad's movement before admission whatever the outcome, so a
  refused squad stands where it reached the building.
- **"Another side's squad in the building" is any hold on it** (stepping in,
  inside or stepping out) by a living squad of another side. Same-side squads
  share; the give-up stays silent and pops the order.
- **A building's total is its plan's length while a living squad holds it,
  else `seats_for_state` of the standing building.** "First squad seated"
  (which alone opens or replaces the plan) reads the same condition
  (`garrison::held`, which `Windows::close_unheld` now also uses), so a plan
  left by squads that died earlier in the tick never sets the total.
- **Seating takes windows no living soldier of a squad inside or stepping out
  holds**, round the facades in today's order. A first squad sees none taken,
  so single-squad seating is byte-identical.
- **Command answers.** `hold::refuse_oversized` refuses, one by one, a squad
  whose living soldiers exceed the whole host: Board against its carrier,
  Collect against the largest listed carrier, `OccupyBuilding` against the
  building's `seats_for_state` as the side knows it. A refused squad keeps its
  orders and gets no entry. An `OccupyBuilding` whose every squad is refused
  has no whole-order error, as Board doesn't; `NoValidDestination` now means
  no squad got an entry *and* none was refused. Board's duplicate removal
  keeps the order given (no closest-first sort).
- **The building end-of-timer failure publishes `CapacityFull`** (was silent),
  and the old silent "more soldiers than windows" drop is that same check.
- **Tests.** `a_building_takes_one_squad_and_a_second_is_refused_until_it_leaves`
  was already gone (slice 01's contract cut). `squads_beyond_the_seats_are_refused_closest_first`
  merged into `a_squad_boarding_a_carrier_another_squad_fills_finds_no_room`
  (the command refuses nothing; the latecomer hears `CapacityFull` on
  arrival; the first stays in). The building size refusal moved into the
  both-hosts test, leaving `a_squad_smaller_than_the_building_enters_whole_each_soldier_at_a_slot`.
  The enemy-inside test now asserts blue never steps in and hears nothing.
  The both-hosts test turns leavers on the building only (the carrier shares
  `Seats`, `admit` and `seated`); its carrier half lengthens `board_s` itself
  so the third squad of an overflowing Collect arrives while the first two
  still step in. Each claim was falsified once: start check counting leavers,
  no end check, no start check, and seating over held windows each turn it
  red at the matching step.
- **Gates.** Against the branch head before this slice (bfe6b738cc),
  `UNCHANGED_SINGLE_HOSTS` and `UNCHANGED_OTHER_SCENES` are identical.
  `ARRIVAL_ADMISSION_BEFORE_AFTER` moved both contested runs: the garrison
  lab first differs at tick 1231 (the
  scouts, then the second rifle squad, now get in beside the first: the corner
  shop's seats hold all three squads, so its "no room" demo needs slice 07's
  reshaping); transport controls at tick 344 (the AT squad is refused when it
  arrives instead of stepping in and failing at its end check 29 ticks later).

## Slice 04

- **Requesters go in unit order, then today's order within a squad** (its
  mounts in turn, each mount's shooters in member order), not strictly by
  member index. Allocation keeps its per-squad loop over
  `garrison_aims`; buildings never share windows, so one pass over squads in
  unit order is the per-building pass. *Why:* a strict (unit, member) order
  would reorder a lone squad's requests (the rifles before the launcher) and
  move single-squad window fights, which `UNCHANGED_SINGLE_HOSTS` forbids.
- **Who may be displaced:** a living soldier of a squad inside the building
  whose claim, by his own squad's aims, is below the requester's need. A
  soldier of a squad stepping out is never displaced (it keeps its windows
  until it has left) and its windows are not free. No test pins the leaver
  rule; the slice's test list names none.
- **The host's free seat is `hold::Seats::free`**: total less those stepping
  in, inside and stepping out, the same count the end check uses, so a
  replacement can never make a squad fail its own end check. For a squad
  inside a building that count already implies a window nobody holds, so
  supply asks only `Seats`, and `Windows::free_seat` (now host-wide, still
  balancing the squad's own facades) only picks the window.
- **"Checks room when choosing the next need and again when delivering" is
  one check.** `supply::service` chooses the need on the tick it delivers it,
  for each recipient in turn after earlier recipients were served, so a
  second check at delivery could never differ. With no room the need falls
  through (to smoke, or nothing): no stock and no soldier id are spent, and
  the replacement clock pauses, as it does for any ineligible recipient.
- **A squad stepping into a building can be reinforced** while seats remain;
  the replacement stands on the ground beside it and is seated with the
  squad at the end of its timer.
- **Where the tests live.** The new test and the rewritten
  `a_squad_reinforced_during_entry_never_enters_only_partly_seated` live in
  `tests/shared_hosts/occupancy.rs`, a submodule of `shared_hosts.rs` (as
  `garrison/occupy.rs`): with the new test `shared_hosts.rs` passed the size
  budget, and `buildings.rs`, already over it, could not take the rewrite
  without growing. The rewrite is about host seats, not aggregate building
  ownership, which is `buildings.rs`'s subject.
- **The test's casualty is a tank shell with lethality the test sets**
  (`tank_he` damage, and `building_fragment_probability_multiplier` 1): a
  scripted burst wears props but never hurts soldiers, and with shipped
  numbers one fragment cannot kill. The tank shells the gunner's window and
  stops after the first death; the AT team answers and kills it, after which
  service resumes. The rifles' alternating fire is east and west, away from
  the tank, so their rounds don't start a fight.
- **"Doesn't fire while changing windows" is read through
  `Windows::changing_window`**, as the existing window tests do, and the hold
  is checked per window from observed changes: no window changes hands within
  `window_hold_s` of its last change, across both squads.
- **Falsified:** holders only from the requester's own squad (the gunner
  stacks onto the rifleman's window), claims by the requester's aims (no AT
  rifleman ever trades), room as a free window only (the stepping-in carrier
  and building squads are reinforced past the seats), the free window judged
  by the squad's own soldiers (two soldiers at one window after a
  replacement), and no room check (reinforced into the full house).
- **Gates.** Against the branch head before this slice (2d4a124d7d),
  `UNCHANGED_SINGLE_HOSTS` and `UNCHANGED_OTHER_SCENES` are identical.
  `SHARED_OCCUPANCY_BEFORE_AFTER` moved only the garrison lab, first at
  tick 1231, when the scouts are seated beside the first rifle squad. Before,
  each squad saw the others' windows as free, so the scouts and then the
  second rifle squad moved onto the first squad's east windows (two or three
  soldiers per window), and the red tank's shells on those windows killed all
  three squads by tick 1991. Now every soldier keeps his own window and the
  garrison survives the 3000-tick run. Transport controls is identical (no
  building, and no truck).
- **Also fixed:** `garrison::shell` duplicated `garrison::garrisoned_in`;
  window requests now take the hold and plan from `Windows::held`. The
  `window_swap_s` doc said "squadmate"; it is whichever soldier he trades
  with.
## Slice 06

- **The ring's key is the hold's host id, which is already the structure
  owner.** `garrison::occupy` sends `remembered_structure_owner` of the clicked
  part, and the hold keeps it, so `orderLayer` groups by `hold.host.id` with no
  building index. The outline resolves through the map's building index
  (`partBuilding`, then a remains prop's `authoredProp`), because an identified
  enemy's `building` may name any part or its ruin.
- **The brightest occupant paints the ring** (`ringRank`: selected, hovered,
  the player's own over an ally's, moving, then reveal opacity). The others are
  marked `ringShared`: their circle is unchanged, so routes still leave it and
  picking (`unitCircle`) never sees the grouping; only the paint is skipped.
  *Alternative:* draw the first occupant's ring. Lost because a selected squad
  beside a revealed housemate would then read in the reveal's colour.
- **The outline takes the first holder's colour, own before enemy.** One side
  holds a building at a time; only a stale identification could disagree.
- **Seen in the shots** (`throwaway/shared-hosts/shots-06/`, garrison lab, both
  rifle squads sent in at tick 40; the enemy house by intercepting the
  encounter fetch so two red squads occupy it, tree untouched): the leaving
  shot's ring was the selection colour mixed with the leaver's gold and is now
  the selection colour alone; two selected now matches one selected; the Space
  ring was two stacked current-alpha rings and is now one, so it reads fainter.
  Outlines, own and enemy, were coincident duplicates and are now drawn once.
- **A squad stepping out publishes no goal**, its move waiting behind the exit,
  so it draws no route until it is out; it then has its own moving ring and
  route beside the house's ring. Not changed here (simulation, out of scope).
- **Critique.** The unprimed critique saw one ring and one outline per house in
  every state, never doubled, drawn under the model. Its other findings are
  outside the slice variable or predate it: the Space ring's weak contrast
  (ring colour and width are out of scope), the lab legend not matching the
  marks (ring and route colours, the red outline, hollow occupant markers;
  slice 07's labs), info panels over the ring, occupants drawn on the roof,
  a sliver of fog hatching along the enemy house's wall, and a label leader
  under the lab panel.
- **Human review** was shown non-blocking and got no reply; closed and
  continued on the evidence.
