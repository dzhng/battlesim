# Choices ledger (final)

What the implementation decided where the decision walk
([visualizations/decision-map.html](visualizations/decision-map.html)) was silent.
Each entry was re-checked against the shipped code at close. Choices a later pass
reverted or replaced are gone, and the ones that changed are written as they ended up.
The ledger is grouped by verdict and ranked least confident first within each group.

A few terms used throughout:
- A **volley** is what an attack fires before it hands off to the order queued behind
  it ([README](README.md#the-rule-volley-unless-last)).
- A **weapon row** (a *mount*) is one weapon station on a unit. A squad's row is one
  weapon kind carried by several soldiers.
- A **step** is one entry in the list of orders the simulation publishes for an own
  unit (`OwnUnit.steps`). The first step is the current order.

**Review these first:** the two unsound entries (a dogfight's volley, which can starve a
queued order, and a ground-point volley that a passing state can end unfired); the
queued AttackMove that still switches fire at will early; and the fixed reticle size.
The jet that handed off with its last guided missile in flight, found at close, is
fixed (see the launcher entry).

## Unsound

### A jet in a dogfight measures its volley by weapon cycles, and can be stuck there
*When:* planning, built in `2f1244b69` (the volley hand-off). The jet-on-jet test came in
`28bb881b1`.

Scenario: a jet carrying only a gun is told to attack an enemy jet, with a move
shift-queued behind it. Against an enemy jet the attack is a dogfight, not a bombing
run, so its volley is "each working weapon row ends one firing cycle at the target".
The gun cycle ends only when the gun fires. In an evenly matched turning fight the gun
may never bear. Its row is then not *working* (not firing, aiming, reloading,
traversing or guiding). No row ever ends a cycle, yet the gun still *can damage* the
enemy, so the attack never counts as unable to act. It never hands off, and the queued
move waits as long as the dogfight lasts. A jet with missiles, which lock and fire,
does hand off; `order_queue/jets.rs` pins that case.

*Since 2026-11-16:* dogfights are runs of head-on passes (air-war-flight D11′), so a
gun-only jet's gun bears on its first pass and the queued move takes over after it;
the never-bearing case above no longer arises between evenly matched jets.

*The gap:* the walk said a jet attacking without passes uses the cycle rule, but not
what happens when no weapon ever gets to fire.
*The reach:* any queue behind an air-to-air attack by a gun-only jet.
*Verdict:* **unsound.** Corrected decision: with an order queued behind it, every attack
must reach a hand-off in bounded time whatever its weapons. A weapon that can't bring
itself to bear on a target must count toward "nothing can act", not as a cycle still to
come. A follow-up spec that merges the dogfight into the attack rule owns the redo.
*Confidence:* low.

### "Working on the target" is read tick by tick, so a passing state can end a ground-point volley unfired
*When:* `2f1244b69`; guiding was added in `76c45aedc`.

Scenario: an anti-tank team attacks a tank, with a move queued. While its launcher aims,
reloads, turns its turret toward the target or guides a missile, the row counts as
working, and the volley waits for it to end a cycle. Any other state doesn't count,
such as: busy with another target, no weapon that can fire at this one, out of range, blocked, a friendly vehicle in the line of fire, still tracking where the
enemy was last seen, no sight of its own, out of ammunition, no window facing the
target, changing position, holding fire, or a stationary weapon on the move. The rule
lives in `weapons::tick::advance`, which sets `Reach::ordered_working`. With an order
queued, an attack-ground hands off on the first tick no row is working on the point
(decision 3: "If none can fire, the attack hands off at once"). Because the states are
read tick by tick, a passing one does it too: a friendly vehicle crossing the line of
fire for a tick, or, by the code's reading, a rocket launcher arriving from a queued
move, which counts as a stationary weapon on the move on its arrival tick. So
"move → attack-ground → move" can skip the attack-ground without firing.

*The gap:* decision d said "aimed at it… anything the sim reports as unable doesn't
count", without listing the states or saying whether a passing one counts.
*The reach:* every queued attack-ground, artillery above all. A garrisoned squad with no
window facing the target also can't hold a volley; that part is intended. Neither case
has a test.
*Verdict:* **unsound** for a ground point. The rule is right for "can never fire from
here", wrong for a passing state. Corrected decision: with something queued, an
attack-ground hands off at once only when no row could fire at the point from here at
all (out of range, out of ammunition, no window facing it); a row that is momentarily
unable (moving, setting up, a blocked or crossed line) is still a cycle to come.

*Checked after close:* the rocket-launcher case doesn't reproduce. In
`order_queue::a_launcher_queued_through_an_attack_on_the_ground_fires_on_the_way` a
launcher that must stand to fire, sent on "move → attack-ground → move", stops and fires
its salvo before moving on: the attack's first counted fire step finds it standing. The
test guards that case; a friendly crossing the line of fire for a tick is still
unguarded.
*Confidence:* low.

## Needs the user

### A queued attack-move still switches its unit to fire at will as soon as it is queued
*When:* `0f2982a3f` (orders queue behind an attack).

Scenario: a tank set to hold fire is driving a move, and the player shift-queues an
attack-move behind it. On the spot, before the attack-move's turn, the tank switches
to fire at will and starts shooting what it sees on the current drive. A queued
*attack* does not do this: it waits its turn (decision n) and switches fire at will
only when it reaches the front.

*The gap:* decision n named only Attack. Deferring the switch for an attack-move would
need a second "waits its turn" mark on move orders, a copy of the attack's mark, for a
minor quirk.
*The reach:* hold fire set before a queued attack-move is lost at queue time.
*Verdict:* **needs-user.** Provisional call: keep it. To reverse it, give move orders
the same waiting mark and set fire at will in `start_waiting_attacks`
(`battle/attack_order.rs`) instead of in `apply_command.rs`.
*Confidence:* low.

### The reticle is a fixed 6 m half side, square to the world
*When:* `11062439a` (the attack line).

Scenario: you attack an enemy tank, and four corner brackets 12 m across frame it at
default zoom. Attack an enemy squad spread over 20 m, and the reticle frames only the
middle of it. It isn't sized from the target and doesn't turn with it. The size is
`orders.reticle_m` in `fixtures/presentation.json`.

*The gap:* the walk asked for a reticle (decisions 5 and 13) but gave no size.
*The reach:* every attack mark, current and queued, and the chain's legs, which start
at the reticle's edge.
*Verdict:* **needs-user** (taste). Provisional call: keep 6 m. To change it, size the
reticle from the target's footprint.
*Confidence:* low.

### The attack line uses the route's stroke, so it thins at far zoom like every route
*When:* `11062439a`; the queued chain in `bd6bc6c0d`.

Scenario: zoom out to fit a 300 m chain. The shared stroke rule (`line_px`) thins every
leg toward a hairline. Red on green fields holds less contrast than the yellow route,
and the dashed queued legs at queued strength (`queued_alpha`) nearly vanish. An
unprimed critique of decision 17's real-battle chain said it reads poorly there.

*The gap:* the walk said each step draws what it draws when current, at queued
strength, but didn't say whether that strength holds at every zoom.
*The reach:* changing the stroke rule or queued strength restyles every order mark, not
only attacks.
*Verdict:* **needs-user** (taste). Provisional call: keep the shared rules.
*Confidence:* medium.

### A jet's attack line starts where the jet was last published
*When:* `7ded96bf7` (review fixes for the jet line).

Scenario: the jet model on screen is drawn smoothly between
simulation ticks. The attack line starts at the jet's latest published position, which
can be up to a tick ahead of the drawn model, so the line's end can sit just in front of
the nose. The aircraft's drop lines have the same lag.

*The gap:* decision 11 said "from the jet model", without saying which position.
*The reach:* fixing it for this line alone would give air marks two different
positions to start from.
*Verdict:* **needs-user.** Provisional call: leave it. The fix is to draw every mark in
the air per frame from the drawn positions, all at once.
*Confidence:* medium.

## Sound

### A unit or contact target can still be acted on while a row can hurt it and either works on it or the unit can go after it
*When:* `2f1244b69`.

Scenario A: a tank attacks an enemy out of range, with a move queued. No row is working
on the target, but the tank can drive closer, so the attack chases and fires its
volley. Scenario B: a squad in a house attacks an enemy that no window faces. No row
works on it, and a garrison never walks out, so the attack hands off at once. Riflemen
told to attack a tank they can't hurt hand off on the first tick.

*The gap:* the walk said "no row can damage it" hands off, but not what a unit that
can hurt the target but can't reach it should do.
*The reach:* this sets how long a queued order waits behind an out-of-range attack.
Waiting for a chase is unbounded while the target stays known.
*Verdict:* sound. It matches the last attack's behaviour: a ground unit goes after its
target.
*Confidence:* medium.

### A jet attacks in runs anything but an enemy jet in its layer, and hands off at once with no weapon to fly a run with
*When:* `9ca23cbab` (a jet's volley is one run); the helicopter case was pinned in
`28bb881b1`.

Scenario: a jet told to attack a helicopter, a tank or a ground point flies bombing or
strafing runs, and its volley is one run in which it fired. Only an enemy jet in its
own layer (`air::dogfightable`) makes it a dogfight, which uses weapon cycles. A jet
whose weapons can't fly a run at the target (air-to-air missiles only, against a
ground point) has no pass planned, and hands off at once when an order is queued. On a
unit, the run must also be able to hurt it (`Reach::can_damage`).

*The gap:* decision e said "a jet run counts only if its weapon fired" and "no weapon
able to fly a run ends at once", but not which targets get runs.
*The reach:* a helicopter is attacked in runs, not a dogfight.
*Verdict:* sound. It comes from the target and the jet's weapons, not unit labels.
*Confidence:* medium.

### An order queued behind a unit that is driving home is dropped whole
*When:* `687cdd2ca`; side effects were skipped too from `76c45aedc`.

Scenario: a truck is told to return to base, and the player shift-queues a move behind
it. A ground unit only retires once nothing is left for it to drive to, so a queued
move would keep it on the field for ever. `Unit::enqueue` drops the order. A queued
attack or attack-move for that truck is skipped before it can switch fire at will or
start a dogfight.

*The gap:* the walk didn't cover queuing behind a drive home.
*The reach:* the player gets no feedback that the order was dropped.
*Verdict:* sound. "Nothing follows a unit that leaves" was already the rule for a queued
return to base.
*Confidence:* medium.

### A queued attack whose target the side forgets names nothing
*When:* `62f91455d` (step records).

Scenario: you queue an attack on an enemy tank behind a move, and the tank drops out of
sight. Through the side's acquisition grace (the short time the side keeps an unseen
unit's track), the step's point holds where the tank was last seen. After that, the
step publishes no target and no point. The chain draws nothing for it and runs on from
the step before. When its turn comes, that attack ends at once. The current attack is
different: it keeps its last known spot (decision 10).

*The gap:* decision k forbids writing `last_known` for a queued attack, but didn't say
what a queued attack shows once the side has no track.
*The reach:* the voice queue and the chain must accept an attack step with no target.
*Verdict:* sound. Pinned in `order_markers.rs`.
*Confidence:* medium.

### A queued return to base keeps its way home while the origin stays within a fog cell
*When:* `07f39b305`.

Scenario: a truck has a move and then a return to base queued. The return's published
point is the way home from where the move ends, and finding it searches a route for
every road in. Doing that on every publication took a tick from 58 µs to 874 µs on the
small test map, and far more on a city map. `Battle::home_ways` keeps each return's
point, keyed by its unit and its place in the queue, and searches again only once the
step before it ends more than one fog cell from where it searched. A fog cell is the
finest spot the side's sight resolves.

*The gap:* decision o said the point is computed read-only, but set no cost.
*The reach:* presentation state on `Battle` that is never in the digest. The published
point can lag the exact one by up to a fog cell.
*Verdict:* sound.
*Confidence:* medium.

### The renderer's step is a kind and a point; the view carries targets and hosts
*When:* `62f91455d`.

Scenario: the overlay draws a chain. It needs each step's kind and where it is, not who
the target is. `OrderStepView` (the web view) carries the target and host, and
`orderSteps.ts` takes only what it draws and owns the step-kind union the view imports.
That leaves one copy of each type.

*The gap:* the walk didn't define the renderer's input.
*The reach:* a new step kind is added once, in `orderSteps.ts`.
*Verdict:* sound.
*Confidence:* medium.

### Where a unit's orders end counts every step that takes it somewhere
*When:* `62f91455d`.

Scenario: a squad has a move, then a garrison, queued. With Space held, its ghost (the
translucent copy of the unit where its orders end) now stands at the building's
approach, not at the move's end. `ordersEnd` takes the last step with a point that
isn't an attack: a move, garrison, board, collect, unload, exit or return to base.
Before, only queued moves moved the ghost.

*The gap:* the walk's sharp edge said only "never an attack's".
*The reach:* the ghost and the aircraft drop line.
*Verdict:* sound.
*Confidence:* medium.

### The voice queue matches an errand to the front step by kind and named target
*When:* `56c4d0694`; review fixes in `dec7b6d0c`.

Scenario: you order a move, then attack tank 4, then another move. "Arrived" plays at
the first move's end. Nothing plays while the attack chases. When the front step stops
being "attack on tank 4", the attack is over, and the second move's arrival can be
called. An attack whose target the side lost publishes no target, and a missing id
still counts as the same attack. A contact names no lasting target, because the sim
turns the attack onto the unit once it is identified. Two attacks in a row on one
target each end in order, since an ended errand gives way to the next at once.

*The gap:* decision 16 said to track every step kind, not how to match one.
*Verdict:* sound.
*Confidence:* medium.

### Each queued step's mark, where decision 18 was silent
*When:* `bd6bc6c0d`.

Scenario, walking one chain. A move and a return to base each end in the queued ring.
A garrison ends in its building's ring (its corners plus `building_marker_margin_m`, as
a held building's), and an exit leaves that ring. A board or collect ends at the
carrier or squad, whose own circles show, with no ring of its own. An unload marks the
ring before it. A lost attack draws nothing.

*The gap:* decision 18 named the marks but not where each starts and ends.
*Verdict:* sound.
*Confidence:* medium.

### A chase blocked behind its route draws the blocked marks, no reticle
*When:* `11062439a`.

Scenario: a tank chasing an enemy finds its route blocked. The blocked route's cross
sits on the chase point, which is the target. A reticle there would stack two marks on
one spot, so only the blocked marks draw.

*The gap:* decision 13 covered chasing and in range, not blocked.
*Verdict:* sound.
*Confidence:* medium.

### A garrison or exit step carries the building's box as the map places it
*When:* `69f0d68f8`.

Scenario: a squad has "garrison that house" queued, and the house is turned 30° to the
streets. The step carries `footprint` (packed as the step row's `hostX`, `hostY`,
`hostHalfX`, `hostHalfY`), which is the house in its own frame from
`WorldGeometry::building_footprint`. That is the same envelope rule that gives the held
ring's live box (`structure_footprint`), so the queued ring and the held ring match
while the house stands intact. It isn't the live box: that would show a collapse the
side's units haven't seen (a garrison test caught it), whereas the map is known to both
sides. A damaged building's queued ring can therefore stand wider than its held ring.
An exit step's point is the box's centre.

*The gap:* decision 18 said "the building's ring", but the client had no way to place
an intact building.
*The reach:* a field in the publication layout.
*Verdict:* sound.
*Confidence:* medium.

### A jet's attack line aims where the side places the target, resolved in the client
*When:* `dee5173c5`; contacts in `7ded96bf7`.

Scenario: a jet attacks an enemy helicopter it can see. `attackAim` in
`apps/battle-lab/src/battleOverlay.ts` aims the line at the helicopter's observed 3D
position. For a contact it aims where the contact's area hangs (in the air for an air
contact). A ground point is aimed at as ordered, and anything else at the step's 2D
point on the ground.

*The gap:* decision 11 asked for a line to "the target unit or ground point" in 3D, but
a step carries only a 2D point.
*The reach:* publishing a 3D aim would have changed the layout and needed a third
parity re-record. The client resolves it from the side's own view instead.
*Verdict:* sound.
*Confidence:* medium.

### The jet's line is dots in the attack red, spaced as a drop line's
*When:* `dee5173c5`; the shared helper in `7ded96bf7`.

Scenario: hold Space with a jet attacking. A dotted red line runs in the air from the
airframe to its aim, the same dots as an aircraft's drop line. It's the overlay's one
look for marks in the air, and the drop line and the attack line share one helper
(`airDots`). Only the current attack is drawn, and nothing is drawn on the ground.

*The gap:* decision 11 said "an attack line", without its look.
*Verdict:* sound.
*Confidence:* medium.

### The acceptance tests' "fired before handing off" check counts every row's rounds
*When:* `76c45aedc`.

Scenario: in the acceptance queue, the test checks that the unit fired at each attack
before moving on. It counts rounds from all of the unit's rows, not only those at the
target. The volley tests pin target-only cycle ends directly. "Move, then
shift-attack: bombs until dead" has no test of its own. It's the unchanged last-attack
path, which the digests and the existing run tests cover.

*The gap:* decision 17 defined the queue, not how to prove each volley.
*Verdict:* sound.
*Confidence:* medium.

### A garrisoned attack ends when its target's track is lost
*When:* `fb32920bd` (planning).

Scenario: a squad in a house attacks an enemy that walks behind a wall and out of the
side's knowledge. A unit that can move would go to the last known spot. A garrison
can't, so the attack ends, and anything queued (an exit, a move) runs.

*The gap:* the walk's garrison bug card covered death, not loss.
*Verdict:* sound.
*Confidence:* high.

### A jet's run end is read in the fire step, from the pass leaving its dive
*When:* `9ca23cbab`.

Scenario: a jet's bomb is released on the tick its run ends. The volley keeps "was
diving" and "fired this run", and a run ends the tick the pass is no longer in its
dive. That is read after the fire step, so the bomb released on that tick counts and
has already left. The flight code gains no hook, and nothing pops the attack during
movement, where removing it would lose the bomb: once the order is gone, `choose_lock`
drops the ground aim before the fire step releases it.

*The gap:* decision h said the run's end is noted during movement and the attack removed
after the fire step; reading the end in the fire step keeps that order with no hook in
the flight code.
*Verdict:* sound.
*Confidence:* high.

### A launcher guiding a missile holds the volley until the missile is down
*When:* `76c45aedc`.

Scenario: an ATGM team with a move queued fires its one missile. The cycle ends on
launch, but the missile is wire-guided. If the team walked off, the missile would
coast into the ground. The hand-off runs after the fire step's launches and waits
while any row is guiding a round at the target. A jet's run waits for its missiles the
same way. The close audit found that `Volley::observe_run` handed off as soon as the jet
had nothing left to fly a run with, before checking guidance, so a run that spent its
last guided missile handed off with the missile in flight; it now checks guidance first,
as `observe` does (`order_queue::jets::a_jet_guides_its_last_missile_home_before_handing_off`,
which failed before the fix).

*The gap:* the walk's cycle rule counted the cycle's end, not the round's flight.
*Verdict:* sound.
*Confidence:* high.

### When a waiting attack's turn comes, and what handing off clears
*When:* `0f2982a3f`; refined in `76c45aedc`.

Scenario: a tank has a move, then an attack, queued. The attack joined a non-empty
queue, so it carries a "waits its turn" mark and sets nothing on the tank. The move
ends during movement. Before that tick's fire step, `start_waiting_attacks` clears
the mark, switches fire at will on, and starts a dogfight if the target is an enemy jet
(checked against positions at that moment). The same call runs again at the start of
the next tick. When the volley is done, handing off pops the attack and clears its
chase point and dogfight link. An attack left with nothing behind it drops its volley
count.

*The gap:* decision n said a queued attack takes effect when it reaches the front, but
didn't say at which point in the tick.
*Verdict:* sound.
*Confidence:* high.

### The current order is the first step, and there is no separate current-target field
*When:* planning; built in `62f91455d`.

Scenario: the overlay and the voice queue both need the current attack's target, and
both read `steps[0]`. The walk's contract box said the current target is "published the
same way". One list owns both.

*The gap:* the contract box left open whether to keep a second field.
*Verdict:* sound.
*Confidence:* high.

### A move that unloads is a move step and then an unload step
*When:* planning; built in `62f91455d`.

Scenario: a truck is told to drive and let its passengers out. That publishes two
steps: `move`, then `unload`. The overlay draws the unload mark on the move's ring.
`goal_unloads` and `queue_unloads` are deleted, since decision 15 makes unload a step
already, and a flag on moves would be a second owner.

*The gap:* decision 15 made unload a step, but didn't say what happens to the old
flags.
*Verdict:* sound.
*Confidence:* high.

### The step kinds get their own names table, `stepKinds`
*When:* planning; built in `62f91455d`.

Scenario: the packed step row stores its kind as an index into a names table in the
layout, the codec's convention for every enum. Decision l ruled out a new table for
targets only, which reuse `targetKinds`.

*The gap:* decision l covered targets, not step kinds.
*Verdict:* sound.
*Confidence:* high.

### The preview identity hashes each step's intent, never a moving point
*When:* `62f91455d`.

Scenario: the order preview recomputes when the side's view "changes". If it hashed an
attack step's point, every tick a chased target moved would count as a change.
`stepIntent` hashes an attack's target, a board's carrier, a collect's kind alone, and
any other step's point and host.

*The gap:* the walk didn't mention the preview.
*Verdict:* sound.
*Confidence:* high.

### The jet acceptance test runs on a bigger field with a spotter
*When:* `9ca23cbab`.

Scenario: on an 800 × 600 field, the edge guard breaks off every run before it fires.
The jet's test field is 2400 × 1800. The jet comes on last, because its fuel runs from
arrival. Blue's rifle squad walks up to spot the target, since a jet overhead can't
see infantry lying still, and the squad keeps blue on the field.

*The gap:* decision 17 gave the queue, not the test's ground.
*Verdict:* sound. The test fits the unit; no rule changed for it.
*Confidence:* high.

### Trivial discretion (3)

These were placements and fixture details. The volley lives in one pure module,
`crates/sim/src/volley.rs`, rather than an `attack/` folder. The codec vectors cover
every step kind and are written by hand. The jet tests moved to `order_queue/jets.rs`
when `order_queue.rs` reached the size budget.
