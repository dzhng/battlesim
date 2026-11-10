# Reference color-grade analysis

Reference: `refs/norwegian-tank-afghanistan.jpg` (1024×682), also
https://pbs.twimg.com/media/HWi5bydX0AAKTQs.jpg?name=large

David: use this as the reference for color-grading the whole game; the current
grade feels flat. Light the game to the same color response. Keep clear-day
skies light/bright blue (do not grey them). Keep each map's weather. Keep unit
and UI readability and fog-of-war contrast. Render-only — no gameplay or digest
changes.

## Eye read

- Soft diffuse overcast day in the photo; midtones are rich but slightly muted
  yellow-green, with real slope shading on the hills.
- Vehicle is a warm desert tan; highlights stay warm, not bleached.
- Aerial perspective: near grass stays saturated; mid ridges keep green and
  slope light; only the farthest range fades into a pale grey-green sky.
- Blacks are lifted (no crushed shadows); whites are soft (~0.8), never clipped.

## Numerical summary (sRGB 0–1)

| Metric | Value |
| --- | --- |
| Mean RGB | (0.414, 0.497, 0.305) |
| Mean Y (Rec.709) | 0.465 (σ 0.115) |
| Black point (1%) RGB | (0.106, 0.275, 0.000) |
| White point (99%) RGB | (0.812, 0.804, 0.745) |
| Y p5 / p50 / p95 | 0.294 / 0.455 / 0.664 |
| Michelson contrast (p5–p95) | 0.386 |
| Saturation p50 / p90 | 0.375 / 0.833 |

### Hue bands (sat > 0.08, 0.05 < Y < 0.95)

| Band | px | mean RGB | mean Y | mean sat | mean hue° |
| --- | --- | --- | --- | --- | --- |
| green | 611k | (0.391, 0.485, 0.288) | 0.451 | 0.445 | 93.1 |
| yellow_tan | 76k | (0.563, 0.568, 0.398) | 0.554 | 0.371 | 59.5 |
| orange_brown | 0.6k | (0.711, 0.643, 0.541) | 0.650 | 0.252 | 36.4 |
| sky_cyan_blue | ~0 | — | — | — | — |

Sky in this photo is **not blue** — pale warm grey-green (top strip mean RGB
≈ 0.625, 0.655, 0.592, sat ≈ 0.10). David still wants clear-day maps to keep a
lighter brighter blue sky; match the land/vehicle color response, not the
photo's overcast sky.

### Aerial perspective (horizontal strips, top→bottom)

| y frac | mean RGB | mean Y | mean sat |
| --- | --- | --- | --- |
| 0.05 (sky) | (0.640, 0.667, 0.605) | 0.657 | 0.102 |
| 0.15 (far) | (0.511, 0.570, 0.491) | 0.552 | 0.144 |
| 0.35 (mid) | (0.424, 0.509, 0.376) | 0.482 | 0.269 |
| 0.55 | (0.372, 0.472, 0.285) | 0.438 | 0.416 |
| 0.75 (near) | (0.343, 0.434, 0.129) | 0.393 | 0.717 |
| 0.95 | (0.275, 0.394, 0.089) | 0.347 | 0.793 |

Saturation rises sharply toward camera; blue channel collapses near ground.
Far fade is pale grey-green, not milky white.

Raw stats: `throwaway/color-grade/ref-stats.json`.

## Current game defaults (`fixtures/presentation.json` light)

```
exposure 1.7
grade: sat 1.1, contrast 0.62, split_tone 1.0, shadow_lift 0.3
  shadow_tint [1.15, 1.02, 0.85], highlight_tint [1.27, 1.05, 0.77]
  lift [0.03, 0.03, 0.026]
sky: turbidity 4.5, radiance 10.5, fill [1.0, 0.92, 0.78]
  saturation 1.62, tint [1.02, 0.9, 1.05], grade_mix 1.0
haze: distance_scale 4.6, clear_radius 300, range 600→3000 @ strength 0.8
sun_intensity 7.2, elevation 0.7, shadow_floor 0.32
```

Likely causes of the "flat" feel vs the reference:
1. Heavy haze (`distance_scale` 4.6 + `range_strength` 0.8) washes mid-distance
   greens and slope shading — opposite of the photo's clear mid ridges.
2. `grade_mix: 1.0` pushes the warm grade onto the sky, fighting a bright blue
   clear sky.
3. Strong orange highlight_tint (1.27/0.77) and high grade.saturation can look
   posterized rather than the photo's soft yellow-green midtones.
4. Contrast 0.62 with lifted shadows is soft; the photo's slope shading needs
   clearer midtone separation without crushing blacks.

## Target game-wide response (not a capture hack)

Apply in shared `fixtures/presentation.json` (and any code defaults that must
stay in lockstep). Live gameplay, menu, native HLG and SDR capture all read
this light. Do **not** raise turbidity toward overcast to mimic the photo's
sky — that would grey clear-day maps. Do **not** change fog-of-war styles,
gameplay, or digests.

Proposed direction (tune against stills):

1. **Grade** — yellow-green midtones, warm tan highlights, lifted soft blacks:
   - saturation ≈ 0.55–0.85 (less aggressive than 1.1)
   - contrast ≈ 0.72–0.85 (clearer slope shading)
   - split_tone ≈ 0.7–0.9
   - shadow_lift ≈ 0.35–0.45
   - shadow_tint ≈ [1.02, 1.06, 0.92] (slight cool-green in shade, not brown)
   - highlight_tint ≈ [1.08, 1.04, 0.90] (warm tan, not orange crush)
   - lift ≈ [0.035, 0.040, 0.032] (soft floor like the photo's open shadows)
2. **Sky** — keep clear days bright blue:
   - grade_mix ≈ 0.25–0.45 (land gets the grade; sky mostly keeps its own colour)
   - keep/boost saturation (≈ 1.5–1.7) and a light blue tint; do not grey tint
   - leave turbidity at clear-day default (~4.5); maps/weather own heavier weather
3. **Haze / atmosphere** — fade only at real distance:
   - distance_scale ≈ 2.2–3.2 (down from 4.6)
   - clear_radius_m ≈ 400–500
   - range_near/far ≈ 900–1200 / 4500–5500, range_strength ≈ 0.25–0.45
   - mist_color pale grey-green ≈ [0.78, 0.82, 0.78], lower opacity
4. **Sun / fill** — soft diffuse ground fill without killing sun:
   - fill slightly warmer-green for grass bounce if needed
   - keep sun_intensity in a bright daytime range; shadow_floor high enough that
     shade stays readable (units/UI), not near-black

## Proof plan

Before/after stills on at least: CV90 hills (reuse cv90-hill capture pose if
available), Paris city (`paris-corner`), NY/market town (`market-town`), and a
desert or open plain (generated open-country / ground / ambush — pick one with
open terrain). Side-by-side of CV90 hill vs the reference. Phone-size crops.
Run screenshot-critique and compare-screenshots. Check fog-of-war readability
on a tactical still. Run light tests, typecheck, clippy, fmt.

## Non-goals

- No push, no PR (David merges later).
- No capture-only env hacks (`BATTLEGAME_CAPTURE_TINT_*`) as the shipped look.
- No gameplay / balance / digest changes.
- Do not edit `~/dev/battlesim` or other `~/tmp/*` worktrees.
