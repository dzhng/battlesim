# Queue behind an attack

Any order that joins a queue can be shift-queued behind any attack, for every unit,
jets included. **An attack with anything queued behind it fires one volley and hands
off; the last attack in the queue keeps on**: on a unit target until it is dead or
lost, on a ground point for ever (firing while it can reach it, aiming while it can't).
The own unit publishes its whole queue as ordered step records. In the Space view every
unit but a jet draws its chain on the ground through its targets: in range, a red attack
line to a reticle; chasing, its route ending in the reticle; then every queued step
dashed, drawn the way that order draws when it is current. A jet draws only its current
attack, as a dotted red line in the air from the airframe to where it aims.

The feature was planned with the user in a decision walk.
[visualizations/decision-map.html](visualizations/decision-map.html) is that record:
decisions 1–6 and 8–18 were the user's, 7 was settled by the code, and a–o were the
agent's within the scope the user delegated, each marked with who made it. [choices.md](choices.md) is the ledger of what
implementation decided where the walk was silent.

## Contracts

- **Order admission.** A Move, AttackMove or Unload-to-a-point queued behind an attack
  is placed, planned from where the unit's queued movement ends. It used to be refused
  without a word, because an attack had no known end. The `Order` and `CommandEnvelope`
  shapes are unchanged.
- **A queued command's immediate effect.** Joining the queue doesn't cancel a drive home
  or break off a jet's run or dogfight. A queued attack sets fire at will and starts a
  jet's dogfight only when it reaches the front. What still happens at queue time: a
  queued AttackMove switches its unit to fire at will (see choices.md); an order queued
  behind an attack starts that attack's volley count, which is the rule itself; and,
  as before this feature, `Unit::enqueue` resets a deployable unit's standing posture
  to deployed. Stop and an in-place unload never join a queue (see Invariants).
- **`OwnUnit.steps`** ([`crates/contract/src/observation/order_step.rs`](../../../crates/contract/src/observation/order_step.rs)):
  the current order first, then every queued order, as `move`, `attack`, `garrison`,
  `board`, `collect`, `unload`, `exit` or `return_to_base`. This replaces `queue`,
  `queue_unloads` and `goal_unloads`. `goal` keeps its meaning, where the current order
  drives (for an attack, its chase point), because the skirmish AI and the voice lines
  read it.
- **Publication layout.** The own row's `stepCount`, a `steps` section and a
  `stepKinds` names table. A step's target is packed the way a mount row packs one, and
  its host the way the own row does. An old client can't read the new layout, so the
  **server and web deploy together**.
- **Renderer input.** `OrderView.steps`, plus `attackAim` for a winged aircraft's
  current attack. **Voice input.** `Arrivals` reads each unit's published front step.
- **Battle digest.** The pinned digests and replays did not move. The new state folds in
  only when present, and the behaviour changes that reach battles with nothing queued
  behind an attack are named decisions (see Invariants).

## The rule: volley-unless-last

- **With any order queued behind it**, an attack fires one volley and hands off. This
  applies to every target: a unit, a contact or a ground point. **As the last order**,
  it keeps on: a unit target until it is dead or lost, a ground point for ever.
  "Something is queued" is read live, every tick.
- **A volley** is one firing cycle from each weapon row working on the target, and the
  cycle comes from the weapon's data. It is a burst if the weapon has one, else its
  whole magazine, else one shot. Running dry completes a cycle, so a launcher with a
  part-spent magazine fires what it has left. A row is done at its first cycle end, so
  a squad's row is done when the first of its guns finishes. Counting starts the first
  tick the attack is at the front with something queued behind it, so a cycle already
  under way counts when it ends, and a row idle then counts at its next cycle end (a
  burst boundary, an empty magazine or a single shot).
- **A jet attacking in runs** fires a volley of one run in which its weapon fired at
  the target. That covers anything except an enemy jet in its layer, including a
  helicopter. A run broken off with nothing fired doesn't count. A jet in a dogfight
  (an enemy jet) uses the cycle rule. Whether a jet flies runs comes from the target,
  not from a list of unit types.
- **It hands off at once** when, with something queued, nothing can act on the target.
  For a ground point, that means no row is firing, aiming, reloading, traversing or
  guiding at the point; a row busy with a target it picked itself doesn't count. Typical
  causes: every row is out of range, out of ammunition, without a window facing it, blocked or
  crossed by a friendly vehicle, holding fire, changing position, or a stationary weapon
  on the move. For a unit or contact, no row can damage it, or a garrison has no row
  working on it. For a jet's runs, it has no weapon left to fly a run with. A target
  that dies or is lost mid-volley ends the attack early.
- **A launcher stands by its missile.** While a row is still guiding a round at the
  target, the volley waits until the round is down. A jet's run waits the same way while
  the jet still has a weapon to fly a run with (see Known limitations).
- **Lost targets.** A jet's attack ends when its side loses the target's track; with
  nothing queued, the jet circles there. A garrisoned attack ends when its target dies
  or its track is lost, since a garrison can't chase to the last known spot.

**Why this rule.** The walk's standing preferences were that rules come from the data
each unit and weapon carries, never from special cases by unit type, and that an order
is never silently dropped or left hanging. Decision 4 replaced the first draft, "stay on
the target until it is dead even when queued", so that shift-clicking a jet's target
twice gives two bombing runs while a plain right-click, or a move followed by a
shift-attack, still bombs it until it is dead. Behind an attack-ground, which never
ends, the old behaviour would also leave a queued order waiting for ever. Measuring the
volley from each weapon's own data gives each weapon its natural salvo with no table of
counts: a HIMARS GMLRS ripples its six rockets, a PrSM fires one missile, a Tornado-S
fires four rockets.

## Invariants

- **Digest stability (firewall).** The pinned digests and replays did not move: they
  come from battles that never queue anything behind an attack (the skirmish AI and
  endurance runs never queue; encounters queue only moves behind moves). An attack's two new pieces of state, its "waits its
  turn" mark and its volley count, fold into the digest only when present
  (`crates/sim/src/units/digest.rs`), and a unit with nothing queued behind its attack
  never creates a volley. Three behaviour changes reach battles with nothing queued
  behind an attack, each a named decision (decisions f and g, and choices.md). Two apply with nothing
  queued at all: a garrisoned attack ends when its target dies or is lost, and a jet's
  attack ends when its side loses the target (before, both attacked for ever). One
  applies whenever anything is queued: a queued command no longer cancels a drive home
  or breaks off a jet's run or dogfight, and an order queued behind a drive home is
  dropped. If a digest moves, trace it to a named decision; never blanket-bless it.
- **The size ratchet shaped the modules.** A source file over the token budget may
  shrink but never grow ([development.md](../../../development.md), the `ratchet-files`
  skill). `battle.rs`, `weapons.rs`, `publication.rs` and `publication/reader.rs` were
  all over it, so new concepts mostly went into modules beside them. The exceptions
  are small: the `Reach::ordered_working` and `Reach::can_damage` fields sit in
  `weapons.rs`, and the ran-dry cycle end stays in `weapons::fire`.
  Command application and attack ending moved out of `battle.rs` into
  `battle/apply_command.rs` and `battle/attack_order.rs` as a behaviour-neutral commit
  before the feature used them. The cycle end went into the existing `weapons/cycle.rs`.
  The volley, the step walk, the step row and its reader got new modules; the shared row
  cells came with the step row (`publication/cells.rs`) and were consolidated on the
  reader side afterwards (`publication/reader/cells.rs`). The jet tests moved to
  `order_queue/jets.rs` once `order_queue.rs` reached the budget. A change here should
  keep growing new modules, not the old files.
- **Steps are read-only.** Every step's point comes from what the side knows and from
  where the step before ends. Publishing never writes digested state or plans the
  unit's own route. The one thing it keeps is `Battle::home_ways`, a presentation-only
  cache of each queued return to base's estimated way home, which a route search
  produces. A queued attack never writes `last_known`; only the current attack of a
  unit that can chase (not garrisoned, not a jet) does, each tick its side tracks the
  target.
- **Each side sees only its own.** A step names a target the way the side tracks it,
  and names nothing once the track is gone. A garrison or exit step carries the building
  as the map places it, never the live box, which would reveal a collapse the side
  hasn't seen. Allies see none of your attack lines, as with your other orders.
- **An order that joins the queue never jumps it.** It waits its turn. Joining has two
  effects at queue time: behind an attack it starts that attack's volley count, which
  is the rule itself, and a queued AttackMove switches its unit to fire at will at once
  (see choices.md). Some orders never join
  the queue, by design, and act at once even when shift-clicked. Stop, an in-place
  unload (an Unload with no goal) and setting up a deployable unit replace the queue.
  Others leave the orders alone and act on the spot, among them the engagement,
  afterburner and pack toggles (which still clear a drive home and break off a jet's
  run or dogfight, `take_order`) and popping smoke. A queued return to base replaces
  whatever follows it once it reaches the front (`start_withdrawals`).
- **Jets' ground look is unchanged.** A jet gets no attack chain, ground attack line or
  reticle, because a line on the ground under a high jet confuses; its queued moves
  draw as they did before. Its only new mark is the attack line in the air.

### One owner each

| Concept | Owner |
|---|---|
| When a round ends a firing cycle | `Cycle::spend_round` ([`weapons/cycle.rs`](../../../crates/sim/src/weapons/cycle.rs)); a gun that runs dry ends its cycle in `weapons::fire`. The `Shot::cycle_ended` field carries both |
| Which mounts work on the ordered target this tick, and can any hurt it | `Reach::ordered_working` and `Reach::can_damage`, set by `weapons::tick::advance` from `compatible` (never `air::engaging_rows`) |
| Volley progress and "is the volley done" | [`crates/sim/src/volley.rs`](../../../crates/sim/src/volley.rs) (pure: `Volley::observe`, `Volley::observe_run`) |
| A waiting attack's turn, the volley hand-off, a non-jet attack's end | [`battle/attack_order.rs`](../../../crates/sim/src/battle/attack_order.rs). Elsewhere: `apply_command.rs` decides whether a new attack waits; a jet's attack on a dead or lost target ends in `air::step_fixed_wing` (`air/mod.rs`, on the `Hold` task from `air/orders.rs`); an expired contact's attacks are removed from every queue in `battle.rs` |
| Replacing vs joining the queue | [`battle/apply_command.rs`](../../../crates/sim/src/battle/apply_command.rs) and `Unit::enqueue`. The "nothing follows a unit driving home" guard lives in `Unit::enqueue` and is repeated in the Attack and AttackMove branches of `apply_command.rs`, so their side effects are skipped too; `start_withdrawals` (`battle/return_to_base.rs`) applies the same rule when a queued return reaches the front |
| Order → published step | [`battle/order_steps.rs`](../../../crates/sim/src/battle/order_steps.rs) |
| Target, host and footprint cells in a packed row | [`publication/cells.rs`](../../../crates/sim/src/publication/cells.rs), decoded by [`publication/reader/cells.rs`](../../../crates/sim/src/publication/reader/cells.rs). `target_cells` writes the target kind's index by hand, a copy of the `TARGET_KINDS` order in `publication/tags.rs` |
| Where a unit's orders end (ghost, drop line) | `ordersEnd` in [`orderSteps.ts`](../../../packages/battle-renderer/src/orderSteps.ts) |
| How an attack on the ground looks, current or queued | `aimAt` in `orderSteps.ts`, one drawer for both; a jet's current attack is `airAttackLine` in `orderOverlay.ts` |
| How the queued chain looks | `queuedLegs` and `queuedStep` in `orderSteps.ts`; current non-attack orders keep their drawing in `orderOverlay.ts` |
| The reticle | [`reticleMark.ts`](../../../packages/battle-renderer/src/reticleMark.ts), one mark for in range and chase end |
| Attack colour | `gameOrderStyle.attack = [...gameHud.bad, 1]` in [`apps/battle-lab/src/gameOverlay.ts`](../../../apps/battle-lab/src/gameOverlay.ts), the attack-ground footprint's red |

## Where it lives

**Simulation.** `battle/attack_order.rs` holds most of an attack's lifecycle.
`take_effect` switches the unit to fire at will and starts a dogfight.
`start_waiting_attacks` gives a waiting attack its turn; it runs before movement (from
`update_pursuit`) and again before the fire step. `hand_off_volleys` counts the volley
after the fire step's launches, from each tick's `VolleyShot`s, reads a jet's run end
from the pass leaving its dive, and pops the attack once the volley is done.
`update_pursuit` ends a non-jet attack on a dead or lost unit or a vanished contact. `dogfight_with` names
the enemy jet an attack sends a jet after. The fire step in `battle/fire.rs` calls
`start_waiting_attacks` and `hand_off_volleys`. A jet's attack on a target its side no
longer knows (`air::knows_target`) gets the `Hold` task in `air/orders.rs`, and
`air::step_fixed_wing` removes it. `battle/move_placement.rs` places queued legs from
where the queued movement ends.

**Contract and publication.** The step type is in `contract/src/observation/order_step.rs`.
The sim walks orders into steps in `battle/order_steps.rs`; `Battle::home_ways` keeps a
queued return to base's way home while its origin stays within a fog cell. Each step is
packed by `publication/step_row.rs` (`STEP_FIELDS`) and decoded by
`publication/reader/step.rs`.

**Presentation.** `packages/battle-renderer/src/orderSteps.ts` holds the step type the
renderer takes, `ordersEnd`, the attack drawer (`aimAt`) and the queued chain
(`queuedLegs`, `queuedStep`), each leg leaving the reticle or ring the step before ends
in. `orderOverlay.ts` draws a winged aircraft's attack line
in the air (`airAttackLine`, sharing `airDots` with the drop line).
`apps/battle-lab/src/battleOverlay.ts` `attackAim` resolves where that line aims, from
what the side sees. `apps/battle-lab/src/previewRevision.ts` hashes each step's intent
(`stepIntent`). `packages/battle-audio/src/voices/responses.ts` `Arrivals` ends a
journey at the unit's stop. It ends an attack, exit or return to base when the
published front step leaves it, so no voice says "arrived" in the middle of an attack.

**Tests** (each pins behaviour, not data):
- [`crates/sim/tests/order_queue.rs`](../../../crates/sim/tests/order_queue.rs) covers
  the volley rule end to end. It includes the acceptance queue for a ground unit
  (attack → move → attack the same enemy → move → attack another → return to base), a
  launcher's magazine volley, hand-off when nothing can hurt or reach, the guided
  missile flown home, the garrisoned attack ending, and the queued command that keeps
  the drive home.
- [`order_queue/jets.rs`](../../../crates/sim/tests/order_queue/jets.rs) covers the
  same for jets: the acceptance queue, two runs on one target, a corner run, the lost
  target, and a jet attacking an enemy jet or a helicopter that fires, then hands off
  to the move while the enemy still flies.
- [`order_markers.rs`](../../../crates/sim/tests/order_markers.rs) checks that a
  move, attack, move, garrison and return to base publish as steps in order, that a
  queued attack step follows its target and then names nothing, and that garrison and
  exit steps carry the held building's box. The codec vectors cover every step kind's
  packing.
- [`formation.rs`](../../../crates/sim/tests/formation.rs) checks that a destination
  queued behind an attack is placed.
- Web: [`attackLine`](../../../web/tests/attackLine.test.ts),
  [`orderChain`](../../../web/tests/orderChain.test.ts),
  [`queuedLegs`](../../../web/tests/queuedLegs.test.ts),
  [`responses`](../../../web/tests/responses.test.ts) and
  [`unitGhosts`](../../../web/tests/unitGhosts.test.ts) cover the marks, the chain,
  the voice queue and the ghost.

## Dead ends and divergences from the plan

- **A jet's run end is read in the fire step, not noted during movement.** The run
  ends during movement, but the attack can't be removed there: once the order is gone,
  `choose_lock` drops the ground aim before the fire step releases the bomb (the
  landmine behind decision h). The plan noted the run's end in movement and removed the
  attack after the fire step. The build reads the end in the fire step instead, from
  the pass leaving its dive, with the volley keeping "was diving". The flight code
  gains no hook, and the bomb released on that tick counts.
- **The acceptance test needed a bigger field and a spotter for jets.** On an
  800 × 600 field the edge guard breaks off every run, so the jet's test field is
  2400 × 1800. The jet comes on last, because its fuel runs from arrival. A blue rifle
  squad walks up to spot, since a jet overhead can't see infantry lying still.
- **"Always one volley" for attack-ground** was briefly the user's call in the walk,
  then reverted to volley-unless-last, so attack-ground follows the same rule as every
  attack (decision 2).
- **The ATGM walked off its missile.** The first hand-off ran on the launch tick, so an
  ATGM team left and its guided missile coasted into the ground. The hand-off now runs
  after the fire step's launches and waits while a row is still guiding a round.
- **The garrison footprint is published as the map places it.** The lab first
  rebuilt the building's box from the map's parts, which drew a world-axis box larger
  than the held ring for a turned building. Publishing the live box would leak a
  collapse the side hasn't seen. The step therefore carries `WorldGeometry::building_footprint`,
  the same envelope rule that gives the held ring's live box (`structure_footprint`), so
  the two match while the building stands intact.
- **The jet's line is resolved in the client.** A step carries a 2D point. Publishing a
  3D aim would have meant another layout change and another parity re-record, so
  `attackAim` resolves it from the side's own view: an identified enemy's observed
  position, a contact's area where it hangs, a ground point as ordered, else the step's
  point on the ground.
- **Searching the way home every publication was too slow.** It took a tick from 58 µs
  to 874 µs on the 800 × 600 test map. `home_ways` searches
  again only once the step before has moved more than a fog cell from where it last
  searched.
- **Step records reverse an earlier call.** [The transport spec](../transport/choices.md)
  rejected step records because the change touched every reader of `queue`. Once
  attacks and every other order kind joined the published queue, one list was the only
  single owner. The refusal behind attacks
  ([move validity](../move-validity/README.md)) is reversed too. The formation test
  that pinned it was rewritten to the new contract, not weakened.
- **Parity captures.** The recorded battle no longer reproduced on the base commit, so
  its bytes couldn't be re-published natively. The browser captures were recorded again
  once, after the sim work landed.

## Known limitations

- **A jet can hand off with its guided missile still in flight.** `Volley::observe_run`
  returns "hand off" when the jet has nothing left to fly a run with before it checks
  for a round still being guided. A run that spends the jet's last run-capable
  launcher-guided missile leaves it with no pass on the next tick, so the attack hands
  off while the missile flies. The ground rule (`Volley::observe`) checks guiding first.
  This was found by the close audit; no test pins it.
- **A ground-point volley can end before it fires.** With something queued, an
  attack-ground hands off on the first tick no row is working on the point. A friendly
  crossing the line of fire for one tick does it (a friendly vehicle; friendly
  infantry never holds a shot back). So, by the code's reading, does the
  common artillery chain move → attack-ground → move: on the tick the move ends, the
  waiting attack-ground becomes current before the fire step, the unit counts as having
  moved that tick, and a stationary launcher (every rocket artillery weapon is one)
  reports a stationary weapon on the move, so the attack hands off without firing. The
  close audit traced this by reading; no test confirms or pins it. A rocket launcher
  running that chain is the test to write before fixing it.
- **An order shift-queued behind a drive home is dropped without a word**
  (`Unit::enqueue`), against the walk's "never silently dropped" preference.
- **A gun-only, evenly matched dogfight can starve a queued order.** In a dogfight the
  volley is a weapon cycle. A jet whose only weapon is a gun, and which never brings it
  to bear on an even enemy, never ends a cycle. It still counts as able to hurt the
  target, so it never hands off. A follow-up spec that merges the dogfight into the
  attack rule owns the fix.
- **The jet's line starts at the latest published position.** That can be up to a tick
  ahead of the model drawn between ticks, and the drop lines have the same lag. The fix
  belongs to every mark in the air at once (drawn per frame from drawn positions), not
  to this line alone.
- **A long chain reads faintly at the zoom that fits it.** The causes are the
  overlay's shared stroke thinning (every mark), queued strength (`queued_alpha`, every
  queued mark) and the reticle's fixed 6 m half side (`orders.reticle_m` in
  `fixtures/presentation.json`). Each is shared by a whole class of marks, so changing
  one is a restyle, left for the user's eye.
- **Step target ids are packed in one f32 cell**, as mount rows pack them. They are
  exact only below 2²⁴, which a battle's contact and track ids are assumed to stay
  under. Nothing asserts it.

## Visual provenance

- [visualizations/decision-map.html](visualizations/decision-map.html): the decision
  walk with the user. It records every numbered and lettered decision, the contract
  box, and the landmine cards (the lost bomb, the jet attacking a lost target for ever,
  the garrison that never ends) that drove decisions f, g and h. Its embedded draft plan
  is marked superseded.
- [visualizations/attack-line-directions.html](visualizations/attack-line-directions.html):
  four mocks of one queued chain in the overlay's real colours. They were order paint
  with a reticle, hostile red, marching chevrons and a thin tether. The user picked
  **panel B, hostile red**. Decision 8 then swapped the mock's `hud.enemy`, a near-copy
  hue, for the attack-ground footprint's red, `hud.bad`. That red is shared with a
  blocked route, which is told apart by form: dashed with a cross, against solid with a
  reticle. The baseline `web/scenes/baselines/unit-roster/journey-space-orders.png` was
  the line-weight standard, and plain move drawing had to stay matched to it.
- `assets/`: the captures each visual change was judged by. The `-before-after` shots
  are pairs (before left, after right); the rest show the result alone. The fx3, s8 and
  s9 shots were critiqued under the critique skill's no-sub-agent fallback (the
  strongest case against each shot written down before deciding); the d17 and p1 shots
  had an unprimed critique.
  - [fx3-blocked-first-leg-before-after](assets/fx3-blocked-first-leg-before-after.png):
    a blocked unit's first queued leg. The movement lab's real battle, with the tank's
    blocked state injected, since no lab map reproduces a route-blocked unit.
  - [s8-attack-default-zoom](assets/s8-attack-default-zoom.png),
    [s8-attack-before-after](assets/s8-attack-before-after.png),
    [s8-blocked-beside-attack-before-after](assets/s8-blocked-beside-attack-before-after.png)
    and [s8-ground-attack](assets/s8-ground-attack.png): the current attack line and
    reticle. The blocked truck beside an attacking tank (its blocked state injected, as
    in fx3) shows the two read differently by form alone. The last shot is a ground
    point's cross.
  - [s9-garrison-chain-before-after](assets/s9-garrison-chain-before-after.png),
    [s9-garrison-chain](assets/s9-garrison-chain.png) and
    [s9-attack-chain](assets/s9-attack-chain.png): the queued chain. Before, it skipped
    every step but moves.
  - [d17-chain-fitted](assets/d17-chain-fitted.png) and
    [d17-chain-close](assets/d17-chain-close.png): decision 17's chain in a real battle
    (the consequences lab). This was part 1's visual proof, and the evidence for the
    faint-chain limitation above.
  - [p1-jet-attack-lines](assets/p1-jet-attack-lines.png) and
    [p1-jet-attack-line-crop](assets/p1-jet-attack-line-crop.png): the air-ground lab's
    jets with Space held, each line followable from its jet to its aim. There is no
    before image: before, a jet's attack drew no mark at all.
