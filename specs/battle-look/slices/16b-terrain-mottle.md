# 16b — Terrain mottle never reads as cloud shadow

**Status:** done (2026-09-26). The broad mottle is now hue-only dry strips along each plot's rows with firm edges; the fine octave keeps the brightness. The last critique answers no about the mottle at the three village framings and frame b; frame c's grass field keeps a residue ([`choices.md`](../choices.md), slice 16b). **Depends on:** 16, 19b. **Lane:** renderer.

## Contract

Several unprimed critiques (slices 17b, 24 and 25) answered "yes" to the standing question because slice 16's low-frequency terrain mottle reads as **cloud shadow**: soft dark blotches across fields, with no caster. Fix the mottle so the patchwork keeps its painterly variation without large, soft, dark blobs: vary hue more than luminance, with smaller scale or crisper edges following plot structure. Seen pixels stay the real world; no post-processing.

## API seam

The terrain material's mottle term and `fixtures/biomes/summer.json` (`plots.*.mottle`, the mottle scales in `field_rules`). Nothing else.

## Verification

- An unprimed screenshot-critique asked exactly "Could any dark region be mistaken for sun shadow, or any shadow for fog?" answers no about terrain mottle on the default, strategic and ground village framings and on slice 17b's frames b and c.
- compare-screenshots against the WARNO field crops (patchwork variety kept).
- A pixel diff shows only terrain moves.
- Record frame cost (benchmark short run).

## Decision budget

- **Delegated:** the numbers and the noise shape.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Every existing scene and test. `bun run check` and `bun run verify` at closeout.
