# Distant detail

This slice tested two ways to reduce GPU work in the Metro XL 100-versus-100
stress scene: replacing distant parked-car meshes with 2D cards, and omitting
shadows from visible props whose conservative projected footprint is below a
small screen-space threshold.

## Shipped contract

Presentation owns the shadow decision for both browser and native clients. A
visible prop whose projected footprint is below `shadow_caster_px` is omitted
from the caster lists; the threshold is `2` in the production fixture. The
screen test uses camera-forward depth and a two-times footprint margin for the
authored low-sun setup. The check applies only to visible props. Offscreen
casters stay eligible because a shadow can land on visible ground after the
caster leaves the camera view. Simulation, visibility knowledge, and replay
digests do not change.

The implementation is a small extension to `Detail` and the existing prop
selection path. The focused presentation test covers the enabled and disabled
contracts. The renderer still receives ordinary camera and caster lists; no
new GPU resource or client-specific path was added.

## Rejected card candidate

The parked-car card candidate was removed. It changed representation and asset
plumbing across the renderer without improving the measured target or making
cars easier to read. At the wide stress view it increased GPU time from about
32.14 ms to 40.01 ms and reduced actual FPS from 22.8 to 20.4. Close and
tactical views also showed no reliable silhouette or scanability improvement.
The card shader's per-sample coverage and surface-depth experiments were
valuable diagnostics, but they are not production behavior.

## Evidence

The matched benchmark is retained under
`throwaway/car-shadow-20261021/results-final/measurements.json`: Metro XL seed
4, 100 units per side, Low graphics at 0.75 render scale, tick 90 paused,
three repeats, and close/tactical/wide camera distances of 65 m, 260 m and
2,400 m. FPS is the measured frame-rate report; GPU time is the measured GPU
window, not a reciprocal FPS claim.

| View | Baseline | Shadows only | Cards only | Both |
| --- | --- | --- | --- | --- |
| Close | 111.1 FPS / 4.77 ms | 108.0 / 5.07 ms | 111.6 / 4.94 ms | 107.2 / 5.26 ms |
| Tactical | 115.5 / 5.57 ms | 101.5 / 6.33 ms | 98.6 / 6.32 ms | 89.4 / 7.16 ms |
| Wide | 22.8 / 32.14 ms | 25.4 / 29.82 ms | 20.4 / 40.01 ms | 26.2 / 28.17 ms |

The shadow result is a wide-view win because the stress workload contains many
visible props whose shadows are below the perceptual threshold. Close and
tactical views are near the workload's pacing and show the cost of the additional
selection work; they are not presented as universal FPS gains. The combined
row is diagnostic only because the card candidate is rejected.

A controlled paired recapture used one baseline build and one candidate build
under the GPU lock. The fresh visual review found the three candidate views
matched the baseline: no visible prop or ground shadow was added, removed or
misgrounded. Shadows remain broad and subtle at these scales, and the candidate
preserves visible car geometry. The wide view is still dominated by the existing
diagonal fog hatch; that is a separate
readability issue, not a card or shadow acceptance criterion.

## Limits

This is a perceptual detail rule for the authored light, not a general culling
replacement. It does not claim constant frame time for arbitrary maps or
lights, and it does not remove offscreen shadow contributors. Any future
screen-space representation must prove both GPU timing and native-scale visual
readability on the same three-view stress workload before it becomes production
behavior.
