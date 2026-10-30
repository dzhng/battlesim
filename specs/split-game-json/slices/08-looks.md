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

## Done — explicit looks

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
and rerun after 09. The cause-name, visual and verification receipts follow.

### Named delta: light-cause vocabulary

`flash:<weapon>` and `round:<weapon>` become `flash:<look>` and `round:<look>`.
The cause is diagnostic metadata: no random key or renderer input reads it.
The only allowed oracle difference is those strings in `effects.jsonl` (and
the corresponding recorded vector light-cause metadata); all
numeric effect/light bits, sounds, poses, publications and battle digests stay
identical. The light-review scene follows the new vocabulary. This change is
its own commit so the metadata delta is reviewable apart from the data move.

### Final proof and checks

- Before the named cause rename: all 77 oracle files identical. After it:
  76 identical; `effects.jsonl` differs only in the declared cause strings
  (728 of 127,306 rows). Substituting the names leaves every remaining byte
  identical. Sound, poses, events, publications, shaders and battle digests
  never move. All six native plates remain byte-identical; the screenshot
  comparison reports zero pixel difference on each.
- Full presentation suite passed; contract weapon tests and focused simulation
  map/objective/purchase/exact-number checks passed. The affected web audio,
  workbench, projectile and stored-record parity suites passed, as did
  typecheck, ESLint, Oxlint, formatting, the size ratchet and the commit hook's
  workspace/all-target Clippy run. Repository-wide closeout waits for 16.
- `UPDATE_SOUND_VECTORS`, `BLESS_PARITY`, and `UPDATE_RECORDS` ran, followed by
  their unflagged checks. Stored battle rows/digests and sound commands are
  unchanged; the only recorded output delta is light-cause metadata in its
  named commit. The migration script's second run writes nothing.
- The GPU-locked look-sheet run captured every one of the 37 looks, with
  launch, flight, impact, blast where applicable, and smoke aftermath.
  Evidence: `throwaway/evidence/rounds/slice-08-final/`. Every look uses
  the labeled controlled HE lane so all stages have consistent visible
  context. The lane changes only appearance and explicitly represents style,
  not each weapon's physics. The development mechanics snapshot is the input;
  each launch checks its rendered cause at the posed review muzzle. Launch
  captures precede flight by one tick, matching the hull publication contract.
  Contact views leave room around the blast, and aftermath cameras fit the
  full reviewed flight. Eight aftermath views were recaptured at the same
  elapsed time after their own strikes; the grenade sample's absolute tick
  differs by one. Probabilistic tracer looks wait for a visible flight sample
  without changing their chance. Tall sheets are captured in viewport tiles
  and stitched, avoiding Chromium's repeated full-page compositor sections.
  The sheet explains its style carrier; contact frames fit whole and aftermath
  pairs a contextual overview with a strip around the reviewed flight.
- No historical tracer-sheet capture was present in this checkout or the
  supplied baseline directory. That specific sheet comparison is unavailable;
  the six frozen native plates and full-precision oracle provide the before/
  after comparison. The new sheet is the reference for 11, 13 and 15.

### Visual review boundary

The fresh reviews inspected the complete gallery and enlarged features. The
capture pass corrected mislabeled fallback styles, missed short launch flashes,
invisible probabilistic samples, clipped aftermath endpoints and repeated tall
sheet sections. Receipts and reports live in `throwaway/slice08/`.

Default views supply unit context; detail launch views deliberately crop hulls
around the muzzle. The preserved appearance still has faint small-style flashes
at the default camera, saturated blast cores and broad missile streaks. Those
are readability findings for the later content pass, not approval of their
current artistic quality. This slice preserves them under its exact-output
contract. The sheet is a style baseline, not an occlusion or motion proof; the
frozen battle plates and unrounded probe prove this migration's parity.

The final unprimed review (`throwaway/slice08/coverage-critique.md`) passes
evidence coverage and disclosure, while rating visual quality mixed for the
preserved readability findings above. Its receipt check passes. The full gallery,
all enlarged crops and relevant native frames were inspected. Preview was shown
for the non-blocking checkpoint and closed afterward.

**Review follow-ups (open; for the shared fix pass):**
- An unknown `audio.cook_off.landing.as` passes admission and panics when a
  turret lands (`sound/tables.rs:373`). Validate the landing look and its
  contact surface in `SoundTables::from_parts`.
- Look admission doesn't require the parts physics needs: an accelerating ATGM
  with `sound.motor: null` is accepted (`looks.rs:96`). This is slice 10's
  fielding check; make sure it covers motor, blast and shot.
- `impact.surfaces` accepts misspelled keys such as `hulll` (`looks.rs:49`).
  Validate them against the impact-material vocabulary.
- `impacts.*.sound` and `ricochet.sound` are still required and validated,
  but playback takes recordings only from looks (`sound/tables.rs:229`).
  Remove them from Rust, TS and the fixtures; keep the gains.
- The sound workbench saves a candidate that battle startup would refuse
  (`apps/sound-workbench/server.ts:106`). Admit candidates against the looks,
  the mix and the weapon rows before preview and save.
- `decisions.md:20` repeats `map.md:132`. Link to it instead.
- `fixtures/README.md:210` still says "nearest styled ancestor". Say that
  inheritance supplies `appearance`.
- `apps/sound-workbench/README.md:28` attributes movement levels to
  presentation; they belong to `audio.json`.
