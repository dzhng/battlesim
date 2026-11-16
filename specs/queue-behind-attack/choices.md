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
