# Choices ledger

What was decided where the plan (the decision map) was silent. Appended per pass;
rewritten from scratch at closeout.

## Planning (spec synthesis)

- **The current order is `steps[0]`; there is no separate current-target field.**
  *Answered by the agent on the user's behalf* (preference 1, one owner): the map's
  contract box said the current attack target is "published the same way"; one list
  owns both, so overlay and voice read one field.
- **A move that unloads is a `move` step then an `unload` step; `goal_unloads` and
  `queue_unloads` are deleted.** *Agent, on the user's behalf* (one owner): decision 15
  already makes unload a step, so an unload flag on moves would be a second owner.
- **A new names table `stepKinds`.** *Agent*: decision l ruled out a new table for
  targets only; names tables are the codec's convention.
- **(p) A queued AttackMove still switches its unit to fire at will when queued.**
  *Agent* (revised in S2): deferring it would need a "waits its turn" mark on move
  orders too, a second copy of the attack's mark for a minor quirk; decision n names
  only Attack. Revisit if a playtest shows it.
- **A jet attacking without passes (a dogfight) uses the cycle rule.** *Agent*: follows
  from the attack's own data (does it fly passes?), not the unit type.
- **A garrisoned attack ends when its target's track is lost.** *Agent*: a garrison can't
  chase to the last known spot, which is what "lost" means for a unit that can move.

## FX2

- **An order queued behind a drive home is dropped.** *Agent*: a ground unit retires
  only once it has nothing left to drive to, so a queued order would keep it on the
  field; `start_withdrawals` already rules "nothing follows a unit that leaves" for a
  queued return. Owned by `Unit::enqueue`.

## S2

- **The old formation test pinning the refusal was rewritten to the new contract**
  (a destination queued behind an attack is placed) — decision 1 reversed that
  requirement; this is not a weakened gate.
- **A queued attack's "waits its turn" mark is set when it joins a non-empty queue**
  and cleared at the start of `update_pursuit` once it is at the front; the dogfight
  check runs then, against positions at its turn.

## S3

- **"Working on the target" means a mount locked on the attack's target whose reason
  is Firing, Aiming, Reloading, TurretTraversing or Guiding.** Anything else (out of
  range, blocked, out of ammo, no window facing it, changing position, holding fire, a
  stationary weapon on the move) doesn't count. A garrisoned squad with no window
  facing the target therefore can't hold a volley; this follows from the reasons and
  is not pinned by its own test.
- **For a unit or contact target, "can act" is "a row can hurt it, and one is working
  on it or the unit can go after it"** — so a garrisoned unit with a target out of
  every window's view hands off, while a ground unit out of range chases.
- **A jet's attack flown as runs is left to S4;** a jet in a dogfight uses the cycle rule.
- **The volley lives in `crates/sim/src/volley.rs`** (one small module), not an
  `attack/` folder.
- **The plain-riflemen test uses `test_smoke_squad`:** `test_rifle`'s grenadier can hurt
  the test tank's roof, so it correctly waits for its volley.
- **FX1's test now has nothing queued behind the garrisoned attack:** with an order
  queued, the attack correctly fires one volley and hands off before the enemy dies.

## S4

- **A run's end is read in the fire step, from the pass leaving its dive**, not marked
  in `air::step_fixed_wing`: the volley keeps "was diving" and "fired this run", so the
  flight code gains no hook and nothing pops in movement. A bomb released on the cue
  tick counts (the fire step runs after the cue).
- **A jet attacks in runs anything but a target `air::dogfightable` names** (an enemy
  jet in its layer), which uses the cycle rule. A run attack can act while the jet has a
  pass planned and, on a unit, `Reach.can_damage`; with no pass (nothing to run with) it
  hands off at once.
- **The acceptance skirmish scales with the unit**: a jet needs a 2400 × 1800 field (an
  800 × 600 one keeps every run inside the edge guard), comes on last (its fuel runs
  from arrival), and blue's rifle squad walks up to spot (a jet overhead can't see
  infantry lying still) and keeps blue on the field.
- **"Move, then shift-attack: bombs until dead" has no test of its own**: it is the
  unchanged last-attack path, which digests and the existing run tests cover.
## Presentation lane

- **FX3's picture injects the blocked state.** *Agent*: no lab map reproduces a
  route-blocked unit (it takes a jammed lane), so the shot is the movement lab's real
  battle with a real move and queued move, the tank's published state switched to
  `route_blocked` in the paused page before the overlay rebuilt. The before/after diff
  is that one leg and nothing else
  ([assets/fx3-blocked-first-leg-before-after.png](assets/fx3-blocked-first-leg-before-after.png)).
- **The unprimed critique ran as the skill's no-sub-agent fallback** (the lane's brief
  allowed no sub-agents): the strongest case against each shot written before deciding.
- **A queued attack whose target the side forgets names nothing.** *Agent*: decision k
  forbids writing `last_known` for a queued attack, and the side keeps an unseen unit's
  track only through the acquisition grace. Through the grace the step's point holds
  where the track was last seen; after it the step publishes no target and no point
  (at its turn such an attack ends at once). The current attack keeps its `last_known`
  (decision 10). Pinned in `order_markers.rs`.
- **A queued return to base keeps its way home while its origin holds.** *Agent*:
  `way_home_from` searches a route per road in; computed every publication it cost a
  waiting refund 0.8 ms a tick on the 800 × 600 test map (58 → 874 µs a tick), far more
  on a city map. `Battle::home_ways` (presentation only, never in the digest) keeps each
  unit's point keyed by where its step before ends: 60 µs a tick.
- **The renderer's step is a kind and a point.** *Agent*: the view (`OrderStepView`)
  carries the target and host; the overlay (`orderSteps.ts`) takes only what it draws, and
  owns the step-kind union the view imports, so there is one copy of each. S8 resolves an
  attack's target to a pose in the app layer.
- **Where orders end counts every step that takes the unit somewhere.** *Agent*, from
  S6's `ordersEnd` rule ("never an attack's"): a queued garrison, boarding, collection,
  exit or return to base now moves the Space ghost to its point; before, only queued moves
  did.
- **The preview identity hashes each step's intent** (`stepIntent`): an attack's target,
  a board or collect step's kind, any other step's point and host, never a point that
  follows a moving unit.
- **The codec vectors cover every step kind**, hand-authored: the base frame's carrier
  chain (move, unload, attack-move, attack, a lost attack, unload into a building, return
  to base), and the other phases' frames one or two kinds each, the wide frame with a
  building id of 2³² − 1.
- **The parity captures were not re-recorded.** *Agent*, per known unknown 1: the
  paris-corner capture's recorded battle no longer reproduces on the base commit
  (`1230e0cf0`: digest `9683a2cdb73063a3` recorded, `3039b04038158163` replayed at tick 1),
  so its bytes can't be re-published natively, and the new layout can't read the old
  bytes. A fresh browser capture is the parent's call (see S6's state).
- **The voice queue matches an errand to the front step by kind and named target.**
  *Agent*: an attack whose target the side has lost publishes no target, so a missing id
  matches; consecutive errands on one target merge and still end in order, since each
  ended errand gives way to the next at once.
- **The reticle is a fixed 6 m half side** (`presentation.overlay.orders.reticle_m`),
  square to the world. *Agent*: it frames a vehicle at default zoom; an enemy squad
  spreads past it. Sizing it from the target's footprint is the slice's named feedback
  ("the reticle's size"), left for the user's eye.
- **The step's point is the target's observed pose.** *Agent*: an identified target's
  step point is its track, published the same tick as its `identified` pose, so the
  overlay draws the step's point and resolves nothing more.
- **A chase blocked behind its route draws the blocked marks, no reticle.** *Agent*: the
  blocked route's cross sits on the chase point, which is the target; a reticle there
  would stack two marks on one spot.
- **The attack line uses the route's stroke** (`line_px`, the one stroke rule), so at far
  zoom it thins like every route; red on green holds less contrast than the yellow route
  there. Named for the human checkpoint, not changed.
- **Each queued step's mark, by kind** (decision 18, where it was silent): a move and a
  return to base end in the queued ring; a garrison in its building's ring (its corners
  plus `building_marker_margin_m`, as a held building's), an exit leaves that ring; a
  board or collect ends at the carrier or squad with no ring of its own (their own
  circles show); an unload marks the ring before it; an attack whose target the side has
  lost draws nothing and the chain runs on from the step before. *Agent.*
- **The building comes from the map's building index**, passed to `orderLayer` by the
  battle view and the garrison lab; a lab without it draws a garrison as a leg to a
  queued ring. *Agent*: the side's known props list only bodies that changed, so they
  can't place an intact building.

## Review fixes (part 1)

- **A launcher guiding a missile at the target holds the volley until the missile is
  down** (and a jet's run likewise), so an ATGM team doesn't walk off and drop it. The
  hand-off now runs after the fire step's launches, where a guided round's support is set.
- **A queued Attack or AttackMove for a unit driving home is skipped whole**, side
  effects included (fire at will, a dogfight), as `Unit::enqueue` drops it.
- **Handing off clears the attack's chase point and dogfight link**; an attack left
  with nothing behind it drops its volley count.
- **A waiting attack whose turn comes during movement takes effect before the fire
  step** too, not only at the next tick's start.
- **Not done:** the acceptance tests' "fired before handing off" check counts every
  mount's rounds, not only those at the target; the volley tests pin target-only cycle
  ends directly.
