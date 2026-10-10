# 10 — Drawing the fall

**Status:** done. **Depends on:** 04, 06. **Owns:** L2 (renderer).

## Result

- **Contract:** the publication gained the `crashes` group (`id, own, kind, x, y, z, yaw, pitch, roll`) and the header word `crashCount`; the web's `ObservationView.crashes` decodes it. Choices record the id rule, the derived attitude and the drawing path.
- **Tests:** `air_crash::both_sides_that_saw_it_go_down_see_it_fall_to_its_wreck`, `publication::a_falling_airframe_publishes_its_place_and_attitude`, the codec vectors (web `observation.test.ts` decodes a falling airframe), `poseFeed.test.ts` (a helicopter shot down falls as itself, rotors turning, tipped), `cookOffs.test.ts` (it never brews up).
- **Parity:** six publication records re-recorded; only their publication hashes moved, no digest.
- **Scene:** `/lab/air-crash` (`web/scenes/air-crash.mjs`): all checks pass, no GPU warnings; `air-hover` still passes after the shared route.
- **Evidence:** `throwaway/evidence/air-crash/` — `fall-sheet.png` (hit to wreck at rest), frames, `crop-mid-fall-2x.png`, `crop-wreck-2x.png`; `before/` holds the same scene on 9c8867f3; `compare/` the metrics.
- **compare-screenshots:** before, the airframe vanishes at the hit and its wreck appears 2 s later with a burst; after, it falls in a spinning, nose-down arc to the same impact. Distances 0.003–0.005 on the falling frames (only the airframe differs), 0 on the impact and wreck frames (identical). Verdict: the candidate is less wrong.
- **screenshot-critique (unprimed), actionable findings:**
  - The airframe stops dead on impact, and pops from a pitched, painted airframe to a level grey wreck with no rotor: the wreck art (slice 15) and its thrown blades (the wreck's `debris` state is drawn only by a cook-off today) are the follow-up; the impact burst hides most of the swap. A slide on landing would be a slice 04 rule change.
  - It falls without smoke or fire: slice 11's trail covers a falling airframe.
  - The autocannon's mid-air hit puff hangs for 3 s and the crash smoke stays fire-lit: existing effect styles, not this slice's.
  - Diagonal stripes over the frame are the fog hatch (blue has no unit left to see with); fog style, not a defect.

## Contract

The browser draws a downed helicopter's spinning, falling arc from the published `crashes` group, then hands over to the wreck. It no longer vanishes in mid-air, snaps to the ground or teleports.

## API seam

- The feed (`packages/battle-renderer/src/scene.ts`) carries falling airframes.
- `apps/battle-lab/src/cookOffs.ts` skips airborne deaths; its 6 m reach and 2 s memory are shorter than a fall.
- The impact fireball goes at the impact point.

## What you can run or see

The `air` scene, extended with a shoot-down.

## Verification

The scene runs without console or GPU errors. `compare-screenshots` against the cook-off's before shot of the same death.

**Visual variable:** the fall and impact
**Crop or mask:** a contact sheet of the arc, from hit to wreck at rest
**Out of scope (later slices):** smoke trail (11), wreck art (15)

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Fireball timing and size at impact.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the fall reads as too long or too short, the arc's drag in slice 04 changes.
