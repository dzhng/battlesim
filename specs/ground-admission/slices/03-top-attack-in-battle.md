# 03 — Top attack in battle

**Unlocks:** proof that the decided guidance and protection rules hold for a diving
missile, and that a dive pays off, through the real battle authority.

## Seam

No new code seam unless a test exposes a bug. Tests switch the field on with the test
file's rules override (`quick(..., tweak)` in `crates/sim/tests/guidance.rs`) on the generic
`atgm` row, fielded by `test_*` units (`crates/sim/README.md`: mechanic tests run on test
units). No roster row exists yet.

## Tests (write first)

1. **Payoff.** Against a tank whose roof is below the missile's penetration and whose front
   is above it: top attack kills; the same row fired direct does not.
2. **Sight.** The gunner loses sight (target behind a building) or moves mid-flight: the
   missile releases and goes to ground at the fixed point; it misses a target that moved.
3. **APS.** A Trophy-equipped hull (`crates/sim/src/protection.rs` tests) intercepts a
   diving missile, using a real dive, not a flat shot.
4. **Overhead launch.** A launcher garrisoned in a building, and one under a tree canopy,
   fires a top-attack row. Record what happens (the climb leaves the line
   `first_obstruction` checks). If the missile strikes the ceiling or the canopy, fix it
   with a general rule: the launch admission checks the first leg of the lofted path, not
   only the straight line. Never with an `if garrisoned` branch. The fix and its
   war-film justification go in [choices.md](../choices.md).

## Human can see

Test output only; the visual is slice 04.

## Delegated

Scenario geometry; which building and canopy the overhead test uses.

## Stays green

All of `guidance.rs`, the protection tests, and `crates/sim/tests/weapons.rs`.

## Would change this slice

Test 4 finding the overhead case needs more than a launch-check change. Then split it out
as its own slice before 11.
