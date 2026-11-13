# 11: skirmish AI

## Contract

The skirmish AI's balanced doctrine notices when one of its units with a smoke
charge is losing a fight. It then pops that unit's smoke and orders it to
reverse out (D4, D22). It never overwrites that reverse with a new attack.
Player units are unaffected (D3).

## Seam

The logic lives in a new `crates/sim/src/skirmish_ai/smoke.rs`, because
`skirmish_ai/mod.rs` is over budget.
- **State:** `HpWatch { baseline: BTreeMap<UnitId, (f64, Tick)> }` is held in
  `BalancedDoctrine`, next to `held` (`mod.rs:157`). It tracks `hp`, or the sum
  of `member_hp` for squads, because `OwnUnit.hp` is 0 for infantry.
- **Decision:** `smoke_orders(own, &mut HpWatch, &Tuning, tick) -> Vec<Order>`.
  For each unit whose loss within `smoke_window_s` is at least
  `smoke_loss_share` of its maximum and which has `OwnUnit.smoke > 0`, it
  returns `[PopSmoke { units }, Move { direction: Reverse, to: smoke_reverse_m
  straight back }]`, in that order. The rate limit can drop the second order but
  never the pop.
- **Precedence:** these orders come before the combat and force-advance
  branches (`mod.rs:870`, `:889-935`). Those branches skip any unit with
  `direction == Some(Reverse)` and a live goal, the same way `Relocation::Driving`
  is skipped.
- **Determinism:** no random draws are added (`mod.rs:~475`).
- **Tuning:** `smoke_loss_share` 0.2, `smoke_window_s` 10 and `smoke_reverse_m`
  40, in both `fixtures/skirmish-ai.json` and the `Tuning` struct, which has
  `deny_unknown_fields`. These are starting data (choices), not targets.
- The scripted encounter defender (`encounter/defender.rs`) is unchanged.

## What you can see

A skirmish against an AI whose tanks pop smoke and back out when they lose an
exchange.

## Tests (write them first)

These use `skirmish_ai/tests.rs` frames.
- A carrier that is losing gets a pop, then a reverse.
- While reversing, the next decision issues no pop and no attack for it.
- An untouched unit never pops.
- A unit with no charges gets no pop.
- Friendly-fire damage counts. This is an accepted quirk; the test documents it.
- The seeded skirmish corpus replays exactly. Name its digest shift in
  `choices.md`.

## Delegated

You may choose how "straight back" is computed: along the hull's facing,
clamped to the map. You may also choose when `HpWatch` prunes dead units.

## Must stay green

- Every other AI test.
- The self-play replay-digest check.
- The encounter defender.

## Feedback that would change this slice

The user may find the AI pops too eagerly or too late. Retune the three
numbers only.
