# 07 Pipeline

**Unlocks:** exporters that run, one place to put reusable detail, a review
sheet that shows references, and measured triangle counts to budget against.

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
2. **Shared detail parts.** A new `blender/vehicle_parts.py` owns the pieces
   the [detail bar](../README.md#the-detail-bar) names, each a function taking
   frame, size, material set and parent, emitting tiered geometry:
   `tyre_wheel`, `road_wheel`, `sprocket`, `idler`, `return_roller`,
   `track_run` (belt at tiers 1–3, shoes and guide horns at tier 0), `hatch`,
   `periscope`, `sight_housing`, `light_with_guard`, `tow_hook`, `shackle`,
   `smoke_discharger_bank`, `antenna`, `jerrycan`, `stowage_box`, `tarp_roll`,
   `grille`, `exhaust`, `mudflap`, `weld_line`. Lift the existing per-family
   `wheel()` and track belt into it; families switch to it as each is rebuilt
   (slices 10–12), not in this slice. Write each part readable: one statement
   per line, named arguments.
3. **References on the sheet.** `asset sheet` takes `--references`: it reads
   `assets/references/<family>/references.json` (slice 09's layout) and writes
   `reference.png`, the sheet's views each beside the reference photo of the
   same view, missing views marked. No reference file, no panel.
4. **Count.** Record, per runtime vehicle, triangles per tier from the sheet's
   stats, into choices.md as the baseline (dated). The benchmark runs on
   stand-ins (slice 04), so it can't see roster models: measure frame cost in
   a lab scene holding a column of roster vehicles (Abrams, Stryker and one of
   each class), using the benchmark's cost measure, as the "before" for slice 10.

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
