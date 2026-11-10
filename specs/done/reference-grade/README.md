# Reference colour grade

**Shipped (2026-11-10, PR #32).** The game's one light (`presentation.light` in
[`fixtures/presentation.json`](../../../fixtures/presentation.json), admitted by
`LightSettings` in `crates/presentation/src/light.rs`) is graded to match a
reference photo: [`norwegian-tank-afghanistan.jpg`](assets/reference/norwegian-tank-afghanistan.jpg)
([sources](assets/reference/SOURCES.md)). The new light keeps the photo's rich
yellow-green midtones, warm tan highlights and real light and dark on hill
slopes. Haze fades the scene only at real distance. Clear-day skies stay
bright blue. The change only affects rendering, so no digest moves.

The requirement, from the user: the old grade felt flat, and the photo is the
reference for colour-grading the whole game. Light the game to its colour
response. Keep clear-day skies light and bright blue, never grey. Keep each
map's weather. Keep units, the UI and fog of war readable.

## Why it works this way

- **One light for every view.** Live play, the menu, and native SDR and HLG
  capture all read the same `presentation.light`. The look is never a tint that
  only captures apply.
- **Match the ground, not the sky.** The photo was taken on an overcast day,
  and its sky is a pale grey-green. Copying that would grey every clear-day
  map. So the grade is fitted to the land and the vehicle, and the sky takes
  only a small share of it (`sky.grade_mix`, documented on `SkySettings`). Sky
  turbidity stays at the clear-day value, because a map's own weather is what
  makes its sky heavier.
- **Aerial perspective comes from distance.** In the photo, the grass near the
  camera is the most saturated. The ridges in the middle distance keep their
  green and their slope shading, and only the farthest range fades into the
  sky. In the game, the `haze` block owns this. It starts late and fades to a
  pale grey-green, never to milky white.
- **Hills are shaped by the gap between sun and shade.** The photo's modeling
  is the brightness difference between its sunlit and shaded slopes. Four
  values set that difference: `shadow_floor` (how much sunlight a shadowed
  surface keeps), `sky.fill` (the light that reaches shade), and the grade's
  `shadow_lift` and `contrast`. Lowering them darkens shade. The sunlit side
  must stay bright.

## Invariants

- **Fog never reads as shadow** ([battle-look](../battle-look/README.md#invariants-what-must-keep-holding)).
  Darker shade and a cooler fill both move shadows toward the fog's look, so
  any retune of the four values above must pass the `fog-look` scene. It
  renders fog beside sun shadows, and an unprimed critique of its frames must
  find no shadow that reads as fog.
- **Clear skies stay blue, and each map keeps its own weather.**
- **Only rendering changes.** A grade change leaves every digest and replay
  unchanged.

## How the grade was judged

Stills were taken on four maps, from the same camera before and after: the CV90
hill (`cv90-hill`), a town (`market-town`), Paris (`paris-corner`) and open
ground (`ground`). The CV90 hill was also compared side by side with the photo
at the same size and aspect. Each still was measured three ways:

- the median saturation;
- how dark the cast shadows get (the 10th-percentile luma of shadowed pixels);
- the median luma of the sunlit side divided by that of the shaded side.

The target values come from the photo below. Only the direction of each
measure carries over to the next change. The per-map numbers do not: the
shipped values were tuned after the last measured pass.

### Where the plan diverged

- **The first pass overshot toward grey.** It removed the yellow wash, but the
  town fell to a median saturation of 0.27 against the photo's 0.37. The
  second pass raised saturation, warmed the midtone greens, and deepened cast
  shadows and shaded slopes.
- **The planned saturation range was too low.** The plan estimated 0.55–0.85.
  Saturation at those values was the first pass's grey. The grade needed
  close to 1 to reach the photo's saturation.

### Dead ends

- **More turbidity, to mimic the overcast photo.** It greys every clear-day sky.
- **Heavy, early haze** (the old `distance_scale` and `range_strength`). It
  washed out the middle-distance greens and slope shading that the photo keeps
  clear.
- **The full grade on the sky** (`grade_mix` 1). The warm grade fought the blue
  sky.
- **A strong orange highlight tint.** It posterised the midtones. The photo's
  highlights are a soft warm tan.

## The reference, measured

All values are sRGB from 0 to 1, measured over the whole 1024×682 frame.

| Measure | Value |
| --- | --- |
| Mean luma (Rec.709) | 0.465 (σ 0.115) |
| Luma p5 / p50 / p95 | 0.294 / 0.455 / 0.664 |
| Black point (1%) RGB | (0.106, 0.275, 0.000) |
| White point (99%) RGB | (0.812, 0.804, 0.745) |
| Saturation p50 / p90 | 0.375 / 0.833 |
| Cast-shadow luma p10 | 0.305 |
| Sun ÷ shade median luma | 1.275 |

- **Hues.** Green covers most of the frame: mean hue 93°, saturation 0.45. The
  tan highlights sit near 60°. There is almost no blue anywhere, because the sky
  is grey-green with saturation about 0.10.
- **Saturation by depth.** Horizontal strips from the sky down to the
  foreground run sky 0.10, far ridge 0.14, middle distance 0.27, then 0.42,
  then near grass 0.72–0.79. The blue channel falls away close to the ground.
- **Soft ends of the range.** The blacks are lifted and the open shadows are
  never crushed. The whites stop near 0.8 and never clip.
