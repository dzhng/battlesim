# Fire distribution: decisions

Every decision behind the shipped code, checked against it at close. "You"
decided the ones marked as yours in the planning walk; the rest were made by
the implementing agent where the plan was silent. Least confident first
within each group.

## Made by the agent, worth a second look

- **The grenade root flags two high-explosive rows.** `grenade` itself and
  `rpg_general` (an HE rocket for infantry) inherit `avoids_overkill`,
  because you said yes to RPGs and grenades. Both miss infantry often, the
  reason tank HE stayed out; setting the flag false on either row reverses it.
- **The spread weights' starting values.** One engaging unit makes a target
  count about 1.5× as far, and the seeded spread is about ±10%. They are
  guesses, set in `fixtures/rules.json`'s `targeting` section; a battle that
  shows too little or too much spread is a retune there, not a code change.
- **A held crew's readout says "holding fire".** It reuses the existing
  `HoldingFire` reason rather than a new "target already handled" one, so the
  published contract is unchanged. You defaulted this; flip it if players
  need to tell the two apart.
- **Expected arrival is a launch-time estimate.** A target that drives away
  makes the round late, and it stops counting at its estimated tick. Crews may
  then fire a moment early, the over-commit direction.
- **Every held mount re-runs target choice each tick.** Measured at +2.9%
  weapons work on average and a lower peak; a per-target claim cache was
  judged not worth its complexity.
- **The menu reel was not re-recorded.** It plays its encounter live, so it
  already follows the new rules.

## Made by the agent, low risk

- A round at a squad counts at most its healthiest soldier's health, and its
  blast not at all (found in the closing review: before, one missile's full
  damage covered a wounded squad it could only thin by one).

- Arrival ties count as landing first.
- When only doomed targets qualify, the current lock is kept, else the first
  doomed one.
- The hold is checked after a missile's guidance and before a garrison's
  window check.
- Damage of a round in flight is judged on the face the target shows the
  round now; a round launched this tick, on the face it shows the muzzle.
- The ledger is kept per side and target, sorted by arrival with insertion
  order breaking ties.
- `avoids_overkill` is left out of files when false.
- The seeded factor is keyed by the mount's spec index.
- Only units on the field count as engaging.
- Spread weights are refused unless finite, non-negative, and the spread is
  below 1.

## Decided by you

- Automatic targeting only; explicit orders and the AI's orders untouched.
- Real health, not the side's knowledge.
- An opt-in flag per weapon row; on ATGMs (and everything guided from them),
  tank AP, SEAD missiles, grenades and RPGs; not tank HE shells, guns, AA,
  bombs or artillery.
- Doomed at actual remaining health, counting damage after penetration, with
  Trophy charges absorbing interceptable rounds.
- Only damage landing before the crew's own shot counts; rate of fire is not
  predicted.
- Hold rather than fire at a doomed target, with aim kept for an instant
  follow-up, and keep counting as able to engage.
- A guided missile stops counting once its guidance is released.
- One mount, one lock, one hold: a held crew never switches ammo to fire.
- The default gun keeps plinking at what it cannot hurt.
- Delete the purchase-cost ranking; spread among tied targets by weighted
  distance, counting other units, only on fresh choices, with a seeded factor
  that never reshuffles per tick.
