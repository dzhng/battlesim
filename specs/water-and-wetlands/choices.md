# Choices

Decisions made during implementation where the spec was silent, each with a
verdict. [`decisions.md`](decisions.md) holds the user's givens; this file holds
what the build decided on their behalf. Banked entries are settled for later
passes.

## Slice 00: the water scene

### The wetlands map is a `capture` map, so there is no `/labs/wetlands`

- **When:** slice 00.
- **The choice:** the repository has two kinds of saved map: `test` maps,
  which labs and tests open, and `capture` maps, which the capture tool and
  the graphics test film. Each reader refuses the other kind. The spec asked
  for one map that is both a lab (`/labs/wetlands`) and a filmed capture
  (`water.json`), which the readers forbid. Wetlands is a capture map, so it
  can be filmed and replayed on native. Quick looks go through
  `/labs/capture?capture=water&scene=0` and the water scene instead of a lab.
- **The gap:** the spec didn't know about the category rule.
- **The reach:** slices 03, 10 and 15 named the lab. They now point at the
  water scene's page. Slice 15's debug buttons need a lab, which must stand on
  its own `test` copy of the wetlands ground.
- **Verdict:** sound. The repository's rule says to copy a map rather than
  share it across categories, and filming is what this map is for.
- **Confidence:** high.

### The graphics test films any reel named in `reels.ts`, picked with `?capture=`

- **When:** slice 00.
- **The choice:** the native client replays only runs the graphics test
  records, and the graphics test filmed only the main menu's reel. To get the
  water shots onto native, the test now takes `?capture=water`, the menu's
  reel by default. The module that named the menu's reel (`menuReel.ts`) is
  now `reels.ts` and names both. The alternative was a second recorder just
  for water, which would duplicate the parity format.
- **The gap:** the spec said to use "the existing capture format" and
  `native-parity-capture.mjs`, but not how a non-menu capture reaches them.
- **The reach:** any future filmed scene that native must replay adds one
  line to `REELS`. A water run's report names its own workload id, `water`,
  so it never passes for a menu-reel measurement.
- **Verdict:** sound.
- **Confidence:** high.

### The three framings are one scene of three held shots

- **When:** slice 00.
- **The choice:** the parity run writes each scene's files under the scene's
  map name, so three scenes on the wetlands map would overwrite each other.
  The reel is one scene with three held shots (close, tactical, low), and
  each gets a control plate at its middle.
- **The gap:** the spec said "three framings" and left their shape open.
- **The reach:** slice 16's wave warm-up and any added framing go into this
  one scene. The scene names shots by order (`SHOTS` in `water.mjs`), because
  the capture format has no shot names.
- **Verdict:** sound.
- **Confidence:** high.

### The water stills agree within a tolerance, not byte for byte

- **When:** slice 00.
- **The choice:** the graphics test runs on the wall clock. A plate lands at
  "the shot's middle" give or take a frame, about 10–20 ms of animation, so
  grass and idle soldiers move a pixel or two between runs. The spec's
  "identical stills" is met within a baseline's tolerance (it forgives where
  a thing is drawn by a pixel or two), not byte for byte. Native replays the
  browser's exact frame records, so the browser–native comparison is exact.
  The alternative was a fixed-step clock for plated runs, a change to the
  shared reel runner that the parity tool doesn't need.
- **The gap:** the spec assumed a deterministic still.
- **The reach:** a baseline blessed from this scene must use the existing
  position-forgiving match, never an exact one.
- **Verdict:** sound for now. Revisit if a slice needs exact stills, for
  example wave fields compared across runs in slice 16.
- **Confidence:** medium.

### "Native draws water" means the river is bluer than its bank

- **When:** slice 00.
- **The choice:** the native draw check projects the simulation's river into
  each plate and compares the river's mean colour with that of the dry
  ground 8–24 m outside its edge. The river must be at least 12 lower on
  CIELAB's yellow-blue axis (b*). Water measures 27–32 there, and the bare
  earth a missing water surface would show measures 5–7. A plain "different
  colour from the bank" test was rejected, because the brown bed already
  differs from grass.
- **The gap:** the spec said "fails if native draws no water", with no measure.
- **The reach:** later look slices must keep river water bluer than grass by
  that margin. A murky variant (decision 9) that drifts toward the bank's hue
  would trip it. If so, re-judge the measure; don't lower the floor to pass.
- **Verdict:** sound.
- **Confidence:** medium.

### The water scene is part of the harness, and it builds and runs native

- **When:** slice 00.
- **The choice:** `water` is a registered scene, not a `tool`. So `verify`
  runs it, and it builds the native client (`cargo build -p native-client`)
  and replays on it. It is marked `serial`, because native shares the GPU.
  Decision 19 says every slice proves native still runs, and a tool scene
  would let that check be skipped silently.
- **The gap:** the spec named the check, not where it runs.
- **The reach:** `verify` now needs the native client to build on the
  machine. It costs about 45 s, plus native's build when that is stale.
- **Verdict:** sound.
- **Confidence:** medium.

### RedGPU's reference comes from two live demos, at our framings

- **When:** slice 00.
- **The choice:** the frozen reference is `lakeWithLandscape` (a wooded lake
  by daylight) and `characterWalking` (a beach lake at sunset), filmed from
  their live GitHub Pages site at our three framings. The page's UI is
  hidden, the soldier is held still, and the camera is centred on deep water
  near the soldier. The live site may run a build newer than the spec's tag.
  The alternative was building the tag locally, which needs their toolchain.
- **The gap:** the spec left the capture method to the implementer.
- **The reach:** decision 20's side-by-side read uses these and the video
  stills. Their light differs from ours, so later comparisons judge
  technique, not palette.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 02: the water pass

### The water stays multisampled, in its own pass after glass

- **When:** slice 02.
- **The choice:** decision 25 kept single-sampled water unless the probe
  found storing the world's samples cheap. It did: whole-frame GPU time with
  the samples kept was within run-to-run noise of single-sampled water, on
  High and Low (the slice's table). So the world pass now stores its samples,
  and the water draws over them in a pass of its own, with depth attached
  read-only. The single-sampled build also worked on both clients, but its
  waterline stepped where the bank crosses the water.
- **The gap:** the spec chose by cost alone and expected (a) to win.
- **The reach:** every later water slice draws in this pass. A GPU where
  storing 4× samples is dear would flip the verdict; the probe has run on one
  Apple machine only. The probe scene is not kept. It was a throwaway toggle,
  and a rerun means rebuilding it.
- **Verdict:** sound for now. Revisit if a weaker GPU's frame time shows the
  store.
- **Confidence:** medium.

### The scene-colour and fog-mask copies wait for slice 04

- **When:** slice 02.
- **The choice:** the spec put both copies, with mips, in slice 02. Nothing
  reads them before slice 04 (colour) and slice 07 (mips), so they arrive
  with their first reader, which also decides their shape.
- **The gap:** the spec staged the plumbing ahead of its use.
- **The reach:** slice 04 adds the copy and its binding; slice 07 adds the
  mips.
- **Verdict:** sound.
- **Confidence:** high.

### A water shader that reads the frame must fit in four bind groups

- **When:** slice 02.
- **The choice:** none yet, a finding. The water binds four groups (camera,
  terrain, environment, fog), and four is the WebGPU default and what Chrome
  reports here. Reading the depth or the scene colour in the water's shader
  needs a fifth. The trial that worked put the depth in the fog layout, the
  groups that already carry the frame's screen-space ground paint. Every
  group the world pass binds held a 1×1 placeholder, as the fog groups
  already do for missing paint, because that pass writes the depth it would
  otherwise sample.
- **The gap:** the spec assumed a free bind group.
- **The reach:** slice 04 picks the home for the depth and the colour copy.
  The fog layout is the tried route; the terrain layout would need the same
  placeholder. Native mirrors whichever it picks.
- **Verdict:** open, for slice 04.
- **Confidence:** high that the limit binds.

## Slice 03: water look data

### A look row is the biome's look over the shared surface; the shore is one record

- **When:** slice 03.
- **The choice:** each `WaterLook` row in `TerrainParams.waterLooks` packs
  two things. First, its biome look (`biome.water.rivers[i]`: palette,
  opacity, shallows and streak). Second, the shared surface
  (`presentation.terrain.water`: roughness, shadow, ripples and lanes),
  repeated in every row. The water's shader reads one row. The shore's
  tuning (`presentation.terrain.shore`) shapes the bank, which the terrain
  fragment draws without knowing which river it borders. So it is one
  shared `shoreTuning` record, not per look. The alternative, a look index
  in every surface record for the terrain to read, waits for a slice that
  needs a bank per look.
- **The gap:** the spec said the rows come "from the biome data and
  `presentation.terrain`" without saying how the two combine.
- **The reach:** a later look slice makes a shared number per look by moving
  it from `presentation.terrain.water` into the biome's looks, a data change
  plus one packing line. Rows are packed in Rust (`tables::water_looks`) and
  hand-mirrored in `terrainMaterial.ts`, as the rest of `TerrainParams` is.
  The browser now refuses a table whose length differs from the struct's.
- **Verdict:** sound.
- **Confidence:** medium.

### A cell's look rides its quad's vertex colour

- **When:** slice 03.
- **The choice:** water cells are now `WaterCell {corner, surface_z, side,
  look}`, five floats each. The water's mesh carries the look in its vertex
  colour's red: the water takes no tint, so the channel was unused. The
  alternative was a new vertex attribute or an instanced cell record. Either
  would change the one mesh format both clients share.
- **The gap:** the spec named the record, not how the shader reads it.
- **The reach:** `mesh::water` and the water's fragment agree on red. The
  recorded mesh vectors (`ground-mesh-vectors.json`) were re-recorded with
  cells carrying look 1, from a mirror of `mesh::water` that reproduced the
  old digests first.
- **Verdict:** sound for now. A second per-cell value (a swamp's depth, say)
  should move to a real attribute.
- **Confidence:** medium.

### The pick takes the river's index, not a feature

- **When:** slice 03.
- **The choice:** `water::look(seed, river, looks)` picks among one kind's
  looks by weight. It seeds by a salt, the biome's seed and the river's
  index, with tree stands' `stream` and `weighted`, which moved to
  `crate::seed` for it. Slice 13 adds swamps by calling it with the swamps'
  looks and indices (or salt), not by a second pick. `weighted` now never
  returns an index of no weight: rounding past the last weight takes the
  last weighted index. That case was a fall-through before, which tree
  picks could also hit.
- **The gap:** the spec wrote `look(biome_seed, feature)`.
- **The reach:** the export's river stride gained `river`, its index in
  `MapDefinition.rivers` (12 floats now; readers find fields by name). The
  curve-strokes parity oracle was blessed with that column only.
- **Verdict:** sound.
- **Confidence:** high.

### Rivers weigh temperate 3 to murky lowland 1, both today's look

- **When:** slice 03.
- **The choice:** summer's two river looks are identical, today's look, so
  nothing changes on screen. Temperate is picked three times as often as
  murky lowland. Slice 04 makes them differ.
- **The gap:** decision 9 named the variants, not their odds.
- **The reach:** the wetlands map's one river draws whichever its seed picks;
  `?water_look=<row>` shows either.
- **Verdict:** sound; a taste call for the user.
- **Confidence:** low.

### The shore's lattice turns are packed as cosine and sine

- **When:** slice 03.
- **The choice:** the shore's two lattice turns were constants, which the
  shader compiler folds to exact cosines and sines. Read from a uniform, the
  GPU would compute them itself, approximately, and the bank's wander would
  move by a fraction of a pixel. Rust packs `cos` and `sin` of each, so the
  bank stays as it was.
- **The gap:** none; a consequence of the move to uniforms.
- **The reach:** `turned` takes the cosine and sine.
- **Verdict:** sound.
- **Confidence:** high.

### Live tuning rides the dev server's hot update of the biome's file

- **When:** slice 03.
- **The choice:** every battle page reads its biome from one module
  (`gameBiome.ts`). On the dev server it accepts the hot update of
  `summer.json`, and `useGameBiome` hands the new biome to the battle
  session, which rebuilds its ground through the existing `setGround` path.
  The looks then reach the GPU as uniforms, with no shader rebuilt.
  `?water_look=<row>` pins a look by zeroing the other looks' weights, so
  Rust's one pick draws it everywhere. An edit to `game.json`'s shared
  water tuning still reloads the page. The map workbench's protocol, which
  the spec named, serves the workbench's own server and its preset files,
  not a battle page.
- **The gap:** the spec asked to reuse the workbench's reload protocol.
- **The reach:** labs built on `useBattleSession` reload live. A route that
  builds its own layers from `gameBiome` keeps the value it loaded with.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 04: depth colour

### The water composites the bed, its own body and its reflection

- **When:** slice 04.
- **The choice:** the water's colour is the bed under it (`sceneColour`, at
  its pixel) times the light that survives the distance down to it,
  `exp(-e·d)`, per channel. Its lit body colour replaces what the water
  takes, and the sky's and sun's reflection is added whole. The reflection
  is `environment.shade` with no albedo. The body is the look's edge colour
  turning to its channel colour as less light survives. The water is now
  opaque over its bed: its alpha is only its one-pixel edge feather. The
  distance is along the view ray to the world's depth (its first sample),
  capped at 50 m. The water's own light is composed before the air between
  it and the eye, and the air goes on once. The environment's `shade` is now
  `air(surfaceLight(...))`, both exported, and the bed's copy already lies
  behind its own air, so the bed's share of the air is taken off again
  (Codex found the first composite counting the haze twice). A mark lies
  over the water by its coverage, once.
- **The gap:** the spec named RedGPU's formula but not how it joins our one
  lighting owner.
- **The reach:** slices 05 to 09 build on this composite: the shore fade
  (05) multiplies it, refraction (07) moves where `sceneColour` is read,
  and caustics (08) light the bed term. The looks lost `opacity` and
  `shallows_m`, which this replaces.
- **Verdict:** sound.
- **Confidence:** medium.

### Each look carries its extinction per channel

- **When:** slice 04.
- **The choice:** a look's `extinction_per_m` is three numbers, red, green
  and blue, each the `e` in `exp(-e·d)`. RedGPU's fixed ratios
  `(2.4, 0.7, 1.1 + 0.9·turbidity)` turned our beige bed a saturated green
  through mid-depth water, and teal at the banks, against decision 9's "no
  tropical turquoise". With the three as data, a green-grey look takes the
  channels nearly evenly, and a murky look takes more of the blue and reads
  brown. Summer's first values are temperate `[0.9, 0.7, 0.8]` and murky
  lowland `[1.6, 1.4, 1.8]`, over greyer, lighter palettes than slice 03's.
- **The gap:** decision 9 named the looks; the reference's ratios don't
  give them.
- **The reach:** every water look, swamps included (slice 13), tunes these
  three. They are first values, judged by eye and an unprimed critique, not
  targets.
- **Verdict:** sound.
- **Confidence:** medium on the values, high on the shape.

### What lies under the water reaches it through its fog group

- **When:** slice 04.
- **The choice:** the water reads the world's depth and a copy of `lit`
  (`FrameTargets.sceneColour`, made by `copyTextureToTexture` after the
  world pass, before the water's) through the fog layout's two new
  bindings. Only the water's own fog group (`FogGroups.water`, the painted
  faces' layer) holds them. Every group the world pass binds holds 1×1
  placeholders, because that pass writes the depth. This is slice 02's
  tried route for the four-bind-group limit. The copy is single-level;
  slice 07 adds mips when it blurs. `lit` gains `COPY_SRC`. Native mirrors
  each piece.
- **The gap:** settled slice 02's open entry.
- **The reach:** any later water texture that the world pass doesn't
  attach (the wave field, wakes) joins the water's group the same way.
- **Verdict:** sound.
- **Confidence:** medium.

### The bed is unpainted under the water, and the water takes the paint

- **When:** slice 04.
- **The choice:** the terrain's paint fades out over the water's edge
  feather, by the same distance as the water's alpha. The water draws the
  mark over its composite whole, at any depth: the bed's light under it
  shows only where no mark lies. A selected squad's ring at the river's
  edge crosses onto the water as one line (evidence in the slice).
- **The gap:** the README's sharp edge said one layer must own the paint,
  not which.
- **The reach:** refraction (slice 07) displaces only the bed term, so
  marks stay put.
- **Verdict:** sound.
- **Confidence:** high.

### "Native draws water" now means native's river matches the browser's

- **When:** slice 04.
- **The choice:** slice 00's measure, the river at least 12 bluer (CIELAB
  b*) than its bank, stopped telling water from no water once the looks
  turned green-grey and brown. Its own entry said to re-judge it, not lower
  it. The water scene now checks two things. The browser's river must
  stand at least ΔE 10 from its dry bank, against 24 to 35 measured. And
  native's river must lie within ΔE 3 of the browser's, against 0.0
  measured. A native that drew no water would show its bed, which lies 18
  to 35 from the water: measured by standing the bare shore band in for
  the missing surface, as slice 00's falsification did. The check
  immediately caught a stale shader export that native was running.
- **The gap:** slice 00's measure assumed bluish water.
- **The reach:** `WATER_LOOK=<row>` films a pinned look in the browser.
  Native reads its own biome, so a pinned run skips the native match,
  because native draws the river's own look.
- **Verdict:** sound.
- **Confidence:** high.
## Slice 01: one wind

### The record holds no clock: grass keeps the camera's

- **When:** slice 01.
- **The choice:** the contract listed "the clock" among the `Wind` record's
  fields. Grass already sways on the camera uniform's `time`, which every
  stage binds, and smoke runs on the presentation clock on the CPU. A second
  copy of the time in the environment uniform would be written every frame
  and could disagree with the camera's. So the record is static per ground:
  heading (`toward`, a level unit vector), `drift_mps`, `gust_mps` and
  `gust_per_m`, 32 bytes, appended to `Environment` after the haze.
- **The gap:** the spec named the clock without saying whose.
- **The reach:** a later wind that changes over the battle (a gusting
  heading, a storm) writes the record per frame with the view; nothing else
  moves.
- **Verdict:** sound.
- **Confidence:** high.

### The biome's `wind` block sits at its top level; grass keeps a `sway` block

- **When:** slice 01.
- **The choice:** the block is `wind: { heading_deg, drift_mps, gust_m,
  gust_mps }` at the biome's top level, beside `scars`, because no single
  layer owns it. Grass's response moved from `grass.wind` to `grass.sway`
  (`lean`, `gust`, `flutter`, `flutter_hz`), and the shader's response
  tuning in `game.json` (`presentation.grass.wind`: where a gust starts, the
  flutter's phase and its sideways share) was renamed `sway` with it, so
  `wind` names only the one wind. `wind.rs` owns the block's range checks.
- **The gap:** delegated to the implementer.
- **The reach:** a second biome writes its own `wind` block; a biome without
  one is refused.
- **Verdict:** sound.
- **Confidence:** high.

### The drift keeps its old speed and turns to the grass's heading

- **When:** slice 01.
- **The choice:** the old smoke wind was `[0.6, 0.3]` m/s: 0.67 m/s toward
  26.6°. The new block drifts at `drift_mps: 0.67` toward the grass's 35°,
  so smoke turns 8.4° and keeps its speed (decision 26). Gusts keep 45 m
  spacing at 5 m/s, which is a different quantity from the drift.
- **The gap:** decision 26 fixed the heading but not the drift's speed.
- **The reach:** retuning the drift is one number in the biome.
- **Verdict:** sound.
- **Confidence:** high.

### Without a ground the air is calm

- **When:** slice 01.
- **The choice:** the wind is the ground's, so a presentation without a
  presented ground draws smoke and dust rising straight up, and the
  environment's wind words are zero. Before, every presentation drifted on
  the effects' fixed wind. The effects take the biome's block as an option
  (`EffectFrame::build`) and read the record's f32 direction, the one the
  shaders read.
- **The gap:** the spec didn't say what a groundless frame does.
- **The reach:** labs and tests that draw effects with no ground see no
  drift. Native always has a ground.
- **Verdict:** sound.
- **Confidence:** medium. A lab that judges smoke with no ground would see
  it change.

### The browser uploads the wind where the presented ground arrives

- **When:** slice 01.
- **The choice:** `battleFrame.ts`'s `setPresentedGround` hands the ground's
  `Wind` record (`ground_wind`) to the environment frame (`setWind`), beside
  handing the ground to the world pass. The world pass's `setGround` fan-out
  was the other candidate, but it doesn't own the environment, and
  `worldPass.ts` is over the size budget. Native passes the record to
  `Environment::new` once, with the light, after it resolves the ground.
- **The gap:** the spec said "the way the light records are uploaded", and
  the light is fixed at startup while the ground arrives later.
- **The reach:** the wind follows the presented ground even before the
  world's layers arrive; until then nothing draws grass.
- **Verdict:** sound.
- **Confidence:** high.

### Grass's environment and fog groups swapped slots

- **When:** slice 01.
- **The choice:** the grass vertex stage now reads the environment
  (`environmentWind()`), so TypeGPU orders the grass pipelines' groups
  camera, grass, environment, fog instead of camera, grass, fog,
  environment. Both clients bind by layout, so neither needed a change.
- **The gap:** the spec said no bind group gains an entry, which holds; it
  didn't foresee the reordering.
- **The reach:** none outside the export.
- **Verdict:** sound.
- **Confidence:** high.

### The effect vectors gained a bless path and a `wind` input

- **When:** slice 01.
- **The choice:** `tests/effects.rs` had no way to re-record its vectors.
  `BLESS_PARITY=1` now rewrites each frame's instances, and only those, as
  `poses.rs` and `fog_sight.rs` do; lights and counts must still hold. The
  vectors' inputs gained the biome's `wind` block in place of the effects'
  `wind_mps`. The grass vectors moved only their input block (`wind` to
  `sway`); their recorded words are unchanged. `exact-numbers.json` moved
  only by the renamed and deleted keys; its digests are unchanged.
- **The gap:** the spec asked for a re-record with no path to record by.
- **The reach:** any later change that moves smoke re-records with one
  command.
- **Verdict:** sound.
- **Confidence:** high.

### Two over-budget files made room by moving a concept to its owner

- **When:** slice 01.
- **The choice:** the size ratchet refuses any growth in a file over budget,
  measured against the commit before. A prior split that leaves the file
  over budget doesn't help. `effects/frame.rs` was split under budget in its
  own commit: drawing to `frame/draw.rs` and the pool's spawning to
  `frame/pool.rs`. `presentation.rs` is far over budget (72 KB), so the wind
  commit itself moves `aircraft_shadow`, the sun-thrown shadow's position,
  to its owner `shadow.rs`, and the file shrinks. `biome.rs` shrinks
  because the wind's checks moved to `wind.rs`.
- **The gap:** the ratchet's rule against the spec's touch list.
- **The reach:** the next change to `presentation.rs` faces the same wall;
  it needs a real split.
- **Verdict:** sound, but the next change pays.
- **Confidence:** high.

## Slice 05: the soft shore

### The water's edge is its depth; the bank's own bands stay

- **When:** slice 05.
- **The choice:** the water fades in from where its depth is zero, over
  `presentation.terrain.water.shore_fade_m` (0.25 m), or over three pixels
  seen at the pixel's slant where that is wider. This replaces its analytic
  edge feather (the old `waterSurface`, now `waterStreaks`, which keeps
  only the streaks of light). The bank's wet silt and bare earth, and the
  bed's colour under the water, are the ground's look on its own side of
  the waterline. They stay, though the contract listed "the shore and bed
  bands" for deletion. Deleting them would run grass to the water's edge
  with no bank, and the river scene's check of those bands, which the
  slice didn't expect to fail, still says what the player should see.
- **The gap:** the contract's deletion list mixed the water's edge with the
  ground's.
  A mark is its own layer over the faded water, covering by
  `p + edge·(1 − p)`, so a ring crossing the bank keeps one strength (Codex
  caught it dimming in the shallows, where the bed is unpainted).
- **The reach:** swamps and puddles (slices 13, 14) take their edge from
  this fade, whatever their shape. The ground still paints and colours by
  the simulation's analytic distance, so the two can sit up to a triangle
  apart where the 4 m grid crosses the water.
- **Verdict:** sound; a call the user may overrule.
- **Confidence:** medium.

### The fade's footprint is taken at the pixel's centre

- **When:** slice 05.
- **The choice:** the "three pixels" floor measures a pixel from the water's
  surface point rebuilt at the pixel's centre (`atPixel`, from `clip` and
  `invViewProj`). The world varying is interpolated at the centroid, and
  its derivatives step on a partly covered pixel. That drew the water
  cells' triangle edges as faint lines in the shallows, which an unprimed
  critique caught. The slant floor is 0.35, not RedGPU's 0.12: theirs
  widened the shallows into a milky band at a low camera, where the light's
  path through the water already lengthens on its own. The bed's distance
  is now the mean over the depth's four samples, as its colour is their
  resolve.
- **The gap:** the reference has no MSAA and no centroid varying.
- **The reach:** any other water term that reads a footprint near its
  triangles' edges should measure it the same way; the streaks and ripples
  still use the centroid one (slice 06 replaces them).
- **Verdict:** sound.
- **Confidence:** medium.

### The river scene's shore checks stand unchanged

- **When:** slice 05.
- **The choice:** the slice expected the river scene's shore-placement and
  flicker checks to fail and need rewriting. Both passed. Re-judged, they
  still say what the player should see: the bank's bands at the
  simulation's distances, and a waterline that doesn't flicker as the
  camera moves by a centimetre. The depth fade can't fight the ground the
  way the old edge could. No check was rewritten or loosened. A check
  that the edge "sits where the depth reaches zero" would restate the
  shader's formula.
- **The gap:** the spec predicted a failure that didn't come.
- **The reach:** none.
- **Verdict:** sound.
- **Confidence:** medium.
Decisions the implementing agent made where the plan was silent or wrong
against the code. Each says what was chosen, why, and how sure the agent is.
[`decisions.md`](decisions.md) still holds the user's givens.

## Slice 10: swamp terrain

1. **A swamp's outline is a polygon.** Admission refuses a stroke. *Why:* a
   swamp is an area, and a polygon gives an exact distance in from its edge,
   which the cut, the membership and presentation all read. A stroke's
   square-cut ends would need their own distance rule. *Confidence:* high.
2. **The water stands at one level per swamp, and the land under it must be
   level.** The level is the land's height at the outline's first point.
   Admission refuses a swamp where any height sample it cuts, or a triangle
   it moves, stands at another height. *Why:* decision 1 puts the water flat
   at the old ground level, which only exists when the land is level; one
   level makes the water a plane, as a river's is, for `water_hit` and for
   drawing. Generated maps have no relief, so mapgen loses nothing.
   *Confidence:* high.
3. **The edge eases in over three height samples (12 m on a 4 m grid) by a
   smoothstep.** Admission refuses a depth whose steepest grade (1.5 × depth
   over the edge's width) exceeds `steepest_grade`. *Why:* decision 21; the
   smoothstep has no kink at either end, so no triangle is steeper than the
   curve's middle. *Confidence:* high for the shape, medium for the width,
   which slice 13 may want wider on screen.
4. **A road ends the cut a triangle clear of its side, and eases in from
   there as from the outline.** The cut is the smoothstep of the lesser of
   the distance in from the outline and the distance from the nearest road
   less one cell diagonal. *Why:* a cut sample lowers every triangle it is a
   corner of, so a cut that started at the road's edge would tilt the road's
   own sides; easing in keeps the causeway's sides as gentle as the outline's.
   Road distance is the exact distance to a stroke's stretches, square ends
   included (`contract::ground::stretch_distance`). *Confidence:* high.
5. **A road across a swamp is dry ground.** `Surface.swamp` and
   `water_hit` exclude road surfaces: the causeway takes marks and rounds
   land on it. *Why:* decision 4 makes the road a causeway; standing water
   would otherwise cover it at its own height. *Confidence:* high.
6. **Minimum size: some height sample lies (1.5 − √½) cells inside the
   outline.** *Why:* any swamp holding a disc three cells across has such a
   sample wherever the grid falls on it (decision 22), and a sliver much
   narrower does not. *Confidence:* medium; the threshold is a loose
   tripwire, not a target.
7. **River gap: no height sample within an edge's width and one more sample
   of the outline is river water or river-cut bank.** *Why:* a swamp's rim
   then stands on the land's own height on every side, and the carve's two
   owners never meet. *Confidence:* medium on the exact width.
8. **Buildings and bridge approaches: no height sample the swamp cuts lies
   within a cell diagonal of a building part's footprint, or of a bridge deck
   lengthened by an edge's width at each end.** *Why:* no triangle under them
   is then cut, so their absolute `base_z` and the deck's ramp stay true.
   *Confidence:* high.
9. **Overlapping swamps are allowed;** the deeper cut wins at each point.
   *Why:* level land makes their water the same height, so nothing needs
   refusing. *Confidence:* medium.
10. **The export is one stride per outline edge:** `ax, ay, bx, by, swamp,
    surfaceZ, depth` (`SWAMP_STRIDE`, `swampFields` in the layout). *Why:*
    presentation rebuilds each outline and its membership from it, and
    `swamp` is the index slice 03's variant pick seeds from. *Confidence:*
    high.
11. **`NavGrid::swamp_at` is a public query on what the planner reads.**
    Today only the seam test calls it; slice 11's cost reads `Cell.swamp`
    directly. *Why:* the slice's seam test compares the planner's cell with
    movement's surface, and navigation keeps its cells private. Delete it in
    slice 11 if the planning tests there make it redundant. *Confidence:*
    medium.
12. **Decision 21's sight test is run on the ground sight sweep, not on
    identification.** With water at the land's level stopping sight
    (decision 32), a line between two points above the water never passes
    under the rim, so the dip cannot hide a target that stands above its
    water, and one whose points lie under the water is hidden from everyone.
    The terrain-only sweep (the fog) does show the mechanic: on a 3 m test
    dip, a far observer on flat ground loses the dip's floor and a near or
    high one sees it. *Why:* decisions 21 and 32 cannot both hold for unit
    identification; 32 is the slice contract's one water rule. *Confidence:*
    high on the geometry. Whether a deep swamp should hide units from far
    observers at all is the user's call.
13. **The rays (`raycast`, `sight_clear`, `water_hit`) moved to
    `world/rays.rs`, and mapgen's plan types to `mapgen/src/plan.rs`,** each
    in its own commit with no behaviour change. *Why:* both files were over
    the size budget and slice 10 had to touch them. *Confidence:* high.

## Slice 11: swamp slowdown

1. **First shipped factors: foot 0.5, wheeled 0.3, tracked 0.55**
   (`movement.swamp` in `fixtures/game.json`). *Why:* wading thigh-deep
   halves a soldier's pace, trucks bog worst, tanks churn through a little
   faster than a soldier wades. They are tuning, not requirements; the
   paired battle report is still owed (see below). *Confidence:* low, by
   design.
2. **`Footing { road_factor, forest, swamp, slope_deg }` lives beside
   `Mobility` and is built by `Surface::footing()`, `Cell::footing()` and a
   road `Arc::footing()`.** *Why:* each owner of a piece of ground says what
   a mover's speed reads of it; the five read sites shrank rather than grew,
   which the size budget on `soldier.rs`, `journey.rs` and `navigation.rs`
   needed. *Confidence:* high.
3. **Off a road the speed is open speed × min(forest factor, swamp
   factor),** each 1 where its ground is absent. *Why:* decision 4's "the
   slower applies"; for ground with neither it is bit-identical to the old
   arithmetic, so only the rules hash moves digests. *Confidence:* high.
4. **`SlowGround` keeps three counts per tile off any road: forest only,
   swamp only, both,** and the estimate weighs "both" by the slower factor.
   The expression keeps the old one's order, so a map with no swamp
   estimates bit for bit as before. *Confidence:* high.
5. **`NavGrid::swamp_at` is deleted.** The planning seam test now compares
   each cell's planned cost with movement's speed at its middle
   (`ground_speed::planning_costs_each_cell_at_the_speed_movement_takes_there`),
   which is the claim the query stood in for. *Confidence:* high.
6. **`MovementRules` and `SwampRules` refuse unknown keys; `DriveRules`
   does not yet.** *Why:* the slice named `MovementRules`; the drive row is
   unchanged here. *Confidence:* medium.
7. **The dense reference planner in `examples/navigation_quality` reads
   `sim::navigation::Mobility` and its `speed`;** its road cells are road
   factor 1 and never swamp. *Why:* the slice deletes its private copy.
   *Confidence:* high.
8. **Ground-speed tests live in `tests/ground_speed.rs`,** with the old
   `speed_follows_surface_and_slope` folded in as a slope test. *Why:*
   `tests/navigation.rs` was over the size budget, and speed over ground is
   one concept. *Confidence:* high.
9. **No paired battle report on the `wetlands` map yet.** That map comes
   from slice 00 in the other lane; run the report once it is merged.

## Slice 06: wind-driven normals

### The normal map is a sum of whole-cycle waves, baked once beside the terrain's tables

- **When:** slice 06.
- **The choice:** `waterNormalsBake` (`frame/waterNormals.ts`) draws a
  256×256 `rgba16float` slope map once at load. It sums 24 waves, each
  turning a whole number of times across the tile (spread by the golden
  angle, two to about twelve cycles, shorter ones lower), so it tiles at any
  scale. That is a formula, with no seeded randomness. The terrain source
  owns it, and the terrain layout binds it with a repeat sampler
  (`waterNormals`, `waterTiles`), because the water's surface material
  already reads that layout. Native replays the same exported bake into its
  own texture (`water_normals.rs`).
- **The gap:** decision 16 named the bake, not the generator or its owner.
- **The reach:** swamps and puddles (slices 13, 14) sample the same map.
  The wave field (16) adds its own texture beside it.
- **Verdict:** sound.
- **Confidence:** medium.

### No mips: each layer flattens as a pixel outgrows its texels

- **When:** slice 06.
- **The choice:** the map has one level. Each of the three layers (tiles of
  9, 31 and 2.2 m, turned 0.6 and −1.1 rad off the primary) leans the
  normal by `strength` (0.35). Each flattens as a pixel spans 1.5 to 6 of
  its texels, so a layer finer than the pixel fades instead of aliasing.
  The layers drift downwind at 0.6, 0.3 and 1.4 of the wind's drift, on the
  camera's clock. These are first values in
  `presentation.terrain.water.normals`. An unprimed critique of the first
  tuning found glints blown into blobs and a soft near field; this tuning
  breaks them up.
- **The gap:** the implementer's to pick.
- **The reach:** mips would keep detail at a distance, as averaged
  roughness; add them with slice 07's scene-colour mips if far water reads
  flat.
- **Verdict:** sound for now.
- **Confidence:** medium.

### No vertex waves

- **When:** slice 06.
- **The choice:** vertex waves were not built. RedGPU's are centimetres
  high, and at the low framing (80 m out) a pixel spans several
  centimetres at the near water and metres at the far, so a vertex swell
  would move the surface by less than a pixel. The normals carry the waves.
- **The gap:** the slice said to judge them at the low framing.
- **The reach:** wakes (slice 16) displace no vertices either; they are a
  normal field.
- **Verdict:** sound; judged by measure, not by a build.
- **Confidence:** medium.

### The streaks of light stay, for slice 09

- **When:** slice 06.
- **The choice:** the streaks laid in lanes along the stream (now
  `waterStreaks`) are not ripples, so this slice didn't delete them. From
  a steep camera they still own the water's look, which the critique read
  as brushed metal. Slice 09's highlights decide whether they stay.
- **The gap:** none; a scope line.
- **The reach:** slice 09.
- **Verdict:** sound.
- **Confidence:** high.

### Motion is judged in the graphics test, whose clock runs

- **When:** slice 06.
- **The choice:** scene runs hold the wall clock, so a lab's water stands
  still even as its ticks advance (the presentation clock interpolates on
  wall time). The motion pair came from a throwaway two-shot water reel in
  the graphics test, its plates 0.49 s apart. Pausing freezes the water by
  the same rule, since the clock stops with the battle.
- **The gap:** the slice assumed a lab could step the presentation clock.
- **The reach:** slice 16's wave field must be judged the same way.
- **Verdict:** sound.
- **Confidence:** medium.
