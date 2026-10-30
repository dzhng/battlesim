# 08 · Collapse copies and loop the cascades

**Contract.** One authored body per algorithm.

## Seam

- `fogUnpackBuild` becomes `fogUnpack` (byte-identical today,
  `fog_sight.wgsl:251-259`).
- `fogSample` and `grassSample` become one height lookup (identical today,
  `fog_sight.wgsl:225`, `grass.wgsl:90`).
- The nine `{color, fog}` fragment output structs in `fog_sight.wgsl` become
  one.
- The shadow cascade blend (`shadow.wgsl:25-80`) loops over the middle
  cascades. Cascade 0 (`:36`, weights by depth differently) and the last
  (`:73`, no margin) stay explicit (map unknown 2). The loop index replaces a
  literal cascade index, which is the likeliest source of a rounding change.

## Verify

- All three reels byte-identical; GPU pixel tests; browser scenes `fog`,
  `ground`, `street`.
- If the cascade loop changes a byte, keep the unrolled form and record it in
  `choices.md` (map unknown 1). Don't accept drift.

**Delegated:** the merged names.
**Stays green:** every plate.

**Outcome:** the copies collapsed; the cascade loop changed one pixel by one
level, so the blend stays unrolled (`choices.md`).
