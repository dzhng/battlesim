# Slice 01 — Ballistic contract and synthetic weapon probe

## Contract unlocked

A synthetic artillery weapon can be admitted and fired through the production flight/weapon path at a long distance (target capability up to about 2,500 m), chooses the indirect high arc, respects real terrain and prop collision, produces distance-scaled miss radius from angular scatter, and holds fire inside its authored minimum range without advancing toward the target.

This slice is a feasibility gate. If the requested range cannot pass the existing coupled flight constraints, stop and record the failing constraint and the smallest data decision needed; do not loosen validation or invent a second ballistic solver.

## API seam and owner

- `contract::ballistics::Trajectory::Indirect` and `WeaponBallistics` remain the data seam ([crates/contract/src/ballistics.rs](../../../crates/contract/src/ballistics.rs)).
- `crates/sim/src/flight/solve.rs` remains the high/low arc and collision owner.
- `crates/sim/src/weapons.rs` remains the minimum-range, target assessment, automatic acquisition and spread owner.
- `Order::Attack`, `Order::AttackMove`, `TargetRef::Ground` and `Engagement` remain unchanged.

Use a test-only `test_artillery` weapon/unit or fixture patch; do not make roster rows physical yet.

## Required behavior and tests

Write tests first, then implement only what turns them green:

- Indirect launch chooses the high arc and lands at the commanded point within the existing flight tolerance.
- A ridge, prop or building blocks the actual curved path before a target behind it; no obstacle is ignored because the weapon is indirect.
- Two equal angular spreads produce a larger miss radius at the farther aim point.
- A target inside `min_range_m` leaves the mount at `ActionReason::HoldingFire`, consumes no round and does not cause movement toward the target. When the target leaves the dead zone, firing resumes.
- The same weapon fires through normal identified-target Attack, Attack Move, automatic acquisition and Ground attack. `ReturnFireOnly` still blocks unprovoked ground fire.
- The profile is admitted by the real weapon/flight validation path at its selected long-range value; record the accepted speed, lifetime, gravity and scatter values in the slice evidence.

Likely owners of focused tests are [flight_ballistics.rs](../../../crates/sim/tests/flight_ballistics.rs), [flight_collision.rs](../../../crates/sim/tests/flight_collision.rs), [weapons.rs](../../../crates/sim/tests/weapons.rs), and [contract weapons.rs](../../../crates/contract/tests/weapons.rs).

## Human-visible artifact

Produce a native deterministic probe report (ignored scratch output) with near/far scatter, minimum-range hold and high-arc/blocked-path outcomes. No browser shot is required; visual projectile presentation is deferred to slice 04.

## Verification

```bash
CARGO_TARGET_DIR=throwaway/target cargo test -p sim --test sim flight_ballistics flight_collision weapons
CARGO_TARGET_DIR=throwaway/target cargo test -p contract --test weapons
```

## Firewalls and delegated choices

Do not add a manual-fire flag, an artillery command, a new damage rule, a new range ruler or a deployment timer. Internal helper names and the exact synthetic test fixture are delegated. Weapon tuning may choose distinct howitzer/mortar/rocket values later, but this slice must prove the shared mechanics and the upper-range feasibility.

## Evidence

The synthetic probe uses an admitted indirect profile with `range_m = 2,500`, `speed_mps = 160`, `gravity_scale = 1`, `lifetime_s = 30` and `scatter_mrad = 12`; the high-arc flight time is 26.1888604769 s and lands within 0.02 m of its 2,500 m aim point. A tall test wall blocks the curved high arc, and seeded equal-angle scatter scales 10× from 100 m to 1,000 m. The ignored native probe report is `throwaway/evidence/artillery-ballistic-probe.txt`. Existing production weapon tests cover minimum-range hold/resume, normal Attack/Attack Move/Ground paths and ReturnFireOnly policy.

## Acceptance and feedback

The slice is accepted when the production path proves all listed behaviors and the long-range feasibility result is recorded. If the probe reveals a coupled constraint that changes range, speed, lifetime or spread, update the README decisions ledger before moving to slice 02.
