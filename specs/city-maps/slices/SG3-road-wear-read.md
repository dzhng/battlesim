# SG3: road wear read

**Depends on:** none (start here). **Kind:** slice.

## Question
Do a packed core, a jittered shoulder of 2–4 m and soft ruts avoid reading as an outline or a shadow at the default camera (Q-G2, Q-G3)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG3.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
Paint the shoulder and ruts through the existing per-fragment road loop (the village has 2 roads), replacing the road-keyed verge (`terrainMaterial.ts:415-424`). Try two palettes (country road, dirt).

## What the human can run or see
250, 65 and 25 m at the village road.

## Verification
- **Note:** at 65 m a 0.4 m rut is 4–8 px wide, so the "fade below 2 px" rule never kicks in where the game is judged.
- **Kill if**, after two tuning rounds, the unprimed critique still says yes to "does the road read as an outline?" or "could any dark region read as shadow?", or the shoulder must go below grass luminance to read at all.
- **Fallbacks:**
  1. shoulder by hue and grass thinning only;
  2. ruts as roughness and normal only;
  3. ruts cut.

## Delegated to the implementer
Palettes. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.
