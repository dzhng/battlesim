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
