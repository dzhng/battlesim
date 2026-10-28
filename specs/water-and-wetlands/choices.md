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
