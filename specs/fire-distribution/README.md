# Fire distribution

**Status:** planned, nothing implemented. **Updated:** 2026-11-19.

Two changes to how a crew picks and keeps its target in automatic targeting:

- **Target spread.** Crews spread across targets that tie on priority instead
  of all taking the nearest. A target other friendly units already engage
  counts as farther away, a small seeded random factor breaks ties, and
  purchase cost stops deciding anything.
- **Overkill coordination.** A weapon row that opts in (`avoids_overkill`)
  holds its shot while the damage its side already has flying at the target,
  landing before its own round would, reaches the target's remaining health.
  The crew keeps its aim and fires the moment the target stops being doomed.

## Next Agent Prompt

You are implementing the fire-distribution spec in `crates/sim`. Nothing has
landed yet. Start at [slice 00](slices/00-make-room.md) and go in order. Each
slice file names its contract, seam, tests, what stays green and what you may
decide yourself. Anything else it leaves open is a gap: record it in the
decisions ledger below rather than inventing it silently.

Before coding, read `AGENTS.md`, `development.md` (game rules, check ladder),
`crates/sim/README.md` (determinism, seeding, tests) and load the
`write-tests` and `tweak-mechanics` skills. Write each slice's tests first.
Iterate on `cargo test -p sim --test sim <filter>` only. Run the full checks and
the balance report once, in slice 04.

Warnings:

- `crates/sim/src/weapons.rs` and `crates/contract/src/scenario.rs` are over
  the file-size budget and may not grow. Slice 00 makes the room; never add
  lines to either without removing more.
- Slices 01 and 03 change digests on purpose. Name each change in its commit
  message. Never loosen an existing test to get green: read why its outcome
  moved and decide whether the new outcome is the rule working.
- There is no visual surface. No screenshot gates apply.

TODO:

- [ ] [00](slices/00-make-room.md) Move target choice into `weapons/choice.rs`; digests identical.
- [ ] [01](slices/01-target-spread.md) Target spread: drop the cost key, weighted distance, seeded spread.
- [ ] [02](slices/02-committed-ledger.md) The committed-fire ledger as a pure, unit-tested owner; the contract flag.
- [ ] [03](slices/03-overkill-in-play.md) Overkill in play: build the ledger, tally launches, the hold gate, reconsider.
- [ ] [04](slices/04-author-and-closeout.md) Author the flags, regenerate, full checks, balance report, docs, close.

Update this section (status, date, ticks, warnings) before you end each pass.

## Contracts

| Contract | Change |
|---|---|
| Weapon row (`WeaponDefinition`, `crates/contract/src/weapons.rs`) | New `avoids_overkill: bool`, serde default false, skipped when false, inherited through `extends`. |
| Rules | New `targeting` section (`TargetingRules`, its own file under `crates/contract/src/scenario/`) holding the two target-spread tuning keys: the distance penalty per engaging unit and the random spread. Docs name the keys, never their values. |
| `Round` (`crates/sim/src/battle/fire.rs`) | New expected-arrival tick, set at launch from the shot's time of flight. Digested, like every other `Round` field. |
| `FireContext` (`crates/sim/src/weapons.rs`) | Gains the battle seed (target spread) and a read-only `&Committed` (overkill). Becomes `Clone + Copy`; all its fields are references or scalars. |
| Publication and observation | Unchanged. A held mount reports the existing `ActionReason::HoldingFire`. |
| Fixtures | `rules.json`: `avoids_overkill: true` on `atgm`, `tank_ap`, `sead_missile`, `grenade`; the new `targeting` section. `catalog.json` regenerated. |

No backward compatibility, migrations or compat shims. Recorded fixtures and the
menu reel are re-recorded if they move.

## The rules

### Target spread (automatic targeting only)

`select` ranks the targets a mount could shoot now by, in order:

1. a target a loaded seeker row engages;
2. a target that is shooting at this unit (return-fire threat);
3. the largest damage share through the face the target shows;
4. **weighted distance** = distance × (1 + penalty × engaging units) × the
   unit's seeded spread factor for that target;
5. observed id.

Purchase cost is no longer a key. "Engaging units" counts other friendly units
with any mount locked on the target, read from the unit array as `tick::advance`
leaves it (units earlier in id order this tick, the rest from last tick). The
seeded factor is drawn from the battle seed, the unit, the mount and the target,
never the tick. The default gun's plinking fallback (nearest target it can't
hurt) uses the same weighted distance. Spread acts only when a crew picks a
fresh target; sticky locks are unchanged.

### Overkill coordination (automatic targeting only)

A **counted round** is one of the side's rounds in flight whose weapon row sets
`avoids_overkill`, aimed at a unit (`Round.aimed_at`), whose expected arrival
has not passed, and that is still steering if its row is guided (seeker row:
`Round.seeker_target` set; launcher-guided row: its projectile's guidance is
supported). Earlier launches in the same tick are counted rounds too.

A counted round's damage is the row's `damage` × `pierce_share` on the face
the target shows the round (bounces 0); a soldier takes all of it. While the
target has active-protection charges, its first N interceptable counted rounds
by arrival count zero, N being its charges left (cooldown ignored).

A target is **doomed** for a mount when the counted damage arriving no later
than the mount's own shot (`Ready.time_of_flight_s`) reaches the target's real
remaining health: a vehicle's `hp`, or the sum of a squad's living soldiers'
`hp`.

Then:

- **Hold.** A mount whose chosen kind against its lock (`Ready.kind`) is
  flagged, and whose target is doomed, does not fire. It keeps its lock, aim,
  reload and turret tracking, stays `engaging`, reports `HoldingFire`, and fires
  on the first tick the target is no longer doomed.
- **Escape only by changing target.** A held crew never switches ammo or weapon
  at the same target. One mount, one lock, one hold.
- **Reconsider.** A doomed lock makes the crew look again; `select` returns a
  doomed target only when nothing else qualifies, and then the current lock
  wins. A mount guiding a missile does not reconsider.
- **Untouched:** explicit attack orders (exempt by `lock.target` equal to the
  ordered target), unflagged weapons, and the default gun firing at what it
  cannot hurt.
- A held mount still sets `Reach.can_engage`, so attack-move halts.

## One owner per concept

| Concept | Owner |
|---|---|
| Automatic target choice and its ranking | `crates/sim/src/weapons/choice.rs` (moved out of `weapons.rs` in slice 00) |
| Committed fire: counted rounds, their damage, Trophy discount, remaining health, `doomed` | `crates/sim/src/weapons/overkill.rs`. Only it and the contract mention `avoids_overkill`. |
| Whether a round is still steering | Read from the flight state `guide()` already owns (`battle/fire.rs`); no new release flag. |
| The tick's running tally of launches | `weapons::tick::advance` |
| When a round is expected to arrive | `Round`'s expected-arrival tick, set once at launch |
| Target-spread tuning | `TargetingRules` in the rules |

No second "rounds aimed at a unit" owner: `shouts.rs`, `damage.rs` and
`state_digest.rs` each read `aimed_at` per round and stay as they are.
`IncomingRound` stays the enemy's view of interceptable rounds and is not
reused. The end state reads as designed today: target choice in one module,
overkill as one pure owner that choice and the fire gate consult.

## Slice graph

```
00 make room ─┬─ 01 target spread ──────────────┐
              └─ 02 committed ledger ─ 03 overkill in play ─┴─ 04 author + closeout
```

01 and 02 are independent after 00. 03 needs both 00 and 02.

## Decisions ledger

Closed by the user in the walk unless marked. The walk's map is in
[visualizations/unknowns-map.html](visualizations/unknowns-map.html).
Where this ledger differs from the map, this ledger wins.

### Overkill

| # | Decision | Why |
|---|---|---|
| D1 | Automatic targeting only; explicit orders (player and AI) untouched. | User: not for the human, not for the game AI. |
| D2 | Opt-in weapon-row flag `avoids_overkill`. A flagged row's rounds count and a flagged row holds; unflagged rows do neither. | User asked per mount; a tank's `cannon` mount carries AP and HE, so the flag sits on the row (territory). |
| D3 | Authored on `atgm`, `tank_ap`, `sead_missile`, `grenade`. Through `extends` this also flags `tow`, `kornet`, `spike`, `javelin`, `akeron_mp`, `bastion`, `heli_atgm`, `air_ground_missile`, the tank AP rows, and `rpg_at_light`, `rpg_at_heavy`, `rpg_general`. Not `tank_he`, AA or air missiles, guns, bombs, artillery. | User: accurate fire only. `rpg_general` (an HE RPG) is flagged because the user said RPGs yes; flip it in data if that's wrong. |
| D4 | Doomed at 1× real remaining health. | User dropped the 2× accuracy margin once only accurate weapons count. |
| D5 | Health from real state: vehicle `hp`, or sum of living soldiers' `hp`. | User: internal automatic behaviour; simplest. |
| D6 | Counted rounds are rounds in flight plus same-tick launches; locks never count. | Counting locks makes two mutually locked crews both leave. |
| D7 | Damage × `pierce_share` on the face the target shows the round. | User asked for real damage after penetration. |
| D8 | Only damage arriving no later than this mount's own shot counts. Rate of fire is not predicted. | User: a fast gun must not idle behind a slow missile. |
| D9 | Trophy: first N interceptable counted rounds by arrival count zero; cooldown ignored. | User: two ATGMs still fire at a one-charge Trophy. |
| D10 | A guided round stops counting once released. | User. |
| D11 | Hold rather than fire at a doomed target; reuse `HoldingFire`. | User: crews may hold fire. |
| D12 | The hold is a gate at the firing step; aim, reload and tracking continue. | User: a miss gets an instant follow-up. Territory: dropping the lock or failing the assessment resets aim (`weapons/tick.rs`). |
| D13 | One mount, one lock, one hold; escape only by changing target. | User. |
| D14 | A doomed lock triggers a fresh look; doomed targets rank last. | Sticky locks would otherwise make the rule inert. |
| D15 | A held mount keeps `Reach.can_engage`. | User. |
| D16 | Same-tick launches tallied in unit-id order. | The most common overkill is two tanks finishing their aim together. |
| D17 | Default-gun plinking unchanged. | User: cinematic. |
| D18 | Committed fire is derived from digested state; the only new digested state is the round's expected arrival (A1). | Determinism. |

### Synthesis decisions (agent, shown to the user 2026-11-19)

| # | Decision | Why |
|---|---|---|
| A1 | A round's arrival is its expected-arrival tick, recorded at launch from `Ready.time_of_flight_s`, replacing distance ÷ current speed. A round past its expected arrival stops counting. | A missile is slow off the rail, so distance ÷ speed made a held team fire a tick later (draft C). The same field drops a shell that missed and flew past (draft B). One field, two fixes. |
| A2 | A held mount stays `engaging`. | `engaging = false` clears the cycle's start and turns off `weapons::engaged`, which cover facing and helicopter descent read (`cover.rs`, `movement/air.rs`). |
| A3 | A mount guiding a missile does not reconsider. | Its own missile would doom its lock and swing the turret away mid-flight. It can't fire anyway (`Guiding`). |
| A4 | When every candidate is doomed, the current lock wins. | No aim-resetting swaps between doomed targets. |
| A5 | Arrival ties count as landing first (≤). | Symmetric layouts give equal flight times. |
| A6 | Explicit-order exemption by `lock.target == ordered`, not "has an order". | While the ordered target can't be shot, `choose_lock` falls back to `automatic_lock`, and those locks must follow the rule. |
| A7 | One face function in `overkill.rs`: the face the target shows the round's current position; a same-tick launch uses its muzzle. | One owner of "which face", consistent for lofted rounds. |
| A8 | Committed is keyed by (side, target). | Each side counts only its own rounds. |
| A9 | `avoids_overkill` is skipped when false in serialization. | Keeps `catalog.json` and re-serialized setups from gaining a `false` on every row. |

### Target spread

| # | Decision | Why |
|---|---|---|
| T1 | Delete the purchase-cost key from `select`. | User: cheap units are cannon fodder; an expensive unit is targeted because of spread or position, never its price. |
| T2 | Weighted distance replaces the distance key; seeker, return fire and armour share stay above it. | User chose this over a single weighted score. |
| T3 | "Engaging" counts other friendly units with any mount locked on the target, in unit-id order; own unit excluded; per unit, not per mount. | User ok. Id order means two crews never both dodge. |
| T4 | Spread only affects fresh picks; sticky locks unchanged. | User ok. |
| T5 | The default gun's plinking fallback uses weighted distance too. | User ok. |
| T6 | Two tuning keys in a new rules `targeting` section. | User ok (the walk said the `weapons` section; that section is the weapon rows, so the keys get their own). |
| T7 | Seeded by battle seed, unit, mount and target; never the tick. | User ok; the sim README's seeding rule. |
| T8 | One spec covering both mechanics. | User ok. |

### Rejected

- **2× health margin:** replaced by D4 and D8.
- **Exemptions derived from indirect fire or airborne targets:** no flagged row fires indirect or at aircraft, so the checks would be dead.
- **Filtering a unit's own rounds:** moot once only accurate rows count; a Javelin team should hold its second missile while the first flies.
- **Counting a lock as a claim (overkill):** target spread spreads fire at choice time for every weapon.
- **A single weighted score for ranking:** bigger behaviour change, more tuning.
- **Predicting sustained fire:** needs a claim order between crews.
- **Estimating arrival with the motor model:** A1 is simpler and covers missed rounds too.

### Ledger for decisions made during implementation

| # | Slice | Decision | Why |
|---|---|---|---|

## Sharp edges

- An expected arrival is a launch-time estimate; a target that drives away makes
  the round late, and the round stops counting at its expected tick. Slight
  overkill, the accepted direction.
- A second flagged mount on the same unit sees that unit's same-tick launches
  only at the gate, not in its choice. It holds this tick and switches next tick.
- Window allocation for garrisons (`garrison_aims`) sees only the
  start-of-tick ledger. Harmless.
- Every held mount re-runs `select` each tick. Watch instruction counts in the
  balance report.

## Verification

- Behaviour tests live in one file, `crates/sim/tests/fire_distribution.rs`,
  as a few bigger functional tests over the test units (`test_*` via
  `fixtures::test_rules`), never the roster.
- Tests set `avoids_overkill` on the rows they use in their own rules and
  derive hp thresholds from the row's damage at run time. No test pins a
  config value, a roster's flag list or a tuning key's value.
- The ledger's ordering logic gets unit tests inside `overkill.rs`, the one
  place fast unit feedback pays.
- Quick sample while iterating: the battle sweep in quick mode
  (`crates/mapgen/examples/battle_sweep.rs`). Full balance report once, in slice 04.
