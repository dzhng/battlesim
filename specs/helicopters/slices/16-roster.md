# 16 — The other 18 helicopters

**Status:** planned. **Depends on:** 15. **Owns:** D5, D12, D41, L8, L15.

## Contract

All 19 helicopters are live. The transport types are door-gun HMG gunships until [transport](../../transport/README.md) lands.

## API seam

- D5 rows by profile.
- Exporter mount nodes for the eight models that have no modelled weapon: Mi-8, Z-20, Merlin, Wildcat, both Chinooks, NH90 and MH-6M.
- Rocket pods for the Tiger UHT.
- Door guns as turrets (D28).
- Manifest entries removed and appearances added.

## What you can run or see

The unit sheet (`workbench`) and the `air` scene with one helicopter per faction.

## Verification

Gates: `asset check`, the catalog tests, and the HEL tab re-approved in `web/scenes/ui.mjs`.

**Visual variable:** roster silhouettes and mounts
**Crop or mask:** the workbench sheet's helicopter rows
**Out of scope (later slices):** none

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Exact mount placement per model.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

None expected.
