# 27e — Holo-tactical in-world UI

**Status:** planned. **Depends on:** 27b (overlay depth, feeds). It runs beside 27d. **Lane:** renderer and presentation. **Given:** the user's request of 2026-09-27, with references `assets/reference/user/holo-glow.png` (glow and material) and `assets/reference/armaphract/x-urban-fog-t9s.jpg` (a callout on a leader line off the unit).

## Contract

The in-world UI reads as a **holographic tactical projection**, not flat stickers:
- thin, luminous lines with a soft glow;
- readouts that float off their unit on a leader line;
- no opaque boxes.

It stays legible at the default and strategic cameras and over grass, road and fog.

**Improve on the references (user, 2026-09-27): "they are not perfect."** They're starting points. The bar is a better game UI than either: clearer at a glance, legible over busy ground and fog, less clutter, a hierarchy where what matters in a fight reads first, and callouts that don't fight each other or the units.

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
4. **The HUD panel (DOM) gets the same language, and reads as a game UI, not B2B SaaS (user, 2026-09-27):**
   - no default browser controls, pill buttons or dashboard layout;
   - commands read as a game command bar (icons, hotkey hints, glowing active states);
   - status reads as a tactical readout;
   - translucent dark glass, thin glowing borders, monospace labels, the same accent colours.
   
   Controls, behaviour and accessible names are unchanged, so scene checks still click them.

## User feedback on the first pass (2026-09-27)

- **Selection:** visible parts are never tinted. Selection is the marker under the unit, in a yellow "selected" colour.
- **The body highlight is for occluded parts only** (user: "a cool effect that we can use for certain things"). A unit behind a building or ridge has its hidden parts highlighted: pale blue for own units, and the selected yellow when selected. It's kept as a reusable capability: one per-instance highlight colour for occluded parts, driven by presentation, ready for later uses such as a spotted target behind cover.
- **One colour for order markers and routes.** Drop the per-order-kind pink, orange and amber. Exceptions only for meaning a player needs (cover-tier pips; perhaps "blocked"), each justified. A reverse move is a single back-pointing chevron in the same colour.
- **Squad destination:** no spoke lines to each soldier. Keep the soldier spots. The route ends at the edge of the squad's **area ring** (27d's anchor and radius), and the ring is drawn.
- **Teardrop markers (user):** the marker under a unit, a vehicle's destination marker and each soldier's spot marker are teardrops whose point is the facing. The squad's area ring stays a circle. Travel direction is a pair of detached chevrons beside the marker, in the same colour: along the facing on a forward move, and against it on a reverse move, which makes reversing obvious. A unit at rest shows no chevrons. The chevrons are animated, a marching pulse in the travel direction on about a 1 s cycle, driven by the presentation clock so held-clock captures stay stable.
- **Second pass (user):**
  - no reverse chevrons at the destination marker, since the unit's own marker shows them;
  - a big circle under the squad where it stands, with the route running from its edge to the destination area's edge;
  - infantry area rings drawn a little smaller, as a visual scale only;
  - a smaller tank marker, even smaller than its hull;
  - the marker shape is **(a), a circle with a filled arrowhead on its rim** (user picked it from a variant sheet of five). The squad area ring uses the same language: a filled arrowhead on its rim at the squad's final facing replaces the loose facing arrow.
- **Command bar:** bottom of the screen, full width, like a strategy game (unit card plus a command grid with icons and hotkeys). Battle status and replay controls go in a slim top bar or a corner.

## Colours and numbers

Every colour, width, glow radius and alpha lives in the fixture's `presentation` block (one owner), including the side accents. Keep the existing meanings:
- route colour per order kind;
- cover tier colours;
- reverse amber;
- x-ray blue for own units behind geometry (27b).

## Verification

- Before and after at the default, ground and strategic cameras (`VILLAGE_TOURS=orders`, `village-watch` battle frames), judged with compare-screenshots against the two references, for **the UI's line weight, glow and callout only**. The world and the models are out of scope. The verdict names at least one way the result improves on the references, not only how close it gets.
- An unprimed screenshot-critique, asking about legibility over grass, road and fog, whether anything reads as a sticker or a box, and whether it reads as a game UI or a web app.
- The overlay isolation check still passes (compositing algebra). Retune a pixel check only with a `decisions.md` entry.
- `bun run check` and `bun run verify`. Frame cost as a benchmark row if the glow costs anything measurable.
- Preview the shots for the user (non-blocking).

## Decision budget

- **Delegated:** the exact accent colours within the references' spirit, the glow technique, callout placement rules, and the font choice among local or system fonts (no web fonts that aren't already in the repo).
- Anything else goes in `choices.md`.
