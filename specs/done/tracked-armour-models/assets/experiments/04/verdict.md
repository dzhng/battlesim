# 04: painted-seam experiment, verdict sheet

**Verdict: grooves.** Ship painted seams as shallow geometry grooves that the
existing per-vertex occlusion bake darkens, at **2 cm wide × 2 cm deep**. The
per-model baked normal+AO map did not clearly win. An unprimed critique
ranked it third and fourth of five, below both groove sizes, and scored its
surface 0.5–2.5 points lower. It also costs tens to hundreds of MB per model.
No follow-up spec is opened. The `panel_seam` signature for slice 13 is in
[`decisions.md`](../../../decisions.md) (the "Lane E2" entries).

## What was compared

There are five candidates on two models: the T-72B3 glacis and hull side,
and the Leopard 2A6 turret cheek (the arrowhead wedge).

| Key | Candidate |
|---|---|
| B | control: the model as its script builds it today |
| D | grooves, 1 cm wide × 6 mm deep (the slice's starting guess) |
| A | grooves, 2 cm × 2 cm (the one deeper re-run, after D vanished on the T-72) |
| C | bake: unique UV, then tangent normal and AO (0.1 m) maps baked in Cycles from D's geometry onto B, at 400 px/m |
| E | bake, as C but from A's geometry |

Every sheet in `sheets/` stacks the five as rows **A, B, C, D, E** from top to
bottom. That is the order the critique saw them in, unlabelled.

The seams were the same probe layout on every candidate:
- **T-72B3:** an inset lid line 4.5 cm inside each glacis reactive armour
  tile, three joints across each fender strip, and an inset lid line 6 cm
  inside each Relikt side panel.
- **Leopard 2A6:** a joint parallel to the ridge on the wedge's upper and lower
  faces, and two joints running down its side and angled faces.

The layout is a probe for readability, not a reference-faithful one; see the
lessons below.

## Method

- **Build.** A throwaway wrapper ran each family's own `build`, then cut the
  grooves into the tier-0 plates before the shared `vehicle_export` run baked
  paint, wear and occlusion into vertex colour (`parts.finish`), and exported.
  - Crew are dropped, because slice 12 removes them and the source GLB isn't
    pulled.
  - Lint is skipped.
- **Battle zoom.** At its closest zoom the battle camera orbits 25 m out at
  a 0.22 rad pitch, with a 0.8 rad vertical field of view
  (`fixtures/presentation.json:28-35`). That was rendered at 1920×1080. At this
  distance tier 0 is drawn: the model's largest extent, about 9.5 m, projects
  to about 485 px against the 150 px tier-0 threshold (`view.rs:308-321`).
  One pixel covers about 2 cm.
- **Cinematic.** Eye 1.5 m up and 10 m out on the same front-left bearing,
  60 mm lens.
- **Renderer.** Blender EEVEE, so all five go through one renderer; the bake
  can only be judged in Blender (decision S8).
  - The material matches the game's model shader (`models_material.wgsl`):
    the albedo tile × vertex colour × the material's `colour_scale`, with the
    tiled normal and ORM maps.
  - The sun is at the fixture's 0.7 rad elevation, with a sky fill. The game's
    wear threshold and its lighting are not reproduced.
- **Closeups** (`closeups/`). Through `closeups.py`, shaded and matcap, for B,
  D and A. Workbench ignores normal maps and vertex colour, so the bake has
  no closeup.

## Costs

**Grooves** (Blender's Manifold boolean, one swept box per segment):

| Model | Seamed parts | Tier-0 triangles | GLB bytes |
|---|---|---|---|
| T-72B3, B | — | 62,382 | 10,611,400 |
| T-72B3, D (1 cm × 6 mm) | 36 | 66,638 (+4,256, +6.8%) | 10,961,632 (+3.3%) |
| T-72B3, A (2 × 2 cm) | 36 | 66,588 (+4,206, +6.7%) | 10,952,720 (+3.2%) |
| Leopard 2A6, B | — | 56,256 | 9,138,248 |
| Leopard 2A6, D | 2 | 57,210 (+954, +1.7%) | 9,253,148 (+1.3%) |
| Leopard 2A6, A | 2 | 57,268 (+1,012, +1.8%) | 9,251,868 (+1.2%) |

Tiers 1–3 are unchanged. That works out to about 120 tier-0 triangles per
seamed plate, including the paint-edge splitting `finish` adds.

**Bake** (per-part maps at 400 px/m, a 2.5 mm texel):

| Model | Parts baked | Area | Texels | Textures | PNG bytes |
|---|---|---|---|---|---|
| T-72B3 (left side only) | 24 | 34.6 m² | 14.2 M | 48 (normal + AO each) | 8.3 MB |
| Leopard 2A6 (both wedges) | 2 | 10.5 m² | 8.4 M | 4 | 3.4–3.5 MB |

Scaled to a whole model as one atlas (60% packing), with runtime textures
uncompressed RGBA8 at 4 B/texel (`bundle.rs:315-316`) and normal + AO packed
into one texture:

| Density | Map | Per model, no mips | 37 appearances |
|---|---|---|---|
| 400 px/m | 8192² | 268 MB | far over budget |
| 100 px/m (1 cm texel; a seam at most 2 texels) | 2048² | 16.8 MB | about 620 MB |

The paint area at tier 0 is 194.9 m² on the T-72B3 and 236.7 m² on the
Leopard. The whole catalog's load tripwire is 640 MiB.

The bake also needs:
- a second unique UV;
- bundle support;
- a shader change (`models_material.wgsl:35-43`);
- about 10 minutes of single-threaded Cycles per tank side.

## Telemetry (battle_zoom, native crops against B)

From `compare-screenshots`. `d16` is the share of pixels that moved by at
least 16 of 255 grey levels; `edge Δ32` is the share whose edge strength
moved materially.

| Candidate | T-72 glacis d16 | T-72 side d16 | Leopard cheek d16 | Edge Δ32 (glacis / side / cheek) |
|---|---|---|---|---|
| D grooves 1 cm × 6 mm | 0.01% | 0.07% | 2.31% | 0.4% / 0.7% / 4.5% |
| A grooves 2 × 2 cm | 0.29% | 0.92% | 3.36% | 1.5% / 4.1% / 6.5% |
| C bake from D | 0.00% | 0.00% | 1.02% | 0.2% / 0.5% / 2.7% |
| E bake from A | 0.11% | 0.53% | 1.82% | 1.1% / 4.0% / 3.9% |

No candidate moved any T-72 pixel by 32 levels.

What this shows:
- **D vanishes on the T-72.** By the slice's rule that triggered the one
  deeper re-run, A.
- **A is the largest mover everywhere.**
- **Why D vanishes.** `parts.finish` starts its occlusion rays 12 mm off the
  surface (`parts.py:664`), so a 6 mm groove's floor clears the groove. The
  groove floors came out about 4% darker than the plate at 6 mm, and about
  30% darker at 20 mm (vertex colour luminance 0.18 against 0.26).

## Critique (unprimed, unlabelled, the last check)

Surface scores out of 10 (6 means a studio would accept it as a base at that
distance). Each cell is battle zoom / cinematic, with the rank in brackets.

| Key | Candidate | T-72B3 | Leopard 2A6 |
|---|---|---|---|
| A | grooves 2 × 2 cm | 6 (1st) / 6 (2nd) | 5.5 (1st) / 5.5 (1st) |
| D | grooves 1 cm × 6 mm | 6 (2nd) / 6 (1st) | 5.5 (2nd) / 5.5 (2nd) |
| E | bake from A | 6 (3rd) / 5.5 (3rd) | 5 (3rd) / 4.5 (3rd) |
| C | bake from D | 5.5 (4th) / 5 (4th) | 4.5 (4th) / 3 (5th) |
| B | control | 5 (5th) / 4 (5th) | 4 (5th) / 3.5 (4th) |

The critique's observations:
- **T-72B3 at native size:** rows A, D and E can't be told apart; only the
  control looks plainer.
- **Leopard at native size:** the wedge joint shows in A, D and E, and is
  absent in the control.
- **Bake artifacts:** the bakes add short dark dashes, an X-shaped mark and a
  stray slash along the seams. These are unique-UV island edges.
- **Groove lips:** A's joints read as crisp continuous lines, perhaps a
  touch heavy. D's thinner seams catch a light lip from the paint's
  edge-wear highlight.

## Lessons for slice 13

1. **Seams follow the reference's real joints.** The probe's inset lid
   rectangles made the T-72 side panels read as "cabinet doors". The
   reference skirts are flat plates with vertical joints only.
2. **Keep a seam about 10 cm or more from an existing edge or gap.** The
   T-72 glacis lid lines sat 4.5 cm in from the tile gaps, merged with them,
   and vanished.
3. **Depth must clear the occlusion bake's 12 mm ray offset,** or the groove
   gets no darkening.
4. **Use the Manifold boolean solver.** Blender's Exact solver deleted the
   left Leopard wedge (148 → 2 triangles) on a closed, positive-volume mesh.
5. **Out of scope here, for lane B.** Every Leopard row, the control
   included, shows hard-edged light and dark triangles across the wedge
   faces that follow its mesh triangulation (high confidence). It is a
   wedge-shading defect for the Leopard family slice (22), not a seam
   effect.

## Files

- **`sheets/`:** the A–E montages, which are the checkpoint shots.
  - `<tank>-battle_zoom.png` is at native size;
  - `-x4` is the same pixels enlarged 4× (nearest neighbour);
  - `-cinematic`: the T-72's rows show the glacis and the side; the
    Leopard's show the left cheek.
- **`frames/`:** full 1920×1080 battle-zoom frames of B and A.
- **`closeups/`:** `closeups.py` shaded and matcap shots of B, D and A.
- **`bake-maps/`:** one AO and one normal map from E, for a T-72 glacis tile
  and the left Leopard wedge.
- **References judged against:**
  - `assets/references/t72/eastern_t_72_t_72b3_2016-three_quarter_front.jpg`
    (CC-BY-SA-4.0; review only);
  - `assets/references/leopard/europe_leopard_2_2a6-three_quarter_front.jpg`
    (CC0);
  - `assets/references/leopard/europe_leopard_2_2a6-detail.jpg` (public
    domain).
