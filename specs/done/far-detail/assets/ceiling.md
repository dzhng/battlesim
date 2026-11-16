# Ceiling probe

> Recorded while the work was under way. Slice numbers name its build order,
> and forecasts here were settled later: see [what shipped](../README.md).

Does drawing the top kinds from a far-class mesh save GPU time at the wide
hold, now that tiny visible props may skip their shadow? Measured 2026-11-15
for [far detail](../README.md) on base commit 811bb8c27.

## Verdict: go

The candidate won all three wide pairs, each by about 11.5 ms (a third of the
frame's GPU time), against an A-against-A spread of 0.01 to 0.03 ms. Counts
were checked before timing and held exactly in every batch: equal instances in
every pass, and B submitting precisely the expected triangle drop. Where the
time went: the view pass and the casts both shrink, but the casts carry most of
it. Of the 50.6 million triangles a frame the probe removes at the wide hold,
35.7 million are casts, because cars and walls stand a pixel tall yet nearly
all of them cast into three or four cascades ([census](census.md)). Slices 01
to 04 proceed. The census names parked cars for 03 and courtyard walls as 04's
first kind.

What the probe overstates: its tier 3 is the far mesh everywhere, so it also
coarsens the casts of cars and walls outside the view, and the tactical view.
The finished design keeps tier 3 for casters outside the view
([decision 3](../decisions.md)). Those casters are 8.1 million of the 50.6
million triangles removed (5.05 million from cars, 3.01 million from walls), so
the finished feature will save less at the wide hold. If time followed
triangles linearly, the saving would be about 9.7 ms, but that is an
estimate, not a measurement; slice 03 measures it. The tactical speed-up
below is a probe artefact too: at 260 m cars stand 6 to 14 px and walls 4 to
8 px, above any far threshold, so the finished feature leaves that view alone.

## The state and the arms

- **State.** The [fresh control](control.json): the receipt's scene, settings
  and camera rule at today's digest `ef325ab1cc76c583` (the receipt's
  `60ede87ba60aab0b` no longer reproduces; the generated map changed, see the
  [census](census.md)). Holds at 65, 260 and 2,400 m on (4864.04, 4851.69),
  1280×720 page, 960×540 canvas, Low.
- **A.** A production build of staging (811bb8c27).
- **B.** The same build with two baked bundles changed and nothing else: the
  parked car's tier 3 a closed, unbevelled body hull plus a cabin hull in its
  own paint (172 → 28 triangles), and the courtyard wall's tier 3 a two-band
  hull, brick plinth and rendered body up to the ridge (88 → 24). Tiers 0 to 2
  were left as authored. Made by editing the Blender sources in a scratch
  checkout, exported on Blender 5.2.1, baked (exactly those two bundles
  changed) and passed by `asset check`. Both arms' WebAssembly modules are
  byte-identical; the builds differ only in the two bundles, the runtime
  catalog and the script chunks that embed it.
- **Harness.** The paused view-cost hold (`web/scenes/endurance.mjs`,
  `web/scenes/_viewCost.mjs`), driven against each prebuilt arm in its own
  fresh page, under the GPU lock. Each batch holds every view three times
  (≥120 GPU-timed frames each); a batch's number is the median of its three
  holds.

## Counts before time

Every batch of both arms, every view, matched the census build's submissions
(144 checks, all passing):

| View | Model instances, depth/colour/c0/c1/c2/c3 | B's drop, view pass (each of depth and colour) | B's drop, cascades c0/c1/c2/c3 |
| --- | --- | ---: | --- |
| Close | 283/289/194/408/678/1,172 | 8,000 | 15,296/31,344/51,024/84,144 |
| Tactical | 3,254/3,346/4,736/7,138/10,182/13,340 | 236,480 | 330,272/504,144/715,536/930,528 |
| Wide | 120,405/120,981/111,282/126,513/138,678/145,638 | 7,454,400 | 7,735,552/8,678,992/9,432,192/9,824,032 |

The drops are exactly the census's tier-3 instances and casts of the two
kinds multiplied by the triangles removed (144 per car, 64 per wall). Every
batch read digest `ef325ab1cc76c583` at tick 90. Buildings, scenery and
terrain submissions were identical in all batches.

## Pairs at the wide hold

GPU milliseconds, median of three holds. Load is the 1-minute average at the
batch's start and end.

| Pair | A | B | B − A | Load A | Load B | Note |
| --- | ---: | ---: | ---: | --- | --- | --- |
| A vs A, session 1 | 34.59 | 34.62 (second A) | +0.03 | 3.73→2.45 | 2.45→**5.67** | suspect: load more than doubled in the second batch |
| A vs A, rerun | 34.60 | 34.59 (second A) | −0.01 | 2.86→2.51 | 2.51→3.98 | |
| 1 | 34.58 | 22.95 | −11.63 | 5.67→2.34 | 2.34→3.24 | |
| 2 | 34.59 | 23.00 | −11.59 | 3.24→3.09 | 3.09→3.43 | |
| 3, session 1 | 34.66 | 23.05 | −11.61 | 3.43→**9.72** | **9.72**→3.48 | suspect: load spike in A, B began straight after it |
| 3, rerun | 34.38 | 22.97 | −11.41 | 3.98→3.99 | 3.99→4.65 | |

Session 1 ran 17:18–17:38 and the rerun 18:11–18:21 (each queued behind other
checkouts' GPU jobs, then held the lock for its whole order). The A-against-A
pair and pair 3 were rerun once for that named cause, load spikes from outside
this checkout during those batches. The original and rerun numbers agree to
within 0.25 ms.

## The other views and the tripwires

| View | A, median of every clean hold | B, median of every clean hold | Change | Single-hold range, A / B |
| --- | ---: | ---: | ---: | --- |
| Close | 6.53 ms (18 holds) | 6.55 ms (9 holds) | +0.3% | 6.00–7.33 / 6.01–7.74 |
| Tactical | 7.58 ms | 6.40 ms | −16% | 7.50–8.32 / 6.38–7.42 |
| Wide | 34.59 ms | 22.97 ms | −34% | 34.34–34.62 / 22.86–23.00 |

Clean holds are those of the batches not marked suspect, with the reruns in
place of the suspect ones.

- **Wide wins:** 3 of 3, each by more than 300 times the A-against-A spread.
  Met.
- **No view's median GPU time rises more than 5%:** missed at close when read
  pair by pair. Close pair 1 rose 7.1% (6.24 → 6.68) and the rerun's pair 3
  rose 10.9% (6.25 → 6.93); pair 2 fell 5.8%. Pooled over every hold the two
  arms are equal (6.53 and 6.55 ms). At close, one batch's three holds spread
  by as much as 1.2 ms (18% of the median), so three holds per batch cannot
  resolve 5%. The B arm submits 8,000 fewer view triangles and 182,000 fewer
  cast triangles there and draws every in-view model at the same tier. This is
  recorded as missed, not retuned. Slice 03 should hold close for more frames
  if it needs that tripwire.
- **CPU per frame within 5%:** met; per-batch frame-callback p50 is 6.2 to
  7.0 ms at the wide hold in both arms.
- **GPU memory within 5%:** met; buffers 622.5 MiB (A) and 622.4 MiB (B) at
  the wide hold, textures 335.7 MiB in both.

## Reachability

`presentation.camera.zoom_max` in `fixtures/presentation.json` is 2000, in
metres of orbit distance from the target to the eye
(`packages/renderer-core/src/cameraController.ts`). It is only the base. Every
battle builds its rig with `gameCamera.forMap`
(`apps/battle-lab/src/gameCamera.ts`), which widens `zoom_max` to 1.5 times
the rendered map span and adds the overview pitch. On Metro XL (rendered span
13,000 m) the player's zoom_max is **19,500 m**, so the 2,400 m hold is inside
it and an acceptance view. Its pitch, 0.8860277, is exactly the curve's value
at 2,400 m on that rig.

## Evidence

Scratch, deleted with the builds: per-batch evidence, logs and the analysis
under `throwaway/far-detail/runs/` and `throwaway/far-detail/` in the probe's
checkout.
