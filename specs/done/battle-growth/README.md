# Battle growth: a new unit type costs only its own art

When a running battle first shows a unit type, the page does work for that
type's art and nothing else. Before this, every arrival rebuilt the whole
scene's art: the player's large battle froze for 2.9–9.9 s each time, five
times, and the new unit appeared 11–40 s later
([the freeze report](assets/freeze-report-2026-11-14.json), which also holds
the reference battle's link).

What the player sees now: a unit whose art is not ready stands as a plain
block for a moment, can be clicked and attacked, and then becomes its model.
In the small test skirmish an arrival no longer produces a frame the browser
reports as long. In the reference battle an arrival is a few frames of about
50–120 ms, which is the gap left against "never notice": see
[Left open](#left-open).

## Why it is built this way

The freeze had two causes.

**Nothing a scene's art needed outlived a generation.** Every admission
rebuilt the texture arrays and the impostor cards whole, re-sent every art
file from the page, and resolved the description of every fielded unit again,
wherever the new type sat.

**And what was rebuilt could not have been kept anyway.** The map of packed
geometry lived for one admission only, and a model's material numbers were
baked into its vertices in the order names were first seen, so even a mesh
that had been kept would have been wrong after a type that sorts earlier.

The fix is one rule for both: **what is stored for a piece of art depends only
on that art's content, and it is stored once per scene.** Each piece gets an
address when it is first seen and keeps it for the life of the scene. New art
takes the next free address. Nothing that exists is renumbered, repacked,
re-uploaded or re-sent.

The alternatives were rejected for these reasons:

- **A per-draw material offset in the shaders** would have removed the
  renumbering without stable addresses, but touches every model pipeline and
  the instance record has no spare field.
- **Caching inflated files on the page** would have held the catalog's
  geometry a fourth time. The page keeps its parsed art and no copy made only
  to cross into Rust.
- **Preloading the enemy faction onto the GPU** was the player's first choice
  and was built behind a switch, off by default, and measured at 3.0 GB more
  settled memory. The player then ruled out the switch: the game is
  zero-config, so a feature is on for everyone or not built. Between those
  two the agent chose not built, because a one-type arrival no longer needed
  it. The download warm-up it rode on stayed.
- **A worker or thread** for staging: the renderer's WASM memory is not
  shared across threads, which is a larger change than the hitch was.

## What must stay true

These are the rules the code is meant to keep. Each says how far it is
enforced; where nothing but review holds it, it says so. The
[renderer core's contract](../../../crates/renderer-core/README.md) states
the first four for the art store.

1. **Identity is content, never a name or a row.** A runtime catalog entry's
   identity is its bundle hash and, for skinned art, its skeleton hash
   ([`ContentId`](../../../crates/contract/src/identity.rs)). A reuse key holds
   every input that changes the stored bytes and nothing else. A pose is not
   one for geometry: living and dead bodies of a type share one mesh.
2. **Addresses are append-only.** A material slot or a texture or card layer
   never moves while the [store](../../../crates/renderer-core/src/models/store.rs)
   lives. A dropped preparation keeps what it reserved. Running out is an
   error or a fresh store, never a wrap.
3. **The last holder destroys.** Generations hold GPU payloads; the store's
   indexes are weak. Retiring one generation cannot free what another draws.
4. **One admission path.** The first scene, a recreated device and an
   exhausted store are the same admission (`Models::from_world`) with an empty
   store. Warming a store ahead of an admission
   ([`GeometryDraft`](../../../crates/renderer-core/src/models/warming.rs))
   only makes that admission find what it needs. A recreated device is
   exercised by the `frame` scene, not by a test that reads its rebuild
   reason.
5. **The swap is atomic.** Nothing of a preparing generation is drawn before
   it commits. A stale, failed, dropped or superseded preparation leaves live
   art as it was.
6. **No transport copy of geometry stays on the page.** The page's parsed art
   is its one durable copy. It declares wanted
   art by name and identity, asks which identities are missing, and sends
   bundles for those alone. A buffer made only to cross into Rust is
   [released](../../../web/src/battle/transport.ts) when its last reader has
   copied it. The description of a fielded battle stays in Rust behind a
   handle (`Fielded` in
   [`presentation-wasm`](../../../crates/presentation-wasm/src/fielded.rs),
   built in [`fielded/result.rs`](../../../crates/presentation/src/fielded/result.rs)).
   Not covered: encoded textures and skeleton clips are cached on the page
   and sent again with any missing bundle that names them
   ([Left open](#left-open)).
7. **Content work at an arrival shares one per-frame budget**
   ([`artWork.ts`](../../../web/src/battle/artWork.ts), bound beside the
   store's policy): supplying bundles, absorbing fielding facts and warming
   geometry. Nothing enforces that new staging work joins it, and the final
   admission, the card bake, the commit and the library's parse of new files
   are outside it.
8. **A unit the side sees is on screen and clickable before its art
   arrives.** Its block and its click box come from one physical body
   ([`BodyBox`](../../../crates/presentation/src/world.rs)), so they cannot
   differ, and they are the same before and after its art commits. This holds
   where the shell supplies a stand-in, as the page does; native capture and
   the showcase supply none and get the click box alone.
9. **No configuration in the game.** No switch, flag or default-off path for
   art loading. Review holds this, nothing else.
10. Unchanged by all of it: shaders, the vertex layout, what each side knows,
    and battle digests. The simulation's own digest and replay tests passed
    in the closeout run, and no commit of this work touches the simulation.
    One scene's pinned digest of the street battle was red in that run; other
    sessions' movement changes landed after the pin, and it is being settled
    apart from this work.

## The proof is a count, not a timer

Every admission reports the work it did
([`AdmissionWork`](../../../crates/renderer-core/src/runtime/work.rs)): bundles
decoded, vertex bytes packed, texture layers written, cards baked, scenery
placements, and why a full rebuild ran if one did. The page adds what it
encoded, sent and inflated, and publishes both on its `art committed` event.
That event fires when the committed art's source changes, so an admission
that commits with the same source reports nothing and its counts roll into
the next: a known gap.

- [`asset_generations`](../../../crates/renderer-core/tests/asset_generations.rs)
  commits {x, y}, stages {a, x, y}, and asserts the work is `a`'s alone (beside
  the small per-generation material table), the
  picture equals a clean build's, and a dropped preparation changes nothing.
- The [`battle-growth`](../../../web/scenes/battle-growth.mjs) scene plays a
  real skirmish to first contact and asserts the same counts against the art
  it saw arrive. Its long-frame check is a loose tripwire, not the gate: a
  timer moves with the machine, a count does not. It holds that art's network
  responses first, checks the new enemy's blocks and attacks them, then
  releases the responses and checks the models take their places. (A separate
  `stand-in` scene did this until it was folded in: it needed a vehicle and a
  late-bought squad in sight at once, which every battle retune reshuffled,
  and [`stand_in.rs`](../../../crates/presentation/tests/stand_in.rs) already
  covers every body type's blocks.)

## What it measured

The small skirmish (`battle-growth`, high graphics), one vehicle arriving:

| | Before | After |
| --- | ---: | ---: |
| Longest frame | 2.5–2.9 s | 54–66 ms with the engine alone; none over 50 ms recorded in the last runs |
| Art files sent to Rust | 201 (710 MB) | 2 (7.9 MB) |
| Vertex bytes packed | 1.13 GB | 8.6 MB |
| Texture layers written | 175 | 3 |
| Cards baked | 189 | 0 (a vehicle has none; an infantry type bakes its own) |

The reference battle (large, mixed, high graphics). "Before" is from two
sources: the player's own session, and the same battle in the test harness.

| | Before, the player's session | Before, in the harness | After, in the harness |
| --- | ---: | ---: | ---: |
| Freeze at an arrival | 2.9–9.9 s | 2.6–3.6 s | frames of about 50–120 ms |
| Arrival to model on screen | 11–40 s | 62–103 s | 0.1–0.3 s for a vehicle, 0.8–0.9 s for an infantry team |

The after column is three runs by the implementing agent on the finished
work. The second row's before is staging to commit and its after is from the
side first learning of the type to commit, which is the longer interval.

Whole-browser memory at the reference battle's deployment, settled, three
interleaved runs each: 6.20 GB before this work, 7.16 GB as the engine first
shipped, 6.02 GB once transport buffers were released. In a later cohort
measured differently (every download warmed first), sharing meshes between
living and dead bodies took 7.11, 7.10 and 7.15 GB to 6.66, 6.62 and 6.63 GB.
The two cohorts cannot be chained.

## Where the build left the plan

- **The counts became the new art's and the frame was still long.** Work
  nothing counted was hiding in it: each row re-derived its rest pose and
  bounds. Those facts moved to the geometry's owner and got a counter. The
  lesson is in invariant 1: a key or a cache that misses an input shows up as
  time no count explains.
- **The engine shipped holding a gigabyte more page memory**, against the
  player's rule that it must not raise peak memory, and stayed on staging
  about three hours before the fix. Two causes, neither a leak: the page
  encoded every model's packet into one list at the first scene and nothing
  released it until the collector ran, and fielding copied every source file
  a second time inside WASM memory, which never shrinks. A whole-browser
  comparison found it; no test did. Invariant 6 is the result.
- **The biggest arrival was one unit type, not several.** An infantry team is
  nine models, and each was packed three times: once living and twice posed
  as a corpse, before anyone had died. Bodies now draw the living mesh held
  in the last pose of the fall, which removed two thirds of an infantry
  type's geometry everywhere, and costs nothing measurable to draw.
- **Stepping one piece of art a frame was the wrong trade.** It tripled the
  time the block stood on screen. A per-frame budget did not, though it still
  keeps a vehicle's block up 50–100 ms longer than no stepping at all.
- **The floor under every arrival was text.** Each arrival serialised the
  whole fielded battle's description, 2.6 million characters, out to the page
  and back into the same module, and assembled all of it again first.
  Keeping it in Rust and assembling only the new kinds took that step from
  37–68 ms to under 1 ms.
- **Animation tables were copied for every soldier at every arrival.** The
  closing review found it: staging a battle's catalog gave each skinned
  appearance its own copy of its skeleton's clip table, 92–115 MB copied and
  held per staged catalog where two tables of 1.5 MB serve the roster. It
  predated this work and was the same catalog-shaped cost. A table is now
  stored once per content and shared
  ([`catalog/facts.rs`](../../../crates/presentation/src/presentation/catalog/facts.rs)),
  which took that step from 9.5–15.6 ms to about 0.2 ms. The frames a player
  notices did not move.
- **The stand-in's colour.** The agent first settled on the side's colour on
  the player's behalf. The player chose neutral grey instead; it was built as
  the HUD's dim blue-grey; the player, seeing it, asked for a real grey. It is
  a plain matte grey with no hue: a new colour, an exception to reusing an
  exact UI colour.

## Left open

- **An arrival is still visible in the reference battle.** Before the last
  change an infantry team showed a commit frame of 65–95 ms and one of about
  100 ms while the GPU baked its 27 cards; after it, the three runs show a
  late frame near commit of 71, 97 and 118 ms. None is attributed. The final
  admission, the bake and the commit run outside the per-frame budget
  (invariant 7), which is where to look first.
- A staging that is superseded while it spans frames drops what it supplied
  and warmed, and the next one packs it again. No test or scene covers two
  types sighted in quick succession.
- The admission after warming plans the generation a second time.
- A fielded result handed to the page is freed by the collector, not by an
  owner.
- A new bundle crosses into Rust twice, inflated for fielding's facts and
  encoded for the renderer, and its facts are derived on both sides (about
  3 ms a bundle each in one profiled run). One owner for both was not built.
- A vehicle's block is on screen 0.1–0.3 s; an infantry team's 0.8–0.9 s,
  most of it the card bake.
- A missing bundle is sent with every texture and skeleton it names, even
  ones Rust already holds, and the sent-bytes count leaves those out. The
  page also keeps its encoded textures and skeleton clips for as long as a
  scene's asset list lives. Neither was measured.
- Growing a texture array copies its layers on the GPU (40–240 MB at a
  time). Timestamp queries put the copy at 0.7–12 ms of GPU time; whether it
  ever costs a visible frame was not measured.
- An admission that commits with the same art source reports no work (see
  the proof section).
- A unit texture larger than any in the scene would rebuild its array and
  re-bake every card. No unit texture today is larger than the scene's
  (all are 256 px or less; the 512 px ones are map art present from the
  start), the rebuild reports itself, and `battle-growth` fails on it.
- The downloader keeps every file in Cache Storage as its own record of what
  it has fetched; the art loader is served by the HTTP cache and never reads
  it. That is 0.83 GB of second copy on disk. It predates this work.
- The stand-in as the fresh critique below reads it: no shadow, and nothing
  that marks it as a unit or as temporary.
- The native client's tests ran in the closeout (one needed a music file the
  checkout had not fetched, and passed with it); it supplies no stand-in
  (invariant 8).
- Seen in the first measurements and never in this plan: the first scene's
  own build joins vertex arrays byte by byte and upscales small textures per
  pixel, which block copies would replace; nearly every colour layer is
  upscaled to the largest texture's size, which per-size arrays would save;
  and art is held three times (parsed on the page, decoded in WASM, packed on
  the GPU).

## The picture it was held to

The standard was a choice, not a reference image. Before anything was built
the player was shown [a mock of first contact](visualizations/first-contact-mock.html)
with four ways to draw a unit whose art is still loading, and picked the
stand-in that swaps to the model. The block itself has no reference: it is
the least a unit can be and still be there.

[assets/stand-in](assets/stand-in/) holds the final look from the former
`stand-in` scene: a vehicle and a squad as blocks (`*-before.png`) and as their models
once the art commits (`*-after.png`). The player saw the grey blocks on
2026-11-15 and said they looked good (in conversation; the slice log does not
record it). No scene baseline was blessed for the block.

An [unprimed critique](assets/stand-in/unprimed-critique.md) of those four
pictures, by a reader told nothing about them, found the blocks sit where
the models then stand, the hull's at about the carrier's size and the
soldiers' clearly wider than a man. It also found that they cast no shadow,
that lit they read as a pale top over near-black sides, that a squad's
blocks merge with its carrier's into one shape, and that they read as a bug
or at best as developer greybox. None of that was changed: a new look is the
player's call.

## Records

- [The decisions ledger](decisions.md): what was decided, by whom and why,
  and the measurements behind each. The slice-by-slice log it condenses is
  `specs/battle-growth/decisions.md` at commit `7a7a422b5`.
- [The pre-ship review](assets/pre-ship-review.md) of the engine.
- [The growth probe](assets/growth-probe.mjs): the scratch scene that plays
  the reference battle and logs each arrival; registered temporarily, never
  committed as a fixture.
