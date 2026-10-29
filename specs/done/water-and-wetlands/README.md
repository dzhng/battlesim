# Water and wetlands

**Status:** built 2026-10-28 on `spec/water-and-wetlands`, in PR #15 to
`staging` (open at close). Rationale record. [`decisions.md`](decisions.md) holds the user's
givens (1–35) from the 2026-10-27 unknowns walk, and [`choices.md`](choices.md)
holds the calls the build made where they were silent.
[The unknowns map](visualizations/unknowns-map.html) is the walk's summary.

## What shipped and why

On 2026-10-27 the user asked for three things:

1. Water with "the same quality as their video": RedGPU's `Lake`, an
   MIT-licensed WebGPU engine ([the reference](#visual-provenance)).
2. Swamps that slow ground units, by a factor for each drive, and puddles.
3. Wakes where units wade through standing water.

What shipped:

- **One water material.** Rivers, swamps and puddles share it. It colours by depth through each look's per-channel extinction,
  fades in from where its depth is zero, shows its bed displaced through
  the surface, lights the bed with caustics, and moves in wind-driven
  normals at three scales. It reflects through the shared light at water's
  F0, in three lobes, and turns to a mirror at a grazing angle. Each kind of
  water comes in variants: two river looks, two swamp looks, two puddle looks
  (decisions 9–12).
- **Swamps are terrain.** A swamp is standing shallow water in a dip it
  carves itself. Any ground mover crosses it slower, by a factor for its
  drive. The dip shapes height, slope and sight through the ordinary terrain
  rules. Map generation places swamps in pairs, one in each half.
- **Puddles are look only** (decision 29). They lie on roads, paving and
  tracks, in a track's wheel ruts, and change no digest.
- **Wakes.** A unit the side can see wading leaves rings, or a V once it
  moves. A round landing in water splashes. A height field around the
  camera's target spreads them.
- **One wind.** Grass, water normals, smoke and dust drift, and tree sway all
  move in one `Wind` record.
- **A Low tier for water.** Low keeps the depth colour and drops refraction,
  caustics and wakes (decision 18).
- **Native** runs and draws everything but wakes, which the
  [water native follow-up](../../water-native/README.md) holds.

Two bugs found on the way were fixed first, in
[PR #17](https://github.com/dzhng/battlesim/pull/17): track wear recorded
under bridge decks, and forest multipliers that were never validated.

Not in scope, until the user says otherwise: fording rivers (rivers stay
impassable, per [battle-foundation](../battle-foundation/requirements.md)),
amphibious units, boats, underwater views, and RedGPU's terrain, foliage,
impostors or KTX2 loader.

## Why it has this shape

**RedGPU's techniques, in our renderer, not the library.** The 2026-10-27
evaluation found four reasons not to adopt it:

- RedGPU is a scene-graph engine that owns culling, LOD, foliage placement
  and lighting. Here Rust presentation decides all of those, and the renderer
  only draws ([battle renderer](../../../packages/battle-renderer/README.md)).
- Its shaders live in its TypeScript runtime. The native client could not run
  them, and we would lose the one exported shader set.
- It scatters foliage procedurally on the GPU, so the screen could disagree
  with what each side knows of the world.
- It is a one-person alpha.

So the techniques were ported one at a time, each judged on screen, and each
through an owner the repo already had: the shared light, the fog layout, the
exported pipelines.

**"The same quality" means the same techniques in our palette.** Depth
colour, a soft shore, the bed, refraction, caustics and ripples, not their
colours (decision 9). RedGPU's fixed extinction ratios turned our beige bed
saturated green and teal, so each look carries its own three numbers.

**Swamps are terrain; puddles are not.** A swamp is big enough to change
which unit is right for the job: trucks bog worse than tanks (decision 3).
That earns three numbers in the rules. A puddle is too small to carve on a
4 m grid (decision 22), and a tank column braking for one would make a
viewer laugh. So a puddle is presentation's alone: placed once per map, cell
by cell, when the ground is built (`PuddleGround::puddles`), absent from the
map contract, and only splashing when crossed
(decisions 29 and 34).

**A swamp is its own map list, not a `SurfaceKind`.** `SurfaceKind::of`
turns any non-road kind into paving, `SurfaceIndex::at` keeps only the
earliest kind, and every kind needs a road `speed_factor` row. A swamp's
factor multiplies off-road speed per drive, which is a different quantity
from a surface's share of road speed. So `MapDefinition.swamps` sits beside
`forests`, and `Surface` and the navigation `Cell` each gain a `swamp` flag.

**The slowdown comes from the swamp, not its depth** (decision 2).
Depth-driven speed would be hard to balance. Depth shapes the terrain, the
look and, through the terrain, sight. The edge is wide and soft (decision
21) so the slope slowdown doesn't leak depth back into speed.

**One standing-water owner, by depth.** `StandingWater` answers what water
stands at a point for drawing, the plots, the posture rule, dust, wakes and
splashes. Water stands where its surface lies above the ground. So a road
across a swamp, which the simulation leaves uncarved, is dry by depth alone,
with no road rule. A unit on a deck over a river is dry because its feet
stand above the surface.

**The Low tier is one uniform word.** The pipeline export never varies by
quality, so a pipeline only Low used would never be exported or checked.
`WaterDetail` reaches the shader as `TerrainParams.view.z`. One pipeline
serves every tier. The targets allocate the copies only `full` reads, and on
Low the fog's 1×1 placeholder stands in for them.

**Water looks are uniforms, not shader text.** Most of
`presentation.terrain` is spliced into the shader as constants; the water's
and the shore's numbers are uniforms instead (`TerrainParams.waterLooks`,
`shoreTuning`). As uniforms, a look retunes live on the dev
server with no shader rebuilt, the export stays one set, and native uploads
Rust's tables as they are.

**Water draws multisampled, in its own pass after glass.** Decision 25
chose single-sampled water unless storing the world's samples proved cheap.
The probe found it within noise ([Measured](#measured)), and the
single-sampled build stair-stepped its waterline. Glass stays in the world
pass: an RTS camera almost never shows water through a pane.

**Wakes are decided in Rust presentation, per tick, from what the side
sees.** The renderer only runs the height field. RedGPU renders opted-in
meshes from below and steps once per rendered frame, so its wakes change
with frame rate and its sources ignore fog. Here wakes follow ticks, never
the pose driver's gait phase, and the field steps on a fixed clock.

**One wind, in the camera.** There were two unconnected winds: grass's and
the effects'. The one record keeps the grass heading, the most visible wind
in the menu reel's liked shots, and smoke turns about 8° to match (decision
26). It rides in the camera record because a sun cascade's caster binds only
its camera, and trees must bend alike in their shadows.

**Hollywood scale where physical scale doesn't read.** Refraction's shift is
eight times the physical one, and caustic cells are several times a real
stream's. At physical scale both are under a pixel at the close framing.

## Principles and invariants

- **Only swamps change digests.** A map without swamps keeps its hash
  (`MapDefinition.swamps` is skipped when empty). Off swamps, ground speed is
  bit-identical to the arithmetic before. The swamp rules joined the rules
  hash, which folds into every digest, so every battle's digest moved once,
  as one named decision. Puddles, wakes, looks, sway and the wind change no
  digest. Anywhere else, a changed digest is a bug.
- **Water keeps one rule.** `Surface::standing_water()`
  (`crates/sim/src/world/mod.rs`) is true for a river or a swamp. Craters and
  track wear refuse it (`ground.rs`), and rounds and sight stop at its
  surface through the one `water_hit` (`world/rays.rs`). No sight rule names
  a swamp (decisions 6, 7, 23, 32). Pinned by `crates/sim/tests/swamps.rs`.
- **The water does not hide a standing soldier.** A soldier's eye, aim and
  centre stand above 0.6 m of water, and the simulation has no prone posture
  (decision 32). Pinned by
  `a_soldier_standing_in_a_swamp_is_seen_and_can_be_hit`.
- **The dip hides through the terrain alone.** With water stopping sight at
  the land's level, a line between two points above the water never passes
  under the rim, so the dip can't hide a unit standing above its water.
  The ground sight sweep shows the mechanic on a deep test dip
  (`a_deep_dip_hides_its_floor_from_far_flat_ground_but_not_from_near_or_high`).
  Decisions 21 and 32 can't both hold for identification; 32 won
  ([choices](choices.md#decision-21s-sight-test-runs-on-the-ground-sight-sweep-not-on-identification)).
- **The dip exists on every map.** `Carve` (`world/carve.rs`) runs on maps
  with no river and registers each swamp's regions for terrain and
  navigation. Without them the planner would see open ground while movement
  slows (`a_swamp_carves_its_dip_on_a_map_with_no_river`).
- **A road always keeps road speed and is never carved** (decision 4). The
  cut ends a triangle clear of the road's side and eases in from there.
- **One speed path.** `Mobility::speed`
  (`crates/sim/src/navigation/mobility.rs`) takes one `Footing`: road wins,
  else open speed times the slower of forest and swamp, once, then slope.
  `Surface::footing()`, `Cell::footing()` and a road arc's `footing()` build
  it at all four read sites (navigation, the journey, movement, the
  soldier). The planner's estimate, `SlowGround`, reads the same factors: it
  counts forest only, swamp only and both, so an overlap is never counted
  twice. Every factor lies in (0, 1],
  checked when the rules load, so nothing stops dead in a swamp. Pinned by
  `crates/sim/tests/ground_speed.rs`.
- **The planner and movement see the same swamp.** Each cell's planned cost
  is movement's speed at its middle
  (`planning_costs_each_cell_at_the_speed_movement_takes_there`).
- **Admission keeps a swamp carvable** (`contract::swamp::validate`): a
  polygon on level land, at least about three height cells across, an edge no
  steeper than the slope cutoff (`deepest_m`), clear of rivers' cut banks,
  buildings and bridge approaches. Map generation runs the same check on its
  lowered map, so a placement miss is a named seed refusal.
- **Generated swamps draw from their own random stream** (`swamps`), so
  rivers, woods and towns don't move. The menu reel's market-town map is
  pinned at `layout-16` as files and is not regenerated.
- **One owner says where water stands, how deep and in which look.**
  `StandingWater` is built from the simulation's export (river and swamp
  strides) and the ground's puddles; the water's cells, the plots, posture,
  wakes and splashes all ask it. One older lookup remains: the surface
  field's `water` distance (`Field::water`, `ground/field.rs`), which knows
  rivers only, shades the bank and keeps scenery clear of rivers (see
  [Open](#open)).
- **Variants are look only, and seeded per feature.** `water::look(seed,
  kind, feature, looks)` picks by the biome's seed, the kind and the
  feature's index, so no other feature moves it, and a look of no weight is
  never drawn. A puddle's look follows the surface under its centre
  (decision 28). Pinned by `crates/presentation/tests/water_looks.rs` and
  `puddles.rs`.
- **A puddle's place is a pure function of the seed and its cell**
  (decision 34), so wakes, both clients and any camera agree
  (`where_a_puddle_lies_is_its_cell_alone`).
- **A wading soldier is never drawn lying down** (decision 33). He takes the
  first posture, from his own upward, whose chest clears the water at his
  feet (`presentation.pose.stance.chest_m`). The dead still lie where they
  fall. Pinned by `crates/presentation/tests/wading.rs`.
- **Wakes come only from what the side sees.** Enemies wade only while
  identified. Squads riding a carrier wade nowhere (`units::riding`, the one
  test interpolation and the effects publication share). Only what stands below the water's
  surface wades (`StandingWater::under`). A V needs water at least
  `wake.vee_depth_m` deep; a boot in a puddle only rings. Pinned by
  `effect_behaviour.rs` and `wake_sources.rs`.
- **Wakes don't depend on frame rate.** Impulses are decided per tick, and
  `WaveClock` runs `floor(clock · hz)` steps, so any split into frames of at
  most 0.2 s runs the same steps; a longer frame drops the excess, capped at
  six (`crates/presentation/tests/wave_field.rs`).
- **One side's waves never reach the other.** A load, a side switch or a
  reset opens a new epoch, and the renderer clears the field on a new epoch
  or an empty packet (`wavePlan`) (decision 24).
- **The wave field uses no storage textures.** It ping-pongs two
  `rgba16float` render targets in fragment passes, so native's export schema
  is unchanged.
- **Pass order:** the world (opaque, grass, backdrop, then glass) resolves
  into `lit` and keeps its samples; the water's pass copies `lit`
  (`waterPass.ts`) and draws over the samples with depth read-only; then the
  effects (`battleFrame.ts`). `worldPass.ts` (`encode`) owns the world's
  order through the water. The world pass still resolves, because Metal skips
  resolving a tile the water's pass leaves empty.
- **A water shader binds four groups.** Four is the WebGPU default. Whatever
  the water reads of the frame (depth, the scene colour, its guard, the wave
  field) rides the fog layout (`fogTerm.ts`), and only
  `FogGroups.water` holds the real textures. Every group the world pass
  binds holds a 1×1 placeholder, because that pass writes the depth.
- **One layer owns the paint.** The bed is unpainted under the water, and
  the water lays a mark over its composite once, so an order line never
  doubles or wobbles through refraction.
- **Refraction never reaches across the fog.** A shifted sample is taken
  only where the fog says it is seen at least as well as the water's own
  pixel, and never from geometry in front of the surface. The blur keeps
  the rule too: a guard beside the copy's mips holds the worst of each
  texel's footprint, unseen and painted, so the bed is read at the pixel,
  unblurred, wherever any pixel its blurred sample weighs is seen less or
  marked.
- **Water's light goes through the shared owner.** Its F0 of 0.02 is half
  metal on a black base in `standardPbr`'s terms (`WATER_METAL`,
  `frame/waterLight.ts`). There is no second lighting model.
- **The prepass, colour and caster stages call the same `sway`.** Otherwise
  the equal-depth test leaves holes. Casters pass no leaf flutter, so shadows
  bend with the trunk and don't shiver. The scenery walk widens each chunk's
  bounds by the sway's reach (`Sway::reach`), or bent crowns are culled at
  the view's and cascades' edges. Felled trees don't sway.
- **Native always starts and draws water** (decision 19). Native refuses the
  whole export on any value it doesn't know, and its pipeline tests never
  draw. The water scene is the draw check: the browser's water must stand at
  least ΔE 10 from its dry bank, and native's within ΔE 3 of the browser's.

## Where it lives

**Contract and simulation**

- `crates/contract/src/swamp.rs`: `Swamp`, its admission (`validate`,
  `deepest_m`, `EDGE_CELLS`).
- `crates/contract/src/movement.rs`: `MovementRules` and each locomotion's
  `SpeedFactors::swamp` (`movement.speed_factors` in `fixtures/game.json`;
  `SwampRules` was folded in by [unit concealment](../unit-concealment/README.md)).
- `crates/sim/src/world/carve.rs`: the dip, beside river and bridge carving.
- `crates/sim/src/world/surfaces.rs` (`SurfaceIndex`), `world/mod.rs`
  (`Surface`), `world/rays.rs` (`water_hit`), `world/export.rs`
  (`SWAMP_STRIDE`).
- `crates/sim/src/navigation/mobility.rs` (`Mobility`, `Footing`),
  `navigation/slow.rs` (`SlowGround`), `units/motion.rs`
  (`ground_mobility`).
- Tests: `crates/sim/tests/swamps.rs`, `crates/sim/tests/ground_speed.rs`.

**Map generation**

- `crates/mapgen/src/layout/swamps.rs` (`place`), its presets in
  `layout/presets/swamps.rs` and the `swamps` section of
  `fixtures/map-presets.json`.
- Tests: `crates/mapgen/tests/swamps.rs`.

**Presentation**

- `crates/presentation/src/static_world/ground/water.rs`: `StandingWater`
  (`at`, `under`, `flooded`, `cells`) and the variant pick (`look`, `Kind`).
- `static_world/ground/puddles.rs`: the puddle walk (`candidate`).
- `static_world/ground/tables.rs` (`water_looks`): the look rows the shader
  reads; their data is `water` in `fixtures/biomes/summer.json` and
  `presentation.terrain.water` in `fixtures/game.json`.
- `crates/presentation/src/wind.rs` (`WindSettings`) and the `Wind` record in
  `records.rs`; the biome's top-level `wind` block.
- `crates/presentation/src/effects/publication.rs` (`Wading`, the `water`
  hit), `effects/frame/wake.rs` (rings and V), `effects/frame.rs`
  (`WakeImpulse`, `EffectFrame::wakes`).
- `crates/presentation/src/wave.rs` (`WaveClock`, `WAVE`, the field's area in
  `Camera.waves`) and `presentation/cameras.rs` (`Presentation::waves`, the
  cascade cameras' clock and wind).
- `crates/presentation/src/units/driver/stance.rs`: the posture rule.
- `static_world/ground/sway.rs`: tree sway's tuning (`SWAY`, `Sway::reach`).
- Tests: `water_looks.rs`, `puddles.rs`, `wading.rs`, `wake_sources.rs`,
  `wave_field.rs` and the wind and wake cases in `effect_behaviour.rs`, all in
  `crates/presentation/tests/`.

**Renderer** (`packages/battle-renderer/src/frame/`)

- `worldPass.ts` (`encode`): pass order. `battleFrame.ts`: the frame's
  targets and quality.
- `waterPass.ts`: the copy of `lit`, and the composite of bed, body and
  reflection. `waterUnder.ts`: the copy's mips and their guard, and what the
  water reads of its bed.
- `waterLook.ts`, `waterSurface.ts`, `waterNormals.ts` (`waterNormalsBake`),
  `waterUnder.ts` (refraction, `sceneColourMip`), `waterCaustics.ts`,
  `waterLight.ts`, `waveField.ts` (`wavePlan`, `waveStep`), `waveNormal.ts`.
- `targets.ts` (`WaterDetail`, `fogMaskUnder`), `terrainSource.ts`,
  `fogTerm.ts` (`fogLayout`), `sway.ts`. The wind is `cameraWind` in
  `world/camera.ts`.
- The tier: `waterDetail` in `apps/battle-lab/src/graphicsSettings.ts`.
  Live look tuning: `apps/battle-lab/src/gameBiome.ts`
  (`?water_look=`, `?swamp_look=`).

**Native**: `native/client/src/world.rs` (`Water`), `water_normals.rs`,
`fog.rs`.

**Judging it**

- `fixtures/maps/wetlands/`: a `capture` map with a river and its bridge, a
  swamp, a country road, a dirt track and a wood. Its `water` encounter
  stands a rifle squad and a tank at the bank, and a second rifle squad in
  the swamp, which the reel's swamp and wading shots frame.
- `fixtures/captures/water.json`: the water reel, one scene of held shots.
  The scene names them in order (`SHOTS` in `web/scenes/water.mjs`): close
  (65 m), tactical (260 m), low (80 m at 0.2 rad), the swamp from above,
  wading at 12 m, and puddles.
- `bun run --cwd web scene -- water` films the reel in the graphics test,
  replays it on native, checks both, and writes a sheet beside RedGPU's
  stills. `web/scenes/_water.mjs` is the one water mask the scenes share.
- The frame scene's Low baselines (`web/scenes/baselines/frame/`) pin the
  depth-colour path.

## How the build diverged from the plan

- **The wetlands map is a capture map, so there is no wetlands lab.** The
  readers refuse a map in both categories. The water scene and
  `/labs/capture?capture=water` stand in, and the graphics test films any
  reel `REELS` names (`web/src/battle/benchmark/reels.ts`).
- **Stills agree within a tolerance, not byte for byte.** The graphics test
  runs on the wall clock. Native replays the browser's own frame records, so
  the two draw the same frames, and their water is judged within ΔE 3 of each
  other (not checked when a look is pinned, nor on the puddle shot).
- **The native check measures distance from the browser,** not "bluer than
  the bank". The green-grey and brown looks broke the first measure, and it
  was re-judged, not lowered.
- **Water stays multisampled** (decision 25, by its own probe).
- **The bank's bands stay.** The plan listed "the shore and bed bands" for
  deletion. Only the water's analytic edge went; the bank's wet silt and
  bare earth are the ground's look.
- **No mips on the normal map.** Each layer flattens as a pixel outgrows its
  texels instead.
- **Refraction projects a world shift,** not RedGPU's view-space UV offset,
  so the falloff comes from the camera and the one number stays a physical
  ratio.
- **No sun in-scatter term.** The body's own lit colour already is one.
- **The wind moved into the camera** (slice 17), revising slice 01's
  environment upload.
- **Swamps come in pairs,** one in each half, and their chance lives in the
  presets' swamp section, not beside `river_chance`.
- **Swamp evidence is a paired crossing,** not a battle report. The
  encounter has no opposing force, so the crossing isolates the variable
  ([Measured](#measured)).
- **Puddles lie only on the paved kinds,** not in fields
  ([Dead ends](#dead-ends)), and in the nearer wheel rut on rutted kinds.
- **A puddle's depth is its shape's,** carried in its water cell, since it
  carves nothing and the depth buffer has none.
- **Captures warm the wave field by playing the reel,** not by a fixed
  warm-up count.
- **The wave field's cost was never measured on the GPU** ([Open](#open)).
- **Native draws no wakes** ([water native](../../water-native/README.md)).
- **No wake debug lab was built.** The seam tests carry the moments it
  would have shown.

## Dead ends

- **Adopting RedGPU's library:** see [above](#why-it-has-this-shape).
- **RedGPU's capture pass for wakes:** rendering meshes from below can't know
  what the side sees and steps per frame. Replaced by impulses splatted from
  a list.
- **RedGPU's fixed extinction ratios** `(2.4, 0.7, 1.1 + 0.9·turbidity)`:
  saturated green and teal over our bed.
- **Single-sampled water after the resolve:** a stair-stepped waterline where
  the bank crosses the water, for no saving.
- **RedGPU's slant floor of 0.12 on the shore fade:** a milky band at a low
  camera. Ours is 0.35.
- **The fade's footprint from the centroid varying:** its derivatives step on
  partly covered pixels and drew the water's triangle edges as faint lines.
  The footprint is rebuilt at the pixel's centre.
- **Vertex waves:** RedGPU's are centimetres. From 80 m they move the surface
  by less than a pixel.
- **The streaks of light along the stream:** every critique read them as
  brushed metal or motion blur. Deleted.
- **Refraction at physical scale:** about a pixel at 65 m; it didn't read.
- **Texel-based caustic fade:** at 65 m a pixel spans 13 texels, so it drew
  no caustics. The fade counts the finest wave instead.
- **Caustic tiles of 8 m and 16 m:** sparkle, then foam. The tile is 24 m.
- **Caustics on the refracted bed point:** the steep-wall mask's derivatives
  flecked the low view with dark pixels. They use the straight depth.
- **A sharper first sun lobe (0.07):** one bloomed hotspot, not finer
  sparkles. The glints' size is the normal map's.
- **Wind-patch roughness for the tactical read:** no visible difference. At
  water's F0 from above, the sky barely shows ([Open](#open)).
- **Screen-space reflections:** not built (decision 15). They would mirror the
  bank at the low angle only; from the tactical height a reflected bank is a
  few percent of a dark band, and the march costs a pass of taps.
- **Puddles in the surface field's water list:** grass would have read them,
  but the shore's bands, sized for rivers, would ring every puddle in mud,
  asphalt too.
- **Puddles on the track's grass strip, and darker puddle mud:** read as
  mounds, stains, then holes. Ruts and lighter mud read as wet mud.
- **RedGPU's push splat and its numbers for the wave field:** 512² at 25 cm
  gave saturated bands; harder damping piled pushes into mounds; a ring
  splat at 25 cm was clear but too big. The field is 1024² at 12.5 cm with a
  ring splat.
- **A cached `StandingWater`:** building it costs about 0.8 µs on the wetlands
  map, so the pose driver builds it each frame.
- **A fixed-step clock for plated runs:** a change to the shared reel runner
  the parity tool doesn't need.
- **A second recorder for the water reel:** it would duplicate the parity
  format.

## Measured

The costs are whole-frame GPU time (`frame/gpuTiming.ts`) in the river lab at
1920×1080; the swamp crossings are simulated time and distance.

**Storing the world's samples** (decision 25), tactical framing, eight
interleaved arms of 120 frames:

| tier | single-sampled water | samples kept, water multisampled |
| ---- | -------------------- | -------------------------------- |
| High | 6.525 ± 0.049 ms     | 6.539 ± 0.057 ms                 |
| Low  | 8.18 ± 0.81 ms       | 8.05 ± 0.90 ms                   |

The water's techniques were measured as a pair on High, on and off, because
Low and High differ in render scale, grass and shadows: a flat normal against
the baked map (High 9.21 ± 1.09 ms with the map against 9.29 ± 0.91 ms flat), refraction
(9.17 ± 0.72 ms against 9.24 ± 0.84 ms), and refraction with caustics against
depth colour alone (7.32 ± 0.24 ms against 7.47 ± 0.85 ms), each within
noise. The wave field was not measured.

**At closeout**, four runs of 120 frames:

| framing          | High            | Low             |
| ---------------- | --------------- | --------------- |
| close (65 m)     | 11.53 ± 0.01 ms | 11.56 ± 0.01 ms |
| tactical (260 m) | 7.24 ± 0.12 ms  | 9.01 ± 0.30 ms  |
| low (80 m)       | 9.90 ± 0.00 ms  | 9.89 ± 0.02 ms  |

Low draws at 0.75 render scale with less grass.

**Swamp crossings** (foot 0.5, wheeled 0.3, tracked 0.55, against every
factor at 1.0, same seed). The wetlands encounter's squad and tank each move
alone from north-west of the swamp, through its middle to (440, 20) and past
its north-east end to (470, 70):

| Unit  | Route    | Goal      | Swamp factors             | Factors 1.0   | Shift                     |
| ----- | -------- | --------- | ------------------------- | ------------- | ------------------------- |
| squad | shortest | (440, 20) | 130.6 s, 259 m            | 95.7 s, 259 m | +36% time                 |
| squad | fastest  | (440, 20) | 111.0 s, 281 m            | 95.7 s, 259 m | +16% time, +22 m round it |
| squad | shortest | (470, 70) | 108.4 s, 254 m            | 94.6 s, 254 m | +15% time                 |
| squad | fastest  | (470, 70) | 103.3 s, 262 m, stays dry | 94.6 s, 254 m | +9% time, +8 m round it   |
| tank  | shortest | (440, 20) | 54.4 s, 233 m             | 41.4 s, 233 m | +31% time                 |
| tank  | fastest  | (440, 20) | 48.0 s, 265 m             | 41.4 s, 233 m | +16% time, +32 m          |
| tank  | shortest | (470, 70) | 43.3 s, 226 m             | 39.7 s, 226 m | +9% time                  |
| tank  | fastest  | (470, 70) | 40.3 s, 235 m, stays dry  | 39.7 s, 226 m | +2% time, +9 m round it   |

Ordered straight through the swamp's middle, the squad loses about a third of
its time and the tank a little less; past its end, a sixth and a tenth. Asked
for the quickest way, both mostly skirt the water and pay well under that. The factors are tuning, not requirements, and were
not retuned.

## Visual provenance

The standard was decision 20: at our camera, next to RedGPU, ours reads as
good. The stills are in [`assets/`](assets/), and
[its README](assets/README.md) says where each came from.

- **The release video's frames.** [RedGPU](https://github.com/redcamel/RedGPU)
  V4.4.0-Alpha (tag commit `95aaf9a9ecb3c5ccfcfc824ccad1a6489392cf68`, code
  under MIT) announced its lake in a
  [41 s video](https://x.com/redcamel15/status/2114606572165046472). Four
  frames from it show the look at its best, from the demo's own cameras,
  overhead and near the water:
  - [`redgpu-depth-colour.jpg`](assets/redgpu-depth-colour.jpg): colour from
    shallow to deep, and the bed through it. It drove depth colour.
  - [`redgpu-overhead-shadows.jpg`](assets/redgpu-overhead-shadows.jpg): the
    overhead read, with shadows on water, caustics and normal detail. It
    drove the normals and caustics.
  - [`redgpu-wading-shore.jpg`](assets/redgpu-wading-shore.jpg): a soft
    shoreline and a unit standing in water. It drove the shore fade,
    refraction and the swamp shots.
  - [`redgpu-wake-rings.jpg`](assets/redgpu-wake-rings.jpg): wake rings around
    a wading unit, from above. It drove the swamp shots and the wave field.
- **The live demos at our framings.** `redgpu-landscape-*` and
  `redgpu-beach-*` are the `lakeWithLandscape` and `characterWalking` demos,
  filmed on 2026-10-27 at the water reel's close, tactical and low framings.
  They are the like-for-like targets for the highlights and for the final
  whole-frame read. The live site may run a newer build than the tag.

Their sky and light differ from ours (the beach is lit by a sunset), so each
step judged technique, not palette. The demo's normal maps carry no licence,
so ours are baked in a shader at load (decision 16).

**The final side-by-side read**, an unprimed critique of whole frames beside
RedGPU's:

- **Low (80 m), toward the sun:** as good.
- **Close (65 m):** close. Depth shading and the shallow bed read well. The
  deep channel is blank in a still, and the banks read as a canal lining.
- **Tactical (260 m):** close. It reads as a matte, curved trench with no
  sheen.
- **Wading (12 m), beside `redgpu-wading-shore.jpg`:** close. The submerged
  grass gives depth. There are no reflections of the soldiers, faint rings
  only, and a hard edge where the marsh meets the field (no reeds are built).

The swamp from above (45 m), the puddle shot (22 m) and the trees' sway have no
RedGPU counterpart; each was judged on its own by unprimed critiques.

The final checkpoint in Preview drew no feedback.

## Open

These are the user's to decide. The native items are also on the
[water native](../../water-native/README.md) list.

- **The tactical read.** From 260 m the river reads as a matte trench. The
  band is the depth colour under this sun; wind-patch roughness changed
  nothing. Changing it is a look decision (the palette or bed detail), not a
  technical fix. The summer palette is ours by decision 9.
- **Reeds are not built.** The reed marsh grows rough grass under its water.
  Knee-high reeds out of 0.6 m of water would pass the grass owner's 0.9 m
  cap, which is measured from the ground. The physical fix measures a
  grass's height from the water's surface where it stands in water, plus a
  reed grass kind.
- **Puddles:**
  - they read as wet mud, not reflective water: at the reel's sun the sky
    they reflect is a pale haze near the track's colour;
  - they lie only on paved kinds, with none in fields; field puddles need the
    grass owner to read puddles;
  - a dirt track's centre-strip grass stands through them;
  - mirror puddles were never filmed: the wetlands map has no metalled road.
- **Swamp depth (decision 30).** 0.6 m reads knee-deep on screen, where the
  decision also says thighs. The number is kept; thighs would need more.
- **Tree sway amplitude.** Gusts barely read at the 80 m low shot, where the
  wind blows along the lens.
- **The wave field:**
  - its cost was never measured on the GPU; the splat loop (up to 256 a step)
    is the term to watch in a big battle;
  - captures warm it by playing the reel, with no fixed warm-up;
  - native draws no wakes.
- **A second, older water lookup.** The surface field's `water` distance
  (`Field::water`) knows rivers only: it shades the bank and keeps scenery
  clear of rivers. Map generation keeps woods out of swamps, so nothing
  stands in one today, but a map that put scenery in a swamp would not keep
  it clear. Folding it into `StandingWater` would make it one owner.
- **SSR:** rejected for this build (decision 15, [dead ends](#dead-ends));
  open if a cinematic camera wants the far bank in the water.
- **The frame scene's tick-label flake.** The river lab's "Tick N" HUD label
  differs by a pixel between runs.
- **`ally-help`'s premise.** Swamp generation moved this generated map's
  fifth objective, which widened the team planner's 8×8 strength grid, and
  the objective the scene defends came to sit 15 m inside a cell's edge.
  The planner read one cell, so enemies beside it but across the edge read
  as nothing. It now reads every cell the objective's square reaches
  ([choices](choices.md)); skirmish digests moved with it. The scene still
  fails, and no setup passes it honestly. Help reads only enemies the side
  sees inside the objective's square; the defender usually dies on the
  approach, or to shooters outside the square or out of sight; a lane is
  gone with its last unit; and one squad holding the objective settles
  under the trigger. The scene's own check also passes falsely, on an ally
  retargeting the objective after the defender dies. Making it pass is a
  rule decision: count attackers the side knows of and its lane's losses,
  or keep a lane a while after its last unit.
- **Duplication left for a cleanup:**
  - the look rows' order (rivers, swamps, puddles) is written twice, by the
    table (`tables.rs`) and by `StandingWater`'s row picks;
  - two `Sway` types: grass's biome tuning and the trees' bend;
  - the scenes' water helper (`web/scenes/_water.mjs`) reads the first
    river's surface for every river.
