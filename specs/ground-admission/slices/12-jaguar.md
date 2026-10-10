# 12 — EBRC Jaguar

**Unlocks:** Europe's armed wheeled recon vehicle is buyable, and it fires the autocannon
and top-attack Akeron.

Depends on 05 (recipe) and 11 (the `akeron_mp` row).

## Seam

[Admission recipe](../README.md#admission-recipe):

- `extends roster_profile_wheeled_recon`; mounts `autocannon` and `launcher` firing
  `akeron_mp`, the launcher bound to the HMG rig (`jaguar.py` already rigs it that way).
- `jaguar.py` switches to `run`; the folder `jaguar/` already matches.
- Edge wear: apply slice 01's decision.
- The real launcher arm raises to fire. It stays stowed; animating it is out of scope.

## Verification

- Catalog smoke test and gate; `asset validate --type`, `bake`, `check`, `icons`.
- Shot in the `unit-roster` scene beside Fennek and VBCI.
  [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against them,
  then an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  as the last check.
- A quick sample: a Jaguar fires a diving Akeron at a tank from recon range.

## Delegated

Numbers from the Fennek and VBCI; Akeron ammo carried.

## Stays green

Catalog tests; existing digests.
