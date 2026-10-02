# C23: far tier

**Depends on:** C22. **Kind:** slice.

## Question
Does the far tier keep each building's massing and roofline, with no visible pop?

## Contract it unlocks
A pure deterministic far-tier builder from reusable descriptor geometry/roof recipes, assembled at load/runtime through C22's bounded chunks. It needs no per-map Blender bake. Tile sizes and any very-far aggregation follow S3/G0's full-overview verdict; 256 m is an initial experiment, not a fixed full-map allocation. The tiles draw **through C22's static-chunk owner** (batching and far shadow casters follow S3/G0's measured limits), not a second draw path. The graphs' own LOW tiers are **not** the far tier (they're 47–86k triangles).

## API seam
`packages/battle-renderer/src/city/farTier.ts` (mesh builder only), C22's chunks.

## What the human can run or see
A fixed camera-distance sequence from the near/far transition through the full Large overview, with matched runtime/saved inputs.

## Verification
- Per-tier triangle/instance budgets from S3/G0; the earlier ~300 triangles per building is only an initial far-tier target and cannot substitute for whole-map overview cost.
- Strategic-window GPU and buffer bytes.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**silhouette and roofline across the transition**) against **C22's shot at the transition distance**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Materials, glass, interiors.


## Delegated to the implementer
Tile size; roof detail; transition distance within G0's radius. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Near hero density fixed.

## Feedback that would change this slice
Distant buildings that pop or lose town character change far-tier thresholds/representation within G0 budgets.

## Outcome

**2026-10-02: tier 3 is the far tier. No tile builder, no fifth representation.** The far tier this slice asks for already exists as the coarsest tier of every template, drawn as one static population for the whole map ([C22](C22-placement-chunks.md#outcome)). It keeps each building's massing and roofline across every transition, it costs at most 4.1 ms of GPU at any camera on a Metro Large, and from 1,064 m a town drawn at tier 2 and the same town drawn at tier 3 are hard to tell apart. What does pop is art inside the coarse tiers of the five sets, listed below for their owners; and what the whole-map overview lacks is not a building tier at all.

**The tool.** `/lab/city-lineup` stands every template of the installed library on flat ground, in rows by category, through the frame's own buildings path (`apps/battle-lab/src/cityLineup.ts`, `routes/cityLineup.tsx`). Its address chooses a set or category, a forced tier, a state and a station; the stations include one at each tier boundary, so the tier before and the tier after are drawn from one pose. `scene -- city-lineup` holds every template at every tier to its contract on screen (drawn; nothing outside its parts grown by its set's fit; every part filled and meeting the ground) and writes the sheets. `/lab/city-block` gained `?tier=`, a 2 km oblique station, and `CITY_SEQUENCE=1` (the distance sequence, and the town at each boundary at both tiers). Both labs force a tier through the building style's own thresholds, so the forced picture is the renderer's, not a second path's.

**Measured**, on Metro Large seed 1 (9,853 buildings, 21,346 tier 3 records in 3,809 chunks), 1920 x 1080, development build, Apple Metal. The buildings' GPU time is the median of six paired differences (120 forced frames with them, 120 without, interleaved): `BUILDING_COST=1 CITY_MAP=metro:large:1 scene -- city-block`. Two runs; the table is the second, with the machine's load average at 14. The first, at a load of 99, read the street view at 4.0 to 12.9 ms and agrees elsewhere.

| Camera | Buildings, GPU (range of six) | Whole frame, GPU | Instances at tiers 0 / 1 / 2 / 3 | Triangles at tiers 0 / 1 / 2 / 3 | Draws a pass, and triangles a cascade |
|---|---|---|---|---|---|
| Street, 25 m, along the street | 4.07 ms (3.93 to 4.24) | 7.95 ms | 3,962 / 5,192 / 178 / 10,598 | 581k / 404k / 593k / 1,509k | 192, 1,802k |
| Tactical, 65 m | 0.39 ms (0.01 to 0.51) | 2.97 ms | 1,707 / 0 / 0 / 5 | 354k / 0 / 0 / 4k | 89, 131k |
| 250 m | 1.10 ms (0.95 to 1.24) | 2.86 ms | 0 / 5,220 / 20 / 178 | 0 / 396k / 76k / 129k | 96, 268k |
| Oblique, 2 km at 0.3 rad | 4.06 ms (3.58 to 4.75) | 7.85 ms | 0 / 0 / 1,861 / 20,359 | 0 / 0 / 137k / 2,313k | 59, 1,828k |
| Whole map, 15 km | 1.90 ms (0.72 to 2.42) | 11.00 ms | 0 / 0 / 0 / 21,346 | 2,353k | 31, no caster |

- **Budget.** The static city is under 15 ms of GPU at every camera: 11.0 ms at the worst, the whole map, where the buildings are 1.9 ms of it and the rest is the forest's, as in S3. Tier 3 costs most in the two views with a horizon, about 4 ms.
- **Triangles.** The whole map draws 2.35 million tier 3 triangles, 239 a building, against the guide of 300. A template's tier 3 is 12 to 46 triangles for a house, 36 to 74 for a farm, 56 to 224 for industry, 218 to 396 for a tower and 656 to 1,766 for an apartment block (`throwaway/evidence/city-lineup/costs.json`).
- **Bytes.** The tier 3 population's record buffer is 1.30 MiB for the map; the pool is 8.0 MiB whatever the map; all buffers 121.9 MiB, textures 397.1 MiB. A merged tile a chunk would add vertex bytes for every building to save draws the view does not have: 31 at the whole map.
- **Thresholds.** `lod_px_per_m` stays `[10, 4, 1.2]` (128 m, 319 m and 1,064 m at 1080 pixels). No boundary is in the wrong place: each pop below is a difference between the art on its two sides, which moving the boundary would only move. Starting tier 0 nearer, so that the China kit's centimetre bars stop drawing as stipple at the tactical camera, was weighed and refused ([choices](../choices.md#c23-far-tier)).

**Pictures** (all under `throwaway/evidence/`, after `scene -- city-lineup` and `CITY_MAP=metro:large:1 CITY_SEQUENCE=1 scene -- city-block`; the second set was copied to `city-block-metro-large/`):
- `city-lineup/sheet-<category>-tier<0..3>.png`: each template alone at each tier. `city-lineup/row-<category>-tier<0..3>.png`: the labelled rows.
- `city-lineup/pairs-transition-<1..3>-<category>.png`, and `pair-transition-<n>-<template>.png`: every template across each boundary, from the boundary's distance.
- `city-block-metro-large/sequence-<metres>m.png` and `sequence-overview.png`: the town from 65 m to the 15 km overview. `boundary-<1..3>-pair.png`: the town at a boundary, at the tier before and after.
- `city-block-metro-large/cost-oblique-1920x1080.png`: the 2 km oblique view.

**Compared** (`compare-screenshots`' helper over each template's two crops at a boundary; its distances are telemetry). Nothing moves far: the largest distance is 0.13 of 1. Tier 0 to 1 moves the four towers most (0.06 to 0.12; everything else under 0.04). Tier 1 to 2 moves the apartment slabs, the slab tower and the 16-floor tower (0.08 to 0.09). Tier 2 to 3 moves the shop row, the two warehouses and the corner shop (0.10 to 0.13), where edge density halves: their openings vanish. The whole town at a boundary moves 0.014, 0.040 and 0.038.

**Critique** (unprimed, on all 18 pair sheets and the town frames). Silhouette, roofline and roof colour hold in every pair; nothing floats, sinks or shows a missing face. What it found is the list below, and these, which are other owners':
- **The whole-map overview does not read as a town.** At 15 km a metre is 0.085 pixels: a house is under a pixel and a 47 m slab is 4 by 1, so roofs are pink dust and only the road grid reads (on a Mixed Small, at 9 km, a house is 1.4 pixels). At 4.5 km the town is still recognised by its roads and its lawn, not its buildings: most of a suburb's ground is the same green as a field, and its brown-roofed houses are specks on it. No building tier fixes either. Opaque geometry cannot hold a minimum size on screen; what reads from that height is the ground a town stands on, so the fix is a built-up ground tint under settlements, which is the ground lane's (C28 and after). The buildings stop casting between 3 and 4.5 km; at these sizes a shadow would not carry the town either.
- **Flat lawn facets, field lines running under buildings, streets that end in grass:** the map's and the ground lane's. **A hard fog line at the horizon in the oblique view, radial haze at the overview:** the fog look's (C20). **Red tile on every flat apartment roof:** the China kit's material, as C22 recorded.

### What the coarse tiers must keep, per set

For the set owners; this slice edited no script. The pictures named are under `throwaway/evidence/city-lineup/`.

**`towers`** (`pairs-transition-1-highrise.png`, `pairs-transition-2-highrise.png`):
- *Tier 1, all four towers.* The shell's window texture is pale pink-grey and tan where tier 0's glass is dark navy, so the whole facade lightens at 128 m: the nearest and largest pop in the game. Tier 1's windows take tier 0's glass colour, and its share of the wall.
- *Tier 1, `china-tower-16f`.* The balcony stacks gain white bands tier 0 does not have; tier 2 turns them uniform teal. One pattern for the stacks at all four tiers.
- *Tier 1, `china-tower-slab-10f`.* Scattered balcony colours become clean vertical stripes; tier 2 makes them blue and yellow columns. Keep tier 0's mix, or make tier 0 the stripes.
- *Tier 2, all four.* The roof hut takes the wall's tint (yellow on `china-tower-12f`) where tiers 0 and 1 have a white concrete hut with a door, and the roof vents vanish. The hut keeps its own colour to tier 3; the vents may go.
- Tier 2 to 3 is clean.

**`china_apartments`** (`pairs-transition-2-urban_apartment.png`, `pairs-transition-3-urban_apartment.png`):
- *Tier 2, every block; worst on `china-apartment-point-20x20-7f`, then the U, the court and the two 14 m slabs.* Balconies are solid bright cream slabs where tier 1 has dark recesses with laundry and tier 3 has dark window speckle: the facade goes dark, light, dark as the camera pulls out. Tier 2's balcony stacks take the dark of the recess behind them, as tier 3's do.
- *Tier 2.* The ground-floor shop strip loses its coloured signs and awnings to a dark band, and tier 3 gives a coloured strip back. One coloured strip at both.
- *Tier 2, `china-apartment-point-20x20-7f`.* Dark diagonal smudges on the wall beside the balcony columns.
- *Tier 3.* Roof tanks and clutter vanish to a clean roof: accepted at 1,064 m.
- *Tier 0, for C24.* Window cages, balcony rails and the solar racks are bars a centimetre or two thick. At the tactical camera a metre is 19.7 pixels, so a 2 cm bar covers 0.4 of one and draws as stipple (`crops/tactical-tier0/china-apartment-slab-35x11-4f.png` against the same file under `crops/tactical-tier1/`). A bar that thin covers a pixel only inside 25 m, the camera's nearest: it belongs in a cutout texture with mips, not in geometry at any tier.

**`homes`** (`pairs-transition-2-detached_home.png`, `pairs-transition-2-attached_home.png`, `pairs-transition-3-detached_home.png`, `pairs-transition-3-attached_home.png`):
- *Tier 3, all ten.* The shell (12 to 46 triangles) has blank walls: every window and door vanishes at 1,064 m, where a window is still a pixel or two, and the shop row and the corner shop lose their dark shopfront band, so the wall brightens. Tier 3 keeps the openings as wall texture or vertex colour, on tier 2's grid; the budget has room for them as quads.
- *Tier 2, `china-home-10x8-1f`.* Four shuttered windows become two bare ones. *`china-home-ell-2f`:* about six windows a floor become three, and the wall turns a darker olive. *`china-terrace-5x3f`:* fewer window columns a house. Tier 2 keeps tier 1's window count and wall colour.
- *Tier 2, all.* White frames, sills and lintels vanish, and chimneys turn from grey to roof orange. Chimneys keep their colour; a pale surround would keep the windows' weight.
- *Tier 2, `china-corner-shop-3f`.* The shop door goes and the awning band thickens.

**`farmsteads`** (`pairs-transition-2-farmstead.png`, `pairs-transition-3-farmstead.png`):
- *Tier 3, all three.* Blank walls, as the homes: windows, the barns' dark doors and the red barn door of `china-farmstead-small` vanish, and the chimneys go. Keep the openings as texture or vertex colour, the barn doors above all: they are the largest dark shape on the building.
- *Tier 2, `china-farmstead-yard`.* The house's dense shuttered bands become four plain windows a floor and its wall goes from cream to grey-olive. *`china-farmstead-long`:* grouped windows become five evenly spaced. Tier 2 keeps tier 1's grouping and wall colour.

**`industry`** (`pairs-transition-2-industry.png`, `pairs-transition-3-industry.png`):
- *Tier 3, `china-warehouse-72x33`.* The roof reads as four bays at tier 2 and two at tier 3, and whiter. Keep four.
- *Tier 3, `china-warehouse-48x24`.* The pale parapet rim and every vent vanish, and the roof goes flat and darker. Keep the rim.
- *Tier 3, `china-works-54x36`.* The office loses its windows and roof furniture, and the hall's window row becomes a flat, brighter red band. Keep the office's windows and the band's darkness.
- *Tier 3, `china-shed-15x24`.* The rust patches and the ridge line vanish.
- *Tier 2, `china-warehouse-48x24`.* The row of dark dock bays becomes a flat pale canopy band. Keep the bays dark.
- `china-depot-90x39` is clean at every boundary.

**Checked.** Unit tests of where a template stands, the references handed to the frame, a forced tier, the station at a boundary against the renderer's own selection, and the framing to fit (`web/tests/cityLineup.test.ts`; the layout, the forced tier and the station each falsified by a mutation). `scene -- city-lineup`: 29 templates at four tiers, 116 views, every contract check passing, no validation warning. `scene -- city-block`: its checks unchanged and passing. The decisions are in [choices](../choices.md#c23-far-tier).

**Not done.** `packages/battle-renderer/src/city/farTier.ts` was not written: the contract it would unlock is met by the templates' own tier 3. The visual checkpoint with the user was not held (the pass ran unattended); the pictures above are what it would show.
