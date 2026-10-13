# Native rendering parity: the choices ledger

These are the decisions this spec left in the code. Each was checked against the shipped code on `main` (2026-10-13), not against the pass where it landed. A decision a later pass reverted is not listed. That covers Godot and everything built on it, the JavaScript-arithmetic emulation, `resend_statics`, the exported placement tables and the per-pass scaffolding. Gate results and evidence live in the slice files.

**Terms used throughout:**
- **Presentation crate** (`crates/presentation`): the Rust code both clients share. It turns one side's publications into what to draw.
- **Publication:** the words the simulation sends a side each tick, holding only what that side can see.
- **`Frame`:** what the crate hands a renderer for one drawn frame, as flat records: batches of instances, palettes, effects, lights, the camera and the counts.
- **`World`:** what the crate hands a renderer once per battle.
- **Batch:** one run of instances of one model at one detail tier.
- **Renderer:** the browser's TypeGPU code, or `native/client` on `wgpu`.
- **Control:** a fresh browser run that native is compared against.
- **Plate:** one frame of a control, saved as a PNG.

**Decided by:** *User* means you decided it. *Agent* means an implementing agent, coordinator or orchestrator decided it on your behalf. **Confidence** is how sure the audit is that you would have made the same call.

**Review these first:**
1. **Native's replay still takes its units, effects and light from the browser's manifest** (needs you, low).
2. **Native no longer checks that a capture is the battle its manifest names** (unsound, low).
3. **Labs that hand the renderer their own models keep a second copy of the caster rule** (unsound, low).

## Needs you

### Native's replay still takes its units, effects and light from the browser's manifest
*Agent, slices 14, 17 and 20. Confidence: low.*

Picture native replaying the market-town reel. Native reads the ground, the buildings, the scenery and the props itself, from the saved map and the asset files. Four things still come from the `native-visual-presentation` manifest, the file the browser's capture run writes beside its capture:
- the `units` section: each unit type's facts, the appearances each side wears, every body's joints, bounds and tiers;
- the `effects` section: effect tuning, hull muzzles, wreck looks;
- the raw `light`;
- `props.buildings`: each building's part props and the kinds a gutted building's parts become.

Native also reads each skinned body's clip table (`units-<table>.u32`) from beside the manifest. It hands all of this to `load_world` (`native/client/src/capture.rs`).

The page's TypeScript resolves these sections from the scene-assets and unit catalogs. So a native client with no browser run behind it cannot field a single soldier. The alternative is for Rust to resolve them from the catalogs, as 20b did for buildings, scenery and props. Then `load_world` takes catalogs, as 18b first proposed.

- **Reach:** a live native client, one that plays a battle rather than replaying one, needs this first. Until then, the browser's resolvers (`unitPresentation.ts`, `effectPresentation.ts`, `propPresentation.ts`) are a second owner of facts Rust otherwise owns.
- **Provisional call:** keep it for the replay, and move units, effects and building parts into Rust before native plays live. To reverse it, delete the manifest sections once `load_world` reads the catalogs.

### Native draws far soldiers as meshes, not impostor cards
*Agent, slices 14 and 25 (step 5a). Confidence: medium.*

An impostor card is a small picture of a model, baked once and drawn as a flat card when the model is under about 10 pixels tall.
- **Browser:** bakes cards for skinned bodies and draws a distant soldier as one card (two triangles).
- **Native:** bakes nothing. At the card tier it draws the body's coarsest mesh, posed once in the far pose the card would show (a few hundred triangles), and a corpse's coarsest lying mesh (`native/client/src/models.rs`).

Counts are equal, and the picture is the same few pixels. The bake pipeline (`bake.wgsl`) is exported, but native does not run it.
- **Reach:** a large battle with hundreds of far soldiers costs native more triangles than the browser. Slice 22 (completeness) owns closing it.
- **Provisional call:** keep the mesh until a measure shows the triangles matter. To reverse it, run the exported bake at load and draw tier 4 as cards.

## Unsound

### Native no longer checks that a capture is the battle its manifest names
*Agent, slice 25 (wgpu port). Confidence: low.*

The manifest names its battle by identity: the capture's workload and the SHA-256 hashes of the capture file and the map file. Phase 1 decided native must refuse to render on any mismatch, so a plate can never be compared against a different battle. Godot's runner checked this.

`native/client` reads only `identity.map`, to know which saved map to load. Suppose someone copies a stale `capture.json` into a scene folder, or edits the map after the control ran. Native would replay it without complaint, and the plates would differ for a reason no gate names.
- **Corrected decision:** native hashes `capture.json` and the saved map it loads and refuses to run on any difference from `manifest.identity`, naming the file that differs.

### Labs that hand the renderer their own models keep a second copy of the caster rule
*Agent, `derive2rust`. Confidence: low.*

"Cast one tier coarser" means a model's shadow is drawn from a cheaper mesh than the model itself. It has one owner in Rust: `view::caster_tier`, written into each batch as `Batch.caster`, together with `Batch.fog` (which fog group the batch binds). Battles route by those fields.

A lab with no battle that calls `setModels` with its own instance list has no batches. The TypeScript model layer therefore keeps its own `CASTER_COARSER` and fog rule for that path (`modelLayer.ts`). If someone changes the rule in Rust, labs would keep the old shadows.
- **Corrected decision:** labs build batch records too, as 18b's "labs build their own records" says, so the TypeScript copy is deleted.

## Sound

Least confident first.

### An aircraft's shadow-visibility test assumes it flies at cruise height
*Agent, slice 20. Confidence: medium.*

A helicopter can be out of view while its shadow is on screen, so it must stay drawn. The crate decides this by dropping the shadow point straight below the aircraft by a fixed `cruise_agl_m` from the rules, along the sun's direction. It does not use the terrain under the aircraft. This was provisional because Rust had no terrain heights before 20b. Rust has them now, and the rule stayed.
- **Reach:** over a hill or a valley, the test is off by the height difference. It only decides whether an aircraft is drawn, never where.
- **Verdict:** sound for now. Read the ground height if an aircraft ever pops in or out at a frame edge.

### Ghosts arrive with the ground height the UI computed
*Agent, slice 20. Confidence: medium.*

A ghost is the translucent preview of a placement, a purchase or a destination while Space is held. The UI passes each ghost to `frame()` as a short JSON list with the ground height under it. Rust adds an aircraft's cruise height, then fields, poses and colours the ghost. Ghosts draw in their own pass, `GHOST_PASS`, with no shadow and no depth write. Rust now has the ground, but the UI still supplies the height.
- **Verdict:** sound. It is one number per ghost, and only while placing.

### The browser's battle score is seeded from `Math.random`, and native's replay from 0
*Agent, slice 25 (step 7b). Confidence: medium.*

The score picks its next track with a shuffle bag on its own seeded `Mulberry32`. The browser seeds it once per battle from `Math.random` (`appAudio.ts`), so each battle still opens on a different track, as the TypeScript did. Native's replay seeds it 0 (`SCORE_SEED`), because a replay has no seed to take. The seed is an input to the crate, so the crate itself stays deterministic. A saved replay in the browser, however, may play a different score each time.
- **Verdict:** sound. Seed it from the battle's seed if you want replays to sound the same.

### Native's music duck and reverb are close, not exact
*Agent, slice 25 (steps 7 and 7b). Confidence: medium.*

When a voice line plays, the music dips. That dip is the duck. The two players get there differently:
- **Duck:** the browser glides toward the new level with a time constant. Native glides linearly over the same span, because `kira` has no exponential approach. The depth is exact.
- **Reverb:** the browser convolves with an impulse it synthesizes in TypeScript (`reverbImpulse`). Native uses `kira`'s algorithmic reverb.

Rust decides every sound command, so these are differences in the players only.
- **Verdict:** sound; the differences are named. They matter only if native audio is ever compared by ear.

### The unseen-enemy shout pool stays in TypeScript, so native is silent there
*Agent, slice 25 (step 7b). Confidence: medium.*

When an enemy the side cannot see shouts, the browser picks a take from a shared pool (`shoutPool.ts`). That selection rule was never moved to Rust, so in a native replay an unseen enemy's shout makes no sound.
- **Reach:** slice 22 (completeness). Moving the selection into `presentation::sound` would serve both clients.
- **Verdict:** sound as a named gap.

### `observation()` returns JSON, about 36 µs slower per publication than the old decoder
*Agent, slice 19. Confidence: medium.*

The UI reads the side's observation (units, contacts, orders) once per new publication. Rust serializes the typed view with serde, and the page runs `JSON.parse`. That costs about 180 µs a publication, against the deleted TypeScript decoder's 144 µs: roughly 1 ms per second at 30 Hz. The fog bits and the ground runs skip JSON (`fog_bits`, `ground_runs`).
- **The alternative:** hand-written zero-copy views for every entity, which is a lot more code.
- **Verdict:** sound. Frame rate did not move. Every per-frame record in `Frame` is already zero-copy.

### A stale publication is dropped quietly, not refused
*Agent, slice 19. Confidence: medium.*

18b said `push_publication` errors on a stale record. In a lab, switching sides leaves records for the old side in flight. `push_publication` returns `false` for those, and `invalidate(side)` resets the reader, as the TypeScript decoder did. A malformed record, or an index outside its table, is still an error.
- **Verdict:** sound; a small, deliberate deviation from 18b.

### Fog is prepared at encode time, by a second call
*Agent, slice 25 (step 6b). Confidence: medium.*

Fog of war's sight is decided in Rust: the eyes, the occluders the side knows, the map slots, the rebuild schedule and `FogParams`. The renderer receives it as one `FogUpload` per frame. It is not part of `frame()`, though. The renderer calls `prepare_fog(camera)` when it encodes, because the rebuild schedule assumes every prepared frame dispatches, and the page sometimes frames its presentation on ticks it does not draw.
- **Reach:** a renderer makes two calls per drawn frame instead of 18b's one.
- **Verdict:** sound. Merging them would need `frame()` to know whether the frame will be drawn.

### Fog's foliage is still handed in from outside the crate
*Agent, slice 25 (step 6b). Confidence: medium.*

Foliage the side has seen cleared stops blocking its sight. Working out which foliage cells are cleared needs the simulation's world. The page's presentation does not hold that world, so the page passes the cells in (`WorldView.foliage_cleared`), and native computes them with `files::side_foliage`.
- **Verdict:** sound. It is the one fog input not derived inside the crate.

### The building walk places everything in one call, with no per-frame budget
*Agent, slice 20b (step 1). Confidence: medium.*

The walk decides which buildings are resident (expanded into their modules) for the current view, nearest first, while the pool has rows. The TypeScript spread expansion over several frames (`expand_rows`) to avoid a hitch. In Rust a row costs no JavaScript object, so the budget and its packing-by-kind were deleted, and both clients run the same rule.
- **Reach:** a hard camera cut into a dense city expands everything in that frame. No hitch has been measured.
- **Verdict:** sound. Reintroduce a budget in `walk.rs` if a cut ever hitches.

### Counts gates count what each list holds, not what the GPU draws
*Agent, slices 14, 15 and 17. Confidence: medium.*

Each plate carries counts the browser took, and native must equal them:
- **`modelCounts`:** the posed population (soldiers, vehicles, aircraft, corpses lying or fading), not what is in view.
- **`effectCounts`:** the effects that have started, by kind, shape and light.
- **`instanceCounts`:** what each static layer draws into view (building modules per module and tier, scenery per population, props per appearance and tier).

These are exact and do not move with machine load. They prove the same lists were built. They do not prove the pixels: the plates are still judged by eye.
- **Verdict:** sound. Since both clients run one crate, the counts mostly check that each renderer drew what its `Frame` listed.

### The grass's window is read from the last frame's camera
*Agent, `derive2rust`. Confidence: medium.*

The grass is grown on the GPU over a window around the view. Rust works out that window and the grass pass's per-frame parameters from the camera the last `frame()` call received (`grass_window()`, `grass_frame_params`). Neither renderer passes a camera anywhere.
- **Reach:** a renderer must call `frame()` before it asks for grass. Asking first gives the previous frame's window.
- **Verdict:** sound. It is the cost of having one owner for the camera.

### The "scars off" lab switch no longer brings cleared shrubs back
*Agent, `derive2rust`. Confidence: medium.*

The ground lab can hide the learned scars (`suppressScars`). It used to do that by handing the scenery an empty cleared ground, which also stood up every shrub on cleared ground. Now the scenery follows the learned ground inside the crate, and the switch hides only the scars.
- **Verdict:** sound. Scars and scenery are separate things to compare.

### Labs without a battle make buildings fall through a stand-in for known props
*Agent, slice 20b. Confidence: medium.*

In a battle, a building's state (intact, ruined, gutted) comes from the props the side has learned of. The city line-up has no battle. It passes `ui.known`, a stand-in list with each prop's id and box, and the crate reads it by the same rule. The line-up gives each building one synthetic part. A stand-in without a box affects buildings only.
- **Verdict:** sound. There is one rule for battles and labs, and no lab-only fallen list.

### A scene has one `Presentation`, and its world is loaded whole
*Agent, slice 20b. Confidence: medium.*

A `Presentation` is the crate's per-side object. Each scene owns exactly one: the battle's, with its buildings loaded into it, or an empty one for a lab with no battle. The page stages the battle's half and the buildings' half (`setBattle`, `setBuildings`), and the next frame loads them together, because `load_world` replaces everything. Until the first publication arrives, the viewport draws the static world alone, so buildings show behind the loading cover.
- **Verdict:** sound.

### Native reads two of its tables out of the browser's TypeScript source
*Agent, slice 25 (steps 4 and 6). Confidence: medium.*

Two small tables have their only copy in browser source files: the split-sum lighting table (`dfgLut.ts`) and the list of effect flipbooks (`flipbooks.ts`). Native includes those files' text at compile time and parses the numbers out (`environment.rs`, `effects.rs`), so each table keeps one owner. Native builds its flipbook mips as a simple box-filter chain. The browser's are resized by Chrome, which differs only for small, distant flipbooks.
- **Reach:** reformatting either TypeScript file can break the native build. The break is loud, at compile or start time.
- **Verdict:** sound. A JSON file both read would be cleaner if a third table joins them.

### Water is judged by shader vectors, never on a plate
*Agent, slice 16. Confidence: medium.*

Neither reel map has water, and filming the `river` test map would need a new workload. Native draws water through the browser's own exported WGSL, so the water's look is the browser's by construction. No plate has ever shown it.
- **Verdict:** sound. Film a map with water if water's look ever changes.

### The comparison runs under fixed conditions
*Agent (coordinator, on your behalf), slices 07, 12 and 25. Confidence: medium.*

A performance comparison is run like this:
- plugged in (the bench records the power source with `pmset`);
- at 1280×720, three runs interleaved, under the shared GPU lock;
- native unpaced, with its window borderless and off-screen;
- the browser headless and paced by `requestAnimationFrame`.

On macOS, an off-screen `wgpu` window presents nothing, so the bench keeps at most two frames on the GPU, as a swap chain would. Both clients write raw per-frame intervals, CPU time and GPU time, and one summariser (`frameComparison.ts`) computes every number for both.
- **Why:** on battery the headless browser is held at 30 FPS for a reason never found. A shown window adds presenting cost that the headless browser never pays.
- **Verdict:** sound. The browser's pacing does not bind while its frames take over 8.3 ms. A browser faster than that would need GPU time compared instead.

### The decode reference is the deleted decoder's last output
*Agent, slice 19. Confidence: medium.*

The committed `<scene>-observations.json` files are what the TypeScript decoder produced on two capture fixtures. That decoder was deleted when the Rust reader replaced it. `crates/sim/tests/publication_reader.rs` still holds the reader to those files. No fixture carries guided missiles, crashes, member leans, the encounter or skirmish objectives. Those rows are proven only by the web tests that now run through the reader.
- **Verdict:** sound. Re-record the reference from Rust when the layout changes, rather than keeping a shim.

### `min`, `max`, `round` and `signum` are Rust's own
*User, `jsretire`. Confidence: high.*

JavaScript's versions differ only for NaN, −0 and negative halves. No battle draws on those on purpose.

### Every transcendental function is `libm`'s
*Agent, `jsretire` (after slice 20b). Confidence: high.*

`hypot`, the trigonometry, `exp`, `log`, `pow` and `cbrt` all come from the `libm` crate (pinned `=0.2.16`), in both clients. Without that, Rust's `sin` is Apple's on native and `libm`'s in WebAssembly. A tree placed from a cosine could then land a hair apart on the two clients, and the counts gates would catch it as a difference. The simulation's `contract` crate uses `libm` for the same reason.

### Records are defined once in Rust and generated for TypeScript
*Agent (records); orchestrator with your confirmation (`Instance`), 18b and slice 20. Confidence: high.*

`crates/presentation/src/records.rs` declares every record a renderer reads: `Batch`, `Instance`, `Joint`, `Effect`, `CastLight`, `View`, `Camera`, the light and fog uniforms, and more. Each is declared through one macro. A test (`records_are_current`) regenerates `packages/battle-renderer/src/records.gen.ts`, with TypeGPU schemas for each, and fails if the checked-in file differs. `Instance` is the browser's existing 64-byte model record (placement and yaw, data, tint, scale), not 18b's first 3×4 affine, which existed only for Godot.

### Batches name their caster tier and fog group
*Agent, `derive2rust`. Confidence: high.*

`Batch` is 48 bytes and carries `caster` (the tier its shadow is drawn from, or none) and `fog` (`FOG_UNITS` never fogged, `FOG_GROUND` as the ground under it, face by face, or painted faces). Renderers only route by these fields. An impostor card casts nothing, as a card never did.

### Joint palettes are built in Rust
*Agent (orchestrator), under your "decide by profiling" rule, 18b question 1. Confidence: high.*

A palette is the list of joint matrices that bend a soldier's mesh. The browser was GPU-bound, and posing cost under 1% either way, so Rust samples the clips and writes 48-byte joint rows. Both GPU pose kernels were deleted, and clip tables never reach a renderer.

### Rust chooses the cast lights
*Agent, slice 20. Confidence: high.*

Muzzle flashes and fires light the scene. Rust culls them to the view, ranks them and caps them at 24 (`CAST_LIGHTS_MAX`). The renderer uploads the rows.

### Static batches are rewritten only when their list changes
*Agent, slices 20b and 25. Confidence: high.*

Buildings, scenery and props are static batches.
- **Prints:** each list has a print, a fingerprint of what it holds, and a batch is written only when its print changes. Otherwise its `first` is `NONE`, meaning "unchanged". Felled trees print by what they hold, not by a counter.
- **Generation:** the renderer passes a generation number to `frame()` and bumps it whenever it holds no static batches: a new GPU frame, a new world, or a replay jumping between plates. A new generation rewrites every static batch. This replaced the old `resend_statics()` call.
- **Buffers:** each renderer holds a batch's buffer, sized to a power of two of records, while the frame lists it, and drops it when the frame stops listing it. Native keeps them in `statics.rs`, with the models layer.

### Publications that arrive before the world loads are kept
*Agent, slice 20. Confidence: high.*

Appearances take seconds to install, and shots fired meanwhile must still produce effects. The crate keeps up to two minutes of publications (`PENDING_MAX`, 3,600 at 30 Hz) and replays them into the effects when the world loads.

### Corpses are tiered one by one each frame
*Agent, slice 20. Confidence: high.*

A thousand sphere tests cost microseconds, so corpses take the same per-model tier rule as the living. The renderer's corpse chunks were deleted.

### Native poses every frame record, including the ones it does not draw
*Agent, slice 14. Confidence: high.*

Poses carry state from frame to frame: a walker's stride phase, an easing gun, a fall. The replay therefore frames every record in order, from the capture's first record, and draws only the plate frames. Each record's publication is the sample at its tick, blended by its `simTime`, exactly as the browser's presentation clock blends.

### Fallen buildings are derived in Rust from the side's known props
*Agent, slice 20b. Confidence: high.*

When the side learns that a building's parts are rubble, the building switches to its ruined or gutted state. This is re-derived only when the publication's known-prop group changes. A state the building's template lacks fails the frame by name, rather than drawing nothing.

### A side draws the map's props, less what it knows was replaced, plus what it knows
*Agent, slice 20b (step 5). Confidence: high.*

One rule serves every scene. It replaced a split where movable and destroyable props came from knowledge and the rest from the map, which left cars and fences out of labs that have no side. Replacement is atomic: the list that drops a wall carries its rubble.

### A car keeps its paint wherever it is shoved
*Agent, slice 20b (step 5). Confidence: high.*

A car's paint is seeded by the authored prop it is. A body the map did not author is seeded by its position in decimetres. Seeding by the published id made a shoved car change colour.

### Props are fitted in Rust from the art files
*Agent, slice 25 (step 5c). Confidence: high.*

Each prop has a box from the simulation. Rust picks the appearance with the nearest footprint, scales it per axis or repeats a wall module along the box, and keeps the wreck that belongs to its own unit. It reads the art (footprints, paints, wrecks, bounds) from the asset files itself (`files::prop_art`), as it does for buildings and scenery. Props take the units' tier thresholds as authored and are bucketed in 64 m chunks for the walk.

### Native fights a map's battle under its category's catalog
*Agent, slice 25 (step 5c). Confidence: high.*

A menu map uses the menu's units, and so on: the content rule in the root README. Native has no session to ask, so it reads the category from the saved map's metadata.

### The static world is resolved in Rust from the saved map
*Agent (orchestrator for the fold), slice 20b. Confidence: high.*

`static_world` resolves the following from the saved map, the biome and the template art, identically for both clients:
- the buildings and their modules;
- the ground's shapes, plots and surface fields, and its terrain and grass tables and meshes;
- the height pages that fog and grass read;
- the scenery placement and the props.

Rust reads the runtime catalog, the template library and the kits' bounds itself (feature `files`, with `flate2` for the gzip transport). The page hands over bytes it already has. Nothing static travels through the manifest any more.

### Terrain and grass tunings are data in `fixtures/game.json`
*Agent, slice 20b (step 3). Confidence: high.*

`presentation.terrain`, `presentation.grass` and `presentation.fog_geometry` hold the numbers that used to be shader literals. Rust reads and checks them (the biome through serde, which refuses a missing field). The browser's shaders splice the same values from the same file, so there is one owner and the WGSL did not change.

### The scenery is placed from each kind's finest mesh, in Rust
*Agent, slice 25 (step 5b). Confidence: high.*

- **`place_scenery`:** reads a tree's size, trunk and crown off its finest mesh.
- **What counts as a tree:** a biome `trees.species` kind, which agrees with the catalog on every kind.
- **The felled stump:** cut in Rust (`stump_mesh`).
- **Felled trees:** pose in the vertex shader, so a falling tree is two `Instance` records. `Appearance.draws` tells a renderer which scenery shader draws a row.
- **Tiers:** native shares a kind's tiers across its rows, where the browser uploads a copy per row.

### Grass blade shapes are packed in Rust
*Agent, slice 25. Confidence: high.*

Rust packs each grass kind's blade shapes from the bundle tiers the page hands it (`pack_shapes`), and gives the strip indices. Native derives none of the grass itself.

### The learned ground and its scar cache are Rust's
*Agent, slice 25 (step 4). Confidence: high.*

Scars are the craters, scorch and tracks a side has seen. Rust keeps the learned ground and a bounded GPU cache of its pages. A renderer uploads the words it is handed, and draws region by region when the marks do not fit at once. Native draws every region rather than scissoring each one to the screen as the browser does. The picture is the same, and that path is cold in both reels.

### The camera, the view and the sun's cascades come with each frame
*Agent, `derive2rust`. Confidence: high.*

The renderer passes the camera's eye, target and lens to `frame()` in `f64`. It gets back:
- the `View` (with `target`, where the haze is measured from, and the sky's corner rays);
- the camera uniform;
- the four sun cascades, fitted after `set_shadow(light, terrain)` was given once.

The browser's `frameCamera.ts` and `SunShadowFit` and native's camera code were deleted.

### Clearing lands one frame earlier than before
*User accepted (named decision), `derive2rust`. Confidence: high.*

The scenery follows the ground the side has learned cleared inside `frame()`, from the publications already pushed. Both clients used to hand the cleared ground back after drawing, so a shrub on newly cleared ground left one frame later. `set_cleared` is deleted.

### The light is resolved in Rust into uniform records
*Agent, slice 25 (step 3). Confidence: high.*

`presentation::light` resolves `presentation.light` into `SkyLight`, `Aerial` (the haze), `Sun`, `Bloom` and `Grade`, each laid out as its shader's struct. The browser reads them through `light_records(json)` (a frame can be built before any battle), and native through `LightSettings::records()`.

### The browser's shaders are exported once and built natively
*Agent, slice 25 (steps 1 and 2). Confidence: high.*

A Vitest test runs the browser's real frame code on a recording WebGPU device and writes every pipeline's WGSL and settings to `packages/battle-renderer/pipelines/`. `crates/pipelines` validates them with naga, and a drift test fails when the export is stale. Each pipeline carries a unique name, which is the key native looks it up by. The battle's light reaches the shaders through uniforms rather than spliced literals, so one compiled set serves every map and size.

### Native is one ordered pass list, with no render graph
*Agent, slice 25 (step 3). Confidence: high.*

`Vec<Box<dyn Pass>>` is encoded in order each drawn frame, mirroring the browser's `battleFrame`: sky, prepass, world, effects, fog mask, ghosts, post, fog rim. Native presents by copying its display target to the window, so plates and the window read the same texture.

### Native's window is off-screen, and plates never depend on presenting
*Agent, slice 25 (step 3). Confidence: high.*

The window opens borderless and off-screen, so nothing appears on your screen. On macOS, a window that is not visible presents nothing, so plates are read from the display target, not from the swap chain.

### Fog of war binds by what a surface is
*Agent, slice 25 (step 6b). Confidence: high.*

Units are never fogged. Corpses are fogged as the ground under them. The static world is fogged face by face. Native binds each surface's fog group by its layer (`fog::Layer`), following the browser's rule, and the fog's look is a Rust record (`FogLook`) both read. A fog reel (`?fog=1`, `PARITY_FOG=1`) is the control for fog, since the menu reel draws none.

### Rust decides every sound, and players carry out commands
*Agent, slice 25 (step 7). Confidence: high.*

The crate emits commands: start, glide and stop a voice, and listen from here. A player never reads `Frame.emitters`. Both players are as dumb as a mixer: equal-power panning is the only geometry they do. The browser's bank synthesizes and calibrates through the same Rust (`synthesize`, `source_normalization`). Native renders only offline (`--sound <wav>`) and never plays aloud in a replay.

### Music is three commands
*Agent, on your contract, slice 25 (step 7b). Confidence: high.*

The commands are:
- `music`: stream a track from a time, looping, fading in;
- `music_stop`: fade every sounding track out, then free it;
- `duck`: dip the bus.

A crossfade is `music_stop` followed by `music` over the same span. Players keep no crossfade or duck state of their own.

### Native decodes its MP3s in place
*Agent, slice 25 (step 7b). Confidence: high.*

Native decodes music and voice takes packet by packet with `symphonia`, the decoder `kira` uses. `kira`'s own streaming decodes on a thread and gaps unpredictably when rendered to a buffer. Native reads and decodes a voice take when a line plays it, and keeps the last 32 decoded, as the browser's `DECODED_TAKES` does.

### The worker reports the rules' tick rate as given
*Agent, slice 25 (effect counts). Confidence: high.*

The page used to derive its tick as `1000 / (1000 / 30)`, which is 29.999999999999996 Hz, and every effect it timed drifted from native's at tick boundaries. The worker's `ready` now reports `tick_hz`, and the capture records the rate the page's presentation ran at.

### The publication reader lives beside its writer
*Agent, slice 19. Confidence: high.*

`crates/sim/src/publication/reader.rs` and the writer share one group table (`GROUP_LAYOUTS`), so offsets are checked at compile time. The observation is typed: one struct per entity, declared by `view!` and serialized by serde. The reader is stricter than the old TypeScript: an index outside its table is an error.

### One runtime-bundle reader serves the crate and native
*Agent, spec review. Confidence: high.*

`presentation::assets` reads the scene-assets container, the catalog and the gzip transport. Native loads its models through it, so the bundle format has one reader in Rust.

### Pre-existing bugs found on the way are fixed here
*User, slice 20. Confidence: high.*

Bugs found along the way are fixed in this spec, not logged as open. The first one: a paused clock turned walking soldiers to idle. It was ported faithfully from the TypeScript, then fixed in the crate.

## Your decisions

These are your calls, recorded so later work treats them as givens.

### A Rust core with one shared presentation crate
*User, 2026-10-11 (phase 4).*

Per-frame presentation (decoding, interpolation, poses, effects, cook-offs, fog's sight, sound) lives in one Rust crate. The browser runs it through `crates/presentation-wasm` and native links it. The TypeScript originals and the native ports were deleted rather than kept in step by tests. The WASM module links the reader and the crate but not the rules, so the main thread never loads the simulation (18b question 7).

### Rust as smart as possible, renderers as dumb as possible
*User, 2026-10-11.*

Rust decides everything per entity: what is drawn, at which tier, where, in which light, fogged how. A renderer draws the buffers it is given. Presentation reads only a side's publication, so it cannot see past fog of war. It never touches the simulation's state, so tuning it never changes a digest or a replay. `frame()` reads no wall clock and no unseeded randomness.

### A small, data-oriented API between Rust and every renderer, reviewed by you
*User, 2026-10-11 (18b).*

You approved:
- the shape: `World` once, `Frame` per drawn frame, flat typed records, no per-entity calls;
- static placement through `Frame`'s batches rather than `World` lists (question 3);
- fog's look in `Frame` (question 5);
- audio reading the presentation's emitters (question 8, now reached through commands);
- `ballistics` drawing its trajectories as overlay lines (question 6).

You decided separately that order lines, contact glyphs, markers and the ruler stay UI-built overlays read from `observation()`, and that ghosts stay in Rust (question 4).

### Native on `wgpu`, running the browser's own WGSL; Godot deleted
*User, 2026-10-11 (phase 5).*

Godot's per-primitive cost was the remaining GPU gap, and its roughly 2,000 lines of hand-ported shaders were a second copy of the browser's. Native is `native/client`, Rust on `wgpu` over the 18b API. Godot was frozen first (no Godot runs, no Godot glue), and it left no tech debt: nothing Godot-shaped entered shared code, and its project, binding and shaders are deleted. Bevy is consulted as a reference, not adopted. Whether the browser later moves to the same Rust `wgpu` renderer is decided after native is measured against TypeGPU.

### Web first
*User, 2026-10-11.*

Every shared-code change lands in the browser first, with clean before-and-after frame rates, and only then does native switch to it. Each move into Rust deletes the TypeScript it replaces.

### Less code over speed
*User, 2026-10-11.*

Each shared-code slice must reduce net code. A small measured browser cost is acceptable for that, and a meaningful regression still blocks. Promises do not stack: when one step grew code (20b step 2), it landed only together with the step that made the pair net-negative.

### Recompute over compatibility layers
*User, 2026-10-12 (`jsretire`).*

Once the TypeScript the crate was proven against was deleted, the emulation of JavaScript's number text, string hashing, `hypot` and rounding went with it. Every seed is now `Mulberry32` over a `seed::Key`: an FNV-1a hash of the numbers and names of what it decides for. Labels that cross the boundary stay strings. The vectors that moved (sound, chatter, effects, grass) were re-recorded from Rust.

### Music's mood is decided in Rust (battle voices and music's N2)
*User, 2026-10-12.*

This settles [N2](../done/battle-voices-music/choices.md#n2-the-scores-mood-logic-stays-in-typescript). Rust's `sound::music::Score` picks the mood (tension, combat) from the side's own barks, picks the next track as a catalog id, and dips the music under voice lines. Players resolve an id to a file, and Rust never sees a file name. Menu music stays the app's (`AppAudio`).

### Restage street rather than change the HE rule
*User, slice 23.*

After the shared code landed, red's tank killed street's rifle fight and street-watch's observer from 620–650 m across the open west. The two journeys were rerouted round the south of the west wood. The high-explosive rule was left as it is.

### What parity means
*User, phase 1.*

The authority is a fresh browser control from the same tree, never the archived plates. A plate is paired with frame records from its own run, because the browser frame is not reproducible across runs.
- **Workload:** both clients render the same menu reel at 1280×720, one pixel per CSS pixel.
- **Judging:** each visual variable is judged by eye (the same shot, light, ground and grass impression), with no pixel budget. The only numeric gate on looks is grass within about 10% per screen region.
- **Acceptance:** the agent accepts a slice without blocking on you.
- **Platform:** only macOS with Metal is verified. Windows is not a requirement, but no native dependency may rule it out.

### One automated capture run, with the drawn view recorded
*User (base), phase 1; agent added the clock and size fields.*

A headless run of `/graphics-test` writes each scene's capture, manifest and plates. Each frame record holds the view as the camera rig actually drew it (`eye`, `target` and `up`, after clamping, clearance and lag), the reel time, the tick, the interpolated sim time and the canvas size. The lens is recorded once per scene. `?plates=1` reads the canvas back in the task that drew it, so the shipped game's canvas is configured as before.

### No schema versions and no migrations
*User, phase 1.*

The ids are `battle-presentation-capture` and `native-visual-presentation`, with no version suffix. A format change updates its writer, reader and tests together, and the strict readers refuse a stale file rather than migrating it.
