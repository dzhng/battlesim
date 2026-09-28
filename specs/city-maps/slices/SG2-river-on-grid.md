# SG2: river on grid

**Depends on:** none (start here). **Kind:** slice.

## Question
Does a river of 12 m or more, carved as a gentle bank into the 4 m flat-shaded grid, read round, not stepped (Q-G5, Q-G19)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG2.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
Hack `HeightField::build` (`world/terrain.rs:33-55`) to carve by distance to a densified meander at 12 m and 24 m widths and 2 m depth. Draw the water as a ribbon clipped per fragment to the distance edge, and shade the wet band by distance.

## What the human can run or see
Top-down, 250, 65 and 25 m, at a low sun.

## Verification
- Measure the waterline's deviation from the exact edge (m), and the largest luma step between adjacent triangles on the bank.
- Critique: "does any river or bank read as blocky or stepped?"
- **Kill if:** deviation >0.5 m after clipping; or a facet step >8% of grass luminance; or it reads blocky at 65 m.
- **Fallbacks:**
  1. clip plus a wet band that covers the zigzag;
  2. a render-only shading normal from the analytic bank profile, inside the bank band only (normals, never positions);
  3. a 16 m minimum with gentler banks;
  4. reslice: rivers need the finer grid Q-G5 defers.
- **Also sets** the mud and wet-bank palette at or above the grass's luminance (L-G3).

## Delegated to the implementer
The hack. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.
