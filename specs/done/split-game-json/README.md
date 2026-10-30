# One owner per game setting

The old combined game fixture is split into files named for their readers:

| File | What it holds | Who reads it |
| --- | --- | --- |
| [`fixtures/rules.json`](../../../fixtures/rules.json) | The battle rules, exactly what Rust's `Rules` reads | Hashed into `rules_hash`; encounters can override it field by field |
| [`fixtures/presentation.json`](../../../fixtures/presentation.json) | Visual and handling settings, plus each weapon look (`weapon_looks`) | Admitted by the typed `PresentationSettings`; browser code reads the admitted JSON |
| [`fixtures/audio.json`](../../../fixtures/audio.json) | The mix: buses, distances, cues, music direction, vehicle engines by class, per-material impacts, ambience, and the default blast, motor and impact recordings a look may leave empty (firing has no default) | `AudioMix` |
| [`fixtures/sounds.json`](../../../fixtures/sounds.json) | The sound library: sources, their provenance receipts, clips, recipes, and the music tracks with their mood playlists | `SoundCatalog` |

The [fixture contract](../../../fixtures/README.md) describes each file's
shape. [`choices.md`](choices.md) lists the choices made on your behalf while
building it, with verdicts.

## The problem it solved

An audit of the old fixture found four kinds of problem:
- **dead numbers:** values no code read, but which looked authoritative;
- **numbers with two owners:** the same value in data and in code, or in two
  readers;
- **unchecked tables keyed by weapon id:** about a dozen of them;
- **live bugs these hid:**
  - derived tank guns never kicked, because recoil was matched by exact weapon
    id;
  - RPGs played the grenade's sound about 11 dB quiet, after a rename;
  - the BMP-3's missile launched with its 100 mm gun's flash;
  - the native client drew presentation from the base rules, ignoring an
    encounter's overrides.

The rule now is that each setting has one owner. The code that reads a setting
refuses fields it doesn't know and values out of range when the data loads.
That applies to shipped fixtures, encounter overrides and uploaded bundles
alike.

## Why it works this way

**A weapon names one look, as a unit names its model.** Each weapon row
carries `appearance: "<look>"`. The catalog's ordinary `extends` merge
inherits it like any other field. A look (`weapon_looks.<look>`) is
everything a round shows and sounds like: tracer, flash, impact, blast, hull
and soldier kick, plus its `sound` block. Sight and sound sit in one look
because they belong together. A near-duplicate look is a sign to make the
visuals more distinct or the sounds fewer, not to split the key.

Rejected alternatives:
- walking `extends` per presentation table, which is a second inheritance
  mechanism;
- one key per aspect, which multiplies permutations;
- separate look and sound keys.

**Changing which look a weapon names moves `rules_hash`; retuning a look does
not.** `appearance` is a rules field, hashed and overridable like a unit's.
A look's own values live in `presentation.json`, which no hash reads.

**One admission owner for rules.** `sim::admit` (`crates/sim/src/admission.rs`)
runs every rules check. That includes the flight solver checks, which need
the simulation, so admission lives in `sim`, not `contract`. Every way rules
arrive calls it:
- fixture loading;
- encounter overrides;
- uploaded bundles;
- the Wasm battle and world handles;
- the native client;
- the map generator, which is why `mapgen` depends on `sim`.

`Battle::new` keeps its asserts as invariants those callers guarantee. An
`Admitted` wrapper type was rejected: there are hundreds of `Battle::new`
calls, and `ScenarioDefinition` lives in `contract`, which can't hold a `sim`
type.

**Whether a look fits its weapon is checked when a battle is fielded, not at
rules admission.** The simulation never sees looks or recordings. Fielding
refuses a battle whose weapons:
- name an unknown look;
- lack a look part their physics needs (a blast for a positive blast radius,
  a motor for acceleration);
- are mounted by a unit, so fired, but lack a tracer, impact, kick, shot or
  flash;
- name a recording that doesn't exist;
- have a burst recording that doesn't match their rate of fire.

The cadence check is in `SoundTables::from_parts`
(`crates/presentation/src/sound/tables.rs`): a multi-report recording must
fit the magazine and burst and match the shot interval within 5 ms. A weapon
no unit mounts, such as the crash row, may have a blast-only look. There is
no `default` look. A look that leaves its blast or motor recording empty, or
names no recording for an impact surface, takes `audio.json`'s
`weapon_defaults`; firing has no default.

**The firing look follows the round actually fired.** A mount publishes
`WeaponPose.round` (`crates/contract/src/observation.rs`): the index, in the
mount's own weapon list, of the round it last fired. Presentation looks up
that row's flash, kick and sound. A vehicle's round segments name their
weapon but not which mount fired them, so nothing published could tie a
muzzle flash to its row. The field is left out of `Mount::digest`, so it
moves no battle digest.

**Presentation reads the battle's own resolved rules.** Both clients pass the
battle's rules through `contract::weapons::resolve_weapons`. So raw-file,
catalog and encounter rules all reach effects and sound in one shape. The
native client builds its ground, fog sensors and buildings from those same
rules.

**Rust is the only place presentation settings are checked.**
`PresentationSettings::from_value` (`crates/presentation/src/settings.rs`)
reads each section into its own struct and refuses unknown fields. The
browser admits the imported document once through it (via presentation-wasm)
and then reads the admitted JSON. The TypeScript validators are gone.
`WeaponLook::check` (`crates/presentation/src/looks.rs`) owns a look's local
limits, which the document, the sound workbench and battle fielding share.

**Numbers shared by the CPU and the GPU come from one Rust table.**
`presentation::shader_constants` holds them:
- `renderer-core`'s `builtin_source` splices them into the WGSL prelude;
- `records/typescript.rs` exports them to TypeScript.

The prelude carries typed integers, because naga refuses an `f32` in
`@workgroup_size`. Numbers only the GPU reads are named constants in the
WGSL that uses them; the fixture no longer holds them.

**Limits on what the catalog may field are code, not rules.**
`contract::HullLimits::SHIPPED` and `contract::AirSpeedLimits::SHIPPED`
(`crates/contract/src/limits.rs`) are not hashed and not overridable.
`sim::admit` checks against them. Tests that need larger units check the
catalog against `sim::fixtures::lift_hull_limits()` instead of editing the
rules.

Helicopter routing has its own constant, `ROUTING_MARGIN_M` (10 m), in
`crates/sim/src/navigation/air.rs`. Its doc says why it is fixed: taking the
margin from whichever catalog had loaded made the simulation and the browser
route differently, twice.

**Each number is authored once:**
- **Range:** a weapon's `range_m` is derived from its largest target-layer
  range. Shipped data carries none, and resolved rows emit it for TypeScript
  readers. An authored value that disagrees is refused, so resolved rules
  survive a round trip. Tested by
  `an_authored_range_must_match_the_largest_target_layer`.
- **Ammunition cost:** finite ammunition is priced per weapon row
  (`round_cost`), and derived rows inherit it.
- **Arc:** `gravity_scale` is documented honestly as the arc chosen for the
  screen, a flat line for a sabot and a lob for a grenade. It is not the
  real round's line.

## Invariants

- **No battle digest moved.** The baseline stream of 63,653 digest lines is
  byte-identical from before the split to the final code. It is the
  weapon-by-weapon firing probe kept in
  `throwaway/split-game-json/before/digests.jsonl`, which is ignored. It still
  matched after the two later simulation fixes: restoring the refusal of
  unreachable move orders, and having unloaded squads take cover where they
  land.
- **`rules_hash` moved only at named decisions:**
  - when looks were introduced;
  - when they were folded;
  - when artillery was split into `artillery`, `mortar` and
    `rocket_artillery`;
  - in the rules cleanups: limits; forest floor; range; round costs; and the
    redundant cover, flare and bark data.

  The limits, forest-floor and range cleanups also moved the map generator's
  `config_hash`.
- **Every look a weapon names exists, and every shipped look is used.** The
  per-battle reader refuses an unknown look. A fixture test refuses a look in
  `presentation.json` that no weapon in the shipped `rules.json` names.
- **No presentation or audio code walks `extends`, keys a table by weapon id,
  names a weapon in a rule, or falls back to a `default` look.** The defaults
  that do exist are separate contracts in `audio.json`:
  - impact material;
  - vehicle class;
  - fire kind;
  - the `weapon_defaults` recordings.
- **A burst recording matches its weapon.** A mod or encounter that retunes a
  weapon's rate of fire must bring a matching recording, or the battle is
  refused with the weapon named.
- **Every sound records where it came from** in `sounds.json`:
  - a third-party source carries its URL, author and license;
  - a generated source carries its generator, model and prompt;
  - each clip and each download receipt records its processing.

  A test checks that each download receipt's file exists with the recorded
  SHA-256. The repository is private, so All-Rights-Reserved sources are
  allowed.

## Pointers

- **Weapon looks:**
  - `crates/presentation/src/looks.rs`: `WeaponLook`, `WeaponLooks`,
    `check_firing`, `check_sounds`.
  - Tests: `crates/presentation/tests/looks.rs`, `firing_round.rs`,
    `effect_behaviour.rs`.
- **Settings and mix:**
  - `crates/presentation/src/settings.rs` and `settings/`;
  - `crates/presentation/src/sound/tables.rs` (`SoundTables`, `AudioMix`);
  - `crates/presentation/src/sound/catalog/check.rs`.
  - Tests: `crates/presentation/tests/settings.rs`, `sound.rs`,
    `sound_mix.rs`, `sound_setup.rs`, `sound_cadence.rs`.
- **Rules admission:**
  - `crates/sim/src/admission.rs`, `crates/contract/src/limits.rs`.
  - Tests for rules reading: `crates/contract/tests/rules.rs` and
    `weapons.rs`.
  - Tests for admission: `crates/game-wasm/tests/admission.rs`,
    `crates/sim/tests/catalog/limits.rs`, `crates/sim/tests/air/limits.rs`.
- **Last-fired row:** `WeaponPose.round` in
  `crates/contract/src/observation.rs`.
  - Published from `crates/sim/src/publication/layout.rs`.
  - Resolved to the row's look in `crates/presentation/src/effects/publication.rs`
    (flash and sound) and `crates/presentation/src/units/feed.rs` (kick).
- **Shader constants:** `crates/presentation/src/shader_constants.rs`.
- **Sound workbench:** `apps/sound-workbench/` saves only a look's sound
  block into `presentation.json`. Which look a weapon names stays a
  `rules.json` edit.
- **Look sheet:** `web/scenes/_roundsSheet.mjs`.

## Divergences and dead ends

- **Looks folded from 37 to 23.** The first migration reproduced every old
  table combination exactly, bugs included, as 37 temporary looks, so the move
  could be proven byte for byte before anything changed. The fold took each
  group's median level in dB, which gave 21 looks. The heavy RPG kept its own
  look, because its blast differs by 3.25 dB. The content pass, where looks
  were retuned to fit their weapons, then split `mortar` and
  `rocket_artillery` out of `artillery`, making 23.
- **No existing signal said which row a vehicle fired.** Matching by muzzle
  position was rejected as a guess, so the published `WeaponPose.round` was
  added.
- **Forest-floor logs and boulders were deleted, not switched on.** Turning
  them on doubled battle setup time, moved every digest and left a frame-time
  check open. Their prop rows, models and use as field cover remain.
- **Constructed firing sounds:** the AA burst, the LMG at 80 ms, rocket pods,
  rocket artillery and bomb release use labelled fallback recipes where no
  matching recording was found. The Apache rocket recording was not admitted:
  its rotor noise couldn't be shown to separate from the motor. See
  [`choices.md`](choices.md) N2.
- **The look sheet fires every look from one controlled HE gun.** It shows
  style, not each weapon's trajectory or whether it explodes. Real AP rounds
  make no blast; on the sheet, the sabot row explodes only because of the
  carrier.
- **A test edited to pass hid a gameplay reversal.** A closeout balance fix
  let move orders to unreachable ground be accepted. Two tests were then made
  green by disabling the formation search in their setup. Both were restored
  under shipped rules, and the fix narrowed: the reachability of the ordered
  point decides refusal (`crates/sim/src/battle/move_placement.rs`).
- **An AI ally rarely answers a human's call for help in real battles.** The
  team planner's influence map adds an enemy's strength only to the cell it
  stands in, so enemies firing at an objective from range don't count as
  pressure on it. The `ally-help` scene therefore runs a controlled lab battle
  (`/labs/ally-help`). See [`choices.md`](choices.md) N4.

## Visual provenance

Nothing visual was meant to change until the content pass. The standard was:
- six frozen native plates, compared byte for byte;
- the unrounded probe of effects, poses and sound commands.

The content pass was judged on the look sheet, one row per look, with unprimed
screenshot critiques. Those captures and critiques are in the ignored scratch
folder (`throwaway/split-game-json/evidence-*`), not in-tree; no image here
defined the look. Still open: screen-wide bloom that hides cores, the low
contrast of small-arms tracers against wheat, and the RPG's disc-shaped body.

## Not done here

- Light and heavy tank guns share the `tank_sabot` and `tank_he` looks.
  Giving the light gun its own look, with a smaller explosion, is a data
  change.
- Weapon `description` stays in the data; no UI shows it yet.
