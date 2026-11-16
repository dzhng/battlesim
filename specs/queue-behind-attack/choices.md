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
- **(p) A queued AttackMove switches to fire at will at its turn, not when queued.**
  *Agent, on the user's behalf* (preference 2, a queued order never jumps the queue):
  decision n named only Attack.
- **A jet attacking without passes (a dogfight) uses the cycle rule.** *Agent*: follows
  from the attack's own data (does it fly passes?), not the unit type.
- **A garrisoned attack ends when its target's track is lost.** *Agent*: a garrison can't
  chase to the last known spot, which is what "lost" means for a unit that can move.
