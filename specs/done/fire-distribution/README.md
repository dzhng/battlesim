# Fire distribution

Automatic targeting spreads fire across targets instead of piling onto one,
and accurate weapons hold a shot that would only add to a kill already on its
way. Both are rules of the simulation's automatic target choice; explicit
attack orders, from a player or the skirmish AI, are untouched.

- **Target spread.** Among targets that tie on priority, a crew takes the one
  at the least *weighted distance*: farther for each other friendly unit
  already locked on it, and scaled by one seeded factor per unit, mount and
  target. Purchase cost no longer ranks targets.
- **Overkill coordination.** A weapon row that opts in (`avoids_overkill`)
  holds its shot while the damage its side already has flying at the target,
  landing no later than its own round, covers the target's remaining health.
  The crew keeps its aim and fires the moment the target is no longer doomed.

The walk that settled every decision is kept as
[visualizations/unknowns-map.html](visualizations/unknowns-map.html); where it
differs from this record, this record and [decisions.md](decisions.md) win.

## Why it works this way

**Fire looked wrong.** Every crew ranked targets the same way, so a whole line
fired at one enemy, usually the most expensive or the nearest, while others
walked in untouched. In a war film the crews split up, and a missile team
doesn't fire a second missile at a jeep the first one is about to destroy.

**Cost was the wrong signal.** Cheap units are meant to be cannon fodder. An
expensive unit should draw fire because it is at the front or because the line
spreads onto it, never for its price, so the cost key was deleted rather than
reweighted.

**Spread acts only on a fresh choice.** Locks stay sticky: a crew keeps its
target until it can no longer shoot it (or, for a flagged row, until the
target is doomed), so spread shows exactly where it is visible, when a group
of enemies appears and crews fan out across it, and never as crews hopping
between targets as the ranking shifts.

**Overkill counts rounds already fired, never intentions.** Counting other
crews' locks would let two crews on one target each see the other covering
it and both leave. Rounds in flight (plus this tick's launches, in unit order)
are facts, so the rule can never empty a target. Only damage landing before
the crew's own round counts, so a fast gun never idles behind a slow missile;
rate of fire is not predicted.

**Only accurate weapons opt in.** With misses ignored, inaccurate fire (tank
HE shells on infantry, machine guns, AA, artillery) would hold for rounds that
won't hit.
The flag sits on the weapon row, not the mount, because one mount (a tank's
cannon) carries both AP and HE. It is authored on `atgm`, `tank_ap`,
`sead_missile` and `grenade`, so every ATGM, tank sabot, SEAD missile, grenade
and RPG inherits it through `extends`, including the high-explosive grenade
and general-purpose RPG, which were flagged on request with the other RPGs. Artillery and AA stay out
because nobody flags them, not because code checks for them.

**It reads real state.** Remaining health and active-protection charges come
from the simulation itself, not the side's knowledge. This is internal
automatic behaviour, never shown to a player or read by the AI; the cost is
that a player could infer a nearly dead enemy from their own crews holding.

## Invariants

- Automatic choice ranks by seeker match, then return fire, then the damage
  share through the target's armour, then weighted distance, then observed id.
  No key reads purchase cost.
- Only friendly units on the field count as engaging a target; a dead or
  riding unit's leftover lock engages nothing.
- The seeded spread factor is drawn from the battle seed, the unit, the
  mount's spec index and the target, never the tick, so a crew's preference
  between equal targets holds still and replays exactly.
- A counted round is a flagged row's round aimed at a unit, before its
  expected arrival, and still steering if its row is guided: a seeker row
  while its round keeps a seeker target, a launcher-guided row while its
  projectile's guidance is supported. Guidedness is read from the row, since a
  seeker round launches with launcher support off.
- A round's expected arrival is fixed at launch from the shot's time of
  flight. It is digested with the round. A shell that misses stops counting
  once its arrival passes.
- Active protection absorbs the earliest interceptable counted rounds by
  arrival, one per charge left; its cooldown is ignored, which can only make
  crews over-commit.
- A held mount keeps its lock, aim, reload, turret tracking and `engaging`
  flag, reports `HoldingFire`, and still sets `Reach.can_engage`, so
  attack-move halts and cover facing holds.
- A held crew escapes only by changing target, never by switching ammo or
  weapon at the same target. A mount guiding a missile never drops its target
  for being doomed (losing the shot still frees it).
- When every candidate is doomed, the current lock wins.
- An explicit order is exempt by its target: a lock on the ordered target
  never holds, while locks taken while the ordered target can't be shot do.

## Where it lives

| Concept | Code |
|---|---|
| Automatic target choice, weighted distance, engaging count, seeded factor | `crates/sim/src/weapons/choice.rs` (`select`, `engaging`, `spread_factor`, `automatic_lock`) |
| Spread weights | `TargetingRules` in `crates/contract/src/scenario/targeting.rs`; the `targeting` section of `fixtures/rules.json` |
| The weapon-row flag | `WeaponDefinition.avoids_overkill` in `crates/contract/src/weapons.rs` |
| Committed fire: counted rounds, Trophy discount, remaining health, the doomed test | `crates/sim/src/weapons/overkill.rs` (`Committed`, `counted`, `arrival_tick`, `doomed`, `overkill`) |
| Building the ledger from rounds in flight, expected arrival | `Battle::fire` and `Round.expected_arrival_tick` in `crates/sim/src/battle/fire.rs` |
| Same-tick tally and the hold gate | `advance` in `crates/sim/src/weapons/tick.rs` |
| Behaviour tests | `crates/sim/tests/fire_distribution.rs`, `crates/sim/tests/target_priority.rs`, unit tests in `overkill.rs` |

The rules are stated for readers of the simulation in
[crates/sim/README.md](../../../crates/sim/README.md) under battle rules. Any
whole rules document must carry the `targeting` section; the map generator's
record `fixtures/parity/map-layout/physical-rules.json` is one.

## Measured effect

Measured at close, not pinned by any test. In the endurance soak (100 a side,
10 simulated minutes, seed 1), the same
losses (171 vs 170 units alive, 28 vs 29 wrecks) took 4.3% fewer rounds, for
1.3% more step instructions and a lower peak weapons tick. The generated-map
battle sweep barely reaches a fight and showed no change. No retune followed.

## Dead ends

- **A 2× health margin for misses.** Dropped once only accurate rows count and
  only damage landing first is counted.
- **Exemptions derived from indirect fire or an airborne target.** No flagged
  row fires indirect or at aircraft, so the checks were dead; data decides.
- **Ignoring a unit's own rounds.** Unneeded once only accurate rows count; a
  missile team should hold its second missile while the first flies.
- **Counting a lock as a claim.** Target spread already spreads fire at choice
  time for every weapon, and claims need an ordering between crews.
- **One weighted score for ranking.** A bigger behaviour change with more
  tuning; the strict priority keys stay and only distance is weighted.
- **Arrival as distance ÷ current speed.** A missile slow off the rail made a
  held team fire a tick later; the launch-time estimate also retires missed
  shells.
- **Predicting sustained fire.** Needs a claim order between crews; a fast gun
  may still finish a target a missile is flying at, which reads well on screen.
