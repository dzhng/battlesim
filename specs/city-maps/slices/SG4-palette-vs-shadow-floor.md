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

## Outcome

Run on the way to C84, as `fog-look` on the starting commit and on two drafts of the palette. Nothing of the spike merged but its verdict.

**Verdict: kill fired.** A desaturated meadow under the houses fails the darkest-seen check, by hue under one style at one framing. The rule is not threatened where the spike expected (a darker field) but where it did not look: main already holds it there by a hair.

| `default-wall` (darkest 1%, luma 0–255; a\*b\* distance) | seen | unseen `dusk` | unseen `grey-veil` | hue `grey-veil` | verdict |
|---|---|---|---|---|---|
| main (meadow C\* 25) | 48 | 47 | 49 | 13.1 | passes, by 1 level and by 1.1 of the 12 margin |
| draft A (meadow C\* 17, L\* +1.3) | 48 | 48 | 50 | 11.9 | **fails** `grey-veil` |
| draft B (meadow C\* 21, L\* +1.3) | 48 | 48 | 50 | 12.1 | passes by 0.1 |

Every other style and framing kept a margin of 3 or more either way on all three.

**Why.** At that framing the darkest seen ground is the shaded floor of the wood (its own palette: 48 on every draft), and the unseen wedge is sunlit settlement ground dimmed by the style. Shaded seen meadow and fog-dimmed lit meadow come out at the same luminance whatever the palette (medians 65 to 68 against 66 under `grey-veil`): the shadow floor and the styles' `dim` put them there, so only hue tells them apart, and `grey-veil` has no hue of its own. What the check bounds is therefore how *light* and how *grey* the ground round the houses may be, not how dark a field may be.

**What C84 took from it.**
- The ground under the settlement keeps main's colours as a kind of its own (`green`), so the frames the check reads are main's (C84's run: every number within 0.7 of main's). Open country takes the desaturated palette.
- **The L\* floor is 24** (`PLOT_MIN_LSTAR`, `terrain/biome.ts`, CIELAB L\* of a plot's darkest albedo: its palette's darkest colour at the low end of the per-plot jitter, rows at their mean). It is what main's own darkest ground measures (24.5), and it guards the case the check's framings do not hold: a sunlit field as dark as ordinary ground under fog.
- `light.shadow_floor` was not touched.

**Open, for whoever owns the fog styles.** `grey-veil` at `default-wall` passes on main by 1.1 a\*b\* units and `dusk` by one luma level; any change to the ground there can flip it, and no palette can widen it (the two luminances are tied by the light). Until that margin is widened in the style or the light, the settlement's ground cannot be desaturated with the fields.

**Later in the same pass** the plots were recut (the land's grain), and the unseen wedge at `default-wall` fell on other ground: the framing then passed by luminance under every style (seen 45, unseen 37 to 40). The margin there belongs to the layout as much as to the palette; the check should not be read as a measure of either.
