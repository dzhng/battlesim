# 12 — Drop line, ground ring and ghosts

**Status:** done (2026-10-10). **Depends on:** 03, 06. **Owns:** D18, L14.

## Contract

You can read where the helicopter is over the ground: a thin drop line from the airframe to a ring on the ground. Placement and destination ghosts show it at cruise altitude, not landed.

## API seam

- Designed first through [game-ui](../../../.agents/skills/game-ui/SKILL.md), keeping the D11 sign's family from slice 02.
- The drop line is an overlay segment (`MeshBuilder.segment`), not a painted mark.
- The ring stays ground paint (`orderOverlay.ts`).
- `unitGhosts.ts` lifts air ghosts to cruise height.

## What you can run or see

The `air` scene.

## Verification

The scene runs without console or GPU errors.

**Visual variable:** reading where the helicopter is over the ground
**Crop or mask:** 2× crop of airframe, line and ring, over flat ground and over a roof
**Out of scope (later slices):** the contact sign (13)

1. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Line width and opacity within the game-ui rules.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

Any 'too busy' feedback here changes line opacity, not the rule.

## Result

**What the player sees.** Every aircraft the side sees carries the vehicle's unit marker on the ground under it. That is the circle with its facing arrowhead, sized from the hull. A drop line joins the marker's centre to the airframe. This applies to own and identified enemy aircraft, and the marker shows whatever the aircraft is doing.
- The marker is ground paint (`orderOverlay.ts`, `aircraftMarker`).
- The line is an overlay segment (`MeshBuilder.segment`), depth-tested. A roof under the aircraft cuts the line where it meets the roof, and the roof hides the marker.
- With Space held, an aircraft's ghost flies at cruise height over the end of its orders (`ghostAloft` in `unitGhosts.ts`), and a drop line joins the ghost to that end's marker.

**Selection.**
- Own aircraft, unselected: the orders' colour at `current_alpha`.
- Own aircraft, moving under a shown order: the full order colour.
- Selected: the marker and line take the selection colour.
- Identified enemy aircraft: `hud.enemy`.
- The line keeps its own opacity (`drop_line_alpha`, 1) whatever the marker's. Its width is `drop_line_px` (4) by the one stroke rule. Both are data in `presentation.overlay.orders`.

**Lab.** `/lab/air-hover` has three variants, which are three saved encounters of the `air` map:
- `hover`: the slice-06 frame.
- `roof`: an own helicopter over a house, and an identified enemy across the road.
- `fog`: an enemy over ground that the house hides from blue's jeep.

The `air` map gained one house (`paris-home-12x9-2f`) and so names the `paris` region.

**Verification.**
- The scene runs with no console or GPU errors.
- `air-hover` checks:
  - the drop line is drawn from the ground to the airframe, and not beside it;
  - the selected marker changes colour at its rim;
  - over the roof, the line shows above the roof and is hidden below it;
  - the enemy's line is red;
  - the line is still drawn at map zoom and over unseen ground;
  - the ghost is drawn at cruise height, not on the ground, and its drop line is drawn.
- Narrow tests: `web/tests/aircraftMarks.test.ts`, which covers the marker and line geometry, colours, stroke-rule width and the ghost line. The ghost-height case is in `web/tests/unitGhosts.test.ts`. `orderOverlay`, `facingPreview` and `mapCatalogue` stay green, and so do the sim's `maps::` and `menu_reel` tests. No digest changes: only presentation data and a lab-only map changed.

**Evidence.**
- `throwaway/evidence/air-hover/`: `frame-*` and 2× `crop-drop-*` crops for flat ground (`t0`), selected, roof, enemy, fog, map-roof, map-fog and ghost.
- Before and after: `throwaway/evidence/air-hover-before/` and `throwaway/evidence/air-hover-compare/`.

**Before against after** (compare-screenshots, hover t0). Before, the helicopter reads as parked by the road; its shadow was the only height cue. After, the line and marker put it over the field south of the road.
- The field crops changed too. Adding the house changed the map's region and hash, and so its field dressing. That accounts for most of the pixel distance (mae 18, pixelmatch 0.41).
- Verdict: the candidate is better.

**Unprimed critique, last run.**

Reads well:
- the rings lie flat in perspective;
- the lines are vertical, and rotors pass in front of them;
- the house hides part of the ring;
- the red enemy marker is clean on grass;
- every ground spot is findable at map zoom.

Findings, recorded:
1. **The marker reads as a pin or lollipop (high).** The line ends at the ring's centre with no foot. This is carried to the checkpoint below.
2. **Ring and line colours differ (high).** The ring is paint: lit, grass showing through, and taking `fog_keep` of the fog, so it turns pale pink and speckled over unseen ground. The line is unlit overlay. This is inherent to the contract's split (the ring is paint, the line is overlay). A fix belongs to the paint/overlay colour match, not to this slice.
3. **The ring's surface is grainy (medium).** This is the existing paint look on every marker.
4. **Over a roof, the ring circles the house, as if the house were selected (medium-high).** This is the contract's rule: the ring stays on the ground. Open question for the user.
5. **The shadow competes with the ring as a ground cue (medium).** This is physical, and the line is what links airframe to ring.
6. **Weight conflict.** This run calls the line too heavy at map zoom (medium). The first run, at 3 px and 0.55 alpha, called it nearly invisible there. The weight is now 4 px. That is the delegated knob, and "too busy" feedback lowers `drop_line_alpha`.
7. **The ghost's line is as solid as a live unit's (medium).** The ghost model also shows its internal mesh edges, which is the existing ghost look.
8. **The arrowhead looks like a speech-bubble tail (low-medium).** It is the existing unit marker, the user's earlier pick.

**Human checkpoint (non-blocking).** Per the brief, the shots were left in evidence and not opened. Decided on the evidence: the marker and line ship as described. Two questions remain open for the user:
- the pin reading (finding 1);
- the ring around a roofed house (finding 4).

**Not done.** The right-drag move preview and the purchase placement ghost for aircraft are lifted to cruise height, but carry no drop line.
