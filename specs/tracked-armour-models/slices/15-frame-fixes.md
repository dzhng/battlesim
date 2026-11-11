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

1. **Edit the roster JSON.** The rows come from `assets/frames/*.json`. Respect
   the caps in `crates/contract/src/limits.rs:21-24`.
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
     (`unitArt.ts:44-45`).
   - Re-check hatches against moved pivots (`specs/done/off-centre-turrets.md`).
   - The 2A8 inherits the 2A7V frame.
6. **Minimal fit edits.** Families whose frame changed get only the edits
   needed to fit the new frame, so `fit.hull_extents` (`validate.ts:1033`)
   passes. Their full rework is their family slice.
7. **Re-bless parity:** `BLESS_PARITY=1` on every suite that reads it:
   - `crates/sim/tests/common/mod.rs`;
   - `publication_reader.rs`;
   - `crates/presentation/tests/effects.rs`;
   - the mapgen tests that name it.

   `paris-corner-*.json` hold frozen Abrams data and only change if an Abrams
   frame changed.
8. **Digest check.** The digest diff may change only for the fixed units and the
   Ajax. Record every changed digest's cause in `decisions.md`.

## Known rows

- **Leopard 2A6/2A7V (and so 2A8).** Height 3.0 m → about 2.6 m, if the survey
  confirms it. The HMG pivot z and `eye_m` drop with the roof.
- **Ajax.** Add the roof machine gun mount:
  - shape: `{"id":"HMG","weapons":["hmg"],"turret":true,"on":"autocannon", ...}`,
    the same as the Leopard's (`europe.json:85`);
  - pivot and muzzle from the survey;
  - `assets/catalog.json` gains its appearance mount draw;
  - the art parents the existing RWS parts (`ajax.py:119-123`) under the new
    mount's nodes, in slice 27.
- **T-15.** `muzzle_m`, if the survey moved it.
- **Possible:** the T-90M pivot, the Puma MELLS, the howitzers, as the survey
  finds.

## Verification

- `cargo test` for sim, presentation and mapgen, after the blessings;
- `bun run --cwd web asset -- validate` for every changed unit, after slice 16
  re-exports it;
- the Leopard hit-rate note is already in `decisions.md` (Q9).

## Delegated

None. The numbers come from the survey.

## Feedback that would change it

A row the user wants kept as it is: remove it, and record why.
