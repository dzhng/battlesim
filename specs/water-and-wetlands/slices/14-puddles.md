# 14: Puddles

**Lane:** wet. **Depends on:** 13. **Visual variable:** the puddle look.
**Status:** done, its look unfinished (2026-10-28).

## Contract

- **Puddles are look only** (decision 29). They are not in the map contract,
  have no dip, and change no digest.
- **Placement is a pure function** of the biome seed and a 14 m cell
  (`ground::puddles::candidate`), so wakes and both clients get the same
  answer on any camera (decision 34). `Ground.puddles` is every cell's,
  walked once. A puddle on a rutted kind lies in the nearer wheel rut.
- **Where they lie:** only on the paved kinds (`water.puddles.on`), where no
  plot grass or dressing grows. A dirt track's centre-strip grass can still
  stand where a puddle reaches it.
- **Look:** mirror on `road` and `paving`, muddy on `country_road` and
  `dirt_track`, judged at the puddle's centre (decision 28), from the
  biome's `water.puddles.looks`.
- **Depth:** puddles don't carve. Each is one water cell over its disc,
  carrying its centre and radius in its vertex colour. The one water
  material reads its depth from its shape, fades it in over its rim, and
  draws its water still.
- `StandingWater.at` includes puddles, so slice 15's wakes and dust see them.

## What changed from the plan

- No puddles on open fields ([choices](../choices.md)).
- The water cell record grew from 5 floats to 8; the recorded browser mesh
  vectors were re-encoded.

## Verification (done)

1. A presentation test: a puddle's look follows the surface under its
   centre, on the river lab with a metalled road added, and on the wetlands
   map, over six seeds, and both looks appear.
2. A presentation test: each cell's puddle, asked for alone and in reverse
   order, is the walk's, and the walk repeats.
3. A presentation test: the standing water holds a puddle, deepest at its
   middle and shoaling to its rim.
4. Battle digests are unchanged: the exact-numbers fixture is too, and the
   simulation is untouched.
5. `compare-screenshots` on the water reel's new puddle shot (a dirt track
   at 22 m) and the frame scene's river and geometry labs, against the run
   before puddles: puddles appear on the tracks and nowhere else. The frame
   baselines (river, geometry) were re-blessed for them.
6. Codex found, and these were fixed: a puddle took the first paved shape
   under it, not the simulation's precedence; a puddle's disc could reach a
   river or a swamp; a puddle read the source's strokes, not those drawn; a
   rut could move a puddle onto another surface; a native test fed the old
   cells; and the scene could skip any shot whose water it lost, not only
   the puddle shot. Tests pin the first two.
7. The frame scene's river still differs by a pixel between runs, on the
   lab's "Tick" label, not the world: slice 18 chases it.
8. Two rounds of unprimed `screenshot-critique`. The first read them as
   mounds or stains on the track's grass strip, with no sky in them. Moving
   them into the ruts, stilling their water and greying the mud left them
   reading as wet mud patches, not yet as reflective water. That is open,
   for slice 18 ([README](../README.md#still-open)). Mirror puddles were not
   filmed: the wetlands map has no metalled road.

## Native

Free: the shared presentation and shader.

## Must stay green

Battle digests and replays, and grass and road crops away from puddles.
