# 07 Pipeline

**Unlocks:** exporters that run, one place to put reusable detail, a review
sheet that shows references, and measured triangle counts to budget against.

## Known state of the exporters (2026-10-06)

- Five helpers open `specs/unit-roster/manifests` with no fallback and fail
  (`armor`, `eastern_armor`, `eastern_tanks`, `europe_carriers`,
  `light_armor`); three already fall back to the archive (`remaining_ground`,
  `remaining_tanks`, `infantry_equipment`); `logistics.py` takes dimensions inline.
- 13 family receipts already disagree with their helper's current hash
  (Bradley, JLTV, LAV, Stryker; Challenger, KF51, Leclerc, Type 99; Fennek,
  Puma, Tigr, VBCI, VBL): re-exports will differ for reasons the catalog
  switch didn't cause. Nothing ties a GLB to its script; recipe images are
  embedded at export, so a recipe change shows only after re-export.
- Mount articulation roles live in `assets/catalog.json` `mounts`, not
  `fixtures/catalog.json`: read both.
- `asset blender` expects `/Applications/Blender.app` (`web/asset.mjs:114`);
  the pinned 5.2.1 is elsewhere on this machine, so set `BLENDER`.
- **The HMMWV is the JLTV.** `assets/source/roster/humvee/us_m1151_hmmwv_hmg.glb`
  is the same Git blob as `jltv/us_m1278_jltv_hmg.glb` (`ccf5f60f`), its frame
  in the fixtures is the JLTV's (6.2 × 2.5 × 2.6 m, eye 2.885, HMG pivot z
  2.665), and the `humvee_*` branch in `light_armor.py` was never exported.
  **Decision 3:** give the HMMWV its real frame (about 5.2 × 2.1 × 2.2 m, eye
  2.42, pivot z 2.28 per the archived manifest; check against slice 11's
  references) and export the HMMWV branch. A physics change: digests move,
  and a quick balance sample for the HMMWV only, named in choices.md.

## Work

1. **Frames from the fixture catalog.** Every roster exporter
   (`packages/scene-assets/blender/roster/*.py`) that opens
   `specs/unit-roster/manifests/<family>.json` reads the variant's frame
   (`half_extents_m`, `eye_m`, mounts with `pivot_m`, `muzzle_m`, `on`, role)
   from `fixtures/catalog.json` by type id instead, through one helper in
   `parts.py` (or a sibling module). Export path and variant list come from the
   catalog's appearance rows. Body dimensions are `2 × half_extents_m`.
   Test: re-export one family per legacy helper (Bradley, BMP, Boxer, Fennek,
   HEMTT, T-90) unchanged and confirm each GLB validates and the bake's art
   hash for that appearance is unchanged, or explain each difference. If the
   export proves not byte-stable even with an unchanged script, compare the
   validator's node, frame and triangle stats instead and record that in
   choices.md.
1b. The HMMWV frame and export (decision 3 above).
2. **Shared detail parts.** A new `blender/vehicle_parts.py` owns the pieces
   the [detail bar](../README.md#the-detail-bar) names, each a function taking
   frame, size, material set and parent, emitting tiered geometry:
   `tyre_wheel`, `road_wheel`, `sprocket`, `idler`, `return_roller`,
   `track_run` (a belt whose shoes and guide horns are texture and normal map, since track UVs scroll), `hatch`,
   `periscope`, `sight_housing`, `light_with_guard`, `tow_hook`, `shackle`,
   `smoke_discharger_bank`, `antenna`, `jerrycan`, `stowage_box`, `tarp_roll`,
   `grille`, `exhaust`, `mudflap`, `weld_line`. Lift the existing per-family
   `wheel()` and track belt into it; families switch to it as each is rebuilt
   (slices 12–14), not in this slice. Write each part readable: one statement
   per line, named arguments.
   Box UVs take only the first material's tile (`parts.py:655-668`): each
   part puts each material on its own object, so a tyre and its rim keep their
   own texel scale. Track nodes scroll every vertex's UV beneath them
   (`modelLayer.ts:308-330`): `track_run` keeps link relief in texture and
   normal maps, never modelled shoes, and nothing else is parented under a
   track node.
3. **References on the sheet.** `asset sheet` takes `--references`: it reads
   `assets/references/<family>/references.json` (slice 11's layout) and writes
   `reference.png`, the sheet's views each beside the reference photo of the
   same view, missing views marked. No reference file, no panel.
4. **Count.** Record, per runtime vehicle, triangles per tier from the sheet's
   stats, into choices.md as the baseline (dated). The benchmark runs on
   test units (slice 05), so it can't see roster models: measure frame cost in
   a lab scene holding a column of roster vehicles (Abrams, Stryker and one of
   each class), using the benchmark's cost measure, as the "before" for slice 12.

## Seam

Exporter input: type id → frame, from the resolved catalog. Part library:
pure Blender functions, no catalog access. Sheet: appearance in, PNGs out.

## Verify

- The six re-exports above validate; bake and `check` the touched appearances.
- `asset sheet us_m1_abrams_sep_v3_trophy --references` with a stub
  references.json writes `reference.png`. Look at it.
- No screenshot gate beyond looking: this slice changes no art.

## Delegated

Module name and part signatures; whether the frame helper lives in `parts.py`.

## Stays green

Every existing GLB's validation; the archived manifests are not edited.
