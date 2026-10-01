# C69: rivers contract

**Depends on:** C65, C60, SG2. **Kind:** slice.

## Question
Is water a round river that the sim and shading read from one distance, impassable and crossed at bridges (Q-G5)?

## Contract it unlocks
- `MapDefinition.rivers: Vec<River { points: Vec<{ xy, width_m, depth_m }>, surface_z }>`, densified by C65's loader. **`water` is deleted on every map.**
- `HeightField::build` carves a gentle bank from distance, with a ramp at bridge ends.
- The sim classifies water by distance through C03's `SurfaceIndex`, replacing the rect scans at `world/mod.rs:213, 221`.
- Validation rejects a point under 12 m wide, a surface above its bank, or a bank steeper than GG's slope.
- The export carries river segments, C63 gains the water channel, and the water surface becomes a ribbon along the centerline (`worldMesh.ts:136-172` rects go). There's no new look yet.
- Plots cut along river control runs.
- A new catalogued map, `river` (lab), with a meander, a bridge, the 12 m minimum, a 30 m section, a country road, a dirt track and stations.
- **Named lab changes:** the `geometry` and `movement` labs' rects become straight rivers. The village has no water.

## API seam
`contract::map`, `sim::world::{mod.rs, terrain.rs}`, `world/export.rs`, `worldMesh.ts`.

## What the human can run or see
`/lab/river`: a nav PNG and a GIF of a squad and a tank routing over the bridge and never entering the water.

## Verification
- Run tweak-mechanics first.
- Tests: exact edge classification; bank slope under `slope_cutoff_deg` outside the water; the bridge ramp stays traversable; validation refusals; native equals wasm.
- Named lab digests; village digests identical.

## Delegated to the implementer
Bank profile; ramp length; per-river `surface_z`. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village; bridge deck behaviour.

## Feedback that would change this slice
A river/bridge that blocks an intended route reopens the shared physical contract before bank appearance work.

## Outcome

**The physical half is done; the look waits for the specialist, with one thing owed to movement.** Water is `MapDefinition.rivers`, rounded by C65's centreline. One number, how far a point lies inside the water's edge, is the rule in the simulation, the cut in the terrain and the edge in the drawing. `water` is gone from the schema and from every map, fixture and test.

**The seam.**
- **Map:** `rivers: [{ points: [{ xy, width_m, depth_m }], surface_z }]`. A river's line is rounded like a road's, never straying more than half its narrowest water from the authored runs; width and depth run evenly along it. A river that stops inside the map ends in a round cap.
- **Refusals at load** (`contract::river::validate`, called by the world, the map resolver and the compiler): a point narrower than three height samples (12 m on the 4 m grid); a point so deep for its width that its bank would pass the slope cutoff; a surface above the land at the water's edge, checked every height sample along both edges; a bridge whose deck ends over the water, or so near it that the steepest bank the grid can draw still cuts the ground at the deck's end below the land and the deck.
- **World export:** `rivers()` (11 floats a stretch: both ends, the half width, the bank's grade and the bank's height at each, the surface height) and `river_runs()` (the long runs fields are cut along); `water()` is gone. The layout names both (`riverFields`, `riverRunFields`).
- **Surface field:** a river stretch is a stroke in the water list whose half width runs from one end to the other, followed by a record of its bank. The rect records for water are gone.
- **Compiler:** `MapPlan.rivers` passes into the map. Authored points must lie in the playable rectangle, the rounded samples count toward `max_ground_points`, and a river the terrain cannot carry is refused as `invalid_river`.
- Nothing in the observation or the publication changed.

**The terrain.** The cross-section is a V: the bed falls from the waterline to `depth_m` at the middle, and the bank climbs away at the same grade (depth over half the width) until it has made up the height the land stands above the water at the edge. So the ground meets the water's surface exactly on the rule's edge, and land that rises beside a river keeps its shape. Along a bridge's approach the section takes the steepest grade a mover climbs, easing back to the river's own beside it, so a deck is stepped onto from the land's height. Storage stays sparse: an 8 km river corner to corner stores 376 of the map's 15,625 height pages, and its 177 navigation regions cover 2.4% of the map.

**The rule and its neighbours.**
- Ground movers never stand on water. Ordered into it, a squad near a bank is sent to the bank (its goal moves to the nearest ground it can stand on); a vehicle, or a squad far from any bank, reports its route blocked and stays, as for any goal that cannot be reached.
- Water is neither road nor forest whatever is authored over it, and no trunk stands in it or within the forest rule's clearance of it.
- A round stops at the water's surface, where it used to fly on to the bed: a shell bursts on the river. Water still takes no crater or scorch.
- Sight crosses water as open ground. A body shoved along a deck stays on the deck, and pushed off its side it drops to the bed.

**Measured.**
- Tests: 9 contract (rounding, linear width, caps, refusals) and 1 on the map resolver, 13 simulation (exact edge, index against the all-stretches distance over 139,000 points, the V, every dry point of the lab standable, the ramp, forest and road, rounds, shoving, sparse storage, routes over the deck for a squad and a tank), 2 compiler, 4 web (export layout, the water's squares, fields cut along the runs), and the surface field's unit and GPU checks with river records. Workspace: 413 simulation tests and 532 web tests pass; the geometry, movement, ground, river and village scenes pass (166 checks). Rounded river samples are the same bits native and Wasm (`fixtures/parity/ground/curve-strokes.json`); the compiler's native and Wasm outcomes match on six river cases.
- Village: the six `village_report --quick` digests are unchanged (`5310636ce8486c99 cd75b63bc5473abb 8f42b246877e43f1 f589556c8dc29dc8 73f49a9457da3154 b582f8772eea46d3`), at 5,140 G instructions against 5,145 G. Sixteen terrain-only captures at eight cameras differ in 0 pixels from the slice's base.
- Named lab changes: the geometry and movement labs' rects are straight rivers (24 m and 20 m wide, 2 m deep, the surface half a metre under the land), so their terrain, their fog oracle's digests and the terrain query oracle moved.

**SG2's verdict: a river on the 4 m grid reads round once its bank is shaded from the cross-section; lit by the grid's own triangles it reads stepped.** Measured in the `river` lab at the top-down 250 m and play 65 m cameras:

| Question | Bar | Measured | Verdict |
|---|---|---|---|
| Waterline against the exact edge | 0.5 m | 0 on straight water (the V is one plane through the waterline); at most 0.25 m anywhere in the lab, sampled every half metre, and that a centimetre high beside the bridge's ramp | pass |
| Largest step between neighbouring bank triangles, lit by their own normals | 8% of the flat ground's luminance | 20.6% top-down, 18.9% at the play camera; 237 and 63 pairs over the bar (first lab river, twelve long runs) | **kill** |
| Fallback 1 (the edge cut by distance, the wet band over it) | reads round | both were already in those frames: the waterline and the band are round, but the triangles still show on the bed through the water and on the bank past the 2 m band | not enough |
| Fallback 2 (a shading normal from the cross-section, in the bank band) | 8% | 2.1% top-down, 1.4% at the play camera, walking each bank a metre at a time at 1, 2.5, 4 and 6 m from the water (the same walk lit by the triangles: 18.5% and 18.1%) | pass |

Fallbacks 3 and 4 were not needed. The scene holds the last row as a check. The bank's shading is `groundBank` in the terrain material: it reads each nearby stretch's slope, blends them by how far inside each puts the point (the nearest alone turns the bank's face at a stroke on the inside of a bend), lights the bed flat, and takes over from the triangle's normal as far past the bank's top as a triangle reaches. The ground's height and the water's edge are untouched.

**What fresh eyes found.** Four unprimed critiques of the lab's frames (top-down, the play camera, 25 m, grazing, and a low sun), one after each round of fixes, each asked whether any river or bank read blocky or stepped and whether any dark region could read as shadow, or shadow as fog.
- *On the final frames:* nothing on the river reads blocky, faceted, stepped, scalloped or sawtoothed: the water's edge and the wet band (high confidence) and the bank's shading (medium-high) are smooth at every distance, with no gaps, no ground through the water and nothing floating.
- *What the earlier rounds caught, each fixed:* corners in the waterline of a meander authored every 9 m (the lab is authored every 4 m now, as a generator would write a river); a scalloped outer edge on the wet band, which was its old ragged line, noise on a square lattice (the band now follows the water's edge and nothing else); a pale half to the wide water, which was the bed lit as a V through the water (the bed is lit flat).
- *Dark as shadow:* at a low sun the bank that faces away is a soft dark band on the grass with nothing above it to cast it, and it reads as a smudge because the bank shows no relief. It is the bank's own shading, and it will read as a slope only when the bank looks like one (C70). A dark field with a straight edge was also taken for a shadow or an unseen zone; that is a plot's colour, not the river's.
- *Left for the look:* from straight above the water is matte and the wet band a plain brown line, so the river reads as a road with kerbs; ripples and glints show on one reach only; the pale shallows are wider on the near bank than the far and read a little as haze; a few fields taper to slivers against the bank; the deck has no abutments.

**Owed.**
- **Movement (not this slice's files).** Soldiers do not sidestep ground they cannot enter. A squad's files are wider than a 10 m deck, so its outer soldiers jostle at the deck's edges (34 reversals), and when a route comes onto the deck at an angle it runs along the deck's edge, half the files lie over the water and those soldiers stop on the bank for good. A tank on the same route swings a hull corner 1.9 m off the deck as it turns on. The base commit does the same on the old water rects (`throwaway/c69/base-bridge/`). `movement_scenarios` holds both as pending checks (`c69-river-bridge`, `c69-river-around`); the `river` scene records how many of the squad crossed (6 of 8).
- **The visual pass (C70, C71).** Everything under "what fresh eyes found" that was left: a bank that reads as a slope and not as a shadow, the wet band's colour and width with the mud palette at or above the grass's luminance (L-G3), the shallows, the water's own look, and bank art. With relief beside a river the bank's shading is lit as if the land beyond it were flat.
- **Two rivers meeting.** Nothing joins rivers. Where two with different surfaces come within a water square (8 m) of each other, the square is drawn at the first one's height. A confluence needs its own rule.
- **Relief's arithmetic.** `Relief::height_at` uses the standard library's `hypot` and `tan`, as it did before this slice; it now also feeds river validation and the bank's cut. The parity fixtures agree native and Wasm on every map we have; a map on the edge of a refusal could in principle be judged differently on the two.
- **Generation.** No layout writes a river or a bridge yet; `MapPlan` has no bridges.
