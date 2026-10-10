# 02 — Top-attack flight

**Unlocks:** an authored, validated `top_attack` weapon-row property, and a missile in
the flight store that climbs, pitches over and dives onto its commanded point. No battle,
no roster row yet.

Invoke [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md) first: picture the
shot — a missile rising visibly off the launcher, nosing over, falling steeply onto a turret.
Then [write-tests](../../../.agents/skills/write-tests/SKILL.md): tests first.

## Seam

The [contract in the README](../README.md#contract-top-attack) is binding:

- `crates/contract/src/ballistics.rs`: `WeaponBallistics.top_attack: Option<TopAttack>`,
  `TopAttack { loft_m, dive_deg }`, `deny_unknown_fields`.
- `crates/sim/src/flight/mod.rs`:
  - `FlightConfig::profile` admits it (new `FlightConfigError::TopAttack` beside `Motor`) and
    resolves `LaunchProfile.loft: Option<Loft { height_m, slope }>`;
  - `Guidance.loft`; `Guidance::aim(position) -> V3`; `Flight::acceleration` steers at `aim`;
  - the digest hashes `loft` only when present.
- `crates/sim/src/flight/solve.rs`: the launch copies the loft into its `Guidance`.
- If the mechanics editor validates weapon rows field by field, expose the field there
  (check `apps/mechanics-editor`; unverified).

## Tests (new `crates/sim/tests/top_attack.rs`, flight store only)

1. `aim` table: before the dive cone it returns the lofted point; inside the cone, the
   commanded point; after release, the commanded point.
2. A tuned row fired at a still armoured box at mid range: the apex is at least a set
   fraction of `loft_m` above the line of sight; the struck face is Roof; the descent angle
   at impact is at least a set threshold.
3. Control: the same row without `top_attack` strikes Front.
4. A short shot (about 100 m) still launches and lands.
5. Fired at the row's `range_m`, the missile arrives before `lifetime_s`.
6. Refusals in `crates/sim/tests/flight_load.rs`: `top_attack` without `turn_deg_s`;
   non-positive `loft_m`; `dive_deg` of 0 or 90.
7. Two runs with the same seed give equal digests; an existing guided-missile digest
   (TOW/Kornet) is unchanged.

## Human can see

A printed trace (height against range) from test 2, kept in
`throwaway/ground-admission/traces/`. The visual capture is slice 04.

## Delegated

Test thresholds; whether a released missile drops its loft or keeps a fixed lofted point
(the contract says it drops it — change only with a trace showing why, recorded in
[choices.md](../choices.md)); internal naming.

## Stays green

`crates/sim/tests/guidance.rs`, `flight_load.rs`, the flight tests, and every digest of a
battle that fires no top-attack row.

## Would change this slice

A trace showing the stateless aim cannot reach a steep dive at play ranges even with tuned
loft and turn rate. Then reslice: add explicit phase state, and record why in the README.
