# S3: frame

**Depends on:** S2's counts (it can start synthetic). **Kind:** slice.

## Question
Can an instanced city at hero density near the camera plus a far tier hold the frame, and which interior tiers (O-1) and which kit texture edge are affordable?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write the verdict to `specs/city-maps/spikes/S3.md` (numbers table + one verdict row per question + kill check).

## API seam
A throwaway lab route that instances placeholder modules at S2's triangle and instance counts (upper bound: 4,153 instances per NYC building) over S1's crop. Chunks are 64 m, as in the corpse chunks (`modelDetail.ts:66-110`), with a box-and-roof far tier beyond radius R.

## What the human can run or see
Benchmark tables and a contact sheet of the stress city at ground (25 m), default (65 m, 0.85 rad), mid (250 m) and strategic (2,000 m) cameras.

## Verification
- Arms:
  - interiors off / LOD0 / LOD0–1 / every tier, using a placeholder room quad plus the real flat-perspective lookup and a blended-glass stand-in;
  - shadows on, or cast from the far tier only;
  - kit texture edge 256 / 512 / 1024, in the shared texture array against a kit-own array (`modelTextures.ts:5-8`).
- Measure GPU frame total, CPU prepare ms, instances and triangles drawn, draw calls, buffer and texture bytes, JS heap, and placement upload bytes.
- **Kill:** the static city above **15 ms GPU** at any camera. Fallbacks in order:
  1. interiors at LOD0 only;
  2. shadows from the far tier only;
  3. a smaller hero radius;
  4. merged meshes per chunk.

  If all four miss, reslice at G0.
- **Decides for G0:** the residency radius, triangle budget per tier, the O-1 interior tiers, and the kit texture edge (never inflating the shared array).

## Delegated to the implementer
Prototype plumbing only. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.

## Feedback that would change this slice
If the user prefers fewer interiors to a smaller hero radius, the fallback order flips.
