# C66: road core

**Depends on:** C65, C62. **Kind:** slice.

## Question
Does each road kind's core read as that kind, not a grey blob?

## Contract it unlocks
Per-kind core palette, mottle and roughness: country road is gravel or asphalt grey; dirt track is packed warm brown. This replaces `palettes.road`. The core's luminance stays at or above the grass's (L-G3).

## API seam
`terrain/biome.ts` road rows, the `terrainMaterial.ts` road term.

## What the human can run or see
`bend-25/65` and the lab's dirt-track stations.

## Verification
- A mask-based numeric check: core luminance ≥ the adjacent grass band's.
- Digests untouched.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**core interior only (SD < −0.5 m)**) against **`../assets/reference/ground/forest-road-summer.jpg`, `manor-path-closeup.jpg` and C65's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Shoulder, ruts, grass at the edge.


## Delegated to the implementer
Palettes and mottle. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C28/C31's city pavement rows.

## Feedback that would change this slice
A road core that disappears at tactical distance changes the core read with width/geometry fixed.

## Outcome

**Changed since.** [C28](C28-pavement.md) gave `road` and `sidewalk` rows of their own, lets a row name the `layer` it is painted at (the contract's order is the fallback), and routed round the last Open item: a town's streets are strokes, and their polygons take one look through `area`. Each map now names its rural roads explicitly ([C64](C64-road-kinds.md#outcome)); appearance reads that kind directly. The two luminance checks were restated when the lane was integrated: [`choices.md`](../choices.md) under C31, "A road is judged against the field beside it".

**Built.** Each paved kind is drawn as its own surface, from the biome's `roads.<kind>` rows (`default` is the country road's gravel; `dirt_track` is packed earth). `palettes.road` and `biome.road` are gone.

**The seam.**
- **The export says the kind.** A paved stretch, triangle and boundary edge's `kind` is its area's own kind, an index into the layout's `surfaceAreaKinds` (road, country_road, dirt_track, sidewalk). `terrain/surfaces.ts` holds the same list and refuses a layout that differs.
- **`groundPaved(xy, cell)`** replaces the one paved distance: how far inside each kind's paving the point lies, by tag. `groundSite` takes it (its `road` is still the deepest), and `groundColour` takes it to paint the kinds one over another in the contract's order, the earlier on top. A track therefore ends at the edge of the road it joins, and its earth is carried `join_m` onto that road, thinning out.
- **A road row:** a palette of two colours (the surface, and the hue its patches wear toward: the patch colour is scaled to the surface's own luminance when packed, so a patch never darkens it), `mottle` and `patch_m`, `grain` and `grain_m`, `feather_m`, `join_m`, `roughness`. The grain is three scales of lumps and a finest scale of stones as crisp specks; each scale fades out on its own as it nears a pixel.

**Measured** (the `ground` scene's road checks, `ROADS_ONLY=1`; displayed luminance):

| Station | Core | Grass 5 to 9 m out |
|---|---|---|
| Village `bend-65` (country road) | 0.243 | 0.123 |
| River lab `track-65` (dirt track) | 0.191 | 0.111 |
| River lab `junction-65` (both) | 0.232 | 0.088 |

- At the junction the track's core is warmer than the road's in one frame: red less blue, over red, is 0.45 against 0.24.
- Core texture (the mean luminance step between pixels three apart, over the core's mean): 0.000 before; 0.029 at 65 m and 0.013 at 250 m after. The forest-road reference reads 0.05.
- Both checks were seen to fail: with the track drawn in the gravel palette, and with the gravel darkened below the grass.
- The class masks are byte-identical before and after; every bare-ground shot differs. Digests untouched (`village::a_replay_matches`).
- Frame cost: measured with the shoulder, in C67's row.

**Critique** (unprimed, two rounds). Round one: the patches read as stains and camouflage and showed the noise's lattice as straight edges; the cooler of the two tones read as shadow beside real building shadows; the close view was blurred; the track met the road in a ruler-straight cut. Fixed: patches are sparse, warm only and cut from turned, warped noise; the grain gained stones and a fade per scale; the join blends. Round two found no lattice, no step and no shadow-like region, and named both kinds correctly. What it still says:
- the road at 65 m is soft next to the grass beside it, and the track is "close to a flat blob": structure across the road is C68's;
- the patches still read as faint stains (they are now a third of their first strength);
- nothing says gravel rather than concrete, and a town street does not read as a street: that is C28's pavement row, which `roads.road` waits for.

**Open.**
- The village's roads are kind `road` in its map and take the default row with every other unlisted kind. When C28 gives `road` a street's look, the village needs `country_road` in its map (the map lane's file).
- A polygon union carries one distance for all its kinds, so a kind change inside a town's paving (street to sidewalk) has no edge to feather: C28 and C29 need one.
