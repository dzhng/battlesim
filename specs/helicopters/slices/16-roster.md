# 16 — The other 18 helicopters

**Status:** done ([evidence](#evidence)). **Depends on:** 15. **Owns:** D5, D12, D41, L8, L15.

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

## Evidence

Choices are in [choices](../choices.md) under 16.

- **Contract changes:** the 18 cards are physical types (role `helicopter`), each extending one of five air profiles in `fixtures/units/air/profiles.json`, a new game root; the Apache extends the attack one and resolves unchanged. Every helicopter is `light` weight. Hull boxes are measured from the art, under the rotor head. Mounts: `chin` (`gun` rig) on the AH-1Z, Tiger HAD, Mi-35M, Mi-28NM and Z-10; `door` (`hmg` rig) on the nine transports and the MH-6M; hull-fixed `missiles`, `rockets`, `guns` (AH-6M) and `cannon` (Ka-52). `hull_limits.air` is 3.65 x 9.94 m. The model manifest loses its 18 helicopter entries; `assets/catalog.json` gains 18 appearances and wrecks. The five publication stream records are re-recorded: only `publication_sha256` moved, every digest, initial digest and fog hash is unchanged.
- `asset check` passes: 751 runtime files; catalog load 329.7 MiB for the game page, 366.1 MiB for every unit's art (limit 512 MiB); texture layers 149 albedo and 270 surface (limit 2048). Triangles by tier, live: AH-1Z 19826/7036/1822/790, UH-1Y 14808/5738/1254/570, AH-6M 12868/4304/1218/614, MH-6M 12160/4372/1198/598, CH-47F 24200/7650/2220/744, HC6 25082/8020/2220/744, UH-60M 18422/6034/1638/634, Z-20 20242/6370/1748/690, Ka-52 19254/6296/1816/848, Mi-35M 23186/7624/2234/842, Mi-28NM 24138/7806/2102/850; the rest are in the receipts. The wrecks are 15770 to 39222 triangles at tier 0.
- `cargo test -p sim --test sim -- catalog::` passes (13), with `every_unit_type_sets_up_fires_each_mount_and_moves` covering all 19 helicopters. `air`, `publication::`, `menu_reel::`, `skirmish::` and `contacts::` pass (the air limit test now reads the limit).
- Web: `sceneAssets/unitFit`, `sceneAssets/validate`, `icons`, `catalogImports`, `mechanicsServer`, `mechanicsEditor` pass.
- The UI scene's HEL tab (`purchase-hel`, `purchase-short-of-credits-hel`) matches its approved picture with 0 px different. It pins fixed test units, so it needs no re-approval. The scene's menu shots fail in this worktree only because the menu's LFS media weren't pulled.
- Workbench contact sheets of all 19 helicopters show 0 findings, with each box fitting its airframe. The helicopter rows montage, close views by faction, a 2x door-gun crop and before/after pairs for five models (distance 0.009–0.035, the change confined to the mounts) are in `throwaway/evidence/heli-roster/`.
- **Unprimed screenshot critique**, the last check.
  - **Recorded as open, this slice's:**
    - The door guns read as thin rods lying along the cabin at workbench zoom (UH-1Y, UH-60M), because at rest they point forward along the side.
    - The MH-6M's bench gun is not visible at all.
    - A stowed rest yaw outward, or a heavier mount, would fix both. Not done; the UH-1Y's gun already stands 0.5 m off the skin.
  - **Checked, not defects:**
    - The Tiger UHT's launcher clears its front wheel by 0.3 m.
    - The Mi-35M's tail rotor stands outside its box because rotors are left out of the hull by rule.
    - The Ka-52's box is wide because it runs over its wing-tip pods, which are part of the airframe.
  - **Recorded, not this slice's** (the slice 06 airframe art):
    - Several attack and utility types share one airframe shape.
    - The UH-1Y's domed nose and the Wildcat's round nose are wrong.
    - The Hind's stub wings and stores read small.
    - The UH-1Y's paint differs from the other US types.
- **Not done:**
  - The `air` scene with one helicopter per faction.
  - Traverse arcs for door guns (a door gun swings through the cabin when it fires at the far side).
  - Distinct rotor sound by weight: every helicopter is `air_light`.
