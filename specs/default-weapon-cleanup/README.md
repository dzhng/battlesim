# Default weapon cleanup (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-11-14.

## Next Agent Prompt

You are picking up a rules cleanup: one weapon-row flag, `default`, does three
unrelated jobs in the simulation. Don't start until the user gives the go for a
rules-cleanup pass, and build on the Armory's
[decision U8](../done/armory/unknowns-map.html#u8), not around it. When the
user starts this, walk the open decisions with them and run
`tweak-mechanics`. Then land the scenarios as tests against today's code, and
only then run `write-spec` to slice the split. The cleanup must not change
outcomes: battle digests and replays stay the same, unless the user names a
change. Update this section before you end each pass.

## The problem

`WeaponDefinition.default` (`crates/contract/src/weapons.rs`, documented as
"The unit's always-available gun (W09)") is read for three separate reasons:

1. **The soldier's fallback gun.** `preferred_mount` (`crates/sim/src/weapons.rs`)
   picks the one mount each soldier works this tick, in this order: a special
   mount under a support order; a special mount with a useful shot; a mount with
   a `default` row that has a shot; a special mount that needs reloading; any
   mount with a `default` row. Vehicles skip this step: `select_active`
   (`crates/sim/src/weapons/tick.rs`) returns straight away for a unit with a
   hull, so every vehicle mount fires on its own. For squads, the `default`
   check overlaps the mount's `squad` marker (`MountSpec.squad`, the gun every
   soldier carries).
2. **The jet's strafing gun.** `pass_weapon` (`crates/sim/src/air/pass.rs`)
   sorts each row for a ground-attack pass: an unpowered row is a bomb, a
   `default` row is the strafing gun, and any other row that reaches the ground
   is fired on a missile pass.
3. **Cinematic plinking.** `WeaponDefinition::fires_regardless`
   (`crates/contract/src/weapons.rs`) is
   `default` and unlimited ammo together. A gun like that fires at an identified
   enemy it cannot hurt, to keep heads down. A weapon with finite ammo,
   including a finite `default` gun like the autocannon, fires only at what it
   can hurt. `preferred_kind` and the automatic target choice in `select` read
   it.

Asking "is this the default gun?" for three different reasons breaks the rule in
[`development.md`](../../development.md#game-rules): read the property that
makes a weapon behave this way.

## Evidence

From `fixtures/catalog.json`, recomputed 2026-11-12:

- `default` rows: `aa_ballistic`, `air_gun`, `autocannon`, `carbine`, `hmg`,
  `rifle`. Unlimited rows: `air_gun`, `carbine`, `hmg`, `rifle`, plus
  `helicopter_crash`. So the jets' `air_gun` both strafes and plinks.
- Of 136 armed units, 26 have only a finite `default` gun (autocannon or AA
  gun), for example the AH-64E, BMP-2M, BMP-3, Ka-52M, Mi-28NM, Bradleys and
  the Gepard. Another 24 have no `default` gun at all: the artillery, the SAMs,
  the ATGM carriers, the J-20 and the Tiger UHT.
- Requirement W09 (`specs/done/battle-foundation/requirements.md`) said
  "Every armed unit has an unlimited-ammo default gun", and the data above says
  otherwise. **Decided (user):** the wording was out of date and the data
  stands. The Armory amended W09 to what the game does now.
- A "useful shot" counts reserve ammo, not only a loaded round:
  `Mount::has_rounds` checks the rounds left. So a launcher left unloaded
  during a rifle fight still takes rank 2 when a vehicle appears: the soldier
  switches to it, reloads and fires. No test pins this (S1).
- Existing coverage: `the_default_gun_fires_at_what_it_cannot_hurt_but_specialists_do_not`
  and `attack_move_never_halts_for_a_target_it_cannot_hurt`
  (`crates/sim/tests/weapons.rs`) cover rifle plinking, and the finite weapon
  they check is a special, not a finite `default` gun.
  `crates/sim/tests/attack_passes.rs` covers which weapon a pass uses. Nothing
  covers S1, S2, S3 or S5 directly.

## Cross-spec dependency

The Armory ([decision U8](../done/armory/unknowns-map.html#u8)) moved
`fires_regardless` from a private sim function to a method on the contract's
`WeaponDefinition`. The catalog publishes the result on its weapon rows under
`derived`, so the UI reads a flag instead of working it out
([the published weapon facts](../done/armory/README.md#why-it-works-this-way)).
This cleanup changes what that method reads. It does not add a second owner
for it.

## Scenarios that must stay true

Each one becomes an end-to-end sim test before anything is split. The same
tests, the battle digests and the replays must all pass unchanged afterwards.

- **S1.** An AT soldier's Javelin is empty while his squad trades rifle fire
  with infantry. When an enemy vehicle appears he still picks up the launcher,
  reloads and fires. His first shot waits one reload.
- **S2.** In a quiet moment, with nothing for his rifle to shoot, a soldier
  reloads his special weapon, so it is ready before a target appears.
- **S3.** A vehicle's guns fire independently. A tank's main gun and its roof
  HMG engage at the same time. A vehicle never fires one gun at a time.
- **S4.** A squad's rifles keep firing at an identified tank they cannot hurt,
  and so does a roof HMG. This plinking is intentional and must never be
  removed.
- **S5.** A finite `default` gun, such as an IFV's or attack helicopter's
  autocannon, holds fire at an identified target it cannot hurt and keeps its
  rounds.
- **S6.** A jet on a ground-attack pass strafes with its gun row. It drops bombs
  only at a bombing pass's release point, and fires its other ground rows on a
  missile pass.
- **S7.** A unit with no `default` gun (artillery, a SAM) never plinks, and it
  still engages whatever its weapons can hurt.

## Open decisions

Each option must follow `development.md`. Values are authored on the unit or
weapon, not derived in code, and a missing field may only mean none or off.
Rules read physical properties.

- **Soldier fallback.** Read the mount's existing `squad` flag instead of
  `default`? This would leave one marker for "the gun every soldier carries".
  Check that no squad has a `default` row on a non-`squad` mount, or the other
  way round, so that S1 and S2 keep their digests.
- **Plinking.** (a) Keep it derived (`default` and unlimited ammo) inside the
  [U8](../done/armory/unknowns-map.html#u8) contract method. (b) Author an
  explicit property on each row (for example `fires_regardless`, or a "keeps
  heads down" flag) so the data decides, not a derivation. (b) fits "values
  live on the unit" better. (a) means no data changes.
- **Strafing gun.** Which physical property marks a jet's gun row: its
  delivery, guidance or projectile kind? Or keep a named, authored property?
  It must still sort every current jet row the same way (S6).
- **Does `default` survive?** If all three jobs move to their own properties,
  delete the flag (and its W09 doc comment) rather than keep it for nothing.

## What to do next

1. Write S1–S7 as system tests against the current code, using `write-tests`.
   They should pass now.
2. Walk the open decisions with the user (`tweak-mechanics`), then
   `write-spec` the split: one job per slice, with the tests and digests
   unchanged after each slice.

**Unblocks when:** the user gives the go for a rules-cleanup pass. The
Armory's U8, which this builds on, has landed.
