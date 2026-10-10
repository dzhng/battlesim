# 07 — BRM-3K and Type 15

**Unlocks:** Eastern's tracked recon vehicle and its light tank are buyable and fight. The
`light_tank` profile is proven on its first card.

## Art first (from [slice 01 findings](01-model-triage.md#findings-2026-10-10))

Open with one art-fix commit before admission: sheet → fix → one unprimed critique.
BRM-3K: give the recon turret real mass (it is a 0.5 m slab) and tame the tall rear corner boxes; settle the 0.23 m length gap. Type 15: separate the turret from the stepped hull so it no longer matches the CV90120 at battle-near; settle the 0.12 m length gap. Only these ranked fixes; dropped findings stay dropped.

## Seam

Run the [admission recipe](../README.md#admission-recipe).

- BRM-3K: `extends roster_profile_tracked_recon`, autocannon. Its frame equals the BMP-3's
  (7.14 × 3.2 × 2.4 m).
- Type 15: `extends roster_profile_light_tank` (from slice 05), firing `light_tank_ap` and
  `light_tank_he`, plus an HMG.
- Folders `brm/` and `type15/` already match their scripts.

## Verification

- Catalog smoke test and gate; `asset validate --type`, `bake`, `check`, `icons`.
- Shots in the `unit-roster` scene: BRM-3K beside BMP-3 and Ajax; Type 15 beside T-90M (size
  read: clearly a lighter tank). Fix only what slice 01 ranked.
- [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against the peers,
  then an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
  as the last check.
- A quick battle sample of a Type 15 against an IFV and against an MBT: it should beat the
  first and lose to the second head-on. This is a sanity read; the balance report runs at 13.

## Delegated

Numbers from the nearest peers; the light tank's armour sits between IFV and MBT.

## Stays green

Catalog tests; existing digests.
