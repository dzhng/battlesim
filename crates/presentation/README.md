# Presentation

The per-frame presentation every client shares: one side's publications in,
a `Frame` of flat records out ([the presentation API](../../specs/done/native-rendering-parity/README.md#the-presentation-api-18b)).
The browser runs it through [its WebAssembly binding](../presentation-wasm/README.md),
and [the native client](../../native/client/README.md) links it. Rust is as smart as it can be and a
renderer as dumb: a renderer draws the records it is given and decides nothing
per entity.

It reads only the side's publication, so it never sees past fog of war, and it
never touches the simulation, so tuning it never moves a digest or a replay.
`frame()` is a pure function of the world it loaded, the publications pushed and
the frames asked for: no wall clock, no unseeded randomness. Every seed is
Mulberry32 over a [`Key`](src/seed.rs) hashed from the numbers and names of
what it decides for, and every transcendental function is `libm`'s ([the
simulation's rule](../sim/README.md#determinism-and-evidence)), so a dust puff,
a tracer's chance or a placed tree falls the same in WebAssembly and native.

[Settings admission](src/settings.rs) owns the presentation document's typed
sections and path-bearing load errors for both clients. Unknown fields and
invalid visual ranges fail before installation. Ground receives only its own
tuning sections; UI colours derive from the HUD, and fog selection belongs to
the Rust reader. The [sound library](src/sound/catalog.rs) admits source receipts,
crops, recipes and routing; the [mix](src/sound/mix.rs) checks its references
against that library before preparation. TypeScript carries these values and
calls the same reader for workbench drafts.

## What it owns

- [Records](src/records.rs): every record a renderer reads, defined once;
  `records_are_current` generates `packages/battle-renderer/src/records.gen.ts`,
  with byte offsets and typed views for browser consumers. The renderer's
  shader ABI check reflects WGSL against these canonical Rust layouts.
  [Shader constants](src/shader_constants.rs) names shared CPU/GPU values
  from their concept owners for both the WGSL prelude and TypeScript; fixed
  allocation geometry cannot become an independent fixture setting.
- [The runtime assets](src/assets.rs): the one reader of the scene-assets
  container the template library and the bundles travel in, and (feature
  `files`) of the runtime directory: its catalog and each gzipped transport
  file. The native client reads its models through it.
- [What a battle fields](src/fielded.rs): `load_world`'s units, effects and
  clip tables for the units a battle fields, resolved from presentation
  settings, weapon appearances resolved by the contract, the battle's rules and their catalog,
  and the runtime bundles,
  as a page wearing those units installs them, for both clients. It names
  the runtime contents it reads; the caller reads them (native from the
  runtime directory, feature `files`; the page by fetching them, through
  [its WebAssembly binding](../presentation-wasm/README.md)), and it decides.
- [External models](src/presentation/lab.rs): tooling admits authored art and
  descriptive poses independently of a battle roster. Its instances supplement
  the same frame, sharing pose math, detail, fog and batch identity with battle
  models. [Catalog admission](src/presentation/catalog.rs) stages prospective rows and
  pose and placement facts without replacing the observer. Prop fitting, building
  extents and scenery trunks/stumps follow the new authored art while current
  learned, fallen and held movement state survives. Matching GPU assets and the latest
  model requests commit together; stale or failed work preserves the active
  generation. Battle replacements preserve the pose vocabulary used by retained
  history. Unchanged time still permits edited geometry. Mesh
  selection cues retain actual geometry rather than silently disappearing when
  detail would otherwise select an impostor.
- [Units](src/units/): interpolation between ticks (of units, falling
  airframes and the bodies they shove), the pose feed and driver,
  articulation, joint palettes (and each skinned row's rest palette: a corpse
  as it lies, a body in its far pose, which a renderer poses its mesh by once),
  and where each drawn model's muzzles and rotors are.
- [Effects](src/effects/): the effect publication, launches (the one derivation
  the flashes, the firing soldiers and the gunfire sounds read), the effect frame,
  the cast lights it chooses for the view, cook-offs, and the sparks a hull
  throws where it grinds a metal body it shoves (the simulation publishes
  where they meet).
- [The view](src/view.rs): what the camera means for culling and detail, the one
  tier rule every drawn model takes and the tier it casts with, and the
  frame's `View` record (with the sky's corner rays) and camera uniform,
  the browser's camera math to the bit. Each batch also names the fog group
  it binds (`Batch.fog`). Battle and external tooling models take these same
  policies before becoming canonical frame records.
- [The light](src/light.rs): the fixture's light resolved into the sky's,
  bloom's and grade's uniform records, as the browser's sky model resolves it.
- [The wind](src/wind.rs): the biome's one wind as the `Wind` record, which
  every camera carries on its clock (`Camera.wind`, the sun's cascades
  included), so grass, water and trees move in it in every pass, and which
  the effects drift smoke and dust on.
- [The wave field's clock](src/wave.rs): how many steps a frame runs the
  renderer's height field over the water round the camera, at a fixed rate
  on the presentation clock, when it starts over, and where it lies
  (`Camera.waves`). [Wakes](src/effects/frame/wake.rs) are its splats: rings
  and V-wakes where a unit the side sees wades, decided per tick from what
  is published, never from how often frames are drawn.
- [Scars](src/scars/): the side's learned ground, from its publications'
  patches, and the bounded GPU cache of its scars. A renderer uploads what it
  is handed (the directory's words, the pool's texels, `ScarParams`' words
  before the biome's look) and asks which regions to draw one by one when the
  marks don't fit at once; it derives nothing.
- [Fog of war's sight](src/fog/sight.rs): each frame's eyes (the side's own
  units' at the published tick, or a lab's stand-ins), the occluders the side
  knows stand (the map's occluding props less those it saw fall, plus what it
  learned), the eyes' map slots and rebuild schedule, the structures that
  take fog whole, and the `FogParams` record, as one `FogUpload` a renderer
  uploads as it is. A prop the side has not learned never reaches it. The
  unseen look is [`fog`](src/fog/mod.rs)'s `FogLook`.
- [The sun's shadow](src/shadow.rs): each frame's cascades fitted over where the
  terrain is in view depth, as the receiver block and each cascade's caster camera
  (`Frame.shadow`, once `set_shadow` gives the light and immutable terrain).
  Receiver fitting queries the terrain's draw hierarchy, then clips individual
  triangle bounds with standing-object headroom to the camera. Offscreen remote
  elevations cannot stretch a local fit through a map-wide height envelope.
  This is conservative geometry, not occlusion: horizon-visible terrain still
  contributes, and the light's configured shadow reach still applies.
  Unchanged cameras reuse the installed light/terrain fit; replacing those inputs
  or loading a world invalidates it. Receiver diagnostics count work executed in
  the latest frame, with zero traversal and clipping on reuse. They remain
  separate from terrain draw selection and flow into viewport measurements.
- [The static world](src/static_world/): the buildings resolved from their
  template art, and the ground from the map's exports under the biome: its
  tables, its meshes (terrain, skirt, water, backdrop), its standing water
  (`StandingWater`, the one owner of where rivers, swamps and puddles hold
  water, how deep, and which look each draws), the grass field's
  shapes, and its window and per-frame parameters for the last frame's
  camera, and the scenery; and the map's props fitted to their boxes from the
  installed art, less those the side knows replaced, with what it knows
  (rubble, wrecks, shoved bodies); a body it watches being shoved is drawn
  where it stands each frame, ahead of what it has learned. Buildings, scenery and props are walked
  per view into the frame's static batches, written only when a list
  changes; the scenery leaves the ground the side has learned cleared.
  Buildings, standing scenery and props select casters against fitted cascade cameras, using
  the same conservative clip bounds as terrain. They publish independent
  camera and per-cascade caster lists: shadow selection cannot
  remove a camera mesh, and leaving the camera cannot remove a shadow
  that reaches visible receivers. Each caster list is submitted only to its named
  cascade; overlapping volumes duplicate records to avoid vertex work in unrelated
  cascades. Camera and caster lists use the same side knowledge.
  Static populations query the same bounds index as terrain, then restore source
  order before emitting records so list identity and detail decisions stay stable.
  Known props and ruins rebuild their index when knowledge changes. Traversal
  stats count tested hierarchy bounds, separately from emitted instances.
  Standing trees, felled trees and stumps share an immutable hierarchy enclosing
  full fall motion, with independent camera and cascade membership. New knowledge
  filters selected records rather than rebuilding forest geometry. Standing bounds
  still control standing detail; offscreen animation advances from presentation
  time. Full known-fall snapshots still cost linear reconciliation/equality work,
  distinct from local selection and one-time envelope setup.
- [Sound](src/sound/): what a battle hears, when, where and how loud, from the
  same publication the effects read, the frame's emitters and the side's
  hearing cues, as commands a player carries out (start, glide, stop a voice;
  listen from here); which units' voice lines play, started on the voices
  bus, a shout the side hears in the open asking for a bark in its kind's
  set and spoken where it was shouted; and the synthesized baselines and
  recipe buffers every player prepares alike. `tests/sound.rs` holds it to
  the commands recorded over the sound lab's firefight, and `tests/voices.rs`
  to the lines recorded scripts start and drop.

## How a battle looks

Soldier poses follow actual published soldiers rather than formation slots. A
soldier down in a posture, or shooting, keeps his weapon toward the threat as he
moves and steps, crouches or crawls the way he goes, with a clip for every way;
otherwise, and at a run, he turns to face where he is going. No posture slides
along the ground. His gait reads his speed, ground over time, so while the clock
stands still (paused, or between ticks) he holds mid-stride and resumes the same
step. A soldier dies from the posture he is in, never getting up to
fall: standing, most are thrown onto their backs and the rest pitch onto their
faces; kneeling or lying, they fall forward. His corpse lies as his fall ended,
and past the corpse cap the oldest sinks away.

An effect on a barrel follows the drawn muzzle; its physical launch remains the
simulation's event. A shot kicks its shooter as a war film shows it, harder
than life: a heavy gun throws its hull back on its springs, away from where the
round went, to rock past rest and settle without moving off its spot, and shakes
dust off the whole hull and off the ground ahead of the muzzle; each of a
soldier's own shots snaps his upper body back about his waist, his muzzle
climbing. How hard each round kicks is its weapon look's `kick` and its flash's `dust`;
`presentation.pose` owns the shared spring and recovery curves. The whole-model motion
carries the drawn muzzle. A flying aircraft's rotor is drawn by the effects from the
same drawn pose, as a film camera's shutter sees it: each blade smeared over the
angle it turned while the shutter was open, so a fast rotor blurs at any frame
rate and a paused one shows its blades.

An aircraft's shadow follows its altitude layer, never what kind of aircraft
it is. A low-air one casts its real shadow along the sun. From the high-air
layer's height the sun would throw a shadow hundreds of metres off, where it
reads as nobody's, so a high-air one casts none: a soft silhouette of its plan
lies on the ground straight beneath it instead, turned with its heading,
fainter and softer the higher it flies (`presentation.effects.high_air_shadow`).
It is an effect instance laid on the ground, under every other effect, and
shades only the ground's own surface, never a unit standing in it.
A high-air model is batched for its own pass (`HIGH_PASS`), which casts
nothing: a client that lays cards over the world (the browser) draws those
batches on a plane of their own above the ground and low-air cards, and any
other client draws them with the view.

A hull the side watched die cooks off: fireballs from its ring, its hull jolted
and its turret thrown, both pieces of its own unit's wreck, until they lie where
the wreck has them and the whole wreck takes over without a seam. A wreck without
detachable pieces jolts as one body. What the blast throws clear lies still for a
while, then sinks into the ground and is gone; a wreck found later has none.
Every watched vehicle keeps its last drawn look until the blast and follows its
roll to the resting wreck.

## Checks

Rust is the only owner of everything here, so the recorded vectors are its
own output: first recorded from the TypeScript each module replaced, and
re-recorded from Rust when a change moves them on purpose. `tests/poses.rs` and
`tests/effects.rs` hold it to vectors from native's fixture capture and fixed
effect causes, `tests/light.rs`, `camera.rs`, `ground_meshes.rs`,
`grass_field.rs`, `fog_sight.rs`, `sound.rs`, `voices.rs` and `music.rs` to
recorded vectors bit for bit, and `tests/scars.rs` holds every scar upload to
recorded cache words. `tests/shadow.rs` holds receiver coverage and locality
through the production fit. The behaviour tests beside them say what the game does,
and hold through any re-recording.
