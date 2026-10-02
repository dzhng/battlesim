# C70: river bank bands

**Depends on:** C69. **Kind:** slice.

## Question
Does the river meet the ground through soft bed, wet-bank, mud and grass bands, with no hard edge and no dark shore?

## Contract it unlocks
- Bands by signed distance with a one-sided noisy edge (Selo Empire's shore distance-field technique, as a technique entry).
- The waterline meets the carved bank at the exact half-width line, with no quad overhang.
- Wet bank and mud stay **at or above grass luminance**, differing by hue.
- Grass thins across the mud band.

## API seam
`terrainMaterial.ts` (`groundShore`, `groundWater`), `worldPass` water.

## What the human can run or see
`/lab/river` at `bend-25/65`.

## Verification
- Luminance per band ≥ grass.
- The `shadow_floor` scene check.
- No depth-fight flicker.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the water-to-grass band (0 < SD < 8 m) only; critique leads with the shadow question**) against **`../assets/reference/ground/river-108.jpg`, `river-112.jpg`, `manor-ground-closeup.jpg` and C69's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Bank facet roundness.


## Delegated to the implementer
Band widths, palettes, noise. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C69's geometry and agreement.

## Feedback that would change this slice
A river bank that reads as a cliff changes the bank-band profile; shoreline roundness is judged in C71.

## Outcome

**Changed since.** The bank was recoloured three times, at integration and at the gate (`955f5eaa`, `a51341d6`, `fb4a241f`; [C87](C87-ground-composition-gate.md)): `shore.lift` is 1.27, and the palette is a grey silt and a pale grey-brown earth, not tan.

**The river meets the ground through a thin clear rim, wet silt, bare earth and thinning grass, none of it darker than the field beside it, and the water no longer reads as a road. Fresh eyes still call the bank a flat stripe and are convinced by the water only close and low.** The geometry, the rule and C69's agreement are untouched.

**The seam.**
- **Biome** (`fixtures/biomes/summer.json`, validated by `validateBiome`). `shore` was `{ palette, width_m }` and is `{ palette, wet_m, mud_m, wander, wander_scale_m, lift, relief }`, its palette two colours (wet silt, bare earth). A new `water` row holds the surface's look: `{ opacity: [edge, channel], shallows_m, streak }`; the `water` palette is two colours (channel, edge).
- **Terrain material.** `groundShore(xy, footprint, water)` returns `(wet, earth, bare)` (it was `groundShore(water)`, one weight): how much the silt and the earth cover the ground, and how bare of grass it is. The ground's colour paints both at least `lift` times as light as the field they lie on; the grass build reads `bare`. `groundBank` shades by `relief` of the bank's slope. `waterSurface` keeps its signature. `groundReach`'s water term reads `mud_m`.
- **Rig.** The lab has `meander-low-90`; a generated map with a river has `river-250` and `river-65`, found where the water crosses the map's middle latitude. `STATIONS=river:bend-65+wide-65` shoots named stations alone.

**What is drawn.**
- Wet silt for 0.6 m from the waterline, round as the river. Bare earth behind it to 5 m at most, its outer line wandering in by up to half of that on two noise octaves turned from the map's axes. Grass: none on the silt, thickening across the earth.
- Both bands are the palette's colour scaled up to 1.15 times the field's own luminance where they would be darker, the field's eased to the verge's over 10 m toward a plot's edge so two fields' banks meet in one tone.
- A bank is shaded at half its slope: at a low sun the dark smear C69's critique named is gone, and so is most of the relief.
- Water: teal in the channel, clearing to the bed over 0.35 m at its edge; streaks of light in lanes that hold their distance from the bank and so run with the channel round every bend.

**Measured** (river lab, 1920×1080; the `river` scene holds each as a check).

| Rule | Before | After |
|---|---|---|
| Wet bank against the grass on the same section, median per bank (≥ 1) | 0.58 and 0.73 on two of the four banks; 1.18, 1.40 on the others | 1.02 and 1.12 on those two; 1.42, 1.50 |
| Bare earth against the grass (≥ 1) | 0.76 and 0.81; 1.15, 1.29 | 1.05 and 1.11; 1.49, 1.57 |
| Hue apart from the grass, CIELAB a*b* (> 6) | 11 to 31 | 20 to 27 |
| Grass clumps a metre of band: wet, near half of the earth, far half, field | 0, 632, 1,587, 1,506 | 0, 126, 687, 1,499 |
| Shallows against the channel, one bank over the other (within 1.25) | not measured | 0.99 at both stations |
| The class mask's water distance against the export, 600 pixels within 8 m | 0 off | 0 off |
| Waterline pixels that flicker over six frames a centimetre apart | 0 of 65,952 | 0 of 65,952 |

The base was sampled where the first candidate's bands lay (0.6 and 1.6 m from the water; the earth from 1.2 to 2.7 m, in halves); the final look where its own lie (0.3 and 1.1 m; the earth from 0.6 to 2.5 m).

Station shots differ from the base in 11% to 49% of their pixels wherever the river is in frame; the village's nine stations without grass are byte-identical, so the `fog-look` check was not rerun. The reach did not widen: the bank's shading already read 12 m from the water.

**Against the references.** By distance from the water (`bend-65`, both banks pooled): the bank's first 2.5 m is 1.02 to 1.06 times the grass, tan (119, 99, 62); it was 0.70 to 0.86, brown. The references' banks are 0.67 and 0.96 of their grass and their water 0.08 of it: the dark creek the fog rule forbids here. Ours is 0.50 to 0.56, teal where it was 0.39, grey. Less wrong than the base on every row but the references' darkness, by decision.

**Frame cost: not resolved.** One paired run (this tree against the base in a scratch copy, interleaved in 1.5 s batches, five rounds, two framings) read +0.94 ms at the play camera and +0.38 ms top-down. In the same run this tree with one normal switched off read +1.41 and −0.56 ms against the base, so the spread between trees is as wide as the differences; the machine stood at a load average of 50 to 120 and each batch drew 17 frames against the timer's 240-frame window. What changed per pixel: two noise lookups on ground within 5 m of water (none elsewhere), three on the water's surface, no new loop and no wider reach. The row is owed, on a quieter machine with batches longer than the timer's window.

**What fresh eyes found.** Two unprimed critiques, each of fourteen frames and seven 3× crops of the lab and a generated map (250 m, 65 m, 25 m, low pitch, a low sun), the second after the first's fixes. The second saw the committed look at the stations and, for the lab's top-down, close and low-sun frames, the same look a step earlier (`lift` 1.1, the earth a shade greyer).
- *Dark as shadow, or haze:* on the first look, the wide pale shallows read as haze and the river as "a convex tube"; that band is a thin rim now and neither came back. Still named, at medium confidence: a dark rim just inside the far waterline and a pale one inside the near, which read as an embossed kerb (probably the bank's shading, which runs on under the first 0.75 m of water; not confirmed); under a low sun, half of one reach darker and without streaks; and the earth's soft outer fade over a ploughed field, which has no grass to break it and reads as dust lying on the field.
- *Fixed after the first:* straight seams and dark wedges across the bank where plots meet it (the floor stepped with the plot; it is eased now, and the second critique saw no seam cross the bank); flecks streaked one way across every bend (the light lies in lanes along the stream now); an orange bank that outshouted the water (not named again).
- *Not fixed:* at 250 m the earth's outer line is still a chain of regular lobes with near-straight runs between them, high confidence both times: turning the noise's lattice off the map's axes and dropping its clamp did not cure it. The bank's apparent width still jumps where a field boundary meets it, because how much grass hides the earth changes with the crop. The bank reads as a flat tan stripe of one width, with no slope cue even under a low sun (high confidence): at half its slope's shading the two banks are lit alike. At 250 m the lanes of light read as "lane markings on a running track", and looking straight down from 25 m as paint smears; only the low, close view convinced as water. Under a low sun the water is dark and matte with no glint, and its lanes show a fine cross-hatch from straight above.

**Open.**
- The frame-cost row.
- Bank art: a bank whose width and form differ between the inside and outside of a bend, a broken turf lip, stones; a line for the earth's edge that is not value noise.
- A slope cue on the banks that survives the luminance rule: more `relief` needs more `lift`.
- The water does not move, has no glint from above, and at 250 m its streaks are too regular.
- The river scene ran whole once on the look before the last two biome numbers changed (`lift` 1.1 to 1.15, the earth's colour): eleven checks green, the luminance check red at 0.99 on one bank. After the change its five water's-edge checks ran green; the other seven read nothing those numbers move and were not rerun.
- A river's constant width and its straight reaches are the simulation's shape. Fresh eyes read a straight reach as a canal; changing that is the map's.
- Bridge abutments (out of scope here): the bank and the water read coplanar under the bridge's shadow.
