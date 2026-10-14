# 09 — Nose-on gun and self-firing weapons

**Delivers:** D30, D16.

## Seam
- **Gun rule:** `weapons::nose_on(unit, target, range) -> bool` for a jet's fixed guns. The gun fires whenever its nose is on a legal target in range.
- **A jet's weapons never turn its hull:** drop the `reach.face` path for jets.
- **Delete:** the strafe ban (`weapons.rs` ~962–968) and bomb-first `ground_attack_kind` (~1034–1050). Weapon choice for a pass now lives in `air/pass.rs`.
- **Self-firing weapons:** air-to-air and SEAD missiles fire whenever an enemy is in range and within the seeker's field of view, under any order.

## Tests (red first)
- `the_gun_fires_only_nose_on_a_legal_target_in_range`
- `a_bombing_pass_never_fires_the_gun`
- `a_missile_pass_fires_the_gun_when_nose_on`
- `an_air_missile_fires_at_an_enemy_in_its_cone_during_a_move`
- `a_strafe_is_allowed_while_bombs_remain`

## Firewall and what stays green
- **Firewall:** helicopter guns are unchanged.
- **Stays green:** `radar.rs`, rewriting its bomb-then-strafe test against the new rule.
