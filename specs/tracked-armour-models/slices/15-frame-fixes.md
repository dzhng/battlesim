# 15: frame fixes, in one chain (coordinator, after 07)

## Contract

Every frame row in slice 07 marked "fix", plus the Ajax's roof machine gun,
lands in one chain. Each digest change is named (user decisions Q9 and Q9b).

This slice lands and pushes together with slice 16, because `bun run check`
reads the baked runtime against the frames
(`web/tests/sceneAssets/unitFit.test.ts:91-110`).

## Seam

Data only:
- `fixtures/units/roster/{us,europe,eastern}.json`;
- the blessed `fixtures/catalog.json`;
- `assets/catalog.json`;
- parity records.

## The chain, in order

1. **Edit the roster JSON.** Apply `assets/frames/fix-rows.json` in its order:
   each row names its fixture, JSON path, old and new values, and digest
   consequence (slice 07b, decisions.md C9). Every hull row is already
   inside the caps in `crates/contract/src/limits.rs:21-24`.
2. **Bless the catalog:** `BLESS_CATALOG=1 cargo test -p sim --test sim catalog::`.
   Blender reads frames only from `fixtures/catalog.json` (`catalog_frames.py:21-23`).
3. **Wreck footprints.** Copy each changed hull's half extents into the wreck's
   `footprint_half_m` in `assets/catalog.json`. They must match exactly
   (`validate.ts:1658-1667`).
4. **Track gauge.** Retune `track_gauge` wherever a width changed. It is a
   fraction of half-width (`crates/presentation/src/fielded.rs:440`,
   `crates/sim/src/battle.rs:2139`).
5. **Allowances and hatches.**
   - Check the dressing allowance on every lowered roof, before blessing
     (`unitArt.ts:44-45`). Where the box top stands more than 0.1 m over the
     real roof (C4), a non-dressing part must still reach it:
     `fix-rows.json` lists these under `derived.art_fit`.
   - Re-check hatches against moved pivots (`specs/done/off-centre-turrets.md`).
   - The 2A8 inherits the 2A7V frame.
6. **`air_defence.py` was held back by slice 01.** Its tow hooks hang 12.7 cm
   past the frames that grew on 2026-10-15, on the Buk, the Gepard and the
   wheeled Pantsir, and the Buk's wreck is 11 cm too narrow on one side. Seat
   the hooks and fix the wreck as this slice's minimal fit edit, then
   re-export the whole script. Confirm the Buk-M3's unit class in
   `fixtures/catalog.json` follows its new tracked profile; it is
   `wheeled_medium` today, which sets its sounds and triangle caps.
7. **Minimal fit edits.** Families whose frame changed get only the edits
   needed to fit the new frame, so `fit.hull_extents` (`validate.ts:1033`)
   passes. Their full rework is their family slice.
8. **Re-bless parity:** `BLESS_PARITY=1` on every suite that reads it:
   - `crates/sim/tests/common/mod.rs`;
   - `publication_reader.rs`;
   - `crates/presentation/tests/effects.rs`;
   - the mapgen tests that name it.

   `paris-corner-*.json` hold frozen Abrams data and only change if an Abrams
   frame changed.
9. **Digest check.** The digest diff may change only for the units in
   `fix-rows.json` and the units that extend them (the 2A8, the M1E3). Record every changed digest's cause in `decisions.md`.

## Known rows

The full list is `assets/frames/fix-rows.json` (64 rows). These need more
than a value change:

- **Buk-M3.** It becomes tracked: a new profile and a tracked `mobility`
  block, plus a 9.3 m hull (C6).
- **Ajax.** Add the roof machine gun mount:
  - shape: `{"id":"HMG","weapons":["hmg"],"turret":true,"on":"autocannon", ...}`,
    the same as the Leopard's (`europe.json:85`);
  - pivot and muzzle from the survey;
  - `assets/catalog.json` gains its appearance mount draw;
  - the art parents the existing RWS parts (`ajax.py:119-123`) under the new
    mount's nodes, in slice 27.
- **Challenger 3.** It gains its own `half_extents_m` and cannon `muzzle_m`
  instead of inheriting the Challenger 2's.

## Verification

- `cargo test` for sim, presentation and mapgen, after the blessings;
- `bun run --cwd web asset -- validate` for every changed unit, after slice 16
  re-exports it;
- the Leopard hit-rate note is already in `decisions.md` (Q9).

## Delegated

None. The numbers come from the survey.

## Feedback that would change it

A row the user wants kept as it is: remove it, and record why.
