# 06 — T-14 and T-15 Armata

**Unlocks:** both Armata cards are buyable and fight. Eastern gains an advanced MBT and a
heavy IFV.

## Seam

Run the [admission recipe](../README.md#admission-recipe) for both cards.

- `armata.py` builds two folders (`t14/`, `t15/`). Split it into `t14.py` and `t15.py`
  that import the shared `armata.py`, the way `t90.py` imports `t72.py`, so one script
  maps to one family folder.
- T-14: `extends roster_profile_advanced_mbt`; cannon and HMG mounts.
- T-15: `extends roster_profile_heavy_ifv`; mounts `autocannon` and `launcher` firing
  `kornet` (bound to the HMG rig, as on the Bradley). Its description keeps saying
  passenger carriage is not implemented.
- T-15 turret pivot: apply slice 01's decision.

## Verification

- Catalog smoke test and gate.
- `asset validate --type` for each card; then `bake`, `check`, `icons`.
- Shots in the `unit-roster` scene: T-14 beside T-90M and KF51; T-15 beside BMP-3 and Puma.
  Fix only what slice 01 ranked.
- [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against the peers,
  then an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  as the last check.

## Delegated

Armour, mobility and cost from the nearest live peers (T-90M, KF51; BMP-3, Puma); the
shared `armata.py` helpers' internal shape.

## Stays green

Catalog tests; existing digests.
