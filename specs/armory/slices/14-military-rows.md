# 14 · Military prop rows and their mechanics, on stand-ins

Status: done 2026-11-13.

Map rows: Q2, L2, L3. Depends on L1 lifting.

## Gate

Read `specs/tracked-armour-models/README.md` first. Start only once it has
closed, or once its coordinator has released `fixtures/props/**`. Record what
you found in `decisions.md`.

Then run `tweak-mechanics` before writing any row.

## Contract

K9. New prop rows, every one an ordinary prop:
- the same in every region;
- not garrisonable;
- no building-scaled integrity (`contract/src/catalog/props.rs:168-186`).

Each lands as `appearance.status: "systems_only"` with a `stand_in_tint`, so the
mechanics ship before the art. `catalogFootprints.test.ts:43` skips
systems-only rows.

| Row | Body (values come from `tweak-mechanics`, never pinned by a test) |
|---|---|
| `hangar` | Immovable. Blocks movers and rounds. Occludes. Gives cover at its edges. Has `hp`, and `destroyed.into: hangar_ruin` (L2), following `garage_row → rubble` (`fixtures/props/city/courts.json`). |
| `hangar_ruin` | Low: a twisted frame over rubble. Still gives infantry low cover. Terminal. |
| `revetment` | Stops rounds and gives cover. |
| `hesco` | Stops rounds and gives cover. |
| `tent` | Light. Never stops rounds. |
| `flagpole` | Topples. Gives no cover. |
| `supply_net` | `blocks.infantry: false`, no `cover_tier`, `occludes: false` (L3). The yard and depots place it only over supply stacks and containers. |

**Where everything lives.** The rows go in `fixtures/props/military/*.json`,
admitted by `check_prop` (`catalog.rs:1505-1576`). Rules that admission
enforces:
- `hp` and `destroyed` go together;
- `armor` lies within 0–1;
- destruction chains end.

**Named moves (S3), each re-recorded and named in its commit:**
- `PropKind` ranks renumber (`catalog/props.rs:214`);
- the native counts test (`native/client/tests/counts.rs`);
- the publication-reader parity (`crates/sim/tests/publication_reader.rs`);
- the street-lab digest (`web/scenes/_appJourney.mjs:366`);
- the `ConfigMismatch` tests (`config_digest`).

The battle state digest stays unchanged (`world/mod.rs:748`: prop poses hash no
kind). Prove it by passing the replay tests with no bless.

**Regenerated and hand-edited files:**
- re-bless `fixtures/catalog.json` with `BLESS_CATALOG=1`;
- the frozen `fixtures/parity/map-layout/physical-rules.json` is left for slice
  17, where generation first reads these rows.

## The human runs or sees

A `test` map in the street lab or a sim test map:
- a hangar takes fire and falls to its ruin;
- a soldier shelters behind the ruin;
- a net stands over crates, and soldiers walk through and see through it.

## Verification

Write these first. They test mechanics through each body's fields, on `test_*`
props, and only where no existing test already pins the mechanic:
- `a_ruined_prop_keeps_covering_the_soldier_behind_it`;
- `a_body_with_no_cover_and_no_stop_never_shields`;
- `a_body_that_blocks_no_one_and_occludes_nothing_hides_nothing`.

`cover.rs` and `rays.rs` stay green. Read the load line against the 768 MiB
tripwire (S5). A breach is a named decision for the user, never a silent raise.

**Visual gate.**
- **The one variable:** the destruction state reads, standing hangar against
  ruin, on stand-ins.
- **Crop:** the hangar's footprint plus its ground.
- `screenshot-critique` last.

## Delegated to the implementer

The exact body values, within the rows above, as `tweak-mechanics` concludes.

## Must stay green

- Every sim test apart from the named moves.
- `asset check`.
- The city and forest scenes.

## Feedback that would change it

- **"The hangar should be garrisonable":** reopens L2. That's the user's call.
- **"The ruin should be plain rubble":** change only the `destroyed.into`
  target.
