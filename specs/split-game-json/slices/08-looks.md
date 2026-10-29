# 08 Looks that reproduce today

**Contract:** every weapon names one look, and presentation reads looks
instead of weapon-keyed tables. **Nothing seen or heard changes**, today's
bugs included.

**Seam:**
- **Contract weapon row:** `appearance: String`. The catalog's ordinary
  `extends` merge inherits it. Weapons don't use `abstract`.
- **Data:** `presentation.json` `weapon_looks.<look>`:
  `{ tracer, flash, impact, blast, kick: {hull, soldier}, sound: {shot, blast?, motor?, impacts, gain, far_m, …} }`.
  - `impact` merges today's visual `impact_scale` with the per-surface
    `round_scale`s.
  - `sound` nests inside the look. Its levels are required. Any part may name
    a recording that overrides `audio.json`'s default for that part.
  - The firing sound has no default: every look names its own.
- **`audio.json`** gains the default recording per part (blast, motor, each
  impact surface) and the `effects` role map from `sounds.json` (explosion,
  motor, footstep, tracks, rotor, ricochet).
- **`sounds.json`** becomes only the library. Its per-weapon `defaults` and
  `impacts.<contact>.<weapon>` move into looks.
- **`presentation::looks`** is the one owner:
  - `WeaponLook`;
  - a per-battle reader built from the looks and the battle's weapon rows,
    returning a look by weapon id.

  The browser reaches it through presentation-wasm; `presentedFrame.ts` and
  `soundFeed.ts` pass documents and stop resolving weapons themselves. The
  effects tables (`fielded/effects.rs`), `effects/frame/pool.rs:221`,
  `kick_of` (`units/driver.rs:177`), the sound tables (`sound/tables.rs`
  `shot`/`contact`), `presentedFrame.ts`, `soundFeed.ts` and
  `shippedSounds.ts` all read through it.
- **Deleted:**
  - every weapon-keyed presentation table;
  - `sound::tables::inherit` and every `extends` walk in presentation and
    audio;
  - the `"default"` fallbacks, including `publication.rs:202`'s for a unit
    without a mount.
- **Light causes** become `flash:<look>` and `round:<look>`.
  `audio.cook_off.landing.as` names a look.
- **Looks in this slice** are as many as it takes to reproduce today exactly,
  about 37: one per distinct combination the old tables gave.
  - They're named for what they are, plus a suffix where a bug made them
    differ. Example: `tank_sabot_no_kick` for the derived tank rounds that
    never kicked.
  - Slice [13](13-fold-looks.md) folds them away. They are a transitional
    seam.
- **The look sheet:** `web/scenes/_roundsSheet.mjs` becomes a sheet keyed by
  look, covering flash, tracer, impact and blast. Its first capture is the
  "before" for 11, 13 and 15. It stops walking `extends` itself.

**Run / see:** the look sheet (`WATCH_TOURS=rounds bun run --cwd web scene --
street-watch`, re-keyed), and a battle that looks and sounds the same.

**Verify:**
- Write first:
  - a derived weapon fired through a battle presents its look;
  - an encounter-added weapon that `extends` `tank_ap` presents `tank_ap`'s
    look, not `default`;
  - a fixture test refuses a look that no shipped weapon names.
- 01's probe for all 49 rows: identical bytes. If light-cause names feed
  presentation randomness, that delta is its own named commit with its own
  re-bless.
- Native plates byte-identical. The vectors' outputs are unchanged; their
  inputs are rewritten.
- Run [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  on the new sheet against the old tracer sheet for the tracer rows. Then run
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.

**Moves:**
- `rules_hash` and every replay's config digest (the new `appearance`
  field);
- `catalog.json` text.

Battle digests don't move. One script, `throwaway/split-proof/rewrite-looks.mjs`,
migrates:
- the skirmish records;
- `exact-numbers.json`;
- the `effect-vectors.json` and `paris-corner-units.json` inputs;
- the sound vectors' inputs.

Then re-bless: `UPDATE_SOUND_VECTORS`, `BLESS_PARITY`, `UPDATE_RECORDS`. Run
each again unflagged.

**Depends on:** [04](04-split-files.md) and [05](05-typed-readers.md).
Serialize its record rewrite with [09](09-rules-cleanups.md).

**Delegated:**
- the transient look names;
- internal module layout;


**Would change this slice:** a weapon whose current output no look can
reproduce. Name it here and stop.

## Done — implementation checkpoint

The ordinary contract resolver inherits `appearance`; 37 transient looks preserve
all former table combinations, including exact-id kicks and renamed-RPG bugs.
Presentation, poses and sound consume one `looks` owner. Library assignments and
presentation/audio `extends` walks are removed. The browser carries its battle's
weapon rows unchanged; assignment authoring is read only until 14.

Before light-cause renaming, all 77 oracle files (the probe and composed shaders)
and all six native plates are byte-identical. The derived/encounter battle tests,
unused-look refusal and invalid-surface refusal exercise the new boundary. The
cadence exception and remaining implementation choices live in
[`decisions.md`](../decisions.md).

The integrator's rerunnable stored-input migration is
`/Users/server/dev/battlesim/throwaway/split-game-json/scripts/rewrite-looks.mjs`.
It takes an optional checkout root, rewrites only inputs, and is idempotent.
Record migration/re-blessing is kept in the last commit so it can be discarded
and rerun after 09. Final cause-name, visual and verification receipts follow.

### Named delta: light-cause vocabulary

`flash:<weapon>` and `round:<weapon>` become `flash:<look>` and `round:<look>`.
The cause is diagnostic metadata: no random key or renderer input reads it.
The only allowed oracle difference is those strings in `effects.jsonl` (and
the corresponding recorded vector light-cause metadata); all
numeric effect/light bits, sounds, poses, publications and battle digests stay
identical. The light-review scene follows the new vocabulary. This change is
its own commit so the metadata delta is reviewable apart from the data move.
