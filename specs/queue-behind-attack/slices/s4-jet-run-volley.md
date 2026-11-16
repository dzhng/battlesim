# S4 — a jet's volley is one run in which it fired

**Unlocks.** Decisions e and h; the jet half of decision 17.

**Seams.**
- The fire step reads the run's end from the pass leaving its dive (the volley keeps
  "was diving"); movement never pops (the bomb is released in the fire step of the same
  tick, and `choose_lock` drops a ground aim whose order is gone).
- A run counts only if a shot was fired at the target during it; a run ended with
  nothing fired (edge guard) clears the mark.
- No pass weapon able to fly a run (`pass_weapons` empty) reports "nothing can act", so
  the attack hands off at once. "Can damage" stays `compatible`, never `engaging_rows`.
- `after_fire` pops, after the round has left.

**Tests** (`order_queue.rs`): acceptance (decision 17) for a test jet; same target
shift-attacked twice then a move → two fired runs, then the move; move then shift-attack →
bombs until dead; an air-to-air-only jet attacking ground with a move queued hands off at
once; a jet near a field corner still hands off within a bounded number of runs.
`attack_runs.rs`: the bomb leaves on the run that ends the attack.

**Commit.** `feat(sim): a jet's volley is one run in which it fired`.
