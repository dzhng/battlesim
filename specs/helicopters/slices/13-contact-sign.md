# 13 — The airborne contact sign, built

**Status:** done (2026-10-10). **Depends on:** 02, 05, 12. **Owns:** D11 (built), D33.

## Contract

A lost enemy helicopter leaves the sign picked in slice 02, floating at its last height.

## API seam

`contactGlyph.ts` draws air contacts (layer `low_air`, more than the low-hover height above ground) as the chosen sign at their z. Ground contacts are unchanged.

## What you can run or see

The `air` scene: the helicopter goes behind the tower and out of sight.

## Verification

The scene runs without console or GPU errors.

**Visual variable:** the airborne contact sign
**Crop or mask:** 3× crop of the sign over fog and over a roof
**Out of scope (later slices):** none

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Nothing beyond the slice 02 pick.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

A different pick from the user restarts this slice from slice 02's sheet.

## Result

**Contract change: contacts publish `aloft`.** The sim decides whether a contact floats (D33): it is in the air band and its z is more than the low hover over the ground under it (`air::low_hover`, the one owner). `ApproximateContact.aloft` is packed after `layer` in the contact group (`id, source, x, y, z, layer, aloft, radius, …`), and the web's `ContactView.aloft` reads it.
- The glyph, the panel's anchor (`readouts.tsx`) and picking (`contactPick.ts`) all read `aloft` rather than `layer`. A helicopter heard from the low hover over its supply truck therefore lies on the ground in all three.
- No battle digest moved. Only the published words did: the codec vectors and the five publication stream records were re-recorded, and their digest sequences are unchanged.

**What the player sees.** A lost or heard helicopter well over the ground leaves a red sign at the height where it was last seen.
- It is the ground glyph's own recipe: fill, 45° hatch, pale rim and glow. It is stood up as a disc facing the camera, a perfect circle at every angle, with a thin dark keyline outside the rim.
- A thin pale stem drops from the ground under the sign to where it meets the rim on screen. There is no stem seen from straight above, or when the sign already covers its ground point.
- Ground contacts are unchanged. The ground glyph's mesh is bit-identical to before.
- The type label is the existing last-seen panel, which already hung at an air contact's z. There is no new label.

**API seam.**
- `buildContactGlyphs(contacts, z, style, facing)` draws both stances through one recipe, using a ground pen or a camera-facing pen. `ContactShape` gains `aloft`.
- `ContactFacing` holds the screen's right and up (`screenAxes` in `camera3d.ts`, beside `viewMatrix`, which owns those axes) and the stroke rule at the view's zoom.
- Views get it from `useContactFacing(initialCamera, metresPerPx)` (`apps/battle-lab/src/contactFacing.ts`). That follows the camera's yaw and pitch in 2° steps, so a turning camera rebuilds the overlay only when a sign would visibly turn.
- The sign is lit with the ground's normal, so its red matches the ground glyph's.
- Data lives in `presentation.contacts.air`: fill 0.55, hatch 1, glow 0.6; hatch 2.5 px, rim 3 px, keyline 1.5 px at 0.55; radius 20–44 px; stem 2 px at 0.8.

**Sizing.** The sign spans the contact's uncertainty radius (slice 02's finding), held between 20 and 44 px of radius. Both bounds go through the marks' one stroke rule, so the sign is smaller from the map's zoom (Ø≈50 px) than close in (Ø≈90 px).
- Above the bounds, the hatch spacing scales with the sign. The hatch lines, rim and keyline are strokes.
- The first build drew the full area: 21 m of radius for the test helicopter, about 480 px across at a 90 m camera distance. The unprimed critique's top findings were that it hid the ground it marked and covered its own stem, so it read as a decal. The upper bound is the fix.

**Drop line.** The sign keeps the slice-12 family:
- an overlay segment;
- depth-tested, so a roof cuts it;
- width by the stroke rule.

It is thinner (2 px against 4) and pale rather than red, because the position is uncertain and form carries that. It carries no ground ring, so neither slice-12 question applies to it: the sign is not a pin with a ring, and nothing circles a house. Both questions stay open for the identified aircraft's marker.

**Scene.** `/lab/air-hover?variant=lost` is a new encounter of the `air` map. Blue's jeep sits just south of the house. A red helicopter it sees across the field is scripted (tick 60) to fly behind the house, where the jeep loses it.
- `air-hover.mjs` asserts from the observation that blue holds an aloft `low_air` contact at cruise height over the ground, and that the helicopter is no longer identified.
- It then checks four framings: fog close and from the map, and roof close and from the map. In each the sign is a red circle on screen within its size bounds. In close views a pale stem stands under it. The ground under it is unseen, and in the roof framings the roof lies behind it.
- No GPU warnings. The earlier hover, roof and fog checks still pass.

**Narrow tests.**
- Sim (`air.rs`): a lost helicopter's contact is aloft; a heard one at cruise is aloft; a report from the low hover and a ground report are not. Falsified by dropping the height test.
- Publication round trip, and the paired records.
- Web:
  - `contactGlyph.test.ts`: plane, normal, stem, size bounds and recipe. Falsified by drawing aloft contacts on the ground.
  - `camera3d.test.ts`: screen axes project right and up.
  - `unitControl.test.tsx` (low-hover pick), and the observation, group-delivery, fog-delivery, readout and consumer tests.
- `cargo test` for `maps::`, `menu_reel`, `contacts::`, `sensing::`, `air_supply::`; clippy and tsc are clean.

**Evidence** is in `throwaway/evidence/air-hover/`: `frame-lost-{fog,roof,map-fog,map-roof}-1280x800.png` and `crop-sign-lost-*-3x.png` (the sign and three times its size round it, 3×). Against the pick: `throwaway/evidence/air-hover-compare-sign/D-top-built-bottom.png`, with row D's cells on top and ours below.

**Compare against row D** (compare-screenshots, landmarks):

| Landmark | Row D | Ours | Verdict |
|---|---|---|---|
| Form | Circle with a 45° hatch | Same | match |
| Keyline | Dark keyline | Same | match |
| Stem | Thin pale stem | Same | match |
| Rim | Thick, pure white | Thinner (3 px) and grey-blue | **deviation.** The rim colour is lit like the ground glyph's, keeping one look for one concept. The mockup composited it unlit. |
| Glow | Soft halo | A few pixels | weaker |
| Fill | Reads as solid | Translucent | weaker |
| Size | Ø26/44 | Ø50/90 | larger, by decision (sizing above) |

Verdict: the candidate is the less wrong of the two builds and matches D's design. The rim, glow and fill differences are recorded below.

**Unprimed critique, three rounds.**
1. On the area-sized build: the sign was far too large close in, had no stem, and was see-through. Fixed by the size bounds and the stem, and by raising the fill and hatch.
2. Still see-through, with a weak rim and glow, and the same size from the map as close in. Fixed by sizing through the stroke rule, and by rebalancing the fill to 0.55 and the hatch to 1 at 2.5 px so the stripes read.
3. Last round. It reads well: a true camera-facing circle in every frame, the red is the most noticeable thing on screen at both zooms, the close-view stem reads as hanging, the edges are clean, and it is clearly row D. Findings recorded, not fixed:
   - Still partly see-through: house details show faintly through it (high). An overlay mark is translucent by design, and opaque fill would erase the hatch.
   - The rim is grey-blue and thinner than D's white (high). This is the shared rim colour under the overlays' fixed light.
   - The glow is a thin fringe (medium-high). The spill is 8% of the radius, the ground recipe's.
   - From the map's zoom the stem is short (medium). It is physical: 20 m over a 50 px sign.
   - The stem has no foot mark (medium-low). Row D had none either.
   - The house's order reads as ambiguous when the stem passes in front of it (medium). This is correct depth: the helicopter was on the camera's side of the house.

**Not done.**
- An aloft contact still picks by a sphere of its full radius, which is larger than the sign drawn close in.
- The attack cursor is still offered over an air contact; slice 05 left that to this slice. The order itself is refused (`AirContact`).
- The full `check` and `verify` were not run; they wait for slice 18. The native `menu_reel` test passes.
