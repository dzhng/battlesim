# 15 — The Apache, on real art

**Status:** done ([evidence](#evidence)). **Depends on:** 07, 09, 10. **Owns:** D5 (attack row), D41, L10 (rigs), L8 (manifest), L15.

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

## Evidence

Built without slice 10 (the fall is drawn elsewhere); the wreck art is in, its fall is not. Choices are in [choices](../choices.md) under 15.

- **Contract changes:** the AH-64E is a physical type (`shared.json`, role `helicopter`, mounts `chin`/`rockets`/`missiles`); `hull_limits.air.half_width_m` is 2.7; a mount fixed in the hull is drawn by the hull (`MountRole` `"hull"`); per-rig pitch limits and stroke are gun-node custom properties (`pitch_min_deg`, `pitch_max_deg`, `recoil_max_m`), and `DEFAULT_PITCH_LIMITS` replaces the global `PITCH_LIMITS`; the five publication stream records are re-recorded (digests and fog unchanged).
- `asset check` passes: 682 runtime files; catalog load 282.0 MiB for the game page, 318.7 MiB for every unit's art (limit 512 MiB); texture layers 144 albedo and 260 surface (limit 2048). The Apache is 21022 / 7674 / 1988 / 854 triangles by tier, its wreck 27878 / 8224 / 2010 / 830.
- `cargo test -p sim --test sim -- catalog::` passes (13), with the derived catalog blessed. `publication::`, `skirmish::`, `menu_reel::`, `contacts::` and `air*` pass.
- Web tests: a stated pitch limit draws the gun to it and the bake's bounds hold it; a short stroke caps recoil (`sceneAssets/articulation`); a hull-fixed mount needs no rig while a turning one still does (`sceneAssets/validate`); the drawn gun stops at its model's limit (`vehicleRig`). Each was seen red first.
- `/lab/air-hover?variant=apache`, in the `air-hover` scene, with no page or GPU errors: the Apache at cruise height; its chin gun firing down at -20.3° at a tank 54 m off; the drawn muzzle on the shot's bore within 0.1 m of where the round leaves; and the flash at the barrel's tip. The shots are in `throwaway/evidence/air-hover/` (`frame-apache-firing`, `frame-apache-wide`, `crop-apache-chin-gun-2x`, `crop-apache-airframe-2x`).
- **Unprimed screenshot critiques**, run twice; both are the last check.
  - **First critique.**
    - The flash floated ahead of the barrel, because the chain gun recoiled 0.42 m. Fixed: the gun's stroke is now 0.06 m.
    - The chin gun read as a bare rod. Fixed: a bigger turret drum, yoke and receiver.
  - **Second critique.**
    - The gun sat too far aft, over the main wheels. Fixed: it moved 0.45 m forward.
    - The flash still sits a little ahead of the tip. This is the shared `autocannon` flash style: its fireball is drawn 0.16–0.64 m out along the shot, and the scene measures the drawn muzzle itself on the bore.
  - **Recorded, not this slice's:**
    - **The airframe art** (slice 06 and the disabled card): the tail wheel strut is long, the pylon has a gap under the wing, the paint is dark against the field, the blades show no blur, and the proportions read Mi-28-like.
    - **The flash style** reads weak at map zoom.
    - **The drop line** starts at the airframe's foot under the stores (slice 12).
    - **The out-of-bounds and fog hatching** dominates the frame, the wreck under fog included (fog style is not a bug).
- **Not done:**
  - Slice 10's fall.
  - Any check that the art puts a pod at a hull-fixed muzzle.
  - A rocket or missile flash crop.
  - The skirmish AI (slice 17).
