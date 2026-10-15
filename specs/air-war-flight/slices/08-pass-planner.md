# 08 — Attack passes

**Delivers:** D13, D14, D23. Also: a ground-missile pass flies the strafe profile.

**Unlocks:** attacks are runs, not hovers.

## Seam
`crates/sim/src/air/pass.rs`:
```rust
choose_weapon(&[PassWeapon]) -> Option<PassWeapon>   // limited ground ammunition before the unlimited gun
begin(&JetState, aim, PassWeapon, &JetAir, &mut Rng) -> PassPlan { phase: LineUp | TurnIn | Dive | Release | PullOut, agl }
advance(&mut PassPlan, &JetState, &FixedWingLimits, aim, &Release) -> (Intent, Option<ReleaseCue>)
```
- **Line-up from where the jet is** (closeout fix, [choices](../choices.md)): fly straight on until a full-rate turn in on the aim leaves a full run-in, turn in, and dive only nose-on within the run-in. Every repeat pass dives in from the approach side.
- **Ceiling per pass:** bombs use `bomb_ceiling_m`; guns and ground missiles use `strafe_ceiling_m`.
- **Height:** seeded between `floor_agl_m` and the ceiling. A jet already below the ceiling doesn't climb.
- **Edge:** line-ups go through the edge guard.
- **Delete** the hover attack in `update_pursuit` for jets (`battle.rs` ~2969–3003).

## You see
`/labs/air-ground` on `air-jet`: bomb, ground-missile and strafe passes, with GIFs and a height trace showing the dives. The targets are the mock's bomb-pass and missile-pass scenarios.

## Tests (red first; `crates/sim/tests/jet_runs.rs`)
- `a_bombing_pass_lines_up_dives_under_its_ceiling_releases_and_climbs_back`
- `pass_height_is_seeded_between_floor_and_ceiling`
- `a_jet_below_the_ceiling_doesnt_climb_to_start`
- `a_limited_ground_weapon_is_chosen_before_the_gun`
- `every_loadout_flies_a_pass`: gun-only, missile-only, bomb-only and mixed rows built in the test

## Visual gate
- **Variable:** a pass's height profile and path. **Crop:** the trace sheet plus a side view of a bomb pass.
- Compare against the mock with `compare-screenshots`, run `screenshot-critique` last, then `preview-shots`.

## Firewall and delegation
- **Firewall:** order policy is slice 11; the gun rule is slice 09.
- **Delegated:** line-up length and pull-out distance.
