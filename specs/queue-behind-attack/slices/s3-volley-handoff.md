# S3 — the volley handoff (every attack not flown as passes)

**Unlocks.** The rule (README "The rule, exactly") for ground units, rotorcraft,
garrisons and jets in a dogfight. C3's volley progress.

**Seams.**
- `crates/sim/src/volley.rs` (new, pure, no `Battle`): `Volley` progress and two
  functions — `observe(volley, tick_report)` and `complete(volley, tick_report, aim) ->
  bool`. For a ground point, "can act" is "some row is firing"; for a unit or contact,
  "some row can damage it". Done when nothing can act, or something fired and every row
  firing at the target has ended a cycle since counting began.
- `battle/attack_order.rs::after_fire(report)`, called from `battle/fire.rs` right after
  `weapons::tick::advance`: for each unit whose front order is an attack **and** has
  something behind it, create the volley if absent, observe, pop on complete. A unit with
  nothing queued is never touched (F1).
- Digest: the volley folds only when present.

**Tests** (`crates/sim/tests/order_queue.rs`; weapon shapes patched onto test units via
`fixtures::patch_catalog`, never roster rows):
- **acceptance (decision 17), ground unit**: attack A → move → attack A → move →
  attack B → return to base, all shift-queued: each step runs in order, each attack
  fires, the unit retires, within a step cap;
- the last attack stays until the target dies; a last attack-ground keeps firing past
  several cycles;
- a launcher with a magazine and no burst fires its whole magazine as one volley; a
  part-spent magazine fires what is left (counts read from the test weapon's data);
- riflemen attacking an armoured target with a move queued hand off on the first tick;
- an attack-ground out of every row's range hands off when queued, waits aiming when last;
- a garrisoned squad with no window facing the point hands off (known unknown 2).
Unit test in `volley.rs`: the `complete` truth table.

**Feedback that would change it.** A playtest saying squads leave too early (decision c)
or that the first shift-click feels abrupt mid-burst (decision a).

**Commit.** `feat(sim): an attack with orders behind it fires one volley and hands off`.
