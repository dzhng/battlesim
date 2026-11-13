# 17 · Depots in open country [held: L1]

Map row: L4, plus P3. Depends on 13 and 14. Art from slice 15 is preferred but
not required, because stand-ins draw.

## Gate

Run `tweak-mechanics` first.

## Contract

K10. `open_country.depots` is a `Steads` block (slice 13) with `Bodies` pieces:
3–4 military props per depot, in their own yard, sized from the pieces' catalog
bodies (P3).

**Placement:**
- Its own random stream, `country/depots` (`open_country/mod.rs:296`).
- The halves stay even through `spread` (`mod.rs:431-457`): "each kind is
  placed in the half that holds less of it".
- The count is `per_road_km × road km`, the same scaling homesteads use.
  Depots are rare: a few on large maps, none or one on small ones.
- **A depot that doesn't fit is left out. Generation never refuses for one.**
- Nets go only over the depot's supply pieces.

**Version bumps.**
- The preset `revision` becomes `layout-presets-21`.
- `GENERATOR_VERSION` stays, because slice 13 made the code path inert without
  presets.
- Generated maps change by design. Shared generated-battle links from before
  the bump are refused (the user's hard cutover).

## Seam

**Presets.** `fixtures/map-presets.json`, under `open_country`. Presets refuse
unknown fields, and every section is required
(`layout/presets/definitions.rs:10-26`).

**The frozen parity files:**
- **`fixtures/parity/map-layout/physical-rules.json` (S2).** Add the military
  rows by hand, fully expanded, the way `garage_row` appears there (precedent
  `342711dd2`). It is frozen and never regenerated.
- **The parity records (S1).**
  - Re-pin each of the 17 frozen requests by hand, request by request.
  - The "stale preset revision" record pins the previous revision, `-20`.
  - Precedent: `33003a6bd`, then `9a50e0f9d`.
  - **Never run a blind `BLESS_PARITY=1`.** It records refusals as the
    expectation, and the test still passes (`mapgen/tests/layout_cli.rs:109`).

**The distant-impostors baseline (S6).** Metro XL seed 4 regenerates; update
`specs/distant-impostors/baseline.json`.

**Map tools.** Update the workbench inspection groups, the help mapping and the
plan picture (the swamp precedent: `examples/common/workbench.rs:187-193`,
`workbench_fields.rs`, `inspect.rs`, `plan.rs`).

## The human runs or sees

`/labs/map-picture`, or the map workbench, on large and XL open and mixed seeds:
depots stand by country roads. Small maps rarely have one.

## Verification

Write these first, in `crates/mapgen/tests/open_country.rs`. Each reads the
preset and pins no count:
- **`depots_stand_in_open_country_on_legal_ground`.** It applies the full S4
  keep-out list from day one:
  - road strokes and polygon gaps;
  - woods and swamps;
  - the river;
  - bridge decks and approaches;
  - objectives and entries;
  - towns and homesteads;
  - clearance from polygon edges, not only vertices.
- `the_halves_hold_about_the_same_of_each_thing`, extended to depots.
- `a_depot_that_does_not_fit_is_left_out`: on a cramped preset, generation
  still succeeds.
- `more_road_holds_more_depots`.

In `crates/mapgen/tests/skirmish.rs`, add
`the_corpus_refuses_no_more_requests_than_before`. Today `:216,366` swallow
refusals with `continue`, so without it a depot failure could hide there.

**Visual gate.**
- **The one variable:** a depot reads as a military compound, sited by a road.
- **Crops:**
  - the depot lot from the battle camera;
  - the full plan in `map-picture.mjs` on three seeds.
- `compare-screenshots` before and after on the same seeds.
- `screenshot-critique` last.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

- The `per_road_km` value. It is a loose tripwire, not a target.
- The mix of groups and the size of each yard.
- `apart_m`.

## Must stay green

- `no_sight_circle_is_unbroken_and_the_country_stays_open`.
- `the_same_request_furnishes_the_same_country`.
- `furnished_maps_stay_inside_the_admission_limits`.
- The skirmish corpus.
- Admission.
- The swamp, road and wood tests.

## Feedback that would change it

"Too many" or "too few" is a preset value.

## Reslice if too big

1. Placement on a `test` preset: keep-outs and even halves.
2. Then the shipped presets bump, with the parity re-pins.

The re-pins land with the shipped bump, never apart from it.
