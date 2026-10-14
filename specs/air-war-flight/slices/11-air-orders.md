# 11 — Orders and reactions

**Delivers:** D15, D15″, D18.

## Seam
`crates/sim/src/air/orders.rs::task(unit, &Knowledge, pass: Option<&PassPlan>) -> AirTask { Hold, Orbit, Attack { target, explicit }, Dogfight, Withdraw }`. It is the only place where orders become flight.
- **Explicit attack:** repeats passes until the target dies.
- **Attack-move:** picks up targets along the way, then holds by circling.
- **Holding:** a holding jet starts no runs. Fired on, it attacks the attacker, the unit if seen and otherwise its contact area. Hold fire stops this.
- **Mid-run (D18):** under an explicit attack the jet keeps passing. Otherwise it finishes the pass, then turns on the attacker. A Move breaks off at once.

## You see
`/labs/air-ground?variant=orders` and `/labs/air-dogfight?variant=reactions`.

## Tests (red first)
- `an_explicit_attack_repeats_passes_until_the_target_dies`
- `attack_move_picks_up_targets_then_holds_circling`
- `a_holding_jet_starts_no_runs`
- `a_fired_on_holding_jet_attacks_its_visible_attacker`
- `a_fired_on_holding_jet_attacks_the_contact_area_when_unseen`
- `hold_fire_stops_the_answer`
- `an_explicit_attack_keeps_passing_under_fire`
- `attack_move_finishes_its_pass_then_turns_on_the_attacker`
- `a_move_breaks_off_mid_pass_at_once`

## Reslice
If this runs long, split it into 11a (D15) and 11b (D15″, D18).
