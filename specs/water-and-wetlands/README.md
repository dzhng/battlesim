# Water and wetlands

**Status:** in progress. The unknowns walk ([decisions](decisions.md),
[sharp edges](#sharp-edges), [the map](visualizations/unknowns-map.html))
and the slicing are done; the checklist below says which slices have
shipped. The plan is the [slice graph](#slice-graph).
**Updated:** 2026-10-28.

## Next Agent Prompt

You are implementing a three-part feature the user asked for on 2026-10-27:

1. **Port water quality** from RedGPU's `Lake` (an MIT-licensed WebGPU engine;
   see [the reference](#the-reference)) into our own renderer, aiming for "the
   same quality as their video". Port the techniques, not the library
   ([Why not adopt RedGPU](#why-not-adopt-redgpu)).
2. **Swamps** are terrain that slows ground units, a factor for each drive.
   **Puddles** are look only (decision 29).
3. **Wakes** appear when units the side can see wade through standing water.

Read [`development.md`](../../development.md), the
[battle renderer README](../../packages/battle-renderer/README.md), the
[pipelines README](../../crates/pipelines/README.md) and the
[native client README](../../native/client/README.md).

- Load [`renderer`](../../.agents/skills/renderer/SKILL.md) before GPU work.
- Load [`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md) and
  [`write-tests`](../../.agents/skills/write-tests/SKILL.md) before slices 10
  and 11.
- Use [`implement-spec`](../../.agents/skills/implement-spec/SKILL.md) to work
  through the slices.

[`decisions.md`](decisions.md) is a list of givens. Don't re-ask them. Each
slice file names the decisions it may still make. Anything else you have to
decide goes in a `choices.md` ledger beside this file.

Slice 00 is done: the water scene (`bun run --cwd web scene -- water`) films
the wetlands reel on both clients and is every visual slice's place to judge.
Its preview checkpoint passed without feedback, so the framings stand.
Slice 02 is done: the water draws multisampled in its own pass after glass.
Its cost probe settled decision 25 that way. Before reading the frame in the
water's shader, read its note on the four-bind-group limit.
Slice 01 is done: one `Wind` record, read by grass and by smoke and dust
(slice 17 moved it from the environment uniform into the camera); slice 17 (tree sway) can start. Slice 03 is done: each
river draws one of its biome's water looks, uniforms the water's shader reads,
retuned live on the dev server. Slice 04 is done: the water shows its bed by
depth, through each look's per-channel extinction. Slice 05 is done: the
water fades in from where its depth is zero. Slice 06 is done: a baked
normal map at three scales, drifting with the wind. Slice 07 is done: the
bed shows displaced through the surface, and Low keeps the depth colour
alone through one uniform word (`WaterDetail`). Slice 08 is done: caustics
on the bed, from the surface's own waves. Slice 09 is done: the water
reflects through the shared light at water's F0, in three lobes, the bed's
light loses what the surface reflects, the streaks are gone, and SSR is
not built. **Pick up at [slice 13](slices/13-swamp-look.md)** (the wet
lane), once the sim lane's slice 17 has merged; the tactical read is open
below. The sim lane (slice 10 on)
runs in parallel on `spec/water-and-wetlands-sim` and merges in here.

**Warning:** native must start and draw water at every slice (decision 19). A
slice that changes an exported pipeline's layout, sample count or targets ports
native in the same slice. Anything too hard goes to
[`water-native`](../water-native/README.md), which you keep updated.

Work on the branch `spec/water-and-wetlands`, push to it as you go, and keep
its PR to `staging` (#15) current. Before ending a pass, update this section and
the checklist.

- [x] The unknowns walk and the landmine sweep (2026-10-27).
- [x] Parallel drafts and slicing (2026-10-27).
- [x] [00](slices/00-water-scene.md), the water scene (2026-10-27).
- [x] [02](slices/02-water-pass.md), the water pass (2026-10-28).
- [x] [01](slices/01-one-wind.md), one wind (2026-10-28).
- [x] [03](slices/03-water-look-data.md), water look data (2026-10-28).
- [x] [04](slices/04-depth-colour.md), depth colour (2026-10-28).
- [x] [05](slices/05-soft-shore.md), the soft shore (2026-10-28).
- [x] [06](slices/06-surface-normals.md), wind-driven normals (2026-10-28).
- [x] [07](slices/07-refraction.md), refraction and the Low tier (2026-10-28).
- [x] [08](slices/08-caustics.md), caustics (2026-10-28).
- [x] [09](slices/09-highlights.md), Fresnel and sun highlights (2026-10-28).
- [x] [10](slices/10-swamp-terrain.md), swamp terrain, with the wetlands map's
      swamp (2026-10-28).
- [x] [11](slices/11-swamp-slowdown.md), swamp slowdown; its paired battle
      report on the wetlands map is still owed (2026-10-28).
- [ ] Sim lane: [12](slices/12-mapgen-swamps.md).
- [x] [13](slices/13-swamp-look.md), swamps drawn as water (2026-10-28).
- [x] [14](slices/14-puddles.md), puddles; their look is open (2026-10-28).
- [ ] Wet lane: [15](slices/15-wake-sources.md), [16](slices/16-wave-field.md).
- [ ] [17](slices/17-tree-sway.md), tree sway, any time after 01.
- [ ] [18](slices/18-closeout.md), closeout: run everything once.

## Goals and non-goals

**Goals**

- Rivers, puddles and swamps read as water on screen, using RedGPU's techniques
  in our own palette: colour by depth, a soft shore, a visible bed, refraction,
  caustics and wind-driven ripples. Each kind comes in variants
  ([decisions 9-12](decisions.md#look)).
- Swamps are terrain the simulation knows about. Ground units cross them
  slower, by a factor for each drive, and their dip shapes sight through the
  ordinary terrain rules ([decisions 1-8](decisions.md#terrain-and-rule)).
  Puddles are look only (decision 29).
- A unit the side can see wading through standing water leaves a wake that
  spreads and fades.
- Native always runs and gets as close to parity as each step allows. What's
  too hard goes to the [water native follow-up](../water-native/README.md)
  ([decision 19](decisions.md#scope-and-acceptance)).

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

Their camera is a ground-level third-person view. Ours is an RTS camera. Slice 00
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
| **Soft shoreline fade** (`:373-375`, `:630`) | `smoothstep` on optical depth over a fade distance that is floored by camera distance, so the edge never aliases. | The shore is analytic bands from the river shape. | **Yes.** It also gives swamp and puddle edges for free, whatever their shape. |
| **Refraction** (`:380-413`) | Offsets screen UV by the difference between the perturbed and flat normal, scaled by depth over camera distance and clamped. It is rejected where the offset would sample foreground geometry. The mip level rises with depth and turbidity, giving murky blur for free. | None. | **Yes.** Needs the scene colour copy. |
| **Caustics** (`:435-501`) | Procedural, in the water shader: two normal samples each distort the other's UV, and crests are where they agree. Projected onto the reconstructed bed along the view and sun, faded with depth, and masked off steep walls. Multiplies the refracted colour. | None. | **Yes.** Cheap: four taps. |
| **Normals** (`:280-346`) | One normal map at three scales (primary, a rotated macro layer, a rotated detail layer), each scrolling along the wind at its own rate, blended with RNM (`blendRNM`, `:35`). Faded toward flat with distance. | Two procedural ripple octaves (`waterNormal`). | **Yes**, with our own normal maps (baked or procedural). |
| **Vertex waves** (`lakeVertex.wgsl:30-74`) | Two sines along and 35° off the wind, amplitude scaled by wind strength, with analytic normals. | Flat surface cells. | **Maybe.** Their amplitude is centimetres; it may not read from an RTS height. |
| **Fresnel and sun specular** (`:503-592`) | Schlick with F0 0.02, three GGX sun lobes at rising roughness, sun in-scatter, and back-lit wave translucency. | `environment.shade` with one water roughness. | **Yes**, through our shared light owner, not a second lighting model. |
| **Screen-space reflections** (`:94-236`) | A 64-step world-space march with bisection, faded by screen edge, distance and the reflected ray's height. Hits below the water plane are rejected. | None; the sky reflection only. | **Not built (slice 09's call, decision 15).** At the low framing it would mirror the far bank; from the tactical height a reflected bank is a few percent of a dark band, and the march costs a pass's worth of taps. |
| **Received shadows** | RedGPU water doesn't receive shadows. | We already sample the sun shadow on water. | Keep ours. |

**Pass contract RedGPU relies on** (`Renderer.ts:241-336`): draw all opaque
geometry, copy colour (with a full mip chain) and depth, then draw water over
the same targets. Today our water draws inside the one MSAA world pass, with
read-only depth and no colour copy (`worldPass.ts`, `battleFrame.ts`). This seam
is new work in both clients (native per decision 19), and it is the first real architectural change of
the feature.

### Wakes (`src/water/lakeCore/interaction/`)

| Part | How it works | Port? |
| --- | --- | --- |
| **Wave simulation** (`lakeWaveSimulation.wgsl`, `LakeWaveSimulator.ts`) | A 512² ping-pong height field. A 9-point Laplacian wave equation with damping, a 24-texel absorbing border, and a ripple normal written out. Shifts by whole texels when its domain moves, so waves stay put in the world. Idles itself off once calm. | **Yes**, but stepped at a fixed rate on the presentation clock. RedGPU steps once per rendered frame, so its wakes change with frame rate. |
| **Coupling to the surface** (`lakeFragment.wgsl:348-365`) | Flattens the base normal where ripples are strong, RNM-blends the ripple normal on top, and fades at the domain edge. | **Yes.** |
| **Capture pass** (`LakeCapturePass.ts`, `lakeCapturePenetration.wgsl`) | Renders opted-in meshes from below into an impulse texture, keeping only the parts just under the waterline. | **No.** Replace it with impulses splatted from a list: the Rust presentation crate already decides per-entity effects (dust behind hulls, `effects/frame.rs` `note_movers`), and it should decide wake sources the same way. |
| **Motion tracking** (`LakeInteractionManager.ts`) | Speed from model-matrix deltas, plus a synthetic footstep pulse `|cos(phase)|^8`. | The idea, yes: a soldier's footstep pulse and a hull's bow wave. Computed in presentation, from observed motion. |
| **Domain** | One 20 m square following one target. | **No.** An RTS view needs a domain that follows the camera and is sized to the view, or a few domains. Slice 16 sizes it with a cost probe. |

### Wind (`src/wind/WindManager.ts`, `systemStruct.Wind.wgsl`)

| Part | How it works | Our state |
| --- | --- | --- |
| **One wind** | A 32-byte struct (direction, speed, strength) shared by foliage sway and water. | Two unconnected winds: the biome's `grass.wind` (heading 35°, lean, gusts) and `presentation.effects.wind_mps` for smoke and dust (about 27°). |
| **Tree sway** (`src/landscape/foliage/core/pipeline/foliageInstanced.wgsl:38-110`) | Trunk bend `sin(t·speed+phase) + 0.25·sin(1.5t·speed+1.8·phase)`, phase from position along the wind, weighted by height cubed or vertex colour. Leaf flutter on a separate mask. A trunk-only variant for shadows. | Trees don't sway. `understorey.sway_m` is a static placement offset. |
| **Gusts** | None: a steady sine. | Our grass already has travelling gusts. Keep ours as the richer model. |

Port: **one wind**, resolved in Rust presentation and uploaded like the light,
read by grass, water normals, smoke and dust drift, and tree sway. This replaces
both of today's sources; neither survives beside it.

## Swamps as terrain

Per [`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md), this is
this is the rule. The user approved its shape in decisions 1-8.

**Rule:** a swamp is standing shallow water in a dip it carves itself, and any
ground mover can cross it. Off a road, a ground mover's speed there is
multiplied by the swamp's factor for its drive (foot, wheeled or tracked). Where
a forest overlaps the swamp, the slower of the two factors applies. A road
keeps road speed and is never carved. Craters stack on top. No sight rule names
a swamp; the dip hides what the ordinary terrain sight rules say it hides.
Rivers stay deep and impassable. Puddles are look only.

**Where it lives today, and what it touches**

- **Contract:** `crates/contract/src/map.rs`. `SurfaceKind` (Road, CountryRoad,
  DirtTrack, Paving) is a `SurfaceArea` with a shape. Its speed is the rules'
  `surfaces` row `SurfaceRule.speed_factor` (`scenario.rs`, `game.json`). That
  factor multiplies *road* speed today. Swamp factors multiply *off-road*
  speed per drive, so they are their own rows, not a second meaning for that
  column.
- **Sim:** `crates/sim/src/world/mod.rs` `ground_surface_at` and `Surface`
  (`road_factor`, `forest`, `traversable`). Speed is
  `navigation::Mobility::speed` (`navigation.rs`), read by vehicle movement,
  soldier movement and the route planner.
- **Planner:** `navigation.rs` `SlowGround` keeps per-tile forest and steep
  counts for route cost. Swamp needs its own channel, or units will plough
  straight through swamps when a dry way round is faster.
- **Export to presentation:** `sim/src/world/export.rs` strides, read by
  `presentation/src/static_world/ground/`.
- **Mapgen:** `crates/mapgen/src/layout/`. The river is placed in `rivers.rs`;
  `water.rs` answers distance-to-water queries. Preview changes in the
  [map workbench](../../apps/map-workbench/README.md) (`bun run dev:maps`).

**Narrated moments that shaped the rule**

- *A tank column on a dirt track meets a car-sized puddle.* If every tank
  brakes hard, a viewer laughs. So puddles are look only, with a splash
  (decision 29).
- *A truck tries to cross a swamp.* Bogging down reads well on screen, and
  wheels bog worse than tracks (decision 3).
- *A road runs across a swamp.* The road is a causeway, so the road wins.
- *A swamp lies in a forest.* The slower factor applies (decision 4), so each
  number keeps its meaning.
- *A squad wades a swamp under fire.* It is slower. Seen from far across flat
  ground, a deep swamp's rim can hide it, and it sees less itself. A tank's
  higher eye sees in. All of that comes from the ordinary sight rules
  (decision 6).
- *A side watches an enemy cross a swamp in the fog.* Wakes come only from
  units the side currently sees, as dust does today. Ripples already spreading
  fade on their own.
- **Freeze check:** factors are admitted in (0, 1] when the catalog loads, so
  nothing can stop dead in a swamp.

**Neighbouring rules**

- Ground marks: swamps refuse them, as water does (decision 7).
- Dust: none in standing water; a wake instead (decision 8).
- Mapgen keeps settlements, buildings and bridge approaches out of swamps
  (decision 5).
- The carved dip changes the simulation's height, slope and sight (decision 1).
- Digests: any map that gains a swamp changes its battle digest. Mapgen
  placing swamps changes every generated map. Both are a named decision to
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

## Slice graph

```
render  00 ─┬─ 02 ─ 03 ─ 04 ─ 05 ─ 06 ─ 07 ─ 08 ─ 09
            └─ 01 ──────────────────┘ (06 needs 01)
               01 ─ 17 (tree sway, any time)
sim     10 ─ 11 ─ 12                      (from day one)
wet     05 + 10 ─ 13 ─ 14 ─ 15 ─ 16       (16 also needs 06)
all     ─ 18 closeout
```

| Slice | Judged on | Native |
| --- | --- | --- |
| [00 Water scene and reference](slices/00-water-scene.md) | the targets themselves | adds the draw check |
| [01 One wind](slices/01-one-wind.md) | smoke heading | ports |
| [02 Water pass](slices/02-water-pass.md) | nothing changes | ports |
| [03 Water look data](slices/03-water-look-data.md) | nothing changes | uploads the looks |
| [04 Depth colour](slices/04-depth-colour.md) | colour against depth | free |
| [05 Soft shore](slices/05-soft-shore.md) | shoreline | free |
| [06 Surface normals](slices/06-surface-normals.md) | surface detail and motion | ports the bake, or defers |
| [07 Refraction and Low tier](slices/07-refraction.md) | the bed through water | free, or runs at `depth` |
| [08 Caustics](slices/08-caustics.md) | caustic pattern | free |
| [09 Highlights](slices/09-highlights.md) | highlights, plus the SSR call | free |
| [10 Swamp terrain](slices/10-swamp-terrain.md) | tests: carve, marks, rounds, sight | none |
| [11 Swamp slowdown](slices/11-swamp-slowdown.md) | tests and a battle report | none |
| [12 Mapgen swamps](slices/12-mapgen-swamps.md) | where swamps lie | none |
| [13 Swamp look](slices/13-swamp-look.md) | a swamp with units in it | free |
| [14 Puddles](slices/14-puddles.md) | puddle look | free |
| [15 Wake sources](slices/15-wake-sources.md) | tests only | ignores the records |
| [16 Wave field](slices/16-wave-field.md) | wake rings | deferred |
| [17 Tree sway](slices/17-tree-sway.md) | canopy motion | ports if a layout changes |
| [18 Closeout](slices/18-closeout.md) | the whole frame beside RedGPU | lists every gap |

**Digests:** only slices 10 to 12 change battle digests. Slice 10 changes
only maps that contain swamps. Slice 11 changes every battle once, through the
rules hash. Slice 12 changes generated maps. Each is a named decision. In any
other slice, a changed digest is a bug.

**Standing gates for every visual slice:**

- [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md)
  against slice 00's frozen captures and the stills in `assets/`, on that
  slice's crop only;
- [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md),
  unprimed, as the last check before accepting a shot;
- [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) for the
  user's checkpoints, which are non-blocking: wait about 5 minutes, then
  decide on the evidence, record it, close the shots and continue;
- slice 00's native draw check.

**How the drafts were merged:** four drafts each had a different bias (fewest
slices, risk first, seam quality, feedback loop). Draft D (feedback loop)
supplied the harnesses: `fixtures/captures/water.json` owns the shots, look
data reloads live on the water scene's page, and the map workbench gets a
swamp overlay. Where the drafts agreed, the plan follows them:

- the water scene and a native draw check come first;
- water reads the existing multisampled depth in its shader, so it needs no
  depth copy;
- the swamp's simulation lane runs in parallel from day one;
- only the swamp slices change digests.

Where they split, the plan chose:

- **Separate visual slices**, rather than A's eight-slice merge. A merged
  absorption with shore, and normals with highlights. That broke the rule of
  one visual variable per slice, which A flagged itself as a reslice point.
- **Glass stays in the MSAA world pass** (A's objection, adopted as decision 25
  revised).
- **The look data and variant pick get their own slice with no visual change**
  (C's W3), so every later look slice only tunes data.
- **Swamp water stops sight at its surface** (C's objection, answered by
  decision 32). The simulation has no prone posture, so this adds no
  concealment.

## Owners


- **Water surface:** one material for rivers, swamps and puddles. A swamp or puddle is
  water with a shallow depth and its own look data, not a second water shader.
- **Wind:** one record, resolved in Rust presentation.
- **Wake sources:** decided in Rust presentation from what the side sees.
  The renderer runs the height field and never chooses who makes a wake.
- **Ground speed:** one `Mobility::speed` path. A swamp adds a factor; it does
  not add a second speed function.
- **Look tuning:** `WaterLook` rows from the biome data and
  `presentation.terrain`, uploaded as uniforms, never spliced into shader text
  (slice 03).
- **Standing water in presentation:** `StandingWater.at`, shared by drawing,
  dust suppression and wakes (slices 13 to 15). It is built from the
  simulation's export and reuses the contract's membership test
  (`crates/contract/src/river.rs` `section`). It never re-derives where water
  is with a rule of its own.
- **Standing water in the simulation:** `Surface::standing_water()`, shared by
  craters, track wear and `water_hit` (slice 10).
- **Ground footing:** one `Footing` value feeds all five speed read sites
  (slice 11).
- **Variant pick:** one `look(seed, feature)` function (slice 03).
- **Wave clock:** `presentation::wave::WaveClock`, which publishes steps and an
  epoch. The renderer owns only the field's textures (slice 16).
- **Pass order:** `worldPass.ts` and `battleFrame.ts` alone (slice 02).

**The end state reads as designed today, not as features tacked on.** Each
slice deletes what it replaces:
- the analytic shore bands (05);
- the procedural ripple octaves (06);
- the two old winds (01);
- the shader-text water tuning (03);
- `dense.rs`'s private speed function (11).

No old path survives beside a new one. The only transitional state is native
running at `depth` detail, and its `water-native` item names when that ends.

## Sharp edges

The 2026-10-27 sweep found these. They need no decision, but each must reach its
owning slice. Worst first within each area.

**Simulation and map generation**

- **The dip can silently not exist.** `Carve` returns natural height on a map
  with no river (`crates/sim/src/world/carve.rs`), and terrain and navigation
  only look inside registered regions (`terrain.rs`, `world/mod.rs`
  `navigation_regions`). Swamps must run the carve on every map and register
  their own bounds, or the planner sees open ground while movement slows.
- **Make swamps their own map list, like forests, not a new `SurfaceKind`.**
  `SurfaceKind::of` turns any non-road kind into Paving, `SurfaceIndex::at`
  keeps only the earliest kind, and every kind needs a road `speed_factor`
  row. Add a swamp field to the sim's `Surface` and the nav `Cell`.
- **Admission reads uncarved height.** Buildings carry an absolute `base_z`,
  and river and bridge checks use `relief_height`. Add contract rules keeping
  swamps off buildings and bridge approaches and a gap from river edges, for
  hand-made maps as well as generated ones.
- **There are five speed read sites:** vehicle movement, soldier movement,
  `cell_cost`, journey road arcs, and the `SlowGround::stretch` estimate. Today
  the combination is road, else forest, else open, then slope, then craters
  for vehicles only. Give `SlowGround` one combined channel per mover, so a
  forest and a swamp on the same tile aren't counted twice.
  `examples/navigation_quality/dense.rs` has its own copy of `speed()`.
- **Resolve the per-drive factor in `ground_mobility`** into a field on
  `Mobility`, as `forest_multiplier` is today. `Drive` is documented as never
  read by planning.
- **Serialization:** the new map field needs
  `default, skip_serializing_if = "Vec::is_empty"`, or every map hash changes.
  New rules rows change the rules hash for every battle. Re-record the parity
  fixtures that embed full rules and maps (`fixtures/parity/*`, including
  `publication/` and `capture/`). Bump the mapgen generator and presets
  versions, draw swamps from their own random stream, and re-record
  `map-layout` and the skirmish corpus.
- **Mapgen order:** place swamps after towns and forests, respect the
  reserved areas, and add a fairness check between the top and bottom halves.
  Objectives currently refuse forest only, and `open_ground` would count a
  swamp as open.

**Renderer and native**

- **Native rejects the whole export on any value it doesn't know**
  (`native/client/src/pipelines.rs`, `deny_unknown_fields`). Native's tests
  build pipelines but never draw, so CI stays green while native is broken.
  Decision 19 and the [water native follow-up](../water-native/README.md) own
  this.
- **No depth resolve exists,** and `lit` can't be a copy source in either
  client. Resolving into a new target keeps stale pixels on Metal, because it
  skips empty tiles; resolve into `lit` or draw full-screen.
- **Pass order must be** opaque, then copy, then water, then glass, then
  effects. Today glass draws after water in the same pass and writes no depth.
- **Refraction samples colour before fog,** so a seen water pixel refracting an
  unseen neighbour shows a seam at the fog edge. Copy the fog mask alongside
  the colour. Water's blended fog coverage changes meaning once water is
  opaque.
- **Ground marks under water would show twice,** once on the bed and once on
  the water, and refraction would make order lines double and wobble. One layer
  must own the paint.
- **A Low tier toggle must be a uniform** (plus a stub binding when the copy is
  skipped). The export never varies quality, so a pipeline only Low used would
  never be exported or checked. Water look data must be uniforms too, because
  `presentation.terrain` is written into the shader text.
- **The river frame baseline is captured on Low,** and the river scene checks
  shore placement against the simulation's distance to the edge, plus
  waterline flicker. A depth-taken shore follows 4 m triangles, so expect those
  checks to fail and re-judge them. Don't loosen them to pass.

**Presentation and wakes**

- **Riders make false wakes.** Every own unit becomes a mover record, and a
  carried squad stands at its APC's position. Wake sources must skip held units
  (the `riding()` test) and get a test beside "hidden emits nothing".
- **The wake decision can't see feet or water today.** Mover records carry
  member ids but not positions, and `EffectFrame::note` has no ground access.
  Shots that land in water spawn dust too.
- **Decide wakes per tick, not from the pose driver's gait phase,** which
  follows frame rate. Use smoke's fixed rate for rings and dust's spacing for
  the V-wake.
- **The wave field lives outside presentation's resets.** Rust should publish a
  capped step count per frame and a counter that changes on reset, side switch
  or load. Captures warm up a fixed number of steps from a reset. Native plates
  jump between frames, so leave the wave state out of parity, or warm it up the
  same way on both.
- **Unifying the wind re-records** `grass-vectors.json`, `effect-vectors.json`,
  `fixtures/parity/exact-numbers.json` (compared as text) and native smoke
  plates. The sound's wind is separate.

**Bugs found beyond this feature**, fixed in
[PR #17](https://github.com/dzhng/battlesim/pull/17):

- Track wear was recorded on ground cells under bridge decks, because `wear`
  had no surface check (`crates/sim/src/ground.rs`).
- Forest multipliers were never validated, so a 0 divided by zero in
  `SlowGround::stretch`. Making `MovementRules` refuse unknown keys is left to
  slice 11, because `scenario.rs` has to be split first.

## Still open

- **The tactical read (decision 20).** Slice 09's whole-look checkpoint,
  ours beside RedGPU at the three framings, passed no feedback. Close and
  low read as water: the displaced bed, the caustic net and the mirror at a
  grazing angle carry them. From the tactical height (260 m) the deep
  channel is still an untextured band, which an unprimed critique read as a
  tube or asphalt. The normals fade flat there, so nothing of the surface
  survives. Slice 18 takes it: keep the slope variance a pixel averages
  away, as roughness, so far water keeps a sheen that varies. The summer
  palette stays ours (decision 9).

- **Reeds (slice 13).** The reed marsh grows rough grass under its water:
  reeds knee-high out of 0.6 m of water would pass the grass owner's 0.9 m
  cap, which is measured from the ground. Measure a grass's height from the
  water's surface where it stands in water, and add a reed grass kind.
  Slice 18 takes it.
- **Puddles (slice 14).** They lie only on the paved kinds and read as
  wet mud patches rather than reflective water; field puddles need the
  grass owner to read puddles. Slice 18 judges them.
- **Swamp depth for the user (decision 30).** 0.6 m reaches just above a
  soldier's knee on screen, where the decision also says thighs. The
  number stays until the user picks.

Earlier: the two bugs found beyond this feature are fixed and merged in
[PR #17](https://github.com/dzhng/battlesim/pull/17): track wear under bridge
decks, and unvalidated forest multipliers. Slices 10 and 11 build on it.
