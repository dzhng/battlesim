# 15 — The Apache, on real art

**Status:** planned. **Depends on:** 07, 09, 10. **Owns:** D5 (attack row), D41, L10 (rigs), L8 (manifest), L15.

## Contract

The real AH-64E is a playable unit. Its chin gun aims and fires with its muzzle where the round leaves. It carries missiles and rockets, and dies into its wreck art.

## API seam

- `apache.py`: `turret`, `gun` and `muzzle` rig nodes for the chin gun, plus hardpoints.
- Per-rig pitch limits replace the global `PITCH_LIMITS`, so the chin gun reaches at least -25°.
- The roster row replaces the `planned` card: D5 attack armour, D26 speed, `heli_atgm`, `rocket_pod`.
- Remove its entry from the disabled model manifest.
- Add an `assets/catalog.json` appearance, with its `_wreck`.

## What you can run or see

The Apache in the `air` scene.

## Verification

Gates: `asset check` budgets (L15), the catalog tests in `crates/sim/tests/catalog.rs`, and the scene without errors.

**Visual variable:** the Apache's mounts and muzzle alignment
**Crop or mask:** 2× crop of the chin gun firing down at -20°
**Out of scope (later slices):** the other 18 (16)

1. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Mount positions on the airframe, matched to the model.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

None expected.
