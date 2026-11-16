# Queue behind an attack

Any order can be shift-queued behind any attack, for every unit, jets included.
**An attack with anything queued behind it fires one volley and hands off; the last
attack in the queue keeps today's behaviour** (a unit target until it is dead or lost,
a ground point forever). The own unit publishes its whole queue as ordered step records,
and the Space view draws the chain: a red attack line ending in a reticle, then every
queued step dashed, drawn the way that order draws when current. Part 2, its own push,
draws a jet's attack line in 3D from the airframe to its target.

Every decision behind this plan was made with the user in a decision walk;
[visualizations/decision-map.html](visualizations/decision-map.html) is that record
(decisions 1–18 and a–o, with who made each). This README carries them as givens.
[choices.md](choices.md) records what implementation decides where the plan is silent.

## Next Agent Prompt

*Updated 2026-11-16.* You are implementing this spec. Done so far: the behaviour-neutral
split `00e9efe87` (command application → `battle/apply_command.rs`, attack ending →
`battle/attack_order.rs`), and FX1/FX2 written with tests seen failing first (see the
slice files for state). Pick up at the first unchecked item below. Two lanes may run in
parallel in separate worktrees: the **sim lane** (FX2, S1–S5) and the **presentation
lane** (FX3, S6–S9); S6 merges after S2. Run only the domain tests a change reaches;
the full closeout run happens once, in S10. Before ending a pass, update this section
and tick the list.

Presentation lane (branch `t3/queue-behind-attack-presentation`): FX3 and S6–S9 are done.
Left for the integrator, each in its slice file: one fresh browser parity capture (S6; the
capture's battle is stale on the base commit), the queued-step test behind an attack once
S2 lands (S6), and decision 17's chain capture in a real skirmish (S9). The scene
baselines (`unit-roster` and others with the top bar) predate staging's score meter and
need a person to re-bless them.

- [x] Split `apply_units` / `update_pursuit` out of `battle.rs` (`00e9efe87`)
- [x] [FX1](slices/fx1-garrisoned-attack-ends.md) garrisoned attack ends on target death (`fb32920bd`)
- [x] [FX2](slices/fx2-queued-command-keeps-current.md) a queued command breaks nothing off (`687cdd2ca`)
- [x] [FX3](slices/fx3-blocked-first-leg.md) a blocked unit's first queued leg is drawn
- [x] [S1](slices/s1-cycle-end-and-ordered-fire.md) one owner for a cycle's end; per-mount ordered-fire report (`9e9825920`)
- [x] [S2](slices/s2-admission-and-take-effect.md) queued orders admitted behind attacks; queued orders take effect at their turn (`0f2982a3f`)
- [x] [S3](slices/s3-volley-handoff.md) the volley handoff (every attack not flown as a pass) (`2f1244b69`)
- [x] [S4](slices/s4-jet-run-volley.md) a jet's volley is one run in which it fired (`9ca23cbab`)
- [x] [S5](slices/s5-jet-lost-target.md) a jet's attack ends when its side loses the target (`6a4bce2cf`)
- [x] [S6](slices/s6-step-records.md) step records end to end, no visual change (parity captures await one fresh recording, see the slice)
- [x] [S7](slices/s7-voice-steps.md) the voice queue reads the published steps
- [x] [S8](slices/s8-attack-line.md) current attack line, reticle, chase-end reticle
- [x] [S9](slices/s9-queued-chain.md) the queued chain
- [ ] [S10](slices/s10-closeout.md) docs, `/review`, closeout run, push part 1 to staging
- [ ] [P1](slices/p1-jet-3d-attack-line.md) part 2: a jet's 3D attack line, its own push

## Contracts

| # | Contract | Before | After | Slice |
|---|---|---|---|---|
| C1 | **Order admission** (`battle/move_placement.rs`) | A queued Move, AttackMove or Unload-to-a-point behind an `Attack` is unplaced and dropped without a word (`after_attack`) | Placed, planned from the last queued move's end or the unit's position. The "queued behind an indefinite attack" case of `NoValidDestination` goes away. `Order` and `CommandEnvelope` don't change. | S2 |
| C2 | **A queued command's immediate effect** (`battle/apply_command.rs`) | Every queue-joining command clears `withdrawing`, breaks off a jet's run and dogfight; a queued Attack switches the unit to fire at will and a queued air Attack starts its dogfight at once | A queued command touches nothing on the current order. Fire at will and the dogfight target are set when the order reaches the front. | FX2, S2 |
| C3 | **`UnitOrder::Attack`** (`units.rs`) | `{ target, last_known }` | Plus volley progress, absent until something is queued behind it, and a "waits its turn" mark. Each folds into the digest only when present. | S2, S3 |
| C4 | **Cycle end** (`weapons/cycle.rs`) | Implicit inside `weapons.rs::fire` | One function spends a round and reports whether it ended a firing cycle; `Shot` carries it. | S1 |
| C5 | **Ordered-fire report** (`weapons/tick.rs`) | Folded into `ordered_ok` / `ordered_far` | `advance` reports, per unit with an ordered target, which mounts are firing at it this tick (after the garrison window gate) and whether any can damage it. | S1 |
| C6 | **`OwnUnit`** (`contract/src/observation.rs`) | `queue: [x,y][]`, `queue_unloads: bool[]`, `goal_unloads: bool` — moves only | `steps: OrderStep[]`, the current order first, then every queued order: move, attack, garrison, board, collect, unload, exit, return to base. An attack step carries its target and a read-only point. `queue`, `queue_unloads`, `goal_unloads` are deleted. `goal` still means the chase point. | S6 |
| C7 | **Publication layout** (`publication.rs`, reader) | Own row `queueCount`, `goalUnloads`; section `queue: x,y,unload` | Own row `stepCount`; section `steps`; a new names table `stepKinds`. Targets reuse the `targetKinds` encoding, hosts the `hostKinds` one. Codec vectors hand-edited, stream hashes re-blessed, parity captures re-published once. **Server and web deploy together.** | S6 |
| C8 | **Renderer input** (`OrderView`) | `queue`, `goalUnloads`, `queueUnloads` | `steps`; one function says where a unit's orders end (ghost, drop line). | S6 |
| C9 | **Voice queue input** (`Arrivals`) | Its own command-side copy of the queue | The published front step decides when a non-journey entry ends. | S7 |
| C10 | **Battle digest** | — | Byte-identical for every battle that never queues behind an attack (the skirmish AI, encounters and endurance never queue). Named exceptions, each in its own commit: FX1 and S5, which change behaviour with nothing queued. | all |

## The rule, exactly (decisions 2–4, a–f, h, i)

- **Volley-unless-last.** With any order queued behind it, an attack (unit, contact or
  ground) fires one volley and hands off. As the last order it keeps today's behaviour.
  "Something is queued" is read live each tick.
- **A volley** is one firing cycle per weapon row firing at the target, from the weapon's
  data: its `burst` if it has one, else its whole magazine, else one shot; running dry
  completes it. A squad row is done when the first of its guns finishes a cycle. Counting
  starts the tick something is first queued; a cycle under way then counts when it ends;
  a row not firing then must fire one full cycle.
- **For an attack flown as passes** (a fixed-wing jet on a ground or ground-unit target),
  a volley is one run in which its weapon fired. A run broken off with nothing fired
  doesn't count. A jet attacking without passes (a dogfight) uses the cycle rule.
- **Hands off at once** when, with something queued, no row can fire at the point
  (out of range, no ammo, no window facing it) or no row can damage the unit target
  (`weapons::compatible`, for every unit). A target that dies or is lost mid-volley ends it.
- **A jet's attack ends when its side loses the target's track** (with nothing queued it
  circles there). Garrisoned attacks end on target death or loss (FX1).

## Single-owner invariants

| Concept | One owner | Never |
|---|---|---|
| When a round ends a firing cycle | `Cycle::spend_round` in `weapons/cycle.rs` | Re-derived from round arithmetic elsewhere |
| Which mounts fire at the order this tick | the ordered-fire report from `weapons::tick::advance` | The previous tick's `unit.reach` |
| "Can damage" | `weapons::compatible` | `air::engaging_rows` for this decision |
| Volley progress and "is the volley done" | `crates/sim/src/volley.rs` (pure) | Fields on `Unit` or `Mount` |
| When an attack takes effect, hands off or ends | `battle/attack_order.rs` | New pop sites in `battle.rs`, `air/`, `movement/` |
| Replacing vs joining the queue | `battle/apply_command.rs` + `Unit::enqueue` | Per-order special cases elsewhere |
| Order → published step | one sim function (`battle/order_steps.rs`) | The publisher matching `UnitOrder` itself |
| A step's point | read-only from side knowledge; `way_home` from a given origin for return to base | Writing `last_known` for queued attacks |
| Target cells in a packed row | one encoder/decoder pair shared by mount rows and step rows | A second copy |
| Where a unit's orders end | one TS function over `steps` | `queue.at(-1) ?? goal` copies |
| How a step kind looks | one drawer per kind, current or queued | Separate queued/current paths |
| The reticle | one mark, for in-range and chase-end | Per-call-site geometry |
| Attack colour | `OrderStyle.attack = [...gameHud.bad, 1]` (the attack-ground footprint's red) | `hud.enemy` or a new hue |

The end state reads as one queue, one cycle mechanism, one eligibility rule and one
mark vocabulary — designed today, not tacked on.

## Firewalls

- **F1 Digests** (C10). Every new state is folded only when present; no-queue control
  flow, RNG draws and iteration order are untouched. A moved digest is stopped and traced
  to its named decision, never blanket-blessed.
- **F2 `goal`** stays the chase point; the skirmish AI and voice read it.
- **F3 Jets' look in part 1** is unchanged: no ground chain, line or reticle under a jet.
- **F4 Plain move drawing**: `journey-space-orders.png` stays matched until S8/S9
  deliberately change attack drawing only.
- **F5 Size ratchet**: `battle.rs`, `weapons.rs`, `publication.rs`,
  `publication/reader.rs` and `tests/garrison.rs` are over the 8k-token budget and may
  not grow in any commit. New concepts get their own modules; a split lands as its own
  behaviour-neutral commit first.
- **F6** Unload without a goal still replaces the queue (documented contract).
- **F7** Return to base still replaces trailing orders when it starts.

## Slice graph

```
split (done) ─► FX1 ─► FX2 ─► S1 ─► S2 ─► S3 ─► S4 ─► S5 ─────────────┐
                                     └──────► S6 ─► S7                 ├─► S10 ─► push part 1 ─► P1 ─► push
                         FX3 ──────────────► S6 ─► S8 ─► S9 ──────────┘
```

Sim lane: FX1, FX2, S1–S5. Presentation lane: FX3, S6–S9 (S6 needs S2's admission only
for its end-to-end test of queued attack steps).

## Known unknowns

1. Do FX1 or S5 move a pinned digest? Run the sim crate's digest and replay tests after
   each; a move is that commit's named decision, re-blessed there with the reason. A
   moved parity-capture digest can't be re-blessed (`capture_publication.rs` asserts
   it) — report it before paying for a fresh capture.
2. Is the garrison window gate's result reachable from the ordered-fire report (S1)? A
   garrisoned squad with no window facing the point must count as "can't fire".
3. Edge-guarded runs near a field corner: pin that a jet still hands off within a
   bounded number of runs; if it can't, report it rather than inventing a rule.
4. Step target ids use the mount row's one-f32 encoding; contact and track ids are
   assumed below 2^24 in a battle (debug-asserted).
5. Parity re-publication needs which LFS sources — fetch only what the tests refuse.

## Review map

| Slice | Read first | Question |
|---|---|---|
| FX1–FX2 | the test, then the diff | Fails before, passes after? `battle.rs` didn't grow? |
| S1 | `cycle.rs`, `tick.rs` | Behaviour-identical (digests unchanged)? Instant refill still reports a boundary? |
| S2–S3 | `volley.rs`, `attack_order.rs`, tests | One rule, one function? Any path that can hang? Changed tests weakened? |
| S4–S5 | `air/` diff | The bomb still drops on the run that ends the attack? |
| S6 | `order_steps.rs`, step row, TS view, fixture diff | Zero visual change? Fixtures changed only by bless or by the plan's hand edits? |
| S8–S9 | the screenshots, then the drawers | Reads as an RTS order chain at default and far zoom? |
