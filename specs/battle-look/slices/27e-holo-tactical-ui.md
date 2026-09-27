# 27e — Holo-tactical in-world UI

**Status:** planned. **Depends on:** 27b (overlay depth, feeds). It runs beside 27d. **Lane:** renderer and presentation. **Given:** the user's request of 2026-09-27, with references `assets/reference/user/holo-glow.png` (glow and material) and `assets/reference/armaphract/x-urban-fog-t9s.jpg` (a callout on a leader line off the unit).

## Contract

The in-world UI reads as a **holographic tactical projection**, not flat stickers:
- thin, luminous lines with a soft glow;
- readouts that float off their unit on a leader line;
- no opaque boxes.

It stays legible at the default and strategic cameras and over grass, road and fog.

## What changes

1. **Order overlay (`orderOverlay.ts`) goes thin and luminous.**
   - Route ribbons drop to a line about 2 px wide on screen (keep a minimum in metres, as the play-area border does), in an emissive colour with a soft glow.
   - Final rings, facing arrows, soldier spot markers, spoke lines and reverse chevrons become outlines, not solid discs, and are thinner.
   - Cover icons stay coloured by tier (yellow, light green, dark green), as small glowing pips, no longer filled discs.
   - The user called today's pink ribbons "way too thick" (`assets/reference/user/current-overlay-2026-09-27.png`).
2. **Weapon badges become holo callouts.**
   - No black box. Each selected unit's readout (ammo rings with reload progress, e.g. RIFLES ∞ / GREN 8, CANNON AP20 / HMG ∞) floats up and to the side of the unit.
   - A thin leader line joins it to the unit's anchor, ARMAPHRACT-style.
   - Its text is small caps or monospace. The progress rings are thin glowing arcs.
   - Callouts avoid overlapping each other and the unit (a simple screen-space nudge is fine), and they track the unit smoothly.
3. **Glow.** Overlays composite after post and never enter bloom (a firewall: they're never fogged, graded or tone-mapped). So the glow comes from the overlay itself: a soft halo drawn in the overlay shader (distance-based falloff around lines and shapes), or a small blur of the overlay target composited additively. It's not bloom from the HDR world.
4. **The HUD panel (DOM) gets the same language:** translucent dark glass, thin glowing borders, monospace labels, the same accent colours. Layout and controls are unchanged.

## Colours and numbers

Every colour, width, glow radius and alpha lives in the fixture's `presentation` block (one owner), including the side accents. Keep the existing meanings:
- route colour per order kind;
- cover tier colours;
- reverse amber;
- x-ray blue for own units behind geometry (27b).

## Verification

- Before and after at the default, ground and strategic cameras (`VILLAGE_TOURS=orders`, `village-watch` battle frames), judged with compare-screenshots against the two references, for **the UI's line weight, glow and callout only**. The world and the models are out of scope.
- An unprimed screenshot-critique, asking about legibility over grass, road and fog, and whether anything reads as a sticker or a box.
- The overlay isolation check still passes (compositing algebra). Retune a pixel check only with a `decisions.md` entry.
- `bun run check` and `bun run verify`. Frame cost as a benchmark row if the glow costs anything measurable.
- Preview the shots for the user (non-blocking).

## Decision budget

- **Delegated:** the exact accent colours within the references' spirit, the glow technique, callout placement rules, and the font choice among local or system fonts (no web fonts that aren't already in the repo).
- Anything else goes in `choices.md`.
