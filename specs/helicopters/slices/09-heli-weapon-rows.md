# 09 — Helicopter missile and rocket rows

**Status:** done. **Depends on:** 07, 08. **Owns:** D9, D38.

## Contract

Helicopters fire guided missiles and rocket salvos on the move. Missiles keep guiding while the helicopter moves. Rockets never target air. Ground missile teams are unchanged.

## API seam

- New rows in `fixtures/game.json`: `heli_atgm` (extends `atgm`, `stationary: false`, `guidance: "on_the_move"`) and `rocket_pod` (unguided salvo, `targets: [ground]`).
- A new `guidance: "stationary" | "on_the_move"` property on guided rows (`contract::ballistics`, beside `turn_deg_s`), required whenever `turn_deg_s` is set. `atgm` authors `"stationary"`. Guidance keep (`battle.rs`) becomes "not moved, or the row guides on the move" (D38).
- Validate both rows against the flight rules, per the tweak-mechanics admission rule. Regenerate the browser's derived catalog.

## What you can run or see

`cargo test -p sim --test guidance` and `--test weapons`.

## Verification

Tests:
- `an_on_the_move_launcher_keeps_guiding_while_it_moves`
- `a_stationary_guidance_launcher_releases_when_it_moves` (today's ATGM, unchanged)
- `a_guided_row_without_guidance_is_refused_at_load`
- `rockets_never_target_air`
- `test_attack_heli_kills_a_test_tank_on_the_move`
- infantry missile digests unchanged

## Delegated to the implementer

Rocket salvo size, scatter and range within the D1 classes, as long as the row passes flight admission.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

None expected.
