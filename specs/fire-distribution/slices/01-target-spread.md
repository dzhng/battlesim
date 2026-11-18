# 01 Target spread

## Contract

Automatic target choice ranks seeker → return fire → armour share → weighted
distance → id (README, "Target spread"). Purchase cost no longer ranks.
Decisions T1–T7.

## Seam

- `crates/contract/src/scenario/targeting.rs`: `TargetingRules` with the two
  tuning keys (penalty per engaging unit, random spread), added to `Rules` as
  `targeting`, authored in `fixtures/rules.json`, validated at admission
  (both finite and non-negative; the spread below 1 so the factor stays
  positive).
- `FireContext` gains `seed: u64` (the battle seed), passed from
  `battle/fire.rs`.
- In `weapons/choice.rs`:
  - `engaging(units, unit, target) -> u32`: other units on the same side with
    any mount whose lock is `Target::Unit(target)`.
  - `spread_factor(seed, unit, mount, target) -> f64`: one seeded draw
    (`contract::random::{Rng, mix}`), keyed as the sim README says, never by
    tick. Map to `1 + spread × (2u − 1)`.
  - `select`: the ranking tuple drops `cost` and replaces raw distance with
    weighted distance. The default-gun fallback list orders by weighted
    distance too.

## Verify (tests first, in `crates/sim/tests/fire_distribution.rs`)

One or two functional tests on test units:

- **Fan-out.** Two friendly shooters of the same type, two identical enemy
  targets at nearly the same distance: the shooters end up locked on
  different targets.
- **Engaged loses to a bit farther.** One friendly already locked on the
  nearer of two identical targets; a second shooter picks the farther one.
- **Far still loses.** The same, with the second target far enough away that
  the penalty can't cover it (derive the distance from the rules' penalty and
  spread at run time): the second shooter joins the first.
- **Cost doesn't rank.** Two targets that tie on everything but cost and
  distance: the nearer, cheaper one is taken.
- **Stable.** A shooter's choice between two equal targets is the same when
  the battle is replayed, and doesn't flip from tick to tick while both stay
  available.

## Stays green

All sim tests except those whose outcome legitimately moves because cost no
longer ranks or targets now spread. Read each moved test; update it only if
the new outcome is this rule working. Name the digest change in the commit.

## Delegated

- The exact key mixing for the seeded draw, within the seeding rule.
- Admission error wording.

## Feedback that would change it

The user watching a battle and finding spread too strong or too weak is a
retune of the two keys, not a code change.
