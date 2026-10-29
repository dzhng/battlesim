# One owner per game setting: splitting game.json

**Status:** mapped. The audit ([`audit.md`](audit.md)) and the unknowns walk are
done, and every question is closed. The slices are not cut yet.
**Updated:** 2026-10-29.

## Next Agent Prompt

You are picking up the plan to give every number in
[`fixtures/game.json`](../../fixtures/game.json) exactly one owner and one
reader-side check, and to split the file by who reads it. Read
[`fixtures/README.md`](../../fixtures/README.md), this README and
[`audit.md`](audit.md) (every finding, with file and line). The plan lands in the
same PR as the [shared renderer port](../done/one-renderer/README.md)
(dzhng/battlesim#16), from the same checkout, because that port caused part of
what it fixes. The map below is complete. Confirm the
[facts to check before coding](#facts-to-check-before-coding), then cut the
ladder with [`write-spec`](../../.agents/skills/write-spec/SKILL.md) from the
[build plan](#build-plan), and implement it with
[`implement-spec`](../../.agents/skills/implement-spec/SKILL.md).

## Goal

Each setting has one owner, and the code that reads it refuses what it doesn't
understand:

- a number the code reads lives in data **or** in code, never both;
- a number no code reads is deleted, not left looking authoritative;
- every table keyed by an id (a weapon look, a vehicle class, a prop kind)
  refuses keys that name nothing;
- every reader refuses unknown fields and out-of-range values when the data is
  loaded, for shipped fixtures, encounter overrides and uploaded bundles alike;
- the battle's rules and the presentation's settings live in separate files,
  because they have separate readers, hashes and override rules.

## Known knowns: what the territory says

- The rules loader already treats `presentation`, `controls` and `seed` as not
  simulation: `UncheckedRules` skips them
  ([`scenario.rs`](../../crates/contract/src/scenario.rs), "These sections belong
  to presentation and browser input, never simulation").
- `rules_hash` hashes the resolved rules, not the file's bytes
  ([`bundle.rs`](../../crates/contract/src/bundle.rs)), so a presentation edit
  never changes it today; the split changes no digest by itself.
- Encounters override rules sections field by field (`EncounterDefinition::on`,
  `Rules::overridden`); presentation is not overridable.
- **Units and props already name a look.** A unit's `appearance` names its
  model; a prop's `appearance` block says how it is drawn; vehicles' engine
  sound and track gauge follow the physics-derived vehicle class
  ([`fixtures/README.md`](../../fixtures/README.md): "Presentation follows
  physics, never the id"). A unit's `sound` is the simulation's hearing
  (`sim/hearing.rs`), not presentation.
- **Weapons have no look key.** Their presentation is spread over about ten
  weapon-id tables (effects tracers, flashes, impact and blast scale, impact
  round scale, hull and soldier kick, audio shots, blasts, motors and impact
  scale, `sounds.json` defaults and per-weapon impacts). Some walk the
  weapon's `extends` chain from the raw file and some match exact ids only,
  which is the tank-kick and RPG-volume bug. See
  [`audit.md`](audit.md#live-bugs-already-on-staging).
- **Resolved rules carry no lineage.** `contract::catalog::inherit` flattens
  `extends` and drops `abstract` rows, so `catalog.json` and every scenario's
  `rules.weapons` have none. `abstract` is used by units and props, never by
  weapons.
- **48 fired weapons play 7 distinct firing recordings over 13 distinct visual
  bundles.** The 33 distinct sound rows are mostly ±1 dB loudness nudges on the
  same recording; folding nudges under 2 dB leaves about 18 looks.
- **The browser's session `rules` is the whole file.** `SessionCatalog.rules =
  {...game, catalog}` (`web/src/battle/catalog/compose.ts:57,80`), typed as the
  JSON import in about 30 places, and passed to every game-wasm rules entry.
- **Mixed readers** take both halves: `Fielding` (presentation-wasm; uses
  `rules` for two numbers and the raw file for the rest), `SoundFrame` and
  native `audio::setup` (raw `weapons`, `tick_hz`), `files::ground` (`sensors`),
  native capture (`physics`). Natively `fielded()` passes the raw file as
  `rules`, so encounter overrides never reach native presentation.
- **Stored documents:** moving `presentation` out changes no `Rules`, so no
  hash, digest or skirmish fixture moves; only `fixtures/parity/exact-numbers.json`
  (which embeds the raw test game, presentation included) changes text. Moving
  `hull_limits`/`air_speed_limits_kmh` out of `Rules` changes every `rules_hash`,
  the skirmish fixtures' text, and overridability; `hull_limits.rotor.half_length_m`
  doubles as the helicopter routing margin (`navigation/air.rs:31`).
- **Writers and hashers:** the mechanics editor rewrites the whole file when
  weapons are edited and hot-reloads it; the map workbench hashes its bytes
  into each revision.
- **Admission has three tiers:** contract checks at parse,
  `sim::weapons::check_rules`, and asserts in `Battle::from_prepared`. Only
  `fixtures::admit` runs all three without panicking; `check_rules` needs sim's
  flight solver.
- **Sibling fixtures** each have one validating owner module in TS
  (`shippedSounds.ts`, `gameBiome.ts`) and are read by path or `include_str!`
  in Rust.
- **Tests fire base ids only** (`tank_ap` in `vehicle_poses.rs`,
  `effect_behaviour.rs`, `sound.rs`); nothing fires a derived weapon, nothing
  covers `tables::inherit` in Rust, nothing feeds an overridden weapon into
  presentation.
- **Forest logs and boulders** have real models and are already field cover on
  maps (Market Town has 38 logs, 41 boulders). Forest placement was switched on
  by `4bdfb76f7` and reverted by hand 90 minutes later with no reason recorded;
  `specs/done/city-maps/assets/sim-forest-bodies/README.md` measured battle
  setup doubling (7.6 → 14.8 ms), every digest moving, and an open 33 ms frame
  check.
- **`gravity_scale` is a hand-tuned arc knob.** Its doc
  (`contract/src/ballistics.rs:92`) and the test
  `a_gun_round_is_slowed_for_the_eye_but_flies_the_real_round_s_line` claim it
  is `(speed / real speed)²`, but the values imply real muzzle speeds of 301 m/s
  (tank), 2,667 m/s (rifle, HMG) and 43 m/s (grenade); `6de4cbf74` set the
  grenade's to keep "the old lob height".

## Known unknowns: the decision ledger

Every question the walk named, with its answer and who closed it.

### Decided by the user (2026-10-29)

- **Same PR, same checkout.** The work lands on PR #16 from this worktree.
- **Weapon `description` stays in the data.** All 49 match each weapon's
  mechanics; the UI will show them later (not in this plan). Only
  `helicopter_crash`'s is not a player-facing weapon.
- **A weapon names one look, like a unit.** Each weapon row carries
  `appearance: "<look>"`, inherited by the catalog's ordinary `extends` merge
  like any field, exactly as a unit's `appearance` names its model. A look is
  one entry holding everything the weapon shows and sounds like: tracer, flash,
  impact and blast size, hull and soldier kick, shot, blast, motor and impact
  sound. Presentation reads `weapon_looks[row.appearance]` from the battle's
  rules in both clients; nothing walks a chain, `extends` is still flattened
  away and weapons keep `abstract`. Every weapon must resolve to a look, every
  look a row names must exist, and an unused look is refused; there is no
  `default` look. Two weapons that look alike but sound different are two
  looks. Why: one inheritance mechanism for every catalog row; sight and sound
  of a round belong together, and a look that is a near-copy of another is a
  sign to make the visuals more distinct or the sounds fewer, not to split the
  key. Cost: changing a weapon's look changes `rules_hash`, as a unit's
  `appearance` already does; no outcome changes. Rejected: keeping `extends`
  for a per-table walk (a second inheritance mechanism), one key per aspect
  (permutations), separate look and sound keys (the user wants them coupled).
- **Three files, named by reader.** `fixtures/rules.json` (exactly what Rust's
  `Rules` reads; hashed; overridable), `fixtures/presentation.json` (looks and
  handling), `fixtures/audio.json` (the mix, beside `sounds.json`).
  `game.json` is renamed. The mechanics editor writes only `rules.json`, the
  map workbench hashes only `rules.json`. Why: each file has one reader, one
  check and one editor.
- **Weapon looks live in `presentation.json`; `audio.json` is the mix.** A look
  entry (`weapon_looks.<look>`) holds its visuals and its sound levels;
  `audio.json` keeps what is not per weapon (buses, distances, cues, music
  direction, vehicle engines by class, per-material impacts, ambience). The
  sound workbench may write `presentation.json` to tune a look's sound.
- **A look's sound sets its own levels and may override any default
  recording.** `audio.json` names the default recording for each part of a
  weapon's sound (blast, motor, each impact surface); a look's `sound` must
  give its levels (`gain`, `far_m`, impact loudness) and may name a recording
  for any part to override the default. The firing sound has no default: every
  look names its own, so nothing silently fires the rifle's shot. `sounds.json`
  becomes only the library (sources, clips, recipes): its per-weapon `defaults`
  and `impacts.<contact>.<weapon>` move into looks, and its `effects` role map
  (explosion, motor, footstep, tracks, rotor, ricochet) moves to `audio.json`.
  Every recording a look or a default names must exist in the library.
- **About 18 weapon looks.** The migration folds nudges under 2 dB into one
  level per look (tank AP ×4, ATGMs ×9, SAMs ×6, MANPADS ×3, rifle and carbine)
  and splits only where the difference is real: `ifv_he` (the BMP-3's 100 mm
  main gun) from tank HE (9 dB), the AA guns from the HMG (7 dB), air bombs from
  artillery (20 dB), the RPGs from the grenade (11 dB, lost to the rename bug).
  Audible change: weapons inside a group lose their ±1 dB nudges.
- **The content pass is part of this plan.** What the grouping exposed gets
  fixed here, not later: artillery and bombs have no firing recording of their
  own (they fall back to the default, a rifle shot; the sound catalog has no
  artillery, mortar, rocket-salvo or bomb-release source);
  `artillery_strike_missile` also fires the rifle shot; `rocket_pod` extends
  `tank_he` and looks and sounds like a tank cannon; `ifv_he` shares the tank's
  HE visuals; the marksman rifle and heavy sniper inherit the rifle's 3-round
  burst recording and the AA guns the HMG's, though a burst recording must
  match the weapon's rate of fire (`sound/frame.rs:846-866`, left open in
  `specs/done/unit-models/choices.md`).
- **New sounds are real recordings taken from YouTube,** the way the catalog
  already takes them (`yt-dlp`, with source, author, license and processing
  recorded in `sounds.json`'s provenance). A recipe layered from existing clips
  stands in only where no source is found. The repository is private, so an
  All-Rights-Reserved source is acceptable.
- **Who edits, and what done means.** Agents edit these files now; a workbench
  may later. No live reload is built. Adding a weapon is a rules row that names
  an existing look; a new look is one `presentation.json` entry; the load-time
  checks refuse anything missing, misspelled or unused.
- **Forest logs and boulders are deleted.** Switching them on is two numbers
  but not finished (setup doubled, every digest moves, the frame-time check was
  never closed), so the user's rule (delete unless trivial and done) applies.
  Deleted: forest floor placement (`navigation/floor.rs`,
  `place_forest_bodies`, the four `forests` rule fields and their checks and
  tests, `forests.log`/`forests.boulder`, about 650 lines). Kept: the
  `log`/`boulder` prop rows, models and their use as field cover,
  `forest_floor.py`, `trunk_positions`. No battle digest moves (the densities
  are 0), but `rules_hash` and the map generator's `config_hash` do, so it
  lands with the other hash moves.
- **`gravity_scale` stays an authored arc knob.** No gameplay change and no
  reel re-shoot. The name stays (it is literally the share of gravity a round
  falls under); its doc comment and the flight test stop claiming it
  reproduces a real round's line and say what it is: the arc chosen for the
  screen, a flat line for a sabot, a lob for a grenade.
- **Old bundles, replays and drafts simply stop loading.** The game has not
  launched, so there is no migration and no special refusal message.

### Answered by the agent on the user's behalf

The user opted in to answers on their behalf where their confirmed preferences
decide the question:

1. One mechanism everywhere; no parallel systems.
2. Fewer permutations: merge near-duplicates; give what is truly different its
   own key.
3. What belongs together stays together.
4. Fix what the audit exposes in this plan and PR, not a follow-up.
5. A visible or audible change is fine when it makes the game more right.
6. Unfinished, unprioritised features are deleted, not parked.

- **One admission owner.** `sim::admit(&Rules) -> Result<(), RulesError>` runs
  the contract checks, the flight checks (`check_rules`), the supply check,
  every check `Battle::from_prepared` asserts today, `admit_world`'s rules
  checks, and the missing ranges (`infantry_movement`, `bail_out`, garrison,
  pushing, launch, the cover and ground gaps, a guided missile's lifetime
  covering its range, the pace caps, `helicopter_crash` present, a burst
  recording matching its weapon's cadence where the look names one). Every
  place rules arrive from outside calls it and refuses with the error:
  `fixtures::admit`, encounter overrides (`sim::maps`, wasm
  `encounter_scenario`), uploaded bundles (`check_scenario_bundle`, after the
  contract's validation), the wasm battle and world handles, the map
  generator's rules reader, and native. One end-to-end test drives a bad row
  through each entry path. `Battle::new` keeps its signature and its asserts
  become invariants those paths guarantee. Not chosen: `Battle` taking an
  `Admitted` type (546 `Battle::new` calls; `ScenarioDefinition` lives in
  contract and can't hold a sim type) and moving the flight checks into
  contract (the trajectory solver is 2,600 lines of sim). (Preference 1.)
- **Hull and air-speed limits leave the rules.** They become
  `contract::HullLimits::SHIPPED` and the air-speed equivalent, constants the
  catalog's admission takes as an argument: not hashed, not overridable by an
  encounter, and tests that need them lifted pass lifted limits to admission
  instead of editing the rules (`lift_hull_limits`, `with_units_at_limits`).
  Helicopter routing gets its own margin constant with today's value (10 m),
  so no route moves. `rules_hash` and the skirmish fixtures' text change once.
  (Preferences 1, 4.)
- **Numbers frozen in the shaders leave the data.** The 162 terrain, 23 grass
  and 2 glass numbers are deleted from the fixture and named as constants in
  the WGSL that uses them. The five with two owners (`grass.edge_m`,
  `forest_verge.feather`, `bank.ease_m`, `marks.clear_m`, `scars.depth_cap`)
  and `fog_geometry.tile_px`/`radial_bins` become Rust constants spliced into
  the shader prelude (`builtin_source`, the `SWAY_`/`WAVE_` pattern), so the
  CPU tables and the GPU read one value. Water and shore tuning stays data (it
  reaches the GPU as uniforms). `tuning.rs`'s doc is rewritten to match.
  (Preferences 1, 2.)
- **Shared constants have one table.** The Rust constants spliced into the
  shader prelude and exported to TypeScript (`records/typescript.rs`) come from
  one table; the prelude gains typed integer constants (`tile_px` and
  `radial_bins` size arrays and `@workgroup_size`, which naga refuses as `f32`);
  prelude names carry their owner's prefix. The other Rust/WGSL duplicates the
  sweep found join it: `CASCADES`, `CAST_LIGHTS_MAX`, `FOLIAGE_STEP` and its
  255, `WHOLE_TEXTURE_WIDTH`, `HORIZON_SECTORS`, `BUILD_WORKGROUP`. The fog
  uniform drops the two fields the constants replace. (Preferences 1, 4.)
- **Every audit cleanup is in this PR,** including the ones that move a
  number: weapon `range_m` derived from its largest layer range (the authored
  row type has no `range_m`; the resolved row emits it, so TS readers like the
  range ruler keep it, and an authored copy is refused; the three reel
  encounters that override it are edited); `decoyable` dropped where no target
  carries flares; `service.round_costs` becomes a weapon row's `round_cost`;
  `buildings.cover_strength` deleted and capacity over 32 refused;
  `shouts.talk_s` collapses to one number; the duplicated overlay/HUD colours
  derive from one owner; light is checked in Rust; `light.cascades.count` and
  `light.backdrop.reach_m` leave light; `camera.default.target` moves to the
  labs; one gravity constant; the wake's `soldier_radius_m` is renamed; the
  shipped UI reads `tick_hz` and `air` from the battle's rules; the fog style
  is picked once. (Preferences 4, 5.)
- **The shape of a look.** One `impact` block per look merges today's visual
  `impact_scale` with the per-surface `round_scale`s; `kick` holds hull and
  soldier kick; `sound` is nested inside the look, not flat beside its visuals;
  looks are named for what they are (`tank_sabot`, `tank_he`, `ifv_cannon`,
  `atgm`, `manpads`, `sam`, `aa_gun`, `artillery`, `bomb`, `rpg`, `rifle`, …),
  never after one of the weapons that share them. (Preferences 1–3.)
- **A firing look follows the round fired, not the mount's first weapon.**
  Today a mount publishes its first weapon row as its `kind`
  (`effects/publication.rs:52`), so the BMP-3's Bastion missile flashes, sounds
  and kicks like its 100 mm gun, and tank HE fires with AP's look. Hull launches
  learn the fired round the way soldier launches already do (matched to the
  shot's segment start), so no published field is added. (Preference 1.)
- **The look's parts a weapon needs follow its physics.** A weapon with
  `blast_radius_m > 0` needs a blast, one with `accel_mps2` a motor, a fired
  weapon a shot and a flash; `helicopter_crash`'s look is only a blast. The
  check runs when the battle is fielded. (Preferences 1, 5.)
- **`appearance` is an ordinary rules field.** Encounters may override it like
  any field; the page's `presentationRules` stops laying shipped rows over the
  scenario's. The per-battle reader refuses unknown looks; "every look is
  used" is a fixture test against the shipped `rules.json`. (Preference 1.)
- **Presentation parses weapons through contract's `resolve_weapons`**
  (idempotent on resolved rules), so raw-file, catalog-session and encounter
  rules all reach `Fielding` and sound in one shape; native passes the battle's
  rules, not the raw file; the browser's sound frame is rebuilt per battle.
  (Preference 1.)
- **Light-name causes become look names** (`flash:<look>`, `round:<look>`),
  and `audio.cook_off.landing.as` names a look, not `tank_ap`. (Preference 1.)
- **Stand-in tints go in each prop's `appearance` block,** which the rules
  never read, authored for all 59 props (22 have none today); no fallback
  tint. **Track gauge goes with the model's appearance,** beside
  `recoil_max_m`, not on the physics hull. (Preferences 1, 2.)
- **`controls` and `seed` leave the rules.** `controls` becomes the browser's
  input settings, read where input is handled; `seed` becomes the lobby's
  default in one owner, and the hard-coded lobby copies of 20260925 read it.
  (Preference 1.)
- **Sound checks live in the Rust reader.** The TypeScript-only conventions
  (firing clips are shots or launches, no loops, near and far share a burst,
  effect slots are real synth slots, gain range) move to the Rust
  `SoundCatalog` check with `deny_unknown_fields`, and the gain range admits
  folded levels above 2. (Preference 1.)
- **The sound workbench gets its own slice:** it keys by look, writes
  `presentation.json`, and loses its `extends` walk and its delete-to-fallback
  button. (Preferences 1, 4.)

### Open

None.

## Unknown knowns: what the walk drew out

- **Consistency is the test of a design.** The user probed every weapon
  proposal against how units and props already do it; that turned a
  per-table `extends` walk into the `appearance` key units already use.
- **Sight and sound of a round are one thing.** It turned separate look and
  sound keys into one look, and turned "33 sound rows" into "about 18 looks
  plus a content pass" instead of 33 looks with copied visuals.
- **Exposed gaps are this plan's work.** It pulled the content pass, the
  firing-look fix and every audit cleanup into this PR.
- **Unfinished features go.** It decided forest logs and boulders.
- **The game has not launched.** No compatibility work for old bundles,
  replays or drafts.
- **Editors are agents.** No live reload or editing UI is built; the workbench
  may come later.
- **The repository is private.** Sound sources may be All-Rights-Reserved.

## Unknown unknowns: the landmine sweep

Three reviewers swept the presentation, audio and sound workbench code; the
rules contract, admission, loaders and stored records; and the shaders, their
constants and the forest code. The decided cards are in the ledger above (the
firing look, the parts a look needs, `appearance` overrides, resolving through
`resolve_weapons`, light causes, the constant table, tints and gauge, the
sound checks, the workbench slice, and the revised admission owner). The rest
are sharp edges for the builder:

- **Prove the move before changing anything.** Land looks that reproduce
  today's output exactly first: `sound-vectors.json`, `effect-vectors.json`
  outputs and native captures unchanged. Then make the level folds, splits,
  bug fixes and content as named commits, each re-blessing what it moves.
- **No test would catch a mistyped number among the 187 moved into WGSL.**
  Keep each literal's text exactly; prove with byte-identical composed WGSL
  per pipeline (or a byte-identical native capture) before and after.
- **`Tuning` is read from the whole `presentation` object;** adding
  `deny_unknown_fields` there would refuse every other section. Read each
  section into its own struct.
- **Stored records were hand-migrated and can't be re-blessed by their tests:**
  `skirmish-objectives.json`, `skirmish-purchases.json` (old rules, dead ids),
  `exact-numbers.json`, `scenario-overrides.json` (overrides
  `service.round_costs`), `parity/map-layout/physical-rules.json`,
  `effect-vectors.json` and `paris-corner-units.json` inputs (weapon-keyed
  tables, gauge, kicks, tints). Each rules-shape slice carries a one-shot
  migration script for them, then re-blesses (`BLESS_PARITY`,
  `UPDATE_SOUND_VECTORS`, `UPDATE_RECORDS`; map layout's 17 hashes move).
- **Replays hash the rules** (`replay_identity.rs:39`): any look retune moves
  `rules_hash` and every replay's config digest.
- **The desktop client finds the repository by `fixtures/game.json`**
  (`main_desktop.rs:116-124`) and silently falls back to the build directory;
  rename the sentinel with the file.
- **The seed literal 20260925 appears in about 9 places;** three are unrelated
  Rust test RNG seeds that must not read the lobby default.
- **The file-size ratchet:** `battle.rs`, sim `weapons.rs`, `catalog.rs`,
  `scenario.rs` and `game-wasm/src/lib.rs` are over budget and may not grow;
  remove before adding in each commit.
- **Effects have no pixel baseline.** `_roundsSheet.mjs` lists tracer keys and
  walks `extends` itself; it becomes a sheet keyed by look covering flash,
  tracer, impact and blast, judged by screenshot-critique, for the content pass.
- **`ifv_he` shares its mount with the Bastion** missile; the firing-look fix
  is what lets them differ.
- **The routing margin flip-flopped before** (`1f521bb78`, `70b6de62b`: a
  margin taken from whichever catalog loaded made the sim test and the browser
  lab route differently); the new 10 m constant's doc records why it is fixed.
- **Mechanics editor:** allow-list, hot-reload skip and the frozen startup
  object all name `game.json`; `range_m` is listed as an editable field
  (`fields.ts:290-310`) and leaves the list.

### Facts to check before coding

- The overlay shader reads colours in the same space as the HUD's CSS, so one
  can derive from the other; the x-ray keeps its alpha.
- Matching a hull launch to its round by segment start is unambiguous when one
  mount fires two weapons in the same tick (BMP-3, tank AP and HE).
- naga accepts a prelude integer constant in `@workgroup_size` and as an array
  length in every pipeline that uses `tile_px`/`radial_bins`.
- Which of the nine 20260925 literals are lobby defaults and which are test RNG
  seeds.
- YouTube sources exist for artillery, mortar, rocket salvo and bomb release
  firing sounds, and for burst recordings at the marksman rifle's, heavy
  sniper's and AA guns' cadence.

## Build plan

Sorted by how likely you are to want to tweak it, not by execution order; the
slices are cut from this with `write-spec`.

1. **The look list and its sound.** About 18 looks, their names, which weapon
   names which, and the level each fold keeps. Audible: nudges under 2 dB go.
   The toggle is the 2 dB threshold.
2. **The content pass.** New firing recordings for artillery, bombs, the
   strike missile and rocket pods; distinct visuals for `ifv_cannon`,
   `rocket_pod` and the RPGs; burst recordings that match cadence. Judged by
   eye and ear on the look sheet and in the menu reel.
3. **Bug fixes that change what you see and hear.** Tank guns kick, tank-round
   impacts at their intended size, RPGs back to their own loudness, the
   Bastion and tank HE fire with their own look.
4. **The file boundaries.** What goes in `rules.json`, `presentation.json` and
   `audio.json`, and which editor writes which.
5. **Rules cleanups that move a number:** derived `range_m`, `round_cost`,
   `decoyable`, capacity limit, `talk_s`, hull limits as constants.
6. **Admission:** `sim::admit` and the end-to-end refusal test.

Mechanical, trusted to the builder: the WGSL constant move and its byte-identity
proof, the shared constant table, deleting dead terrain/grass/glass numbers and
forest placement, `deny_unknown_fields` on every reader, moving tints and gauge,
`controls`/`seed`, the `gravity_scale` doc, renames, the stored-record
migrations and re-blesses, and the docs.

## Later, not this plan

- **Light and heavy tank guns look and sound different,** with a smaller
  explosion for the light gun. Looks make this a data change: new looks, and
  the light tank weapons name them. (User, 2026-10-29.)
- **Weapon descriptions in the UI.** The data stays ready for it.

## Findings

[`audit.md`](audit.md) holds the full audit: what this PR caused, the bugs
already on staging, why they hid, and the dead, duplicated and misplaced
numbers per section.
