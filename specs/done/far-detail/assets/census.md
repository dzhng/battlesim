# Census of the wide view

> Recorded while the work was under way. Slice numbers name its build order,
> and forecasts here were settled later: see [what shipped](../README.md).

Which prop appearances cost the most at the wide hold today, counted.
Recorded 2026-11-15 for [far detail](../README.md) on base
commit 811bb8c27.

## The state counted

The [receipt](baseline.json)'s scene, camera rule and viewport: Metro XL seed 4,
the city-arena-2 contact recipe at 100 units a side, production Low, paused at
tick 90, 1280×720 page (960×540 canvas at the 0.75 render scale), yaw −1.57,
FOV 0.8, near 1 m, wide hold 2,400 m at pitch 0.8860277.

The receipt's digest `60ede87ba60aab0b` no longer reproduces. Today's tick-90
digest is **`ef325ab1cc76c583`**. The input that changed is the generated map
itself: the receipt was taken on generator `layout-19` with presets
`layout-presets-19`; staging is on `layout-22` and `layout-presets-22`
(swamps on 2026-10-28, military depots on 11-13, objective homesteads and
road markers on 11-15), besides the simulation and roster changes since. The
harness places every hold on the densest own soldier, which is now at
**(4864.04, 4851.69)** instead of the receipt's (4918.49, 4247.62). The census
and the ceiling's control both use today's state; nothing here is compared with
the old receipt's numbers.

## How it was read

The renderer's existing counts give each prop appearance's in-view instances
per tier and each pass's submitted triangles. Casts per appearance and per
cascade, how many cascades each model casts into, and projected heights are
not exposed today, so a scratch build (staging plus a read-only diagnostic in
the prop population's selection and the renderer's drawn-instance report,
never in either timing arm) added them at the hold. The scratch build's
selection, instance counts and per-pass submissions are identical to staging's
(the ceiling's count checks compare every batch of both arms with it).

Projected size is the measure every tier already uses: the model's size (the
largest extent of its box, which for a car is its length) times pixels per
metre over the eye's distance to its centre, in canvas pixels. The "Height px"
column below is that size; a 4.4 m car at 2,400 m is about 1.2 px.

## The whole wide frame

Submitted triangles at the wide hold, view pass (the depth prepass and the
colour pass each submit it) and the four cascades together:

| Owner | View pass | Cascades 0–3 together |
| --- | ---: | ---: |
| Models (props are all but 30 of 120,405 instances) | 12,536,110 | 54,436,554 |
| Buildings | 1,782,784 | 9,526,304 |
| Scenery (trees and dressing) | 1,768,258 | 9,404,442 |
| Terrain | 10 | 2,871 |

Every prop in view draws tier 3. Props carry about three quarters of all
submitted triangles, and their casts carry more than twice the view pass's two
draws together.

## Per appearance

Ranked by triangles submitted, view (counted twice, depth and colour) and casts
together. "Casters" counts distinct models that cast into at least one
cascade, with those outside the view in brackets; "cascades" is the mean
number each caster is drawn into; "skipped" counts visible models whose shadow
the tiny-prop rule (`shadow_caster_px`) dropped. Heights are the 5th, 50th and
95th centile in canvas pixels. All in-view instances are tier 3.

| # | Appearance | In view | Tier-3 tris | View tris | Casters (off-view) | Cascades | Cast submissions c0/c1/c2/c3 | Cast tris | Height px p5/p50/p95 | Skipped | Share |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | --- | ---: | ---: |
| 1 | parked_car | 28,660 | 172 | 4,929,520 | 39,446 (10,861) | 3.60 | 29,812/34,693/37,868/39,446 | 24,392,868 | 0.77/1.03/1.39 | 0 | 43.2% |
| 2 | courtyard_wall | 51,990 | 88 | 4,575,120 | 64,747 (15,585) | 3.68 | 53,791/57,550/62,175/64,747 | 20,967,144 | 0.51/0.66/0.85 | 2,740 | 37.9% |
| 3 | garden_fence | 6,663 | 60 | 399,780 | 10,692 (4,064) | 3.34 | 6,359/8,649/10,023/10,692 | 2,143,380 | 0.91/1.01/1.75 | 0 | 3.7% |
| 4 | bike_rack | 2,997 | 144 | 431,568 | 2,277 (533) | 3.86 | 2,099/2,152/2,257/2,277 | 1,265,040 | 0.57/0.71/0.90 | 1,249 | 2.7% |
| 5 | planter | 5,550 | 112 | 621,600 | 1,569 (1,569) | 3.28 | 1,067/1,140/1,368/1,569 | 576,128 | 0.33/0.42/0.52 | 5,532 | 2.3% |
| 6 | lamp | 2,884 | 80 | 230,720 | 4,283 (1,413) | 3.52 | 3,086/3,665/4,043/4,283 | 1,206,160 | 1.00/1.33/1.84 | 0 | 2.1% |
| 7 | hedge | 6,209 | 18 | 111,762 | 10,089 (3,909) | 3.40 | 6,243/8,379/9,552/10,089 | 616,734 | 0.98/1.10/1.87 | 0 | 1.1% |
| 8 | bins_china | 2,483 | 96 | 238,368 | 1,103 (1,103) | 3.00 | 589/709/904/1,103 | 317,280 | 0.24/0.32/0.42 | 2,478 | 1.0% |
| 9 | garden_table | 1,131 | 156 | 176,436 | 811 (747) | 3.40 | 525/701/724/811 | 430,716 | 0.33/0.37/0.63 | 1,059 | 1.0% |
| 10 | garden_shed | 1,207 | 76 | 91,732 | 1,741 (786) | 3.48 | 1,176/1,514/1,630/1,741 | 460,636 | 0.39/0.43/0.75 | 247 | 0.8% |
| 11 | laundry_poles | 1,361 | 60 | 81,660 | 1,713 (353) | 3.73 | 1,456/1,574/1,650/1,713 | 383,580 | 1.13/1.43/1.75 | 0 | 0.7% |
| 12 | bench_china | 2,480 | 72 | 178,560 | 709 (709) | 3.07 | 423/459/588/709 | 156,888 | 0.36/0.46/0.57 | 2,476 | 0.6% |

Eighteen more appearances share the remaining 2.9%, none above 0.5%
(outdoor_gym, washing_line, hydrant, scooter, pingpong_table, playground_frame,
car_wreck, bike_shed, utility_box, swing, dumpster, bollard, heras_fence and
five with fewer than a dozen in view).

## What it says

- **Two kinds are four fifths of the prop work.** Parked cars and courtyard
  walls together are 81% of the triangles props submit at the wide hold. The
  next kind, garden fences, is 3.7%. Slice 03 is the parked car; slice 04's
  first kind is the courtyard wall, and nothing after it is worth a far mesh on
  triangle count alone.
- **Casts dominate.** For both kinds the four cascades submit about five times
  the triangles of one view pass, because nearly every one casts into three or
  four cascades.
- **The tiny-prop shadow skip barely touches them.** Cars stand about one pixel
  tall, yet no visible car skips its shadow: the skip judges a conservative
  footprint of the model's horizontal bound, and a 4.4 m car's bound clears
  `shadow_caster_px` (2) at this distance. Walls skip 2,740 of 51,990. The
  small kinds (planters, bins, benches, hydrants, bollards) skip almost all of
  theirs, which is why they cast only from outside the view.
- **Off-view casters are a quarter of the car and wall casts.** Of the 141,819
  car cast submissions, 35,054 come from the 10,861 cars outside the view; of
  the walls' 238,263, 47,025 come from off-view walls. In the finished design
  (as first built; [the shadow rule](../decisions.md) later removed those casts) those keep casting tier 3, so they are not
  part of the far mesh's saving.
- **At close and tactical range nothing changes.** At 65 m the nearest props
  are 13 to 58 px tall and draw tier 2 or 3; at 260 m cars are 6 to 14 px and
  walls 4 to 8 px, all tier 3. Both lie far above any plausible far threshold.
