# What was measured after the probe

The numbers behind [far detail](../README.md)'s shipped car and its rejected
candidates. All on the [control scene](control.json), production builds, Low
graphics, paused at tick 90, with instance and triangle counts checked equal
or as expected before any timing was read. Timing is GPU milliseconds, the
median of each batch's holds, in interleaved pairs after one A-against-A pair.
The whole-map view is the same target at the player's farthest zoom, 19,500 m.
Recorded 2026-11-15 and 16. The pictures and per-batch logs were scratch and
were not kept.

## The shipped car: its coarsest tier without its tyres, 92 of 172 triangles

A is the build before it; B carries it at `far_px` 4.

| View | A against A | Pair 1 | Pair 2, and its rerun | Pair 3, and its rerun | Pooled |
| --- | --- | --- | --- | --- | --- |
| Close | 7.23 / 6.69 | +0.8% | +17.3% / −4.2% | −7.2% / −5.4% | 6.98 to 6.80 |
| Tactical | 7.69 / 7.51 | +0.1% | −0.9% / +2.1% | −1.4% / −0.5% | −1.2% |
| Wide | 34.60 / 34.60 | 34.60 to 31.94 | 34.41 to 31.73 / 34.58 to 31.95 | 34.40 to 31.95 / 34.59 to 31.94 | 34.59 to 31.94 |
| Whole map | 52.44 / 52.41 | −3.8% | −4.1% / −3.9% | −3.0% / −3.7% | 1.95 ms lower |

Pairs 2 and 3 ran during a load spike from other work on the machine and were
rerun once; both readings are shown. The +17.3% at close is that spike.

Counts at the wide view: 28,660 cars drawn far, 80 fewer triangles each in
the depth and colour passes, and 107,002 shadow casts with the far mesh.
34,817 casts from cars outside the view keep tier 3. Close and tactical draw
no car far, and their frames are pixel-identical to A.

Pictures at 65, 260, 500, 600, 850, 1,000 and 2,400 m: each stop gave the same
picture going out and coming back. On isolated white, mid-toned and dark cars
at 850 and 1,000 m the far-drawn car's mean colour is within 0.8 of a level of
A's. What differs is single pixels where black tyres stood, about 8 levels
lighter. A reviewer given only the images could not tell the builds apart
without a difference image.

## The first car far mesh: a reshaped hull, 40 triangles

Wide view 34.60 to 29.51 ms in three of three pairs. Rejected on its pictures.
At the 600 m stop a white car's box read rgb(139,145,135) in A and
rgb(120,125,109) in B, and at the wide view a car park read rgb(133,138,120)
in A and rgb(173,179,174) in B, in every load. With the fog of war switched
off both pairs matched to within 3 levels. The shipped mesh reads the same as
A in both boxes.

`far_px` 6 was tried with it: 32 cars at the tactical view drew far and 160
pixels changed there.

## Small props outside the view casting with the far mesh

| Wide view | A | B | B − A |
| --- | --- | --- | --- |
| A against A | 31.91 | 32.00 | +0.09 |
| Pair 1 | 32.17 | 31.65 | −0.53 |
| Pair 2 | 31.95 | 31.46 | −0.49 |
| Pair 3 | 31.93 | 31.48 | −0.45 |

2,785,360 fewer shadow triangles at the wide view (34,817 casts × 80), and 88
cars at the tactical view. 3 MiB more GPU buffers. At the lowest authored sun
the wide frame was byte-identical to a second load of A. At the day sun about
100 mid-frame pixels moved by at most 2 levels, which two loads of A do not
do at those pixels.

## Courtyard walls, 88 triangles at tier 3

| Far mesh | Triangles | Result |
| --- | --- | --- |
| No piers | 40 | Far walls up to 3 levels darker, with fog on or off |
| One pier a module | 64 | Runs within a level; lone modules 1 to 3 levels; about 1,200 pixels a frame darker at regular spacing along walls, and the fog rim's outline changed where a pier had crossed it |
| Unseen faces dropped, both piers | 66 | 28 pixels at 460 m and 26 at 600 m moved by up to 54 levels, the same in every load |
| The same, keeping the 12 faces that tie the nearest depth | 78 | No pixel moved at 300, 460, 600 or 2,400 m; 23 pixels of at most 5 levels in an oblique view |

None was timed. At the shared threshold about 290 of 1,700 walls on screen at
the tactical view draw far.
