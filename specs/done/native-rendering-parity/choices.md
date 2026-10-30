# Native rendering parity: the choices ledger

These are the decisions this spec left in the code. Each was checked against the shipped code on `main` (2026-10-13), not against the pass where it landed. A decision a later pass reverted is not listed. That covers Godot and everything built on it, the JavaScript-arithmetic emulation, `resend_statics`, the exported placement tables and the per-pass scaffolding. Gate results and evidence lived in the slice files, deleted when the spec closed; they are in git at `2849c8a09` (`specs/native-rendering-parity/slices/`).

**Terms used throughout:**

- **Presentation crate** (`crates/presentation`): the Rust code both clients share. It turns one side's publications into what to draw.
- **Publication:** the words the simulation sends a side each tick, holding only what that side can see.
- **`Frame`:** what the crate hands a renderer for one drawn frame, as flat records: batches of instances, palettes, effects, lights, the camera and the counts.
- **`World`:** what the crate hands a renderer once per battle.
- **Batch:** one run of instances of one model at one detail tier.
- **Renderer:** the browser's TypeGPU code, or `native/client` on `wgpu`.
- **Control:** a fresh browser run that native is compared against.
- **Plate:** one frame of a control, saved as a PNG.

**Decided by:** _User_ means you decided it. _Agent_ means an implementing agent, coordinator or orchestrator decided it on your behalf. **Confidence** is how sure the audit is that you would have made the same call.

**Review these first:** none open. The three least-confident entries were closed on 2026-10-13 (`lastowners`); they are listed under Sound with their resolution. [Native gaps](../../native-gaps/README.md) owns the remaining platform differences.

## Needs you

### Native draws far soldiers as meshes, not impostor cards

Superseded by [specs/done/one-renderer](../one-renderer/README.md).

_Agent, slices 14 and 25 (step 5a). Confidence: medium._

An impostor card is a small picture of a model, baked once and drawn as a flat card when the model is under about 10 pixels tall.

- **Browser:** bakes cards for skinned bodies and draws a distant soldier as one card (two triangles).
- **Native:** bakes nothing. At the card tier it draws the body's coarsest mesh, posed once in the far pose the card would show (a few hundred triangles), and a corpse's coarsest lying mesh (`native/client/src/models.rs`).

Counts are equal, and the picture is the same few pixels. No impostor-bake pipeline is exported (`bake.wgsl` is the sky's lookup bake, not the cards').

- **Reach:** a large battle with hundreds of far soldiers costs native more triangles than the browser. Every other item slice 22 (completeness) listed is drawn by the browser's own shaders on `wgpu`; this is the one left, owned by `native/client/src/models.rs`.
- **The call:** keep the mesh until a measure shows the triangles matter. To reverse it, first export the browser's impostor bake (`packages/battle-renderer/src/models/impostor.ts`), then bake at load and draw tier 4 as cards.

## Sound

Least confident first.

### Both clients read a battle's world from the repository, through one resolver

_Agent, slices 14, 17 and 20; resolved `lastowners`, the browser moved `browserworld`. Confidence: medium._

Native used to take each unit type's facts, its appearances' joints, clips and rigs, the effects' tuning and wrecks, the light and each building's part props from the manifest the browser's capture run wrote, and the browser resolved the same world input in TypeScript. Now Rust resolves it for both, through `presentation::fielded` (with `presentation::assets`):

- **Units and effects:** the units the battle fields, under its unit catalog; their art from the runtime catalog's bundles and skeleton clips, as a page wearing those units installs it; the clip tables built from the skeletons.
- **Buildings:** the parts from the simulation's own `export_buildings`, and the gutted shells from the catalog's props.
- **The light:** `presentation.light` in `fixtures/game.json`.

The caller reads files and Rust decides. `Fielding` takes `fixtures/game.json`, the battle's rules, the resolved unit catalog and the runtime catalog. `contents()` names the runtime contents it reads (each drawn appearance's bundle, each fielded hull's wreck, every skeleton), and `resolve()` takes their inflated bytes. Native reads them from the runtime directory (feature `files`). The page fetches them through `AppearanceLibrary.contents` and calls the WASM `Fielding` (`fieldedBattle`). `unitPresentation.ts`, `effectPresentation.ts`, `clipTable.ts` and `presentationWorld` are deleted.

**Proven before the TypeScript was deleted.** Both resolvers ran side by side in the live page. Market-town's and paris-corner's reel sessions held 42,673 and 12,206 fields; the unit-roster lab's faction skirmish held 1,655 and 2,242. Every field and every clip-table float was equal, with two named exceptions:

- **Posed bounds:** these differ within 3.4e-7 relative. Rust poses them from the clip table's float32 rotations, the same table a renderer poses with.
- **Unfielded wrecks:** the TypeScript listed a wreck look (160 to 171 entries) for every hull whose art the page had installed. The page installs the whole catalog's unit art. Rust lists the fielded kinds' hulls only, so what a battle loads depends on what it fields, not on the page's loading history. A kind the side meets later is fielded again: its wreck comes with it before it can die in view.

Exact equality needed one fix. The crate's JSON parser was off by one float64 ULP in WebAssembly, so `serde_json`'s `float_roundtrip` was turned on, and is now on in every build (see "Every build reads JSON numbers exactly"). The presentation module grew from 2.92 to 3.24 MB (0.99 to 1.09 MB gzipped).

- **Reach:** a kind first requested mid-battle (a purchase, an identified enemy) loads after the page fetches its bundles, a few frames later than the synchronous TypeScript did. Until then the last battle's world stays drawn.
- **Factions:** a replay knows no skirmish factions, so it resolves one look per soldier. `fielded` takes the factions when a caller has them.

### An aircraft's shadow-visibility test assumes it flies at cruise height

_Agent, slice 20. Confidence: medium._

A helicopter can be out of view while its shadow is on screen, so it must stay drawn. The crate decides this by dropping the shadow point straight below the aircraft by a fixed `cruise_agl_m` from the rules, along the sun's direction. It does not use the terrain under the aircraft. This was provisional because Rust had no terrain heights before 20b. Rust has them now, and the rule stayed.

- **Reach:** over a hill or a valley, the test is off by the height difference. It only decides whether an aircraft is drawn, never where.
- **Verdict:** sound for now. Read the ground height if an aircraft ever pops in or out at a frame edge.

### Ghosts arrive with the ground height the UI computed

_Agent, slice 20. Confidence: medium._

A ghost is the translucent preview of a placement, a purchase or a destination while Space is held. The UI passes each ghost to `frame()` as a short JSON list with the ground height under it. Rust adds an aircraft's cruise height, then fields, poses and colours the ghost. Ghosts draw in their own pass, `GHOST_PASS`, with no shadow and no depth write. Rust now has the ground, but the UI still supplies the height.

- **Verdict:** sound. It is one number per ghost, and only while placing.

### The browser's battle score is seeded from `Math.random`, and native's replay from 0

_Agent, slice 25 (step 7b). Confidence: medium._

The score picks its next track with a shuffle bag on its own seeded `Mulberry32`. The browser seeds it once per battle from `Math.random` (`appAudio.ts`), so each battle still opens on a different track, as the TypeScript did. Native's replay seeds it 0 (`SCORE_SEED`), because a replay has no seed to take. The seed is an input to the crate, so the crate itself stays deterministic. A saved replay in the browser, however, may play a different score each time.

- **Verdict:** sound. Seed it from the battle's seed if you want replays to sound the same.

### Native's music duck and reverb are close, not exact

_Agent, slice 25 (steps 7 and 7b). Confidence: medium._

When a voice line plays, the music dips. That dip is the duck. The two players get there differently:

- **Duck:** the browser glides toward the new level with a time constant. Native glides linearly over the same span, because `kira` has no exponential approach. The depth is exact.
- **Reverb:** the browser convolves with an impulse it synthesizes in TypeScript (`reverbImpulse`). Native uses `kira`'s algorithmic reverb.

Rust decides every sound command, so these are differences in the players only.

- **Verdict:** sound; the differences are named. They matter only if native audio is ever compared by ear.

### The unseen-enemy shout pool is loaded only by the browser, so native is silent there

_Agent, slice 25 (step 7b). Confidence: medium._

When an enemy the side cannot see shouts, Rust picks a muffled stressed bark from the enemy faction's pool (`presentation::sound::frame`, `Sound::set_shouts`). Which takes form the pool is still the browser's (`shoutTakes` in `packages/battle-audio/src/voices/shoutPool.ts`), and native loads no pool, so in a native replay an unseen enemy's shout makes no sound.

- **Reach:** owned by `native/client/src/audio.rs`. Moving the pool's take list into `presentation::sound` would give it one owner for both clients.
- **Verdict:** sound as a named gap.

### `observation()` returns JSON, about 36 µs slower per publication than the old decoder

_Agent, slice 19. Confidence: medium._

The UI reads the side's observation (units, contacts, orders) once per new publication. Rust serializes the typed view with serde, and the page runs `JSON.parse`. That costs about 180 µs a publication, against the deleted TypeScript decoder's 144 µs: roughly 1 ms per second at 30 Hz. The fog bits and the ground runs skip JSON (`fog_bits`, `ground_runs`).

- **The alternative:** hand-written zero-copy views for every entity, which is a lot more code.
- **Verdict:** sound. Frame rate did not move. Every per-frame record in `Frame` is already zero-copy.

### A stale publication is dropped quietly, not refused

_Agent, slice 19. Confidence: medium._

18b said `push_publication` errors on a stale record. In a lab, switching sides leaves records for the old side in flight. `push_publication` returns `false` for those, and `invalidate(side)` resets the reader, as the TypeScript decoder did. A malformed record, or an index outside its table, is still an error.

- **Verdict:** sound; a small, deliberate deviation from 18b.

### Fog is prepared at encode time, by a second call

_Agent, slice 25 (step 6b). Confidence: medium._

Fog of war's sight is decided in Rust: the eyes, the occluders the side knows, the map slots, the rebuild schedule and `FogParams`. The renderer receives it as one `FogUpload` per frame. It is not part of `frame()`, though. The renderer calls `prepare_fog(camera)` when it encodes, because the rebuild schedule assumes every prepared frame dispatches, and the page sometimes frames its presentation on ticks it does not draw.

- **Reach:** a renderer makes two calls per drawn frame instead of 18b's one.
- **Verdict:** sound. Merging them would need `frame()` to know whether the frame will be drawn.

### Fog's foliage is still handed in from outside the crate

_Agent, slice 25 (step 6b). Confidence: medium._

Foliage the side has seen cleared stops blocking its sight. Working out which foliage cells are cleared needs the simulation's world. The page's presentation does not hold that world, so the page passes the cells in (`WorldView.foliage_cleared`), and native computes them with `files::side_foliage`.

- **Verdict:** sound. It is the one fog input not derived inside the crate.

### The building walk places everything in one call, with no per-frame budget

_Agent, slice 20b (step 1). Confidence: medium._

The walk decides which buildings are resident (expanded into their modules) for the current view, nearest first, while the pool has rows. The TypeScript spread expansion over several frames (`expand_rows`) to avoid a hitch. In Rust a row costs no JavaScript object, so the budget and its packing-by-kind were deleted, and both clients run the same rule.

- **Reach:** a hard camera cut into a dense city expands everything in that frame. No hitch has been measured.
- **Verdict:** sound. Reintroduce a budget in `walk.rs` if a cut ever hitches.

### Counts gates count what each list holds, not what the GPU draws

_Agent, slices 14, 15 and 17. Confidence: medium._

Each plate carries counts the browser took, and native must equal them:

- **`modelCounts`:** the posed population (soldiers, vehicles, aircraft, corpses lying or fading), not what is in view.
- **`effectCounts`:** the effects that have started, by kind, shape and light.
- **`instanceCounts`:** what each static layer draws into view (building modules per module and tier, scenery per population, props per appearance and tier).

These are exact and do not move with machine load. They prove the same lists were built. They do not prove the pixels: the plates are still judged by eye.

- **Verdict:** sound. Since both clients run one crate, the counts mostly check that each renderer drew what its `Frame` listed.

### The grass's window is read from the last frame's camera

_Agent, `derive2rust`. Confidence: medium._

The grass is grown on the GPU over a window around the view. Rust works out that window and the grass pass's per-frame parameters from the camera the last `frame()` call received (`grass_window()`, `grass_frame_params`). Neither renderer passes a camera anywhere.

- **Reach:** a renderer must call `frame()` before it asks for grass. Asking first gives the previous frame's window.
- **Verdict:** sound. It is the cost of having one owner for the camera.

### The "scars off" lab switch no longer brings cleared shrubs back

_Agent, `derive2rust`. Confidence: medium._

The ground lab can hide the learned scars (`suppressScars`). It used to do that by handing the scenery an empty cleared ground, which also stood up every shrub on cleared ground. Now the scenery follows the learned ground inside the crate, and the switch hides only the scars.

- **Verdict:** sound. Scars and scenery are separate things to compare.

### Labs without a battle make buildings fall through a stand-in for known props

_Agent, slice 20b. Confidence: medium._

In a battle, a building's state (intact, ruined, gutted) comes from the props the side has learned of. The city line-up has no battle. It passes `ui.known`, a stand-in list with each prop's id and box, and the crate reads it by the same rule. The line-up gives each building one synthetic part. A stand-in without a box affects buildings only.

- **Verdict:** sound. There is one rule for battles and labs, and no lab-only fallen list.

### A scene has one `Presentation`, and its world is loaded whole

_Agent, slice 20b. Confidence: medium._

A `Presentation` is the crate's per-side object. Each scene owns exactly one: the battle's, with its buildings loaded into it, or an empty one for a lab with no battle. The page stages the battle's half and the buildings' half (`setBattle`, `setBuildings`), and the next frame loads them together, because `load_world` replaces everything. Until the first publication arrives, the viewport draws the static world alone, so buildings show behind the loading cover.

- **Verdict:** sound.

### Native reads two of its tables out of the browser's TypeScript source

_Agent, slice 25 (steps 4 and 6). Confidence: medium._

Two small tables have their only copy in browser source files: the split-sum lighting table (`dfgLut.ts`) and the list of effect flipbooks (`flipbooks.ts`). Native includes those files' text at compile time and parses the numbers out (`environment.rs`, `effects.rs`), so each table keeps one owner. Native builds its flipbook mips as a simple box-filter chain. The browser's are resized by Chrome, which differs only for small, distant flipbooks.

- **Reach:** reformatting either TypeScript file can break the native build. The break is loud, at compile or start time.
- **Verdict:** sound. A JSON file both read would be cleaner if a third table joins them.

### Native paints no ground marks

_Agent, slice 25 (step 4). Confidence: high._

The painted ground marks are drawn by the UI, and a replay has no UI. Native binds the paint target and leaves it all zeros, which is the browser's own state in the reel, so every shader reads the marks as absent.

- **Reach:** owned by `native/client`, if it ever gains a UI.
- **Verdict:** sound as a named gap.

### The sky's two layouts keep TypeGPU's automatic names

Superseded by [specs/done/one-renderer](../one-renderer/README.md).

_Agent, slice 25 (steps 1 to 3). Confidence: high._

The export names the sky's bake and background layouts `bake - Automatic Bind Group & Layout` and `background - Automatic Bind Group & Layout`, and native looks them up by those names (`native/client/src/sky.rs`). It works, but the names are TypeGPU's defaults, not the browser's.

- **Reach:** owned by `packages/battle-renderer`'s sky, which would name them, and `native/client/src/sky.rs`, which must follow. Rename both together the next time the export is regenerated.
- **Verdict:** sound; cosmetic.

### Water is judged by shader vectors, never on a plate

_Agent, slice 16. Confidence: medium._

Neither reel map has water, and filming the `river` test map would need a new workload. Native draws water through the browser's own exported WGSL, so the water's look is the browser's by construction. No plate has ever shown it.

- **Verdict:** sound. Film a map with water if water's look ever changes.

### Native's GPU frame is 5–7% longer than the browser's

_Agent, on your "less code over speed" rule, slice 25 (step 8). Confidence: high._

Both clients draw the same passes and draws. Native's GPU frame still takes 5–7% longer: market-town 11.5–11.7 ms against 10.87, paris-corner 7.8–7.9 against 7.54. All of the excess is in the world colour pass. `wgpu` leaves Metal's per-resource hazard tracking on, so a later pass waits for the world's shading as a whole. Dawn makes a pass wait only at the stage that reads, so later passes' vertex work overlaps it.

- **No setting reaches it:** `wgpu` 30 has no Metal backend option and no untracked mode. The one setting that could matter, turning off indirect-draw validation, measured no change (step 8, `passoverlap`).
- **Why accept it:** native is at or above the browser on average FPS and well ahead on the 1% lows and frame p99, on both scenes. Closing the gap needs a patched `wgpu-hal` with hand-placed fences, or a frame that starts the next frame's cascades early. Either adds machinery against a budget already met.
- **To reverse it:** adopt untracked resources if `wgpu` ships them, then re-run step 8's interleaved pairs.

### The comparison runs under fixed conditions

_Agent (coordinator, on your behalf), slices 07, 12 and 25. Confidence: medium._

A performance comparison is run like this:

- plugged in (the bench records the power source with `pmset`);
- at 1280×720, three runs interleaved, under the shared GPU lock;
- native unpaced, with its window borderless and off-screen;
- the browser headless and paced by `requestAnimationFrame`.

On macOS, an off-screen `wgpu` window presents nothing, so the bench keeps at most two frames on the GPU, as a swap chain would. Both clients write raw per-frame intervals, CPU time and GPU time, and one summariser (`frameComparison.ts`) computes every number for both.

- **Why:** on battery the headless browser is held at 30 FPS for a reason never found. A shown window adds presenting cost that the headless browser never pays.
- **Verdict:** sound. The browser's pacing does not bind while its frames take over 8.3 ms. A browser faster than that would need GPU time compared instead.

### The decode reference is the deleted decoder's last output

_Agent, slice 19. Confidence: medium._

The committed `<scene>-observations.json` files are what the TypeScript decoder produced on two capture fixtures. That decoder was deleted when the Rust reader replaced it. `crates/sim/tests/publication_reader.rs` still holds the reader to those files. No fixture carries guided missiles, crashes, member leans, the encounter or skirmish objectives. Those rows are proven only by the web tests that now run through the reader.

- **Verdict:** sound. Re-record the reference from Rust when the layout changes, rather than keeping a shim.

### Native refuses a capture that is not the battle its manifest names

_Agent, slice 25; resolved `lastowners`. Confidence: high._

Native hashes `capture.json` and the saved map it loads (SHA-256) and checks both against `manifest.identity`. It also checks the capture's workload (scene, map, encounter, seed) against the identity, and reads the map and encounter from the hashed capture. On any difference it refuses to run and names the file (`native/client/tests/identity.rs`). This is the check Godot's runner did.

### Every build reads JSON numbers exactly

_User, `exactjson` (replacing `effectflake`'s local reader). Confidence: high._

serde_json's default float reader lands one ULP off on about one 17-digit number in fifteen; its `float_roundtrip` reader is exact. Cargo unifies a crate's features per binary, so when only `presentation` enabled it, the simulation read JSON exactly in the binaries that link `presentation` (native, the presentation module) and the default way in the rest (the browser's simulation worker, `mechanics_validate`, the map workbench, `cargo test -p sim`). Two copies of one simulation could read a map's numbers apart, and market-town's map alone holds 9,164 numbers of 16 or more digits.

- **The rule:** `float_roundtrip` is on in the workspace's one `serde_json` entry, which every crate takes. Native's local capture reader (`capture.rs`'s `exact`) is deleted: serde_json now reads the page's frame times exactly.
- **What moved:** nothing. Every digest, paired record and fixture is unchanged; no committed battle's outcome turned on a misread number.
- **Held by** a paired record, `fixtures/parity/exact-numbers.json`: a battle whose map, units and orders carry numbers the default reader misreads. `crates/sim/tests/exact_numbers.rs` checks the simulation reads each one exactly and pins its digests; `web/tests/exactNumbers.test.ts` steps the browser's simulation module on the same battle and must reach the same digests. With the default reader the browser's build ends at a different digest from tick 0. `native/client/tests/counts.rs` still holds native's counts on boundary frames.

### Labs that hand the renderer their own models take Rust's tier, caster and fog rules

_Agent, `derive2rust`; resolved `lastowners` and the final review. Confidence: high._

The tier rule, the caster rule and the fog group each have one owner in Rust: `view::model_detail`, `view::batch_caster` (over `caster_tier`) and `view::model_fog`. Battles write them into `Batch.tier`, `Batch.caster` and `Batch.fog`. A lab that calls `setModels` gets the same answers through presentation-wasm: `model_tiers(view, detail, models)` tiers every lab model in one call for the frame's `View` record (`View::from_record`), and `batch_caster(tier, pass)` and `model_fog(lies, state, ground)` name the rest. No TypeScript copy of any of the three remains.

- **One behaviour change:** in a lab, a model with fewer than four tiers no longer casts from its own coarsest tier. A battle never did this. The only such model is the workbench's deliberately broken tank.
- **Held by** `view.rs`'s tests: the thresholds, the cull past the shadow margin, an airframe drawn for its shadow, and a frame's `View` record tiering a model exactly as the frame's camera did.

### `min`, `max`, `round` and `signum` are Rust's own

_User, `jsretire`. Confidence: high._

JavaScript's versions differ only for NaN, −0 and negative halves. No battle draws on those on purpose.

### Every transcendental function is `libm`'s

_Agent, `jsretire` (after slice 20b). Confidence: high._

`hypot`, the trigonometry, `exp`, `log`, `pow` and `cbrt` all come from the `libm` crate (pinned `=0.2.16`), in both clients. Without that, Rust's `sin` is Apple's on native and `libm`'s in WebAssembly. A tree placed from a cosine could then land a hair apart on the two clients, and the counts gates would catch it as a difference. The simulation's `contract` crate uses `libm` for the same reason.

### Records are defined once in Rust and generated for TypeScript

_Agent (records); orchestrator with your confirmation (`Instance`), 18b and slice 20. Confidence: high._

`crates/presentation/src/records.rs` declares every record a renderer reads: `Batch`, `Instance`, `Joint`, `Effect`, `CastLight`, `View`, `Camera`, the light and fog uniforms, and more. Each is declared through one macro. A test (`records_are_current`) regenerates `packages/battle-renderer/src/records.gen.ts`, with TypeGPU schemas for each, and fails if the checked-in file differs. `Instance` is the browser's existing 64-byte model record (placement and yaw, data, tint, scale), not 18b's first 3×4 affine, which existed only for Godot.

### Batches name their caster tier and fog group

_Agent, `derive2rust`. Confidence: high._

`Batch` is 48 bytes and carries `caster` (the tier its shadow is drawn from, or none) and `fog` (`FOG_UNITS` never fogged, `FOG_GROUND` as the ground under it, face by face, or painted faces). Renderers only route by these fields. An impostor card casts nothing, as a card never did.

### Joint palettes are built in Rust

_Agent (orchestrator), under your "decide by profiling" rule, 18b question 1. Confidence: high._

A palette is the list of joint matrices that bend a soldier's mesh. The browser was GPU-bound, and posing cost under 1% either way, so Rust samples the clips and writes 48-byte joint rows. Both GPU pose kernels were deleted, and clip tables never reach a renderer.

### Rust chooses the cast lights

_Agent, slice 20. Confidence: high._

Muzzle flashes and fires light the scene. Rust culls them to the view, ranks them and caps them at 24 (`CAST_LIGHTS_MAX`). The renderer uploads the rows.

### Static batches are rewritten only when their list changes

_Agent, slices 20b and 25. Confidence: high._

Buildings, scenery and props are static batches.

- **Prints:** each list has a print, a fingerprint of what it holds, and a batch is written only when its print changes. Otherwise its `first` is `NONE`, meaning "unchanged": exactly what was last written, count included. Felled trees print by what they hold, not by a counter. `StaticBatches` mixes each population's print with the list's length, so no population can name a list unchanged under another count (`lastowners`). The scenery walk's prints had collided: a whole chunk's mark equalled an instance's, and the first instance's mark of 0 left a print unmoved. Main's renderer guard against that (`a9bc388fd`) had made the browser's scenery counts stale on one plate per scene.
- **Generation:** the renderer passes a generation number to `frame()` and bumps it whenever it holds no static batches: a new GPU frame, a new world, or a replay jumping between plates. A new generation rewrites every static batch. This replaced the old `resend_statics()` call.
- **Buffers:** each renderer holds a batch's buffer, sized to a power of two of records, while the frame lists it, and drops it when the frame stops listing it. Native keeps them in `statics.rs`, with the models layer.

### Publications that arrive before the world loads are kept

_Agent, slice 20. Confidence: high._

Appearances take seconds to install, and shots fired meanwhile must still produce effects. The crate keeps up to two minutes of publications (`PENDING_MAX`, 3,600 at 30 Hz) and replays them into the effects when the world loads.

### Corpses are tiered one by one each frame

_Agent, slice 20. Confidence: high._

A thousand sphere tests cost microseconds, so corpses take the same per-model tier rule as the living. The renderer's corpse chunks were deleted.

### Native poses every frame record, including the ones it does not draw

_Agent, slice 14. Confidence: high._

Poses carry state from frame to frame: a walker's stride phase, an easing gun, a fall. The replay therefore frames every record in order, from the capture's first record, and draws only the plate frames. Each record's publication is the sample at its tick, blended by its `simTime`, exactly as the browser's presentation clock blends.

### Fallen buildings are derived in Rust from the side's known props

_Agent, slice 20b. Confidence: high._

When the side learns that a building's parts are rubble, the building switches to its ruined or gutted state. This is re-derived only when the publication's known-prop group changes. A state the building's template lacks fails the frame by name, rather than drawing nothing.

### A side draws the map's props, less what it knows was replaced, plus what it knows

_Agent, slice 20b (step 5). Confidence: high._

One rule serves every scene. It replaced a split where movable and destroyable props came from knowledge and the rest from the map, which left cars and fences out of labs that have no side. Replacement is atomic: the list that drops a wall carries its rubble.

### A car keeps its paint wherever it is shoved

_Agent, slice 20b (step 5). Confidence: high._

A car's paint is seeded by the authored prop it is. A body the map did not author is seeded by its position in decimetres. Seeding by the published id made a shoved car change colour.

### Props are fitted in Rust from the art files

_Agent, slice 25 (step 5c). Confidence: high._

Each prop has a box from the simulation. Rust picks the appearance with the nearest footprint, scales it per axis or repeats a wall module along the box, and keeps the wreck that belongs to its own unit. It reads the art (footprints, paints, wrecks, bounds) from the asset files itself (`files::prop_art`), as it does for buildings and scenery. Props take the units' tier thresholds as authored and are bucketed in 64 m chunks for the walk.

### Native fights a map's battle under its category's catalog

_Agent, slice 25 (step 5c). Confidence: high._

A menu map uses the menu's units, and so on: the content rule in the root README. Native has no session to ask, so it reads the category from the saved map's metadata.

### The static world is resolved in Rust from the saved map

_Agent (orchestrator for the fold), slice 20b. Confidence: high._

`static_world` resolves the following from the saved map, the biome and the template art, identically for both clients:

- the buildings and their modules;
- the ground's shapes, plots and surface fields, and its terrain and grass tables and meshes;
- the height pages that fog and grass read;
- the scenery placement and the props.

Rust reads the runtime catalog, the template library and the kits' bounds itself (feature `files`, with `flate2` for the gzip transport). The page hands over bytes it already has. Nothing static travels through the manifest any more.

### Terrain and grass tunings are data in `fixtures/game.json`

_Agent, slice 20b (step 3). Confidence: high._

`presentation.terrain`, `presentation.grass` and `presentation.fog_geometry` hold the numbers that used to be shader literals. Rust reads and checks them (the biome through serde, which refuses a missing field). The browser's shaders splice the same values from the same file, so there is one owner and the WGSL did not change.

### The scenery is placed from each kind's finest mesh, in Rust

_Agent, slice 25 (step 5b). Confidence: high._

- **`place_scenery`:** reads a tree's size, trunk and crown off its finest mesh.
- **What counts as a tree:** a biome `trees.species` kind, which agrees with the catalog on every kind.
- **The felled stump:** cut in Rust (`stump_mesh`).
- **Felled trees:** pose in the vertex shader, so a falling tree is two `Instance` records. `Appearance.draws` tells a renderer which scenery shader draws a row.
- **Tiers:** native shares a kind's tiers across its rows, where the browser uploads a copy per row.

### Grass blade shapes are packed in Rust

_Agent, slice 25. Confidence: high._

Rust packs each grass kind's blade shapes from the bundle tiers the page hands it (`pack_shapes`), and gives the strip indices. Native derives none of the grass itself.

### The learned ground and its scar cache are Rust's

_Agent, slice 25 (step 4). Confidence: high._

Scars are the craters, scorch and tracks a side has seen. Rust keeps the learned ground and a bounded GPU cache of its pages. A renderer uploads the words it is handed, and draws region by region when the marks do not fit at once. Native draws every region rather than scissoring each one to the screen as the browser does. The picture is the same, and that path is cold in both reels.

### The camera, the view and the sun's cascades come with each frame

_Agent, `derive2rust`. Confidence: high._

The renderer passes the camera's eye, target and lens to `frame()` in `f64`. It gets back:

- the `View` (with `target`, where the haze is measured from, and the sky's corner rays);
- the camera uniform;
- the four sun cascades, fitted after `set_shadow(light, terrain)` was given once.

The browser's `frameCamera.ts` and `SunShadowFit` and native's camera code were deleted.

### Clearing lands one frame earlier than before

_User accepted (named decision), `derive2rust`. Confidence: high._

The scenery follows the ground the side has learned cleared inside `frame()`, from the publications already pushed. Both clients used to hand the cleared ground back after drawing, so a shrub on newly cleared ground left one frame later. `set_cleared` is deleted.

### The light is resolved in Rust into uniform records

_Agent, slice 25 (step 3). Confidence: high._

`presentation::light` resolves `presentation.light` into `SkyLight`, `Aerial` (the haze), `Sun`, `Bloom` and `Grade`, each laid out as its shader's struct. The browser reads them through `light_records(json)` (a frame can be built before any battle), and native through `LightSettings::records()`.

### The browser's shaders are exported once and built natively

Superseded by [specs/done/one-renderer](../one-renderer/README.md).

_Agent, slice 25 (steps 1 and 2). Confidence: high._

A Vitest test runs the browser's real frame code on a recording WebGPU device and writes every pipeline's WGSL and settings to `packages/battle-renderer/pipelines/`. `crates/pipelines` validates them with naga, and a drift test fails when the export is stale. Each pipeline carries a unique name, which is the key native looks it up by. The battle's light reaches the shaders through uniforms rather than spliced literals, so one compiled set serves every map and size.

### Native is one ordered pass list, with no render graph

_Agent, slice 25 (step 3). Confidence: high._

`Vec<Box<dyn Pass>>` is encoded in order each drawn frame, mirroring the browser's `battleFrame`: sky, prepass, world, effects, fog mask, ghosts, post, fog rim. Native presents by copying its display target to the window, so plates and the window read the same texture.

### Native's window is off-screen, and plates never depend on presenting

_Agent, slice 25 (step 3). Confidence: high._

The window opens borderless and off-screen, so nothing appears on your screen. On macOS, a window that is not visible presents nothing, so plates are read from the display target, not from the swap chain.

### Fog of war binds by what a surface is

_Agent, slice 25 (step 6b). Confidence: high._

Units are never fogged. Corpses are fogged as the ground under them. The static world is fogged face by face. Native binds each surface's fog group by its layer (`fog::Layer`), following the browser's rule, and the fog's look is a Rust record (`FogLook`) both read. A fog reel (`?fog=1`, `PARITY_FOG=1`) is the control for fog, since the menu reel draws none.

### Rust decides every sound, and players carry out commands

_Agent, slice 25 (step 7). Confidence: high._

The crate emits commands: start, glide and stop a voice, and listen from here. A player never reads `Frame.emitters`. Both players are as dumb as a mixer: equal-power panning is the only geometry they do. The browser's bank synthesizes and calibrates through the same Rust (`synthesize`, `source_normalization`). Native renders only offline (`--sound <wav>`) and never plays aloud in a replay.

### Music is three commands

_Agent, on your contract, slice 25 (step 7b). Confidence: high._

The commands are:

- `music`: stream a track from a time, looping, fading in;
- `music_stop`: fade every sounding track out, then free it;
- `duck`: dip the bus.

A crossfade is `music_stop` followed by `music` over the same span. Players keep no crossfade or duck state of their own.

### Native decodes its MP3s in place

_Agent, slice 25 (step 7b). Confidence: high._

Native decodes music and voice takes packet by packet with `symphonia`, the decoder `kira` uses. `kira`'s own streaming decodes on a thread and gaps unpredictably when rendered to a buffer. Native reads and decodes a voice take when a line plays it, and keeps the last 32 decoded, as the browser's `DECODED_TAKES` does.

### The worker reports the rules' tick rate as given

_Agent, slice 25 (effect counts). Confidence: high._

The page used to derive its tick as `1000 / (1000 / 30)`, which is 29.999999999999996 Hz, and every effect it timed drifted from native's at tick boundaries. The worker's `ready` now reports `tick_hz`, and the capture records the rate the page's presentation ran at.

### The publication reader lives beside its writer

_Agent, slice 19. Confidence: high._

`crates/sim/src/publication/reader.rs` and the writer share one group table (`GROUP_LAYOUTS`), so offsets are checked at compile time. The observation is typed: one struct per entity, declared by `view!` and serialized by serde. The reader is stricter than the old TypeScript: an index outside its table is an error.

### One runtime-bundle reader serves the crate and native

_Agent, spec review. Confidence: high._

`presentation::assets` reads the scene-assets container, the catalog and the gzip transport. Native loads its models through it, so the bundle format has one reader in Rust.

### Pre-existing bugs found on the way are fixed here

_User, slice 20. Confidence: high._

Bugs found along the way are fixed in this spec, not logged as open. The first one: a paused clock turned walking soldiers to idle. It was ported faithfully from the TypeScript, then fixed in the crate.

## Your decisions

These are your calls, recorded so later work treats them as givens.

### A Rust core with one shared presentation crate

_User, 2026-10-11 (phase 4)._

Per-frame presentation (decoding, interpolation, poses, effects, cook-offs, fog's sight, sound) lives in one Rust crate. The browser runs it through `crates/presentation-wasm` and native links it. The TypeScript originals and the native ports were deleted rather than kept in step by tests. The WASM module links the reader and the crate but not the rules, so the main thread never loads the simulation (18b question 7).

### Rust as smart as possible, renderers as dumb as possible

_User, 2026-10-11._

Rust decides everything per entity: what is drawn, at which tier, where, in which light, fogged how. A renderer draws the buffers it is given. Presentation reads only a side's publication, so it cannot see past fog of war. It never touches the simulation's state, so tuning it never changes a digest or a replay. `frame()` reads no wall clock and no unseeded randomness.

### A small, data-oriented API between Rust and every renderer, reviewed by you

_User, 2026-10-11 (18b)._

You approved:

- the shape: `World` once, `Frame` per drawn frame, flat typed records, no per-entity calls;
- static placement through `Frame`'s batches rather than `World` lists (question 3);
- fog's look in `Frame` (question 5);
- audio reading the presentation's emitters (question 8, now reached through commands);
- `ballistics` drawing its trajectories as overlay lines (question 6).

You decided separately that order lines, contact glyphs, markers and the ruler stay UI-built overlays read from `observation()`, and that ghosts stay in Rust (question 4).

### Native on `wgpu`, running the browser's own WGSL; Godot deleted

_User, 2026-10-11 (phase 5)._

Godot's per-primitive cost was the remaining GPU gap, and its roughly 2,000 lines of hand-ported shaders were a second copy of the browser's. Native is `native/client`, Rust on `wgpu` over the 18b API. Godot was frozen first (no Godot runs, no Godot glue), and it left no tech debt: nothing Godot-shaped entered shared code, and its project, binding and shaders are deleted. Bevy is consulted as a reference, not adopted. Whether the browser later moves to the same Rust `wgpu` renderer is decided after native is measured against TypeGPU.

### Web first

_User, 2026-10-11._

Every shared-code change lands in the browser first, with clean before-and-after frame rates, and only then does native switch to it. Each move into Rust deletes the TypeScript it replaces.

### Less code over speed

_User, 2026-10-11._

Each shared-code slice must reduce net code. A small measured browser cost is acceptable for that, and a meaningful regression still blocks. Promises do not stack: when one step grew code (20b step 2), it landed only together with the step that made the pair net-negative.

### Recompute over compatibility layers

_User, 2026-10-12 (`jsretire`)._

Once the TypeScript the crate was proven against was deleted, the emulation of JavaScript's number text, string hashing, `hypot` and rounding went with it. Every seed is now `Mulberry32` over a `seed::Key`: an FNV-1a hash of the numbers and names of what it decides for. Labels that cross the boundary stay strings. The vectors that moved (sound, chatter, effects, grass) were re-recorded from Rust.

### Music's mood is decided in Rust (battle voices and music's N2)

_User, 2026-10-12._

This settles [N2](../battle-voices-music/choices.md#n2-the-scores-mood-logic-stays-in-typescript). Rust's `sound::music::Score` picks the mood (tension, combat) from the side's own barks, picks the next track as a catalog id, and dips the music under voice lines. Players resolve an id to a file, and Rust never sees a file name. Menu music stays the app's (`AppAudio`).

### Restage street rather than change the HE rule

_User, slice 23._

After the shared code landed, red's tank killed street's rifle fight and street-watch's observer from 620–650 m across the open west. The two journeys were rerouted round the south of the west wood. The high-explosive rule was left as it is.

### What parity means

_User, phase 1._

The authority is a fresh browser control from the same tree, never the archived plates. A plate is paired with frame records from its own run, because the browser frame is not reproducible across runs.

- **Workload:** both clients render the same menu reel at 1280×720, one pixel per CSS pixel.
- **Judging:** each visual variable is judged by eye (the same shot, light, ground and grass impression), with no pixel budget. The only numeric gate on looks is grass within about 10% per screen region.
- **Acceptance:** the agent accepts a slice without blocking on you.
- **Platform:** only macOS with Metal is verified. Windows is not a requirement, but no native dependency may rule it out.

### One automated capture run, with the drawn view recorded

_User (base), phase 1; agent added the clock and size fields._

A headless run of `/graphics-test` writes each scene's capture, manifest and plates. Each frame record holds the view as the camera rig actually drew it (`eye`, `target` and `up`, after clamping, clearance and lag), the reel time, the tick, the interpolated sim time and the canvas size. The lens is recorded once per scene. `?plates=1` reads the canvas back in the task that drew it, so the shipped game's canvas is configured as before.

### No schema versions and no migrations

_User, phase 1._

The ids are `battle-presentation-capture` and `native-visual-presentation`, with no version suffix. A format change updates its writer, reader and tests together, and the strict readers refuse a stale file rather than migrating it.
