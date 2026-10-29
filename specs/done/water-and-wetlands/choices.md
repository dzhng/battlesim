# Choices

Decisions made during implementation where the spec was silent or wrong
against the code. [`decisions.md`](decisions.md) holds the user's givens; this
file holds what the build decided on their behalf. The
[README](README.md) is the rationale record.

Entries are grouped by the slice that made them, in slice order. Each names
when it was made and the choice, with its reason. Where known, it also names
the gap (what the plan left open or got wrong), the reach (what it touches
later), a verdict and a confidence. The sim lane (slices 10 to 12) and slice
17 recorded no verdicts; their entries carry a verdict only where the entry
itself settled one.

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
- **Superseded in slice 17:** the wind rides in the camera; `ground_wind`,
  `setWind` and native's argument are gone ([slice 17](#the-wind-moved-from-the-environment-uniform-into-the-camera-record)).

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
- **Superseded in slice 17:** grass reads `cameraWind()` and its groups
  are camera, grass, fog, environment again.

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

## Slice 02: the water pass

### The water stays multisampled, in its own pass after glass

- **When:** slice 02.
- **The choice:** decision 25 kept single-sampled water unless the probe
  found storing the world's samples cheap. It did: whole-frame GPU time with
  the samples kept was within run-to-run noise of single-sampled water, on
  High and Low ([README](README.md#measured)). So the world pass now stores its samples,
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
  edge crosses onto the water as one line (a throwaway probe in the river lab).
- **The gap:** the spec's sharp edges said one layer must own the paint,
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
  A mark is its own layer over the faded water, covering by
  `p + edge·(1 − p)`, so a ring crossing the bank keeps one strength (Codex
  caught it dimming in the shallows, where the bed is unpainted).
- **The gap:** the contract's deletion list mixed the water's edge with the
  ground's.
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

## Slice 07: refraction, and the Low tier

### The bed shifts in metres along the slope, then is projected

- **When:** slice 07.
- **The choice:** `waterUnder` (`frame/waterUnder.ts`) moves the bed
  point along the surface's slope (`n.xy`) by a share of the depth it is
  seen through, capped at `depth_m`. It projects that world shift to the
  screen and clamps it to `most` of the screen's height. RedGPU instead
  scales the normal's view-space delta by depth over camera distance in UV.
  Projecting a world shift gets the perspective falloff from the camera
  itself, and keeps the one number (`share`) a physical ratio. The shift
  is dropped where its sample lies within 0.15 m in front of the water's
  own surface (a bank, a pier), and where the fog mask copy says the sample
  is less seen than the pixel (ramped over half a pixel's unseen coverage).
  Extinction reads the refracted distance; the soft edge keeps the straight
  depth (as RedGPU does).
- **The gap:** delegated (the scale and clamp).
- **The reach:** caustics (slice 08) light the bed this reads; the wave
  field (16) bends the same normal.
- **Verdict:** sound.
- **Confidence:** medium.

### The shift is eight times the physical one

- **When:** slice 07.
- **The choice:** `presentation.terrain.water.refraction` is
  `{ share: 2, depth_m: 2.5, most: 0.015, blur_m: 1 }`. The physical shift
  through water is about a quarter of the depth times the slope, which at
  the close framing (65 m) moves the bed about a pixel: refraction didn't
  read at all. At 2 the shallows break into a wobbling bed; at the
  tactical framing (260 m) it stays under a pixel. A Hollywood choice, by
  eye.
- **The gap:** the implementer's to pick.
- **The reach:** every look shares it; a look of its own would move it
  into the biome's `WaterLook`.
- **Verdict:** sound for now.
- **Confidence:** medium.

### The bed's mips are a 2×2 box drawn by an exported pipeline

- **When:** slice 07.
- **The choice:** `FrameTargets.sceneColour` carries four levels where the
  water refracts. `sceneColourMip` draws each from the one before, with a
  bilinear tap at the shared corner of four texels. Native replays the same
  pipeline (`world::Water`), and reads the level count from the export's
  `sizes` (`sceneColourMips`), as Codex asked, so it can't drift. The bed blurs a level for every metre of water
  it is seen through (`blur_m`), by at most three. Against the same run
  with no blur, the blur moved no visible pixel: the bed shows only in
  the shallows, and the bed there has no detail yet to soften.
- **The gap:** delegated (how mips are generated).
- **The reach:** slice 06's note: the normal map could take averaged
  roughness from mips like these if far water reads flat.
- **Verdict:** sound, but the blur is unproven until the bed has detail.
- **Confidence:** low on the blur's value; high on the mechanism.

### Low's word is `TerrainParams.view.z`; the targets carry the rest

- **When:** slice 07.
- **The choice:** `GraphicsQualityProfile.waterDetail` (`"depth"` on Low
  and the test-only minimum, `"full"` on Medium and High) reaches
  `BattleFrameOptions.quality`. An omitted quality draws `full`, as it
  draws High. `WaterDetail` lives in `frame/targets.ts`. The battle frame
  hands it to the terrain source, which writes `view.z` (1 for `depth`),
  and to the targets, which allocate the mips and the fog mask copy
  (`fogMaskUnder`) only for `full`. The water pass copies and builds mips
  only when that copy exists. The shader's `depth` branch reads the copy at
  the pixel exactly as slice 06 did: the frame scene's Low baselines (river,
  geometry) differ by 0 px. The 1×1 stub is the fog's existing `noScene`
  placeholder. Native draws `full` (its view word is zero).
- **The gap:** the slice named a uniform word, not where; `view.z` was
  the free one.
- **The reach:** slices 08 and 16 read the same word, and skip their
  passes where the targets say the water doesn't refract.
- **Verdict:** sound; one setting, read in two places from one value.
- **Confidence:** medium.

### The fog mask copy and the bed's sampler ride the fog layout

- **When:** slice 07.
- **The choice:** `fogLayout` gained `sceneTaps` (a linear sampler) and
  `seenUnder` (the fog mask before the water), beside slice 04's `depth`
  and `sceneColour`, for the same reason: a pipeline binds four groups and
  the water binds four. The world pass's groups bind placeholders.
- **The gap:** none; slice 02's rule.
- **The reach:** caustics' and wakes' inputs must fit the same groups.
- **Verdict:** sound, but the fog layout is filling with water's inputs.
- **Confidence:** medium.

### Slice 06's saved plates weren't the before

- **When:** slice 07.
- **The choice:** the saved slice 06 plates were taken before its last
  retune, so they differed from this slice even with refraction off, most
  in the low shot's glints. Before and after were judged against a run of
  this build with no shift and no blur, which is slice 06's water. The
  graphics test's plates repeat exactly from run to run, so the pairs are
  comparable.
- **The gap:** a process slip, caught by the paired run.
- **The reach:** keep the before as a run of the slice's own build with
  its feature off.
- **Verdict:** sound.
- **Confidence:** high.

### Cost is judged as a pair on High, not as Low against High

- **When:** slice 07.
- **The choice:** Low and High differ in render scale, grass and shadows,
  so their totals say nothing about the water. The probe measured High
  with refraction against High with it off, interleaved (eight arms of
  120 frames each, at the tactical framing in the river lab): 9.17 ±
  0.72 ms against 9.24 ± 0.84 ms, within noise. Low on the same run:
  10.47 ± 0.64 ms, slower than High as slice 02 found, with other load on
  the machine.
- **The gap:** the slice asked for Low against High.
- **The reach:** slice 08 measured the same pair; slice 16's wave field was not measured.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 08: caustics

### Four taps of the surface's own map, drifting with its layers

- **When:** slice 08.
- **The choice:** `waterCaustics` (`frame/waterCaustics.ts`) samples the
  baked normal map four times, as RedGPU does: two scales (1 and 1.8 of
  the pattern's tile), each bending the other's coordinates by 0.18 of a
  tile, bright where their slopes agree along either diagonal. They drift
  with the primary and detail normal layers' shares of the wind
  (`waterCarried`, now one helper the normals read too), because the
  surface's own waves make the caustics. They light the bed seen down the
  view, from where the sun's ray came down through the surface, its slant
  taken as three quarters of the dry one for the water's bend. The light
  scales the bed's own (`bed × (1 + caustics × sun)`), so it stops in the
  surface's shadow and takes the bed's colour.
- **The gap:** delegated (the tap count).
- **The reach:** puddles and swamps (slices 13, 14) can call the same
  function where their water is clear.
- **Verdict:** sound.
- **Confidence:** medium.

### The caustics fade by the finest wave, not by texels

- **When:** slice 08.
- **The choice:** the pattern fades as a pixel spans a quarter to a half
  of the finer layer's finest wave (`WATER_NORMALS_FINEST`, the bake's
  12.4 cycles a tile). Slice 06's texel-based fade, used first, put a
  pixel over 13 texels at the close framing and drew no caustics at all.
  A crest narrower than half its pixel widens to it and dims in step
  (`crest`), which took out the crawling lines at the low view. They fade
  with depth as RedGPU's do (`fade_per_m` 0.85, gone in the first
  centimetre and whole by 8 cm), fade off walls from about 45° to 72°,
  and weaken as the sun lowers.
- **The gap:** delegated (the depth fade).
- **The reach:** none yet; the wave field (16) adds finer waves the fade
  must count.
- **Verdict:** sound.
- **Confidence:** medium.

### The bed is rebuilt from the straight depth

- **When:** slice 08.
- **The choice:** the caustics' bed point and its steepness come from the
  water's straight depth, not the refracted distance. Refraction moves each
  pixel's bed sample on its own, and the steep-wall mask's derivatives of
  that point flecked the low view with dark single pixels.
- **The gap:** none; a defect the critique found.
- **The reach:** none.
- **Verdict:** sound.
- **Confidence:** high.

### Cells several times larger than real, so they read from 65 m

- **When:** slice 08.
- **The choice:** `presentation.terrain.water.caustics` is
  `{ tile_m: 24, strength: 1.2, fade_per_m: 0.85, sharpness: 5 }`, so a
  cell spans a metre or so. A real stream's cells are a hand across, which
  at the close framing is under a pixel: at a 16 m tile an unprimed
  critique still read them as foam or fine texture at full scale, and at
  an 8 m tile they were sparkle. A Hollywood choice. At the tactical
  framing (260 m) they fade out: no pixel there can hold them.
- **The gap:** the implementer's to pick.
- **The reach:** every look shares them; a murky look could weaken them
  through a look of its own.
- **Verdict:** sound for now.
- **Confidence:** medium.

## Slice 09: Fresnel and sun highlights

### Water's F0 through the shared light is half metal on a black base

- **When:** slice 09.
- **The choice:** `standardPbr` fixes a dielectric's F0 at 0.04. Water's is
  0.02, and on a black base a metalness of a half gives exactly that, in the
  sun's lobe and the sky's (the DFG terms mix the same way), with no diffuse
  light. So the water's reflection (`waterReflection`, `frame/waterLight.ts`)
  stays inside the one light owner: no F0 parameter and no second model.
  The body's own light is still a dielectric's lit surface less its
  reflection, at metalness 0.
- **The gap:** the contract asked for F0 about 0.02 through the shared
  owner, without saying how.
- **The reach:** puddles and swamps (13, 14) reflect the same way.
- **Verdict:** sound; a reader of `WATER_METAL` needs its comment.
- **Confidence:** medium.

### Three lobes at RedGPU's weights; a sharper first lobe only glared

- **When:** slice 09.
- **The choice:** `presentation.terrain.water.lobes` is
  `{ roughness: [0.15, 0.22, 0.32], weights: [0.58, 0.34, 0.08] }`,
  RedGPU's steps over our old single roughness (0.14), replacing it. A
  first lobe of 0.07 put the sun in one bright hotspot with a bloom halo,
  not finer sparkles: the glints' size is the normal map's, whose fine
  layer has faded flat at 80 m. Each lobe is a full `surfaceLight` call,
  three where there was one.
- **The gap:** delegated (the lobe weights and roughness steps).
- **The reach:** finer glints need normal detail that survives distance
  (slice 06's mip note).
- **Verdict:** sound for now.
- **Confidence:** medium.

### What comes up from under the surface loses its reflection

- **When:** slice 09.
- **The choice:** the bed's light, and the air the bed's copy already
  carries, are scaled by Schlick's transmission at F0 0.02
  (`waterTransmission`). From above nearly all of it crosses; at a grazing
  angle the surface turns to a mirror, which two critiques asked for. This
  is a weight on light the shared owner never sees, not a second lighting
  model.
- **The gap:** none; slice 04 left it out.
- **The reach:** puddles (14), whose mirror look depends on it.
- **Verdict:** sound.
- **Confidence:** high.

### The streaks of light are deleted

- **When:** slice 09.
- **The choice:** `waterStreaks`, its lanes tuning and each look's
  `streak` are gone (slice 06 left the call here). Every unprimed critique
  since slice 04 read them as brushed metal or motion blur, and with them
  off the close shot reads calmer and more like water. The tactical river
  loses the faint lines that broke its band, which slice 09's checkpoint
  weighed. The look record's freed words now hold the lobes (`roughness`,
  `lobes`) and the fades (`fades`); its size is unchanged.
- **The gap:** none; slice 06 deferred it here. Slice 03's entries
  still name `streak` as a look field, as it shipped then.
- **The reach:** none.
- **Verdict:** sound.
- **Confidence:** medium.

### No sun in-scatter term beyond the body's own light

- **When:** slice 09.
- **The choice:** RedGPU adds volume in-scatter and a low-sun forward
  scatter. Ours already lights the water's body by the sun and the sky in
  place of what the depth takes (`own`), which is that in-scatter; a
  forward-scatter glow was not built, since no framing looks through a
  wave toward a low sun.
- **The gap:** the contract said "if it reads".
- **The reach:** none.
- **Verdict:** sound.
- **Confidence:** medium.

### No screen-space reflections (decision 15)

- **When:** slice 09.
- **The choice:** at the low framing the surface reflects the sky's
  environment, not the far bank or its trees. SSR would mirror the bank at
  the low cinematic angle only; from the tactical height, where play
  happens, a reflected bank is a few percent of a dark band. It would need
  the world's colour and depth at full resolution in the water's pass, and
  a ray march per pixel. It doesn't earn that now; the [README](README.md#dead-ends) records it.
- **The gap:** decision 15 asked for the call.
- **The reach:** a later cinematic pass could add it for the menu reel.
- **Verdict:** sound.
- **Confidence:** medium.

## Slice 10: swamp terrain

### A swamp's outline is a polygon

- **When:** slice 10.
- **The choice:** admission refuses a stroke. A swamp is an area, and a
  polygon gives an exact distance in from its edge, which the cut, the
  membership and presentation all read. A stroke's square-cut ends would
  need their own distance rule.
- **The gap:** delegated (the swamp shape's representation, within
  `GroundShape`).
- **Confidence:** high.

### The water stands at one level per swamp, and the land under it must be level

- **When:** slice 10.
- **The choice:** the level is the land's height at the outline's first
  point. Admission refuses a swamp where any height sample it cuts, or a
  triangle it moves, stands at another height. Decision 1 puts the water
  flat at the old ground level, which only exists when the land is level;
  one level makes the water a plane, as a river's is, for `water_hit` and
  for drawing. Generated maps have no relief, so mapgen loses nothing.
- **Confidence:** high.

### The edge eases in over three height samples by a smoothstep

- **When:** slice 10.
- **The choice:** the edge spans three height samples (12 m on a 4 m grid).
  Admission refuses a depth whose steepest grade (1.5 × depth over the
  edge's width) exceeds `steepest_grade`. This is decision 21; the
  smoothstep has no kink at either end, so no triangle is steeper than the
  curve's middle.
- **The gap:** delegated (the edge falloff curve).
- **The reach:** slice 13 may want the edge wider on screen.
- **Confidence:** high for the shape, medium for the width.

### A road ends the cut a triangle clear of its side, and eases in from there

- **When:** slice 10.
- **The choice:** the cut is the smoothstep of the lesser of the distance in
  from the outline and the distance from the nearest road less one cell
  diagonal, eased in as from the outline. A cut sample lowers every triangle
  it is a corner of, so a cut that started at the road's edge would tilt the
  road's own sides; easing in keeps the causeway's sides as gentle as the
  outline's. Road distance is the exact distance to a stroke's stretches,
  square ends included (`contract::ground::stretch_distance`).
- **Confidence:** high.

### A road across a swamp is dry ground

- **When:** slice 10.
- **The choice:** `Surface.swamp` and `water_hit` exclude road surfaces: the
  causeway takes marks, and rounds land on it. Decision 4 makes the road a
  causeway; standing water would otherwise cover it at its own height.
- **Confidence:** high.

### Minimum size: some height sample lies (1.5 − √½) cells inside the outline

- **When:** slice 10.
- **The choice:** any swamp holding a disc three cells across has such a
  sample wherever the grid falls on it (decision 22), and a sliver much
  narrower does not.
- **The gap:** delegated (the minimum size).
- **Verdict:** the threshold is a loose tripwire, not a target.
- **Confidence:** medium.

### River gap: no sample within an edge's width and one more of the outline is river water or bank

- **When:** slice 10.
- **The choice:** no height sample within an edge's width and one more
  sample of the outline is river water or river-cut bank. A swamp's rim then
  stands on the land's own height on every side, and the carve's two owners
  never meet.
- **The gap:** delegated (the exact gap from river edges).
- **Confidence:** medium on the exact width.

### Buildings and bridge approaches keep a cell diagonal from any cut sample

- **When:** slice 10.
- **The choice:** no height sample the swamp cuts lies within a cell
  diagonal of a building part's footprint, or of a bridge deck lengthened by
  an edge's width at each end. No triangle under them is then cut, so their
  absolute `base_z` and the deck's ramp stay true.
- **Confidence:** high.

### Overlapping swamps are allowed; the deeper cut wins at each point

- **When:** slice 10.
- **The choice:** level land makes their water the same height, so nothing
  needs refusing.
- **Confidence:** medium.

### The export is one stride per outline edge

- **When:** slice 10.
- **The choice:** `ax, ay, bx, by, swamp, surfaceZ, depth` (`SWAMP_STRIDE`,
  `swampFields` in the layout). Presentation rebuilds each outline and its
  membership from it, and `swamp` is the index slice 03's variant pick seeds
  from.
- **Confidence:** high.

### `NavGrid::swamp_at` is a public query on what the planner reads

- **When:** slice 10.
- **The choice:** only the seam test called it; slice 11's cost reads
  `Cell.swamp` directly. The slice's seam test compares the planner's cell
  with movement's surface, and navigation keeps its cells private.
- **The reach:** delete it in slice 11 if the planning tests there make it
  redundant.
- **Verdict:** superseded in slice 11, which deleted it
  ([`NavGrid::swamp_at` is deleted](#navgridswamp_at-is-deleted)).
- **Confidence:** medium.

### Decision 21's sight test runs on the ground sight sweep, not on identification

- **When:** slice 10.
- **The choice:** with water at the land's level stopping sight (decision
  32), a line between two points above the water never passes under the
  rim. So the dip cannot hide a target that stands above its water, and one
  whose points lie under the water is hidden from everyone. The
  terrain-only sweep (the fog) does show the mechanic: on a 3 m test dip, a
  far observer on flat ground loses the dip's floor, and a near or high one
  sees it. Decisions 21 and 32 cannot both hold for unit identification; 32
  is the slice contract's one water rule.
- **The reach:** whether a deep swamp should hide units from far observers
  at all is the user's call.
- **Confidence:** high on the geometry.

### The rays and mapgen's plan types moved to their own files first

- **When:** slice 10.
- **The choice:** the rays (`raycast`, `sight_clear`, `water_hit`) moved to
  `world/rays.rs`, and mapgen's plan types to `mapgen/src/plan.rs`, each in
  its own commit with no behaviour change. Both files were over the size
  budget, and slice 10 had to touch them.
- **Confidence:** high.

## Slice 11: swamp slowdown

### First shipped factors: foot 0.5, wheeled 0.3, tracked 0.55

- **When:** slice 11.
- **The choice:** `movement.swamp` in `fixtures/game.json`. Wading
  thigh-deep halves a soldier's pace, trucks bog worst, and tanks churn
  through a little faster than a soldier wades.
- **The gap:** delegated (the first shipped values).
- **Verdict:** tuning, not requirements. The paired crossing below measured
  them.
- **Confidence:** low, by design.

### One `Footing` value, built by each owner of a piece of ground

- **When:** slice 11.
- **The choice:** `Footing { road_factor, forest, swamp, slope_deg }` lives
  beside `Mobility` and is built by `Surface::footing()`, `Cell::footing()`
  and a road `Arc::footing()`. Each owner of a piece of ground says what a
  mover's speed reads of it. The five read sites shrank rather than grew,
  which the size budget on `soldier.rs`, `journey.rs` and `navigation.rs`
  needed.
- **The gap:** delegated (`Footing`'s exact shape).
- **Confidence:** high.

### Off a road the speed is open speed × min(forest factor, swamp factor)

- **When:** slice 11.
- **The choice:** each factor is 1 where its ground is absent. This is
  decision 4's "the slower applies". For ground with neither it is
  bit-identical to the old arithmetic, so only the rules hash moves digests.
- **Confidence:** high.

### `SlowGround` keeps three counts per tile: forest only, swamp only, both

- **When:** slice 11.
- **The choice:** off any road, and the estimate weighs "both" by the slower
  factor. The expression keeps the old one's order, so a map with no swamp
  estimates bit for bit as before.
- **Confidence:** high.

### `NavGrid::swamp_at` is deleted

- **When:** slice 11.
- **The choice:** the planning seam test now compares each cell's planned
  cost with movement's speed at its middle
  (`ground_speed::planning_costs_each_cell_at_the_speed_movement_takes_there`),
  which is the claim the query stood in for.
- **Confidence:** high.

### `MovementRules` and `SwampRules` refuse unknown keys; `DriveRules` does not yet

- **When:** slice 11.
- **The choice:** the slice named `MovementRules`; the drive row is
  unchanged here.
- **Confidence:** medium.

### The dense reference planner reads `sim::navigation::Mobility`

- **When:** slice 11.
- **The choice:** the dense reference planner in
  `examples/navigation_quality` reads `sim::navigation::Mobility` and its
  `speed`; its road cells are road factor 1 and never swamp. The slice
  deletes its private copy.
- **Confidence:** high.

### Ground-speed tests live in `tests/ground_speed.rs`

- **When:** slice 11.
- **The choice:** the old `speed_follows_surface_and_slope` is folded in as
  a slope test. `tests/navigation.rs` was over the size budget, and speed
  over ground is one concept.
- **Confidence:** high.

### The owed evidence is a paired crossing, not a full battle report

- **When:** slice 11.
- **The choice:** the `wetlands` water encounter's squad and tank each moved
  across the swamp alone, with the shipped factors and with every factor at
  1.0 (the table is in the [README](README.md#measured)). The encounter has
  no opposing force, so a battle report would measure nothing but this; the
  crossing isolates the one variable. Recorded, not retuned.
- **The gap:** the slice asked for a paired battle report on the
  `wetlands` map.
- **Confidence:** high on the numbers; the factors themselves stay tuning.

## Slice 12: swamps in map generation

### Swamps come in pairs: one in each half, of one drawn size

- **When:** slice 12.
- **The choice:** the halves then hold the same wet ground by construction.
  `verify` still measures the split against `swamps.fairness` and refuses a
  plan outside it, as it does woods.
- **Confidence:** high.

### Each map type's chance is `swamps.chance.<type>` in the presets' swamp section

- **When:** slice 12.
- **The choice:** not a `swamp_chance` field beside `river_chance`.
  `layout/presets.rs`, which holds the type rows, is over the size budget and
  may not grow; the swamp section owns every swamp number. Shipped chances:
  open 0.45, mixed 0.35, metro 0.2.
- **The gap:** the slice asked for a `swamp_chance` like `river_chance`, and
  delegated the chance per preset.
- **Confidence:** high on the place, low on the values (tuning).

### Placement: after towns, woods and the skirmish reservation, clear of everything a swamp would spoil

- **When:** slice 12.
- **The choice:** placement runs before `verify`, from the `swamps` stream.
  A swamp:
  - lies wholly inside its half and the map;
  - keeps `settlement_gap_m` from every settlement outline (both rings'
    vertices) and every skirmish objective (its whole radius) and entry;
  - keeps `road_gap_m` from every road's edge and `wood_gap_m` from every
    wood;
  - keeps off every reserved corridor;
  - keeps `bridge_gap_m` from a deck lengthened by its approach (a circle
    test);
  - keeps `swamp_gap_m` from other swamps;
  - with a river, lies between `river_gap_m` and `river_reach_m` of the
    water, with its long axis along the river's course there.

  The first unprimed critique of a six-seed sheet found roads running
  through swamps, swamps notched into woods, one crowding a village, and
  round ponds unrelated to the river. Generated swamps now keep off all of
  those (hand-made maps may still lay a causeway across one). The second
  critique found no placement error.
- **The gap:** delegated (the placement algorithm, and how big swamps can
  be).
- **Confidence:** medium on the gaps and sizes (tuning).

### Later passes keep off swamps through what they already keep off

- **When:** slice 12.
- **The choice:** the open country counts a swamp with the woods it never
  furnishes (`open_country/country.rs`), and the ground-sight coverage
  counts its depth in the lowest ground. Parcels and street furniture stand
  in settlements, which swamps keep clear of. No new owner, and
  `open_country/mod.rs` is over the size budget.
- **The reach:** street furniture along a rural road across a swamp could
  stand in its water; none of the corpus seeds did.
- **Confidence:** medium.

### The compiler runs `contract::swamp::validate` on the lowered map

- **When:** slice 12.
- **The choice:** it refuses with `InvalidSwamp`, so a placement rule that
  misses a building or a bridge approach is a named seed refusal, not a
  world assertion.

### Preset validation refuses a swamp the terrain cannot carve

- **When:** slice 12.
- **The choice:** it refuses a depth past `contract::swamp::deepest_m` (now
  the contract's one rule), a narrowest drawn swamp under four height
  samples across, and a river gap inside a river's cut bank plus the swamp's
  edge.
- **Confidence:** high.

### Versions: generator `layout-20`, presets `layout-presets-20`

- **When:** slice 12.
- **The choice:** plans and maps without swamps serialize as before
  (`swamps` is skipped when empty).
- **Confidence:** high.

### The deck clearance is belt and braces

- **When:** slice 12.
- **The choice:** with the shipped river gap no swamp reaches a bridge
  approach, so its test is the compiled map's admission, not the placement
  rule alone.

### Splits for the size ratchet

- **When:** slice 12.
- **The choice:** the presets document (`layout/presets/definitions.rs`)
  and the open country's clearance reading (`open_country/country.rs`) moved
  out in their own commits.

### Left for tuning and slice 13: neat ponds of one family

- **When:** slice 12.
- **The choice:** the second critique still read the swamps as neat ponds of
  one family, small at 6 km, and noted the riverside ones never meet the
  river. The gap is the contract's (a swamp may not stand on a river's cut
  bank); size and variety are preset numbers; the crisp outline is the plan
  picture's symbol, and the look on the battlefield is slice 13's.
- **Verdict:** open, for tuning.

### No paired battle report on the `wetlands` map yet

- **When:** slice 12.
- **The choice:** that map came from slice 00 in the other lane; run the
  report once it is merged.
- **Verdict:** superseded by slice 11's paired crossing
  ([The owed evidence is a paired crossing](#the-owed-evidence-is-a-paired-crossing-not-a-full-battle-report)).

## Slice 13: swamps drawn as water

### One standing-water owner, by depth

- **When:** slice 13.
- **The choice:** `StandingWater` (presentation `ground::water`), built from
  the exported rivers and swamps, answers `at(x, y)` (surface, depth and
  look row), lays the one list of water cells, and tells the plots what a
  point floods. Water stands where its surface lies above the ground: so a
  road across a swamp, which the simulation leaves uncarved, is dry by
  depth alone, with no road rule. A river's look rows come first, then the
  swamps'; a swamp's look is picked by the seed, the kind and the swamp's
  index (`look(seed, Kind::Swamp, k)`), on its own salt, so river looks
  are unchanged.
- **The gap:** the contract named the owner; where swamps enter the
  presentation, and the depth rule for roads, were mine (Codex found the
  causeway case).
- **The reach:** slice 15's wakes and dust ask the same owner; slice 13's
  posture rule too.
- **Verdict:** sound.
- **Confidence:** high.

### No crop stands in a swamp: its fields are cut along it, and flood

- **When:** slice 13.
- **The choice:** the first swamp shot showed a wheat field growing through
  the water. Fields are now cut along a swamp's outline as along a river's
  run, and a plot inside it takes the plot kind its swamp look names
  (`ground`: meadow for the flooded meadow, rough for the reed marsh), so
  the grass owner grows that ground's grass under the water. A test pins
  it, and fails with the rule taken out.
- **The gap:** none in the contract; a defect on screen.
- **The reach:** puddles (14) lie on tracks and fields and change no plot.
- **Verdict:** sound.
- **Confidence:** medium.

### Reeds are not built yet

- **When:** slice 13.
- **The choice:** the reed marsh grows the rough plot's grass, under the
  water. Reeds standing knee-high out of 0.6 m water would be over a metre
  tall, past the grass owner's 0.9 m cap ("half a soldier", measured from
  the ground). The physical fix is to measure a grass's height from the
  water's surface where it stands in water, plus a reed grass kind. That
  is the grass owner's change, recorded open for slice 18.
- **The gap:** the contract asked for reeds from the grass owner.
- **The reach:** slice 18, or a follow-up. Still open at close
  ([README](README.md#open)).
- **Verdict:** unfinished.
- **Confidence:** high that it is open.

### The swamp's 0.6 m reads knee-deep, and stays

- **When:** slice 13.
- **The choice:** a wading shot at 12 m (now in the water reel) shows the
  squad in the full 0.6 m of decision 30, which reaches just above a 1.8 m
  soldier's knee. An unprimed critique read it as knee-deep, not thigh-deep.
  The number is the simulation's (the dip lowers eyes for sight, decision
  6), so it stays; the user decides if thighs should win.
- **The gap:** decision 30 says both "about 0.6 m" and "thighs".
- **The reach:** mapgen's swamp depth and the wetlands map.
- **Verdict:** raised with the user.
- **Confidence:** medium.

### The flooded meadow is tinted, so it reads apart from dry meadow

- **When:** slice 13.
- **The choice:** the flooded meadow's extinction is
  `[0.95, 0.8, 1.2]` a metre, a tea-coloured flood. At first (`[0.7, 0.55,
  0.85]`) its clear water over grass stood under ΔE 10 from the dry meadow
  in the wading shot. The scene's floor is a loose tripwire; the tint is
  truer to standing floodwater, and the shot clears it narrowly (10.1).
- **The gap:** delegated (each look's colours).
- **The reach:** none.
- **Verdict:** sound, but the margin is thin.
- **Confidence:** medium.

### A posture is taken only where it holds the chest above the water

- **When:** slice 13, the posture rule (decision 33).
- **The choice:** each posture carries the height it holds a soldier's chest
  over the ground under him: `presentation.pose.stance.chest_m`, standing,
  kneeling, lying, `[1.35, 0.85, 0.3]`. Where a man would kneel or lie, the
  pose driver asks the one `StandingWater` how deep the water at his feet
  stands. He takes the first posture from his own upward whose chest clears
  it, and he stands in any water. Rivers count as swamps do. So in the 0.6 m
  marsh a pinned or firing man who would lie kneels, water at his belt, and
  in a river deeper than 0.85 m he stands. The rule lives with the stance
  (`units/driver/stance.rs`: `Stance::posture`, which now holds the whole
  posture choice: lean, pin, fire, crawl speed, water).
- **The gap:** decision 33 compares against "a lying soldier's height", and
  no pose data held one. The coordinator's note said kneeling in thigh-deep
  water puts the chest under. On one knee a 1.8 m man's chest stands at
  about 0.85 to 0.95 m, so at 0.6 m he kneels with the water at his belt,
  as soldiers hunker in a paddy in a war film. The chest, not the head, is
  the line: a man lying with only his face clear, or kneeling up to his
  chin, reads as drowning.
- **The reach:** the dead still fall and lie where they die, half under
  the water (a body in the marsh is the film's picture). Moving men keep
  moving in their posture, so a crouched move in a river becomes an upright
  walk.
- **Verdict:** sound; the heights are data, tuned by eye like the stance
  shares.
- **Confidence:** medium on the kneeling height.

### The driver takes the ground and asks it for the standing water each frame

- **When:** slice 13, the posture rule.
- **The choice:** `PoseDriver::update(frame, units, ground)` builds
  `ground.standing_water()` once per frame. Only men who would go down look
  up the depth. Building it costs about 0.8 µs on the wetlands map (its swamp
  outline has four edges), and a lookup about 0.75 µs, mostly the river's
  365 stretches. So no cached copy is kept beside the ground.
- **The gap:** the coordinator suggested building it once when the ground
  is set; the measurement showed no need.
- **The reach:** a firefight of 200 men down costs about 0.15 ms a frame
  natively. A bounds check per stretch would cut it if it ever shows.
- **Verdict:** sound.
- **Confidence:** high.

## Slice 14: puddles

### One puddle a cell, from the seed and the cell alone

- **When:** slice 14.
- **The choice:** `ground::puddles::candidate(…, i, j)` draws a cell's
  puddle (14 m cells) from a key of the biome's seed and the cell: a
  centre, a radius (0.7 to 2.2 m), a roll against the chance of the
  surface under the centre. `Ground::new` walks every cell once into
  `Ground.puddles`; a test asks each cell alone, in reverse, and gets the
  walk's puddle. A puddle on a rutted kind moves into the nearer wheel rut
  (its kind's `ruts.offsets_m`) and is no wider than the rut, since rain
  stands in the ruts: an unprimed critique of the first shot found them on
  the grass strip between.
- **The gap:** decision 34 named the purity; the cell size and the rut
  rule were mine.
- **The reach:** slice 15's wakes ask `StandingWater`, which holds the
  puddles.
- **Verdict:** sound.
- **Confidence:** medium.

### Puddles lie only on the paved kinds; the track's centre strip grows through

- **When:** slice 14.
- **The choice:** the contract wanted puddles on tracks and fields, with
  the grass and dressing reading the same function. Puddles here lie only
  on the paved kinds (`puddles.on` names road, paving, country road and
  dirt track), where no plot grass or dressing grows, so neither needs a
  puddle test. Putting them in the surface field's water list would have
  done it for grass, but the shore's bands (a few metres of wet bank and
  earth, sized for rivers) would ring every puddle in mud, asphalt too.
  The dirt track's own centre-strip grass still stands where a puddle
  reaches it; the rut rule keeps most puddles off the strip.
- **The gap:** a deviation from the contract: no puddles in fields, and no
  grass test.
- **The reach:** field puddles need the grass owner to read puddles (a
  water list the shore ignores): slice 18, or a follow-up. Still open at
  close ([README](README.md#open)).
- **Verdict:** a deliberate narrowing, recorded.
- **Confidence:** medium.

### A puddle's depth and edge are its shape's, carried by its cell

- **When:** slice 14.
- **The choice:** a puddle carves nothing, so the depth buffer has none.
  Each puddle is one water cell over its disc whose vertex colour carries
  its look, centre and radius (`[look, cx, cy, r]`; rivers and swamps carry
  a zero radius). The one water material reads that: its depth is
  `depth_m · (1 − (r/R)²)` along the view, it fades in over its rim (a sixth
  of its radius, or a pixel and a half) rather than over a few pixels of
  depth, which never filled in at 6 cm, and its water is still (an
  unrippled normal). The look row carries the depth (`fades.w`). The water
  cell record grew from 5 floats to 8; the recorded browser mesh vectors
  were re-encoded.
- **The gap:** the contract asked for a per-cell input to the one
  material; this is it.
- **The reach:** none.
- **Verdict:** sound.
- **Confidence:** medium.

### Puddles read as wet mud, not yet as reflective water

- **When:** slice 14.
- **The choice:** an unprimed critique read the first puddles as dark
  mounds, stains or cow pats, with no sky in them. Darker mud made them
  read as holes; the last tuning (muddy `[0.11, 0.095, 0.07]`–`[0.15, 0.13,
  0.095]`, extinction 10–12 a metre, mirror near black) reads as wet mud
  patches in the rut. What's missing is sheen: at the reel's sun the sky
  they reflect is a pale haze close to the track's colour, and at water's
  F0 from above it barely shows. Mirror puddles on asphalt were not filmed:
  the wetlands map has no metalled road.
- **The gap:** delegated (density, size, shape).
- **The reach:** slice 18 judges it beside a wet-road reference, with a
  darker wet rim and a stronger sky sheen as the candidates. Still open at
  close ([README](README.md#open)).
- **Verdict:** unfinished.
- **Confidence:** low.

## Slice 15: wake sources

### Who wades: a ground hull at its centre, each soldier in the water

- **When:** slice 15.
- **The choice:** `EffectFeed::publication` takes the ground's
  `StandingWater` and fills `Shooter.wading` with `Wading { at, depth_m }`:
  a hull on the ground at its centre, otherwise each member standing in the
  water. Each sits on the water's surface. Aircraft, and squads riding a
  carrier (`units::riding`, now the one test that both interpolation and the
  feed use), wade nowhere. Enemies wade only while identified, because the feed
  reads only what the side sees. `Loaded::note` moved to
  `presentation/taken.rs` and takes the ground, so `presentation.rs` didn't
  grow.
- **The gap:** the contract named the sources, not where a hull's push
  sits. The centre is what the contract said; a hull half in the water at
  a bank still wades or not by its centre.
- **The reach:** puddles come with slice 14 through the same `at`.
- **Verdict:** sound.
- **Confidence:** high.

### A round on the water splashes: its hit is `water`

- **When:** slice 15.
- **The choice:** the simulation stops a round on standing water as it
  does on ground and publishes `ground`. The feed renames a `ground` hit
  that ends in standing water to `water`, and `impacts.water` draws it as a
  pale spray, white-blue, not the ground's tan dust. Sound gives `water` the
  ground's impact sound and gain (`sound.impacts.water`) until there is a
  splash sound.
- **The gap:** the contract said "splash instead of dust" without a look.
- **The reach:** a blast's dirt (grenades, shells) is still dirt over
  water. A burst's own splash is a later look pass.
- **Verdict:** sound.
- **Confidence:** medium on the spray's colour.

### Impulses carry their time, not a step; kept a second

- **When:** slice 15.
- **The choice:** `WakeImpulse { xy, radius, strength, kind, at }`.
  - `at` is the presentation time it is made, in seconds. The step rate is
    slice 16's `WaveClock`, which maps a time to its step.
  - `kind` (`WakeKind::Rings` or `Vee`) is the contract's `style`. The data
    row already took that name (`WakeStyle`).
  - `EffectFrame::wakes(from, to)` hands a renderer what was made in its
    frame's span. Impulses are kept one second past the last tick taken,
    then pruned.
  - A reset clears them.
- **The gap:** the contract's `step` field before the step clock exists.
- **The reach:** slice 16 reads `wakes(last, now)` each frame.
- **Verdict:** sound.
- **Confidence:** medium.

### Rings at a fixed rate, a V at dust's spacing, split at 1 m/s

- **When:** slice 15.
- **The choice:** `presentation.effects.wake` is
  `{ ring_hz: 2, vee_mps: 1, spacing_m: 1.5, soldier_radius_m: 0.35,
  ring_strength: 0.5, vee_strength: 1 }`.
  - The style follows the unit's speed between ticks (a squad's by its
    centre).
  - Below 1 m/s each wader makes rings every half second of the clock. They
    are counted like damage smoke's puffs (`k / ring_hz` within the tick),
    each strength ±20% from a seed keyed by unit, wader and count.
  - At 1 m/s or faster each wader makes a V push every 1.5 m, carried
    across ticks like dust, where it was at that moment.
  - A hull's push reaches its half width.
  - It is decided in `note_movers` with the dust, in `frame/wake.rs`, so
    it follows ticks, not frames.
- **The gap:** delegated (strengths and the threshold).
- **The reach:** tuned by eye once slice 16 draws waves.
- **Verdict:** sound for now.
- **Confidence:** low, by design.

### The battle-driven seam tests live in `wake_sources.rs`

- **When:** slice 15.
- **The choice:** the frame tests sit beside the dust tests in
  `effect_behaviour.rs`. These are a hull in water makes a wake and no
  dust, a still wader rings and a moving one drags a V, a unit out of sight
  makes none, and wakes don't depend on frame rate. The rider and splash
  tests need real observations, so they run battles in `wake_sources.rs`:
  the transport lab with swamps laid under its boarding ground, and a
  rifle firing at a man in a swamp. The transport lab's red squad is
  identified from its first tick, so the out-of-sight case is proven at the
  frame level, and the battle proves a seen enemy in water wades.
- **The gap:** the slice named one file for all of them.
- **Verdict:** sound.
- **Confidence:** high.
- **Not done:** the slice's debug lab (wake marks on a test copy of the
  wetlands ground). "A soldier crossing a puddle makes rings" landed after
  slice 14, in `wake_sources.rs`: a rifle squad walking a road through the
  river lab's largest road puddle.

### A V-wake needs depth; a puddle only splashes rings

- **When:** after slice 14 (Codex review of 15 and 16).
- **The choice:** `wake.vee_depth_m` (0.2 m). A wader moving at
  `vee_mps` or faster drags a V only where the water stands that deep;
  in shallower water (a puddle, a swamp's shoaling edge) it makes rings at
  the ring rate, moving or not. A boot in a puddle splashes; it raises no
  bow wave (decisions 13 and 29).
- **Verdict:** sound.
- **Confidence:** medium on the depth.

### Only what stands below the water's surface wades

- **When:** after slice 16 (Codex review).
- **The choice:** `StandingWater::under(at)`: the water at a point whose
  surface is above the feet or hull there. A tank or soldier on a bridge's
  deck over the river is dry. The effects' waders, the posture rule and a
  round's splash all use it. A round is looked for 5 cm below where it
  stopped, since the simulation stops it on the surface itself.
- **Verdict:** sound.
- **Confidence:** high.

## Slice 16: the wave field

### The clock counts steps on the presentation clock and lives with the wakes

- **When:** slice 16.
- **The choice:** `wave::WaveClock::advance(clock, camera)` runs the steps
  up to `floor(clock · hz)` since the last, at most `max_steps`, so any
  split into frames runs the same steps. A jump drops the excess, and the
  clock running backwards starts over.
  - `epoch` is a process-wide counter, new for every clock. The clock lives
    in `EffectFrame`, beside the wakes it splats, so a load or a side switch
    (both make a new `Loaded`) opens a new epoch, and so does a reset
    (`EffectFrame::reset`).
  - `EffectFrame::waves` hands each impulse to the step that covers its
    `at`, in field texels.
  - `Presentation::waves` (in `presentation/cameras.rs`, beside the camera
    that places the field) feeds the wasm call `waves()`: a float array of
    steps, epoch, shift, then `WaveSplat` records.
  - The page takes it once per presented frame (`ScenePresentation.frame`)
    and hands it to the renderer with the frame's views (`setCamera`). The
    renderer runs each array once.
- **The gap:** the contract placed the clock and its consumer, not who
  holds the state.
- **Verdict:** sound.
- **Confidence:** high.

### The field follows the camera's target, carried in the camera record

- **When:** slice 16.
- **The choice:** `Camera.waves` holds the field's south-west corner on the
  texel grid round what the camera looks at, its side, and how much shows.
  How much fades from whole at 180 m eye-to-target to none at 320 m. It is
  a pure function of the camera, so the water's shader needs no new
  uniform, and native gets it with the camera it already uploads. On Low
  (`TerrainParams.view.z`) the water ignores the field and the renderer
  runs no steps.
- **Verdict:** sound.
- **Confidence:** high.

### 1024² at 12.5 cm, 30 steps a second, fine rings that die within a metre or two

- **When:** slice 16.
- **The choice:** `WAVE` (`wave.rs`, generated for the shaders): 1024 texels
  of 0.125 m (a 128 m square), 30 Hz, at most 6 steps a frame. RedGPU's
  speed 0.2 and damping 0.08 a step, and slope scale 0.15. Each splat is a
  ring (a dip with a rim summing to nothing), not RedGPU's push, because
  pushes made again and again piled into a mound that read as brown discs.
  A soldier's push reaches 0.15 m (`wake.soldier_radius_m`, down from
  0.35). These came from three rounds of looking:
  - 512² at 25 cm with RedGPU's numbers: saturated bands across the marsh;
  - with harder damping: mounds;
  - the ring splat at 25 cm: clear but too big.

  An unprimed critique of that last round found rings that read as rings on
  water, nothing changed on the river shots, and no seams. It also found
  them too big, regular and strong, like contour lines at 12 m and halo-like
  rings at 45 m. It asked for 10 to 25 cm ripples dying within a metre or
  two. The final round gives fine collars round each man at 12 m and nothing
  at 45 m.
- **The gap:** delegated (texel count, damping, splat shape).
- **The reach:** the cost probe was not measured on the GPU. The frame
  timer measures whole frames only, and an interleaved A/B needs a lab.
  Estimate: a step is one 1024² fragment pass of ten texture loads plus a
  loop over the frame's splats (at most 256), about two steps a frame at
  60 fps. The splat loop is the term to watch in a big battle; binning
  splats or drawing them as quads would bound it.
- **Verdict:** sound for the look; the cost is unmeasured.
- **Confidence:** medium.

### A side switch starts the field over; the renderer obeys every packet

- **When:** after slice 16 (Codex review).
- **The choice:** `Presentation::invalidate` restarts the wave clock
  (`Loaded::forget`), so a side switch opens a new epoch before the new
  side's world loads. The renderer's `wavePlan` clears the field on a new
  epoch or an empty packet (no battle), even in a frame with no step. A
  frame with a camera shift but no step runs a still pass that only moves
  the field, so the field stays laid where `Camera.waves` says.
- **Verdict:** sound.
- **Confidence:** high.

### Captures warm the field by playing the reel, not by a fixed warm-up

- **When:** slice 16.
- **The choice:** the capture tool plays the reel frame by frame at fixed
  reel times, so the field runs the same steps each time and its shots
  repeat. No separate warm-up count was added. Native draws no wakes (see
  [water native](../../water-native/README.md)). The water scene's native check compares the mean
  water colour, which the final wakes move by ΔE 0.1 on the wading shot.
- **The gap:** the slice asked for a fixed warm-up from a reset.
- **The reach:** a capture that jumps straight to a plate would need one.
- **Verdict:** sound for the reel's captures.
- **Confidence:** medium.

### Splits for the size ratchet

- **When:** slice 16.
- **The choice:** the wasm crate's scar upload moved to
  `presentation-wasm/src/scars.rs`, and the page's probe types to
  `web/src/battle/sim/presentationProbe.ts`, to make room.

## Slice 17: tree sway

### The wind moved from the `Environment` uniform into the `Camera` record

- **When:** slice 17.
- **The choice:** `Camera.wind`, one `Wind` in a one-record array. A sun
  cascade's caster binds only its camera, and could not bind the
  environment (it holds the very depth array the caster draws into); the
  camera is the one uniform every vertex stage binds and already carries the
  clock. The environment's wind, `setWind`, `ground_wind` and native's wind
  argument are deleted, so the wind still has one owner.
- **The gap:** slice 01's upload contract put the record in the environment.
  This revises it.
- **Confidence:** high.

### Every cascade camera runs on the frame's clock and wind

- **When:** slice 17.
- **The choice:** in `presentation/cameras.rs`. The fit is still reused;
  only those two fields are refreshed each frame. Before, casters ran at
  time 0.
- **Confidence:** high.

### One `sway` function; casters bend the trunk only

- **When:** slice 17.
- **The choice:** `sway(foot, point, up, height, leaf)` (`frame/sway.ts`) is
  the one function. The standing-tree vertex stage (prepass, forest and
  backdrop colour) passes the vertex's leaf flag; the new `casterVertex`
  passes 0, so shadows bend with the trunk and do not shiver. The tree's
  height is the crown's heart (`shape.w`) over the walk's `HEART`, the
  inverse of how the walk wrote it. Felled trees and forest-floor dressing
  do not sway.
- **Confidence:** high.

### Sway tuning is look only, owned by Rust and generated into `SWAY`

- **When:** slice 17.
- **The choice:** `static_world/ground/sway.rs` owns it. At strength 1 (gust
  speed 6 m/s or more) the top bends `0.18 × height × (0.15 lean + 0.7 ×
  gust front + 0.35 × swing × (1 + front)/2)`: under a passing gust front a
  crown leans up to about 18% of its height and swings harder; between
  fronts it holds a small lean and a slow swing (1.6 rad/s, with RedGPU's
  overtone). Leaves shiver up to 0.15 m at 7 rad/s, more under a front.
  Strength follows the wind's gust speed, so a calm biome stands still.
  Raised from 10% after the coordinator asked for a strong gust to bend a
  crown visibly at the 80 m low shot. The last unprimed critique still found
  the forced full gust hard to tell from the summer breeze in that shot.
  There the summer wind (35°) blows almost along the camera's axis, so
  crowns bend toward the lens, and the tree models already lean at rest.
  From above (tactical) the shift reads, and nothing shears, detaches or
  stretches. I stopped at this round rather than push the amplitude into
  rubber.
- **The gap:** delegated (amplitude, phase and the height weighting).
- **The reach:** still open at close ([README](README.md#open)).
- **Confidence:** low, by design.

### Splits for the size ratchet

- **When:** slice 17.
- **The choice:** the side's fog moved out of `presentation.rs`, and the
  scenery's vertex stages out of `sceneryLayer.ts`, each in its own commit.

### The walk's chunk bounds take in the sway's reach

- **When:** slice 17.
- **The choice:** `Sway::reach`, from the same numbers the shader uses: the
  top's largest bend plus the leaves' shiver, added to the chunk's radius
  and height. A tree's whole bent crown can leave its rest box, so a
  rest-box cull dropped crowns and shadows at the view's and cascades'
  edges.
- **Confidence:** high.

### The frame scene's `geometry` baseline is re-blessed

- **When:** slice 17.
- **The choice:** the geometry lab's trees stand in the summer wind at the
  scene's held clock, so they now bend. The still moved by a pixel at this
  tuning (2 px at an earlier one), the same on repeated runs; foundation and
  river did not move.
- **Confidence:** high.

### The blurred bed is guarded by a max-mip beside the copy's mips

- **When:** after the closeout, from a Codex review.
- **The choice:** the fog mask copy became the water's guard: its unseen
  share and the paint's coverage, seeded in a pass, each mip the worst of
  every texel the copy's mip tap weighs. The water reads the guard's
  texels under its blurred tap, at both levels the sampler may pick, and
  reads the bed at the pixel where any is seen less than its own or marked.
  Checking a few pixels round the tap missed the footprint's far texels, so
  a seen pixel's bed could carry an unseen pixel's unfogged colour.
- **Confidence:** high: Codex found the footprint exact on both clients,
  and the frame scene's water baselines did not move.

### The team planner reads an objective over every cell its square reaches

- **When:** after the closeout, at the user's word.
- **The choice:** `InfluenceGrid::on` sums each cell the objective's square
  reaches, weighted by the share of the square it holds, in place of the one
  cell under its centre. An objective near a cell's edge no longer reads
  the enemy beside it as nothing. Skirmish digests moved; the user does not
  hold them.
- **Confidence:** high.

## The closeout's measurements, for the record

- **When:** the closeout.
- **The choice:** the normals' cost pair (slice 06: High 9.21 ± 1.09 ms with the baked map against 9.29 ± 0.91 ms with a flat normal) and the caustics' (slice 08: refraction and caustics 7.32 ± 0.24 ms against depth colour alone 7.47 ± 0.85 ms), and the closeout benchmark the README's Measured section quotes, were recorded in the slices before they were folded into the README. All are whole-frame GPU time in the river lab at 1920×1080.
- **The gap:** none.
- **The reach:** the README's Measured section.
- **Verdict:** recorded.
- **Confidence:** high.
