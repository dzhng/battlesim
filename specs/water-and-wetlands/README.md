# Water and wetlands (placeholder)

**Status:** placeholder, being mapped. The unknowns walk has settled the
quadrants of known knowns, known unknowns and unknown knowns
([decisions](decisions.md)). The sweep for unknown unknowns is next. Nothing is
sliced or implemented.
**Updated:** 2026-10-27.

## Next Agent Prompt

You are picking up a three-part feature the user asked for on 2026-10-27:

1. **Port water quality** from RedGPU's `Lake` (a MIT WebGPU engine; see
   [the reference](#the-reference)). The user wants "the same quality as their
   video". Port the techniques into our own renderer, not the library. The
   2026-10-27 evaluation rejected adopting RedGPU itself (see
   [Why not adopt RedGPU](#why-not-adopt-redgpu)).
2. **Add puddles and swamps** as a map terrain type that slows all ground units
   going through them.
3. **Draw wakes** when units wade through standing water, like the ripple rings
   in their video.

Read [`development.md`](../../development.md), the
[battle renderer README](../../packages/battle-renderer/README.md), the
[pipelines README](../../crates/pipelines/README.md) and the
[native client README](../../native/client/README.md). Load
[`renderer`](../../.agents/skills/renderer/SKILL.md) before GPU work and
[`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md) before the
wetland rule. Then run [`write-spec`](../../.agents/skills/write-spec/SKILL.md)
for real. [`decisions.md`](decisions.md) holds the user's answers. Treat them
as givens and don't re-ask them. Finish the walk's last quadrant (see
[still open](#still-open)), then fan out the drafts and turn the
[draft ladder](#draft-ladder) into slice files under `slices/`. Do not build
before the ladder is sliced.

Work on the branch `spec/water-and-wetlands`. Keep pushing to it, and keep its
PR to `staging` current.

Before ending a pass, update this section: what is settled, the next pickup
point, and the checklist.

- [x] Known knowns, known unknowns and unknown knowns (2026-10-27,
      [decisions](decisions.md)).
- [ ] Sweep for unknown unknowns, starting with the items under
      [still open](#still-open).
- [ ] Freeze the reference: capture RedGPU's lake demo at our camera angles
      (draft ladder, rung 0).
- [ ] Slice the ladder, then work through it from rung 0.

## Goals and non-goals

**Goals**

- Rivers, puddles and swamps read as water on screen, using RedGPU's techniques
  in our own palette: colour by depth, a soft shore, a visible bed, refraction,
  caustics and wind-driven ripples. Each kind comes in variants
  ([decisions 9-12](decisions.md#look)).
- Puddles and swamps are terrain the simulation knows about. Ground units cross
  them slower, by a factor for each kind and drive
  ([decisions 1-6](decisions.md#terrain-and-rule)).
- A unit the side can see wading through standing water leaves a wake that
  spreads and fades.
- The browser ships each step first, and native catches up step by step on
  the same exported shaders ([decision 19](decisions.md#scope-and-acceptance)).

**Non-goals (until the user says otherwise)**

- Fording rivers. Rivers stay impassable to ground units (the
  [battle-foundation](../done/battle-foundation/requirements.md) decision).
- Amphibious units, boats, or underwater views.
- Adopting RedGPU's terrain, foliage, impostors or KTX2 loader.
- Backward compatibility or migrations. This is a hard cutover: fixtures and
  recorded digests are re-recorded where wetlands change them.

## The reference

[RedGPU](https://github.com/redcamel/RedGPU) V4.4.0-Alpha, tag commit
`95aaf9a9ecb3c5ccfcfc824ccad1a6489392cf68`. The code is MIT (`LICENCE.md`).
The demo's normal maps (`examples/assets/water/lake_normal*.png`) carry no
licence statement, so we make our own.

The source of the stills below is the
[release tweet](https://x.com/redcamel15/status/2114606572165046472)'s 41 s
video. The stills are in [`assets/`](assets/):

| Still | What to judge against it |
| --- | --- |
| [`redgpu-depth-colour.jpg`](assets/redgpu-depth-colour.jpg) | Colour from shallow to deep, and the bed showing through |
| [`redgpu-overhead-shadows.jpg`](assets/redgpu-overhead-shadows.jpg) | Overhead read: shadows on water, caustics, normal detail |
| [`redgpu-wading-shore.jpg`](assets/redgpu-wading-shore.jpg) | Soft shoreline and a unit standing in water |
| [`redgpu-wake-rings.jpg`](assets/redgpu-wake-rings.jpg) | Wake rings around a wading unit, seen from above |

Their camera is a ground-level third-person view. Ours is an RTS camera. Rung 0
re-captures the live demo
([examples](https://redcamel.github.io/RedGPU/examples/?tab=3D&q=lake)) at our
pitches and heights, so later comparisons judge like against like.

## What we can copy

All paths are in RedGPU at the commit above. Their defaults are listed as
evidence of what produced the look. They are not targets, and every distance
constant is tuned for a ground-level camera.

### Water surface (`src/water/lakeCore/shader/lakeFragment.wgsl`)

| Technique | How it works | Our state | Port? |
| --- | --- | --- | --- |
| **Depth-based absorption** (`:415-433`) | Per-channel extinction `exp(−d·k·(2.4, 0.7, 1.1+0.9·turbidity))` blends a shallow colour toward a deep colour by how much light survives. `d` is the optical distance from the water surface to scene depth. | Opacity comes from the analytic distance to the river edge (`terrainMaterial.ts` `waterSurface`), not from depth. | **Yes.** The biggest visual win. |
| **Soft shoreline fade** (`:373-375`, `:630`) | `smoothstep` on optical depth over a fade distance that is floored by camera distance, so the edge never aliases. | The shore is analytic bands from the river shape. | **Yes.** It also gives puddle and swamp edges for free, whatever their shape. |
| **Refraction** (`:380-413`) | Offsets screen UV by the difference between the perturbed and flat normal, scaled by depth over camera distance and clamped. It is rejected where the offset would sample foreground geometry. The mip level rises with depth and turbidity, giving murky blur for free. | None. | **Yes.** Needs the scene colour copy. |
| **Caustics** (`:435-501`) | Procedural, in the water shader: two normal samples each distort the other's UV, and crests are where they agree. Projected onto the reconstructed bed along the view and sun, faded with depth, and masked off steep walls. Multiplies the refracted colour. | None. | **Yes.** Cheap: four taps. |
| **Normals** (`:280-346`) | One normal map at three scales (primary, a rotated macro layer, a rotated detail layer), each scrolling along the wind at its own rate, blended with RNM (`blendRNM`, `:35`). Faded toward flat with distance. | Two procedural ripple octaves (`waterNormal`). | **Yes**, with our own normal maps (baked or procedural). |
| **Vertex waves** (`lakeVertex.wgsl:30-74`) | Two sines along and 35° off the wind, amplitude scaled by wind strength, with analytic normals. | Flat surface cells. | **Maybe.** Their amplitude is centimetres; it may not read from an RTS height. |
| **Fresnel and sun specular** (`:503-592`) | Schlick with F0 0.02, three GGX sun lobes at rising roughness, sun in-scatter, and back-lit wave translucency. | `environment.shade` with one water roughness. | **Yes**, through our shared light owner, not a second lighting model. |
| **Screen-space reflections** (`:94-236`) | A 64-step world-space march with bisection, faded by screen edge, distance and the reflected ray's height. Hits below the water plane are rejected. | None; the sky reflection only. | **Later, optional.** From a steep RTS camera most rays leave the screen. Judge it on the menu reel's low shots first. |
| **Received shadows** | RedGPU water doesn't receive shadows. | We already sample the sun shadow on water. | Keep ours. |

**Pass contract RedGPU relies on** (`Renderer.ts:241-336`): draw all opaque
geometry, copy colour (with a full mip chain) and depth, then draw water over
the same targets. Today our water draws inside the one MSAA world pass, with
read-only depth and no colour copy (`worldPass.ts`, `battleFrame.ts`). This seam
is new work in both clients, and it is the first real architectural change of
the feature.

### Wakes (`src/water/lakeCore/interaction/`)

| Part | How it works | Port? |
| --- | --- | --- |
| **Wave simulation** (`lakeWaveSimulation.wgsl`, `LakeWaveSimulator.ts`) | A 512² ping-pong height field. A 9-point Laplacian wave equation with damping, a 24-texel absorbing border, and a ripple normal written out. Shifts by whole texels when its domain moves, so waves stay put in the world. Idles itself off once calm. | **Yes**, but stepped at a fixed rate on the presentation clock. RedGPU steps once per rendered frame, so its wakes change with frame rate. |
| **Coupling to the surface** (`lakeFragment.wgsl:348-365`) | Flattens the base normal where ripples are strong, RNM-blends the ripple normal on top, and fades at the domain edge. | **Yes.** |
| **Capture pass** (`LakeCapturePass.ts`, `lakeCapturePenetration.wgsl`) | Renders opted-in meshes from below into an impulse texture, keeping only the parts just under the waterline. | **No.** Replace it with impulses splatted from a list: the Rust presentation crate already decides per-entity effects (dust behind hulls, `effects/frame.rs` `note_movers`), and it should decide wake sources the same way. |
| **Motion tracking** (`LakeInteractionManager.ts`) | Speed from model-matrix deltas, plus a synthetic footstep pulse `|cos(phase)|^8`. | The idea, yes: a soldier's footstep pulse and a hull's bow wave. Computed in presentation, from observed motion. |
| **Domain** | One 20 m square following one target. | **No.** An RTS view needs a domain that follows the camera and is sized to the view, or a few domains. This is a sizing and cost question for its rung. |

### Wind (`src/wind/WindManager.ts`, `systemStruct.Wind.wgsl`)

| Part | How it works | Our state |
| --- | --- | --- |
| **One wind** | A 32-byte struct (direction, speed, strength) shared by foliage sway and water. | Two unconnected winds: the biome's `grass.wind` (heading 35°, lean, gusts) and `presentation.effects.wind_mps` for smoke and dust (about 27°). |
| **Tree sway** (`src/landscape/foliage/core/pipeline/foliageInstanced.wgsl:38-110`) | Trunk bend `sin(t·speed+phase) + 0.25·sin(1.5t·speed+1.8·phase)`, phase from position along the wind, weighted by height cubed or vertex colour. Leaf flutter on a separate mask. A trunk-only variant for shadows. | Trees don't sway. `understorey.sway_m` is a static placement offset. |
| **Gusts** | None: a steady sine. | Our grass already has travelling gusts. Keep ours as the richer model. |

Port: **one wind**, resolved in Rust presentation and uploaded like the light,
read by grass, water normals, smoke and dust drift, and tree sway. This replaces
both of today's sources; neither survives beside it.

## Wetlands as terrain

Per [`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md), this is
this is the rule. The user approved its shape in decisions 1-8.

**Rule:** standing shallow water (a puddle or a swamp) sits in a shallow dip
that the wetland carves itself, and any ground mover can cross it. Off a road,
a ground mover's speed there is multiplied by its wetland kind's factor for its
drive (foot, wheeled or tracked). Where a forest overlaps the wetland, the
slower of the two factors applies. A road keeps road speed. Craters stack on
top. Rivers stay deep and impassable.

**Where it lives today, and what it touches**

- **Contract:** `crates/contract/src/map.rs`. `SurfaceKind` (Road, CountryRoad,
  DirtTrack, Paving) is a `SurfaceArea` with a shape. Its speed is the rules'
  `surfaces` row `SurfaceRule.speed_factor` (`scenario.rs`, `game.json`). That
  factor multiplies *road* speed today. Wetland factors multiply *off-road*
  speed per drive, so they are their own rows, not a second meaning for that
  column.
- **Sim:** `crates/sim/src/world/mod.rs` `ground_surface_at` and `Surface`
  (`road_factor`, `forest`, `traversable`). Speed is
  `navigation::Mobility::speed` (`navigation.rs`), read by vehicle movement,
  soldier movement and the route planner.
- **Planner:** `navigation.rs` `SlowGround` keeps per-tile forest and steep
  counts for route cost. Wetland needs its own channel, or units will plough
  straight through swamps when a dry way round is faster.
- **Export to presentation:** `sim/src/world/export.rs` strides, read by
  `presentation/src/static_world/ground/`.
- **Mapgen:** `crates/mapgen/src/layout/`. The river is placed in `rivers.rs`;
  `water.rs` answers distance-to-water queries. Preview changes in the
  [map workbench](../../apps/map-workbench/README.md) (`bun run dev:maps`).

**Narrated moments that shaped the rule**

- *A tank column on a dirt track meets a car-sized puddle.* If every tank
  brakes hard, a viewer laughs. The puddle row's factors are mild, and the
  planner weighs puddles by those factors rather than avoiding them.
- *A truck tries to cross a swamp.* Bogging down reads well on screen, and
  wheels bog worse than tracks (decision 3).
- *A road runs across a swamp.* The road is a causeway, so the road wins.
- *A swamp lies in a forest.* The slower factor applies (decision 4), so each
  number keeps its meaning.
- *A squad wades a swamp under fire.* It is slower and exposed. Swamps give no
  concealment (decision 6).
- *A side watches an enemy cross a swamp in the fog.* Wakes come only from
  units the side currently sees, as dust does today. Ripples already spreading
  fade on their own.
- **Freeze check:** factors are admitted in (0, 1] when the catalog loads, so
  nothing can stop dead in a swamp.

**Neighbouring rules**

- Ground marks: wetlands refuse them, as water does (decision 7).
- Dust: none in standing water; a wake instead (decision 8).
- Mapgen keeps settlements, buildings and bridge approaches out of swamps
  (decision 5).
- The carved dip changes the simulation's height, slope and sight (decision 1).
- Digests: any map that gains a wetland changes its battle digest. Mapgen
  placing wetlands changes every generated map. Both are a named decision to
  record, not a regression.

## Why not adopt RedGPU

The 2026-10-27 evaluation found four reasons:

- RedGPU is a scene-graph engine that owns culling, LOD, foliage placement and
  lighting. In our design, Rust presentation decides all of those and the
  renderer only draws (see the [battle renderer](../../packages/battle-renderer/README.md)).
- Its shaders live in its TypeScript runtime, so the native client couldn't run
  them. We'd lose the one exported shader set.
- It scatters foliage procedurally on the GPU, so the screen could disagree
  with what each side knows of the world.
- It is a one-person alpha.

## Draft ladder

This is a scouting pass for `write-spec`. It is not sliced yet. Each rung names
the one variable it would be judged on.

0. **Freeze the reference.** Capture RedGPU's lake demo at the three views
   that must look best (close ~65 m, tactical ~260 m, menu reel low angles),
   next to our current river at the same framings. Variable: none; this
   produces the targets for the side-by-side read (decision 20).
1. **One wind.** Resolve one wind in Rust presentation, replacing the biome
   grass wind and `effects.wind_mps`. Variable: grass and smoke share a
   heading, with no other visual change.
2. **Scene colour and depth copy before water,** in the browser and the
   pipeline export. Variable: none. The shots must be unchanged; this rung is
   the seam.
3. **Absorption and shoreline fade,** in the two river variants. Variable:
   colour against depth, judged on the water mask only.
4. **Wind-driven multi-scale normals** with normal maps generated in a shader.
   Variable: surface detail and motion.
5. **Refraction and caustics.** Variable: the bed seen through water.
6. **Fresnel and sun specular** through the shared light owner. Variable:
   highlights. SSR is reconsidered after this rung (decision 15).
7. **Wetland contract and rule:** the map contract, the carved dip, the rules
   rows per kind and drive, sim speed, planner cost and admission, with a test
   for each ruled-out moment. Variable: none visual; proven on a fixture map
   with a battle report before and after.
8. **Mapgen wetlands:** placement per decision 5, previewed in the map
   workbench. Variable: where wetlands appear.
9. **Wetlands drawn as water** through the same water owner, with the swamp
   and puddle variants chosen per decision 12. Variable: puddle and swamp look.
10. **Wake sources** from presentation: units seen in standing water, with
    rings or a V-wake by speed (decision 13), and dust suppressed in water.
    Variable: none visual; a hidden unit must emit nothing.
11. **Wave simulation and surface coupling** on a camera-following area at a
    fixed step. Variable: wake rings, against `redgpu-wake-rings.jpg`.
12. **Tree sway** from the one wind, with a trunk-only shadow variant.
    Variable: canopy motion.
13. **Graphics tiers:** Low drops refraction, caustics and wakes and keeps the
    depth colour (decision 18). This may fold into each rung instead.

Native catches up rung by rung after the browser (decision 19).

Later, if wanted: splashes from rounds landing in water, and helicopter rotor
downwash on water. Both would feed the same wake sources.

**Standing gates for every visual rung:**

- [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md)
  against rung 0's frozen captures and the stills in `assets/`;
- [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md) as
  the last check before accepting a shot;
- [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) for the user,
  as a non-blocking checkpoint;
- native parity on the same plate when native catches up. This overlaps the
  [native gaps](../native-gaps/README.md) item "Judge water on a plate".

Rungs that change only presentation must leave battle digests unchanged.

## One owner per concept

These owners must hold through every rung:

- **Water surface:** one material for rivers, puddles and swamps. A wetland is
  water with a shallow depth and its own look data, not a second water shader.
- **Wind:** one record, resolved in Rust presentation.
- **Wake sources:** decided in Rust presentation from what the side sees.
  The renderer runs the height field and never chooses who makes a wake.
- **Ground speed:** one `Mobility::speed` path. A wetland adds a factor; it does
  not add a second speed function.
- **Look tuning:** biome and `presentation.terrain` data, not constants in the
  shader.

## Still open

The walk's unknown-unknowns sweep owns these. Each needs an answer before slicing.

- **How native catches up.** Native fails to start on a pipeline setting it
  doesn't know (`native/client/README.md`). Browser-first steps need a defined
  way for native to skip a pipeline it doesn't support yet, or each browser
  step breaks the native build.
- The rest of the sweep: fog-of-war leaks through wakes and refraction, cost at
  far zoom and on Low, the menu reel, and the digests of generated maps.
