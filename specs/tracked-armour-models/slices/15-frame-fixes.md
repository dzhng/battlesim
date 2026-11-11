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

## Result (2026-11-11)

**Contract.** The battle frames changed as `fix-rows.json` names them:
- all 64 rows were applied, none refused;
- the blessed `fixtures/catalog.json` was regenerated;
- the Buk-M3 is `tracked_medium`;
- the Ajax gains `HMG` `on: autocannon`, drawn by the `hmg` rig.

In `assets/catalog.json`:
- 21 wreck footprints match their hulls exactly (the 20 listed and the
  2A8);
- track gauges: CV9040C 0.75, 2S19M2 0.76;
- four wreck tolerances widened: T-80BVM 2.0, Challenger 3 2.2,
  Challenger 2 TES 2.1, Buk-M3 0.2.

No parity record changed, and no recorded digest moved
(`decisions.md`, "Slice 15 digest record").

**Fit edits.** Each is minimal and in its family script. The reasons are in
`decisions.md`.

| Family | Edit |
|---|---|
| `air_defence` | tow hooks on the tracked hull's sloped nose and tail plates; the Pantsir's own hook under its bumper; the Buk's canisters end at the tail; the Gepard's folded radar is dressing on a mast that reaches the box top |
| `leopard` | the PERI and EMES heads are dressing, the PERI at 3.0 m; the 2A7V/2A8 body is the 2A6's, offset behind the frame's centre, with a 0.35 m longer front module |
| `t14` | turret roof at 2.75 m; sights and soft-kill box as dressing |
| `t80` | 7.01 m hull: a shorter glacis run; nose and rear stations moved 0.195 m |
| `bmp` | BMP-2M roof fittings as dressing; hatches placed from the turret ring |
| `cv90` | CV9040C roof fittings as dressing |
| `cv90120` | sight heads and tarp as dressing; ball at 2.8 m |
| `m10` | roof fittings and dome as dressing; dome at 2.89 m |
| `puma` | PERI 0.11 m lower, its head as dressing |
| `bradley` | the CIV head is dressing |
| `ajax` | ORION head as dressing at 3.0 m; the weapon station as dressing until slice 27 |
| `challenger_3` | panoramic sight on a non-dressing mast |
| `type99` | commander's sight post reaches the box top |

**Proof.**
- Every appearance the 20 affected scripts export passes `asset validate`
  against throwaway exports, as do all their wrecks: 40 live
  appearances, plus the wrecks of every changed footprint and every gun
  row.
- Lint shows no new issues except the Buk's one (`decisions.md`).
- `cargo test`: sim (1,162), presentation (360) and mapgen (200) are
  green.
- Web: `catalogFootprints`, `validate`, `catalogImports`, `sessionCatalog`,
  `icons` and `mountMuzzle` are green.
- Before/after closeups were judged by eye for the T-14, T-80BVM, 2A7V,
  Buk-M3, Gepard, Challenger 3, BMP-2M and M10. The shots are in
  `throwaway/s15/shots/`.

**Red until slice 16.** `web/tests/sceneAssets/unitFit.test.ts`. The baked
runtime still draws the old frames. Its first failure is the BMP-2M's
muzzle, 0.64 m off. Every appearance with a frame row stays red until slice
16 re-exports and bakes.

**Committed art.** Only `air_defence.py`'s full export, live and `--wreck`,
with its receipt (C12). It picks up the shared-helper changes from slices
09–11 too, so the Skyranger and NASAMS 3 changed as well; the US NASAMS
came out byte-identical. Every other family's GLB was a throwaway export,
restored after validation.

**For slice 16 and the family slices.**
- The Buk's lint baseline entry `(hull_upper, track_L_band)` needs a
  refresh before the lint-on re-export (110 → 166 mm, hidden seating).
- Slice 17: the T-80BVM's idler wraps 6 cm past the nose.
- Slice 18: the T-14's turret is squeezed by the shared deck, which is
  0.11 m high.
- Slice 24:
  - the Buk's turntable is at −0.55 m, against the frame's pivot of
    +2.0 m;
  - the Gepard's hull-drawn barrels sit at y ±1.28 m, against the frame's
    0.88 m muzzle offset.
- Slice 27: parent `rws_*` under the HMG's nodes and drop `dressing_rws`.
- Slice 29: the howitzer pivots moved by up to 3.6 m. They are hull-drawn
  mounts, so only the shell origin moved.
