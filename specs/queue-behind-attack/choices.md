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
