# Native rendering parity

The renderer half is superseded by [One renderer](../one-renderer/README.md).

The native [desktop shell](../../../native/client/README.md) and the browser
share [presentation](../../../crates/presentation/README.md) and the
[render runtime](../../../crates/renderer-core/README.md). This spec records why
presentation has one Rust owner and the historical evidence for native parity.
Rust is the only authority for rules, observation, visibility, replay and digests.

Every decision this spec left in the code, with who made it and how confident the
audit is, is in [`choices.md`](choices.md), the final ledger. Treat its entries as
givens. The build history (slice files, per-step evidence and measurements) was
deleted at close and lives in git at `2849c8a09`, under
`specs/native-rendering-parity/slices/`.

## Why it is built this way

The first native client was a Godot project with hand-ported shaders, built
over phases 1 to 3. Its outcome set the shape of what shipped:

- **Ported formulas over exported data worked for looks.** Camera, light, terrain
  and grass matched the browser once native read the browser's recorded frames
  and the fixture data and ported only the formulas. Visual parity was never the
  hard part.
- **Godot was the wrong engine for this game.** Its Forward+ path costs more per
  primitive than the browser's lean passes (a separate depth prepass,
  per-instance material and shadow work at 4× MSAA, about 8.6 ms of fixed setup).
  Its roughly 2,000 lines of shaders were a second hand-kept copy of the
  browser's. GDScript cost about 6.5 ms a frame just to walk the static world. So
  native moved to Rust on `wgpu`, running the browser's own WGSL, and Godot was
  deleted (about 15,000 lines). Nothing Godot-shaped entered shared code.
- **Two clients deriving the same things meant two copies of every rule.** So
  everything per entity moved into one Rust crate both clients run. The
  TypeScript it replaced was deleted rather than kept in step by tests: decoding,
  interpolation, poses, effects, cook-offs, cast lights, the static world's
  placement and walk, the camera and the sun's cascades, fog of war's sight,
  sound and the score's decisions. Each move was proved equal to the deleted
  TypeScript before the delete, and each shared-code move had to reduce net
  code.

[Bevy](https://bevy.org/) was consulted as a reference for pass structure,
batching and windowing. It was not adopted.

## Principles and invariants

These must stay true. Where a test or a type holds one, it is named; the rest
are working rules that review holds, with no check behind them.

- **The presentation API (18b) is the one boundary between Rust and a renderer.**
  It is described [below](#the-presentation-api-18b). A change to it is a
  reviewed decision, not an implementation detail.
- **Rust is as smart as possible and each renderer as dumb as possible.** Rust
  decides everything per entity: what is drawn, at which tier, where, in which
  light, fogged how, and what is heard. A renderer only uploads the records it is
  handed and draws them, routing by fields such as `Batch.caster` and
  `Batch.fog`. A renderer must not decode a publication, interpolate, choose a
  tier or a resident, cull an entity, pick or cap lights, seed randomness, decide
  anything from gameplay or presentation tuning, or name a unit kind, side, state
  or order. A fact a renderer needs goes into a record, as data.
- **One owner per concept.** Records are declared once in
  `crates/presentation/src/records.rs` and generated into
  `packages/battle-renderer/src/records.gen.ts` (`records_are_current` fails on a
  stale file). The [shared renderer](../../../crates/renderer-core/README.md)
  now owns shader composition and typed pipeline settings; its ABI reflection
  validates assembled WGSL with naga. The publication reader lives beside its writer
  (`crates/sim/src/publication/reader.rs`, one `GROUP_LAYOUTS`). Runtime bundles
  have one Rust reader (`presentation::assets`), and a battle's world has one
  resolver (`presentation::fielded`). Visual tunings belong to
  [`fixtures/presentation.json`](../../../fixtures/presentation.json), admitted by Rust.
- **Determinism.** `frame()` is a function of the world loaded, the publications
  pushed and the frame inputs: no wall clock, and no randomness it does not seed
  itself. Every random draw is Mulberry32 (`crates/presentation/src/seed.rs`),
  seeded from what it decides for: a `seed::Key` hashed from names and numbers,
  or an id, a tick or the biome's seed. The one seed a caller supplies is the
  score's, which the browser takes from `Math.random` (`choices.md`). Every
  transcendental function is `libm`'s (pinned), so WebAssembly and native place
  the same tree and start the same puff. The one std call, a `log2` rounded up
  from an integer for the scar cache's width, places nothing. Every build reads JSON numbers
  exactly: `float_roundtrip` is on in the workspace's one `serde_json` entry, held
  by `fixtures/parity/exact-numbers.json` through `crates/sim/tests/exact_numbers.rs`
  and `web/tests/exactNumbers.test.ts`. The page's presentation runs at the
  rules' tick rate as given, never a rate recomputed from milliseconds.
- **Presentation observes; it never decides the battle.** What changes in a
  battle reaches it only through the side's publication, so it cannot see past
  fog of war. Beyond that it reads only the map as authored (the saved map, its
  ground, foliage and props, through the simulation's own export code). It never
  writes the simulation's state, so tuning it never moves a digest or a replay.
  Convention and review hold this; no type does.
- **Less code over speed.** A shared-code move must reduce net code. A small
  measured browser cost is acceptable for that; a meaningful regression still
  blocks. Machinery that only buys speed against a budget already met stays out:
  the historical 5–7% GPU gap at parity close was accepted on this rule.
- **Nothing silent.** Native never falls back, caps or skips anything silently,
  never invents visibility or a presentation layer, and never infers dust or
  smoke from camera motion or map geometry. It refuses a capture or a saved map
  that is not the battle its manifest's identity hashes, naming the file
  (`native/client/tests/identity.rs`).
- **No schema versions and no migrations.** The capture is
  `battle-presentation-capture` and the manifest `native-visual-presentation`. A
  format change updates its writer, reader and tests together, and the strict
  readers refuse a stale file.

## The presentation API (18b)

The user reviewed and approved this boundary on 2026-10-11, before any renderer
was built to it. The reviewed design is pictured in
[`visualizations/presentation-api.html`](visualizations/presentation-api.html).
The code is `crates/presentation/src/presentation.rs` (`Presentation`, `Frame`,
`World`) and `crates/presentation/src/records.rs`.

**Shape.** One `Presentation` per viewed side. A renderer gets a `World` once
per battle (`load_world`) and a `Frame` per drawn frame (`frame`). Both are flat,
typed record arrays. There are no per-entity calls, no callbacks and no
renderer-to-Rust queries about entities. Publications go in as the words the
simulation packed (`push_publication`). The game UI reads the decoded observation
(`observation()`), and overlays (order lines, contact glyphs, markers, the
ruler) stay UI-built from it. Ghosts stay in Rust, because they need the same
fielding and posing as every unit.

**What a frame carries.** The `View` and camera uniform the browser's camera
math gives, bit for bit (`tests/camera.rs`), and the sun's cascades once
`set_shadow` has been given the light and the ground. Then the instance batches
by appearance, tier and pass; joint palettes, which Rust samples (no clip table
reaches a renderer); pick boxes and the gates' counts. Static batches
(buildings, scenery, props) are rewritten only when their list's print changes.
Otherwise `first` is `NONE`, meaning "unchanged". A renderer bumps the
`generation` it passes whenever it holds no static batches, and a new generation
rewrites every one.

**Where it grew beyond the reviewed design.** Each change is a named entry in
`choices.md`:

- `Instance` is the browser's existing 64-byte model record, not the 3×4 affine
  first drawn for Godot's MultiMesh.
- Effects and cast lights are read beside the frame (`Presentation::effects`,
  `lights`). Rust has already culled, ranked and capped the lights (24).
- Fog of war's sight is a second call at encode time (`prepare_fog`, one
  `FogUpload`), because the rebuild schedule assumes every prepared frame
  dispatches.
- The ground and its tables come through `set_ground`, and the grass's window is
  read for the last frame's camera (`grass_window`). A renderer gives its camera
  only to `frame`, and hands the frame's own camera record back to `prepare_fog`.
- Light is five uniform records (`light_records`, `LightSettings::records`),
  available before any battle loads.
- Sound is commands a player carries out (`presentation::sound`, the score in
  `sound::music`). A player never reads `Frame.emitters` itself.
- `push_publication` returns `false` for a stale side's record, and
  `invalidate(side)` resets the reader. A malformed record is an error.
- Setup and per-layer calls sit beside `frame`, each handing a renderer words
  to upload: the scars (`prepare_scars`, `scar_regions`), the scenery and props
  (`place_scenery`, `place_props`, `stump_mesh`), fog and detail settings
  (`set_fog`, `set_detail_scale`), a corpse's rest palette, and the sound's
  hearing (`heard`). The list on `Presentation` in `presentation.rs` is the
  authority.

**Conventions.** Sim space throughout (right-handed, `z` up), metres, radians,
seconds on the sim clock. Projection is reverse-Z with an infinite far plane.
Records are `#[repr(C)]` multiples of 16 bytes, valid as WGSL structs and vertex
attributes as they are. The browser follows the
[binding lifetime contract](../../../crates/presentation-wasm/README.md).

## What parity means, and how it is proved

**The authority is a fresh browser control from the same tree.** Never use the
archived plates in [`../native-rendering/visualizations/`](../native-rendering/visualizations/):
the browser plate has the HUD burned in and was cropped by an unrecorded method,
and the native plate is mirrored. The browser frame is not reproducible across
runs (real-time ticks, follow smoothing and wind on `performance.now()`), so a
plate is paired with the frame records of its own run.

- **The workload.** The menu reel's two scenes, market-town and paris-corner,
  at 1280×720 and one pixel per CSS pixel. A headless run of `/graphics-test`
  (`web/scenes/native-parity-capture.mjs`) writes, per scene, the capture, the
  manifest and the plates. The capture holds the publications and each frame's
  drawn view, reel time, tick, interpolated sim time and canvas size. The
  manifest holds the identity, viewport, lens, drawn layers and per-plate counts.
  `PARITY_FOG=1` films the fog reel, the control for fog. Native reads everything
  else about the battle from the repository.
- **Counts are exact.** Per plate, native's model, building-module, prop,
  scenery and effect counts must equal the browser's. They do on every plate of
  both scenes, fog off and on, over five consecutive fresh captures (120 plates).
  `native/client/tests/counts.rs` holds this on every frame of a committed
  count-only run (`fixtures/parity/capture/`), tick boundaries included, with no
  GPU. Counts prove the same lists were built, not the pixels.
- **Looks are judged by eye,** side by side, with `compare-screenshots` and an
  unprimed `screenshot-critique`. There is no pixel budget. The last whole-set
  critique (all 24 pairs) found differences only in the tank's road wheels,
  soldiers' poses and sub-threshold noise. The grayscale distance was
  0.02–0.11 on market-town and 0–0.06 on paris-corner. Native's market-town
  plates differ in scattered grass pixels between reruns of unchanged code
  (the grass build appends through atomics), so byte comparisons need that
  floor.
- **Performance runs under fixed conditions:** plugged in, at 1280×720, with
  three runs interleaved under the shared GPU lock. Native is unpaced, with its
  window borderless and off-screen; the browser is headless and paced by
  `requestAnimationFrame`. One summariser
  (`web/src/battle/benchmark/frameComparison.ts`) computes every number for both.
  At close, native is at or above the browser on average FPS and clearly ahead
  on the 1% lows and frame p99 on both scenes. Market-town ran 83.2 against
  83.2 FPS, with p99 17.4 against 23.4 ms. Paris-corner ran 123.2 against
  108.0, with p99 11.2 against 16.7. Native's GPU frame was 5–7% longer, an accepted
  historical tradeoff.
- **The browser was proved whole on the shared code:** the web suite, the
  visual suite and every scene green, the medium battle no worse than before the
  move, digests and replays unchanged.

## Map into the code

| Concept                                                          | Owner                                                                                                    |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Per-frame presentation, the API                                  | `crates/presentation` (`presentation.rs`, `records.rs`, `view.rs`)                                       |
| The browser's binding                                            | `crates/presentation-wasm`                                                                               |
| A battle's units, effects and clip tables                        | `presentation::fielded`, `presentation::assets`                                                          |
| Static world: buildings, ground, grass, scenery, props, the walk | `presentation::static_world` (`walk.rs`, `ground`, `props`, `files`)                                     |
| Fog of war's sight and look                                      | `presentation::fog` (`sight.rs`, `FogLook`)                                                              |
| Sound and the score                                              | `presentation::sound` (`frame.rs`, `music.rs`)                                                           |
| Publication reader                                               | `crates/sim/src/publication/reader.rs`, held by `crates/sim/tests/publication_reader.rs`                 |
| Shader composition and validation                                | `crates/renderer-core/src/shaders/`, `crates/renderer-core/tests/pipeline_abi.rs`                       |
| GPU resources and frame encoding; desktop services | [Shared runtime](../../../crates/renderer-core/README.md); [native shell](../../../native/client/README.md) |
| Capture run and control                                          | `web/scenes/native-parity-capture.mjs`, `apps/battle-lab/src/benchmark/reelRun.ts`                       |
| Frame-rate comparison                                            | `web/src/battle/benchmark/frameComparison.ts`, `web/fps-compare.mjs`, native's `--bench`                 |
| Committed parity fixtures                                        | `fixtures/parity/`                                                                                       |

## Dead ends

Each was tried and rejected; don't walk it again.

- **Godot.** Its per-primitive cost and its second copy of every shader. Caster
  bucketing, more frames in flight, threading and a Rust walk inside a
  GDExtension all helped. None closed market-town's GPU gap, which sat in the
  engine and not in the instance set.
- **Moving native's GDScript into Rust inside Godot.** That was slice 21. It was
  dropped, since Godot was leaving anyway.
- **Emulating JavaScript's arithmetic in Rust** (number text, string hashing,
  `hypot`, rounding). It was needed only while the TypeScript the crate was
  proved against existed. It went with that TypeScript, and the vectors were
  re-recorded from Rust.
- **The affine `Instance`** existed only for Godot's MultiMesh. Migrating every
  model shader to it would have been churn with no lasting owner.
- **GPU pose kernels.** The browser was GPU-bound, and posing cost under 1%
  either way. Rust palettes deleted both kernels.
- **Static placement in `World` lists.** That would upload over a million
  instances to draw at most about 63,000. Batches written on change replaced it.
- **The manifest carrying placement, units, props and effects tuning.** All of
  it moved into Rust, read from the repository. The manifest keeps identity,
  viewport, lens, frame records and counts.
- **`resend_statics()`.** The static generation in `frame()` replaced it.
- **A per-frame budget for building expansion.** In Rust a row costs no
  JavaScript object, so both clients expand in one call.
- **A separate `crates/static-world`.** The static world first landed there and
  was folded into `presentation::static_world`.
- **A local exact-number reader in native.** It fixed native alone and left the
  browser's simulation worker reading maps the default way. The workspace-wide
  `float_roundtrip` replaced it.
- **A tick rate derived as `1000 / (1000 / 30)`.** It is 29.999999999999996 Hz,
  and it moved every effect at tick boundaries.
- **Closing the GPU gap by a `wgpu` setting.** `wgpu` 30 has no Metal backend
  option and no untracked mode. Turning off indirect-draw validation, the one
  candidate, measured no change.
- **Comparing on battery or in a shown window.** On battery the headless
  browser holds 30 FPS for a reason never found. A shown window adds presenting
  cost the headless browser never pays.

## Remaining platform gaps

[Native gaps](../../native-gaps/README.md) owns the remaining platform differences;
[One renderer](../one-renderer/README.md) records the shared runtime's rationale
and the limits of historical performance comparisons.

## Visual provenance

- **The fresh browser control** is the standard native was held to. It is
  regenerated from the tree it judges, so no plate is committed as the target.
  The controls named in the build history lived in the ignored scratch folder.
- **[`../native-rendering/visualizations/`](../native-rendering/visualizations/)** is
  the earlier probe's browser and native plates (market-town, a close cut, and
  native's city horizon). They show the gap this spec started from, and are
  provenance only, for the reasons above.
- **[`visualizations/presentation-api.html`](visualizations/presentation-api.html)**
  is the API diagram the user reviewed before any renderer was built to it.
