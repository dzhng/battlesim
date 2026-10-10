# 08 — M10 Booker, CV90120, Centauro II

**Unlocks:** the US and European light tanks and Europe's wheeled tank destroyer are
buyable and fight. Every faction now has a light tank.

## Art first (from [slice 01 findings](01-model-triage.md#findings-2026-10-10))

Open with one art-fix commit before admission: sheet → fix → one unprimed critique.
M10: raise and blunt the nose, shorten and heighten the turret, keep faction tan. CV90120: first replace the side/rear references (they show CV90 IFVs), then give the turret a wide wedge front so it reads apart from the Type 15. Centauro II: no art fix; settle its 0.22 m height gap; edge wear 0.6. Only these ranked fixes; dropped findings stay dropped.

## Seam

Run the [admission recipe](../README.md#admission-recipe) for each card.

- Move the sources out of `assets/source/roster/disabled/` into `m10/`, `cv90120/` and
  `centauro/` (with their source receipts), so `run(family)` finds them.
- M10 and CV90120: `extends roster_profile_light_tank`, `light_tank_*` rows plus an HMG.
- Centauro II: `extends roster_profile_wheeled_tank_destroyer`, wheeled mobility; its gun is
  the existing row closest to a 120 mm tank gun (the fewest-slices draft leaned to
  `ground_tank_*`). Record the choice.
- M10 silhouette and Centauro edge wear: apply slice 01's decisions. Judge the M10 change
  against the SEP v3 at `battle-mid`.

## Verification

- Catalog smoke test and gate; `asset validate --type`, `bake`, `check`, `icons`.
- Shots in the `unit-roster` scene: the three light tanks side by side with an MBT; Centauro
  beside Boxer RCT30 and VBCI. Fix only what slice 01 ranked.
- [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against the
  peers (and, for the M10, before against after), then an unprimed
  [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) as the last check.

## Delegated

Numbers from the nearest peers; the Centauro's gun row; whether the three light tanks share
one cost or differ by faction.

## Stays green

Catalog tests; existing digests; the `disabled/` folder keeps only cards still disabled.
