# 11 Named fixes: kick, impact size, RPG level

**Contract:** three staging bugs the audit found are fixed, each as its own
commit with its own named delta.

1. **Tank guns kick.** No unit fires `tank_ap` directly, so derived tank
   rounds never kicked (`kick_of`'s exact-id match). With looks, every
   weapon on the tank looks gets the tank's hull kick, and the soldier kick
   covers ATGM, RPG, Igla and FN-6.
2. **Tank-round ground impacts at their intended size**, 5x size and 5x
   lifetime (`impacts.*.round_scale`). Today they show at 3x and 1x.
3. **RPGs back to their own loudness.** They have played the grenade's rows
   since the 2026-10-16 rename (`b4410901c`), about 11 dB quieter.

Each fix edits the values of [08](08-looks.md)'s transient bug looks
(`tank_sabot_no_kick` and the like) to match their siblings. No weapon
changes which look it names, so no hash moves. The looks that are now
identical are folded by [13](13-fold-looks.md).

**Run / see / hear:**
- a cropped clip of a tank firing, before and after (hull rock);
- impact plates from the look sheet;
- `firefight.wav` before and after, with the RPG's level in the ledger.

**Verify:**
- Before each re-bless, list the expected deltas: which rows, which outputs.
- Re-bless only those: `UPDATE_SOUND_VECTORS`, `BLESS_PARITY`, native plates.
- Battle digests unchanged.
- For the visual fixes, run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  before against after, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint: open the clips and plates with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md). It is
  non-blocking: about 5 minutes, then decide on the evidence, record it here,
  close the windows and go on.

**Moves:** presentation outputs only. `rules_hash` doesn't move, because no
weapon changes which look it names.

**Depends on:** [10](10-battle-rules.md).

**Delegated:** clip framing.

**Would change this slice:** the user finding a tank's rock or an impact too
big. Retune the look's values. That's a data edit and moves no hash.

## Named delta 1 — recoil (declared before changing values)

The seven derived tank-gun rows (`ground_tank_ap`, `ground_tank_he`,
`advanced_tank_ap`, `advanced_tank_he`, `light_tank_ap`, `light_tank_he`,
`ifv_he`) gain the base tank look's hull kick. The rocket pod keeps its
explicit zero hull kick; a helicopter crash is not a firing gun.

The ATGM-derived looks (`tow`, `bastion`, `kornet`, `spike`, `javelin`,
`akeron_mp`, `heli_atgm`, `air_ground_missile`) get ATGM soldier recoil;
`igla_s` and `fn_6` get MANPADS recoil; `rpg_at_light`, `rpg_at_heavy`,
`rpg_general` get grenade-launcher recoil. These fix default-strength
soldier recoil, rather than an absence of all soldier motion.

Allowed outputs: hull/soldier poses and effects/sound positions carried by
those poses, only for those weapons (including `bmp-3`, whose gun fires
`ifv_he`). The existing all-weapon probe mounts missiles/RPGs on hulls,
so their soldier-only multiplier changes need not move its output.
Sound levels, impact styles, recordings, rules and all battle digests stay
unchanged. Re-bless only affected presentation vectors, then run unflagged.

### Receipt 1

The battle-driven regression failed on the stationary firing hull, then passed
with the data edit. Relative to `after-08-merged/`, the probe changes 459 effect
rows and 3,348 pose rows: each of the seven derived gun cases has 17 effect
and 227 pose rows changed; `bmp-3` has 340 and 1,759. All sound commands,
events, hull publications/layouts and battle digests remain byte-identical.
The report is `after-11-1/delta-report.txt` in the shared proof directory.

Only the corresponding kick inputs change in the effect-vector fixture;
recorded outputs do not change. `UPDATE_SOUND_VECTORS` deliberately exits
with its rerun reminder after writing, then the sound, effects, poses and
look tests all pass unflagged. `BLESS_PARITY` leaves effect outputs unchanged.
Native plates were captured under the shared GPU lock for integration review.

## Named delta 2 — ground impacts (declared before changing values)

For `ground_tank_ap`, `ground_tank_he`, `advanced_tank_ap`,
`advanced_tank_he`, `light_tank_ap`, `light_tank_he` and `ifv_he`, ground
contacts use the base tank round's surface multiplier: 5× material size
and 5× material lifetime. Water and other surfaces do not change. Rocket
pods and helicopter crashes keep their distinct looks for the later content
pass.

Allowed additional probe deltas are effect instances and impact cast lights
for those seven gun cases and the mixed `bmp-3` case. Poses, sound commands,
authoritative events, hull publications/layouts and battle digests must stay
byte-identical to fix 1. The effect-vector migration changes only those
looks' ground-surface inputs before `BLESS_PARITY`; sound is expected to
remain unchanged under `UPDATE_SOUND_VECTORS` and its unflagged rerun.

### Receipt 2

The controlled ground-contact regression first failed on the rendered puff,
then passed for early size, survival beyond the unscaled material lifetime,
and eventual expiration. Only the seven ground-surface inputs change in the
stored effect fixture; its recorded base-weapon output stays unchanged.
Sound/effect re-blessing and all focused sound, effects, pose, look and effect
behaviour checks pass unflagged.

The standard probe has no tank ground-contact rows: its tank shots hit hulls.
Consequently every stream is byte-identical to fix 1, including all battle
digests. This is a coverage limit, not evidence of a visible change;
`after-11-2/delta-report.txt` names it explicitly. A separate controlled ground
capture supplies visual review evidence. Native capture is queued under the
shared GPU lock; review remains with the integrator.
