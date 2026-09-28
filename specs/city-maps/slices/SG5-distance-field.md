# SG5: distance field

**Depends on:** none (start here). **Kind:** slice.

## Question
Which structure gives terrain and grass an exact-enough distance to roads, forests and rivers, so the per-fragment loops can go (Q-G19)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG5.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
- **Arm A:** a signed-distance texture per channel (road, forest, water), r16float at 1 m texels, clamped to ±64 m, sampled bilinearly.
- **Arm B:** a GPU segment-bucket index (per 16 m cell, a list of segments) with an exact distance per fragment. It could share its layout with the sim's `SurfaceIndex`.
- Both are fed densified 2 m strokes (a 1.6 km road is about 800 segments) plus 60 forest strips.

## What the human can run or see
An edge-error heat map at 25 m, and paired GPU ms.

## Verification
- Measure:
  - edge error in px at 25 m, including at joins and where width changes;
  - terrain and grass-build GPU ms against today's loops (they cost 6.84 ms for 120 rects, battle-look row 34b);
  - bytes at 1.6 km and 3.2 km;
  - load time.
- **Kill a given arm if** its edge error is >0.25 px at 25 m, or it takes >16 MB at 1.6 km, or it bakes in >250 ms, or it gives no GPU win.
- **Output:** the chosen structure for C63.

## Delegated to the implementer
Arm internals. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.
