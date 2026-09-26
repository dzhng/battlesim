# 15b — Fog edge: never read as shadow

**Status:** done, gate not fully met: the critique answers no on three of the four gate frames; the single-wall default frame answers yes for a seen-world dark (the forest floor), never for the fog (`choices.md`, slice 15b). **Depends on:** 15 (merged with its visual gate open). **Lane:** renderer.

## Contract

Close slice 15's failed gate. The final unprimed critique still read a sight-shadow wedge leaving a building as a second, blue cast shadow. It read the fog's tone correctly; what it misread was the wedge's *shape*, a dark shape attached to a wall. The user's rule is binding: "it should be clear that it is a fog of war, it should not be mistaken for shadows". This slice adds the cue a sun shadow never has at the fog's *boundary*, and softens slice 14's binary edge.

## API seam

- **A fog mask pass.** After the colour pass, a screen-space pass resolves the per-pixel seen/unseen result (slice 14's `fogSeenSurface`, slice 15's roof rule included) into a mask target. Then:
  - `edge_softness` (in `presentation.fog.styles.<name>`, the key slice 15 could not ship) blurs the transition over a set width in pixels;
  - `rim {width_px, color, alpha}` draws a line along the seen/unseen boundary, on the seen side. A sun shadow has no such line.
- `fogLook` composes with the mask, so the style stays the one owner of the unseen look. The five presets gain `edge_softness` and `rim` values, and new presets are allowed.
- Units stay unfogged. Seen pixels stay bit-identical away from the rim band.

## What you can run or see

`/lab/fog-look`, which has the preset switcher, and `/battle/village` at the default and ground cameras.

## Verification

- The seen pixels outside the rim band are identical with fog on and off (the slice 15 check, extended).
- The rim lies on the boundary: every rim pixel is within the rim width of a seen/unseen transition in the mask, and no rim appears where the whole screen is seen.
- The fog-vs-sweep agreement stays within 5%.
- An unprimed screenshot-critique, **the gate**, asked exactly: "Could any dark region be mistaken for sun shadow, or any shadow for fog?" It must answer no at the default and ground cameras, on the street wedge and single-wall frames that failed slice 15.
- compare-screenshots against the ARMAPHRACT crops, where the reference's fog edge carries a line.
- Record frame cost (benchmark short run).

## Decision budget

- **Delegated:** the mask resolution, the blur kernel, and rim numbers within the fixture.
- **Needs the user (non-blocking checkpoint with preview-shots):** which preset ships as the default once the gate passes. Show at least `dusk` with a rim and `blue-highlight` with a rim.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test. `bun run check` and `bun run verify` at closeout.
