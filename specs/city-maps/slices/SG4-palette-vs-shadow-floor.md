# SG4: palette vs shadow floor

**Depends on:** none (start here). **Kind:** slice.

## Question
Does a desaturated olive, tan and brown field palette keep the rule that the darkest seen ground stays lighter than, or apart in hue from, unseen ground (L-G4)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG4.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
Draft the palette in `fixtures/biomes/summer.json` and run `bun run --cwd web scene -- fog-look` (its check is at `web/scenes/fog-look.mjs:679-726`).

## What the human can run or see
The check's numbers under every light style.

## Verification
- Record the darkest-1% luma and a*b* per style.
- **Kill if** the check fails under any style.
- **Fallbacks:**
  1. desaturate at constant L*, and texture by chroma, not value;
  2. a biome L* floor enforced as a test.
- **Never** retune `light.shadow_floor` to fit a palette.

## Delegated to the implementer
Palette values. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.

## Feedback that would change this slice
A palette indistinguishable from shadow or fog changes the measured colour proposal before C84.
