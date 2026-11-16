# 01 · One building order

**Unlocks:** the command contract's final shape, and `OccupyBuilding` routing
every listed squad in. Admission is unchanged in this slice, so a second squad
still gives up when its timer ends; slice 03 changes that.

## Contract

Lead with this in the commit message.

- `Order::Garrison` deleted. `OccupyBuilding { units, building, gesture }`
  (`facing` deleted: only companion gathering read it).
- `BuildingPreviewRequest { units, building, queued }`.
- `BuildingPlacement { building, entries: Vec<BuildingEntry> }`
  (`entrant`, `destinations` and `unproven` deleted).
- `OrderError::OneSquadPerBuilding` and `::BuildingOccupied` deleted.
- The publication refusal-name table (`crates/sim/src/publication/tags.rs`)
  loses both names. Re-record the publication vectors this moves
  (`fixtures/parity/publication/codec-vectors.json`,
  `crates/presentation/tests/fixtures/fog-sight-vectors.json`) as a named
  layout change: only the tag table may differ. The browser capture files
  re-record once, in slice 07.

## Seam

- **`garrison::occupy` (`crates/sim/src/garrison/occupy.rs`), rewritten.** For
  each listed infantry squad, from the end of its order queue when the order is
  queued, otherwise from where it stands: its approach is
  `garrison::approach(world, side geometry, authored, owner, from,
  entry_distance_m / 2)`, the call `battle.rs` makes for `Order::Garrison`
  today. The squad gets an entry if it is within entry reach or the side's
  grid reaches its approach; otherwise it gets none. Planning reads only the
  side's own knowledge; it never reads occupants.
- **Application** pushes `UnitOrder::Garrison { building, approach }` per
  entry, exactly as the `Order::Garrison` arm does today (no `Move` prefix).
- **`garrison::validate`** keeps `NotABuilding`, refuses `NotInfantry`, and
  drops the one-squad and same-side-occupancy checks. Its whole-host size check
  stays as it is until slice 03.
- **`encounter/defender.rs`** sends `OccupyBuilding { units: [unit], building,
  gesture: 0 }`.
- **Delete:** `movement/certify.rs` and its export in `movement/mod.rs`;
  `occupy`'s `Candidate`, `rank`, `RouteJob`, `points`, `gather`,
  `EntryAction`, `entry_work_keeps_building` and the selected-holder logic;
  the `Order::Garrison` arms in `battle.rs`.
- **TypeScript mirror:** `web/src/battle/sim/protocol.ts` (order kinds and
  `BuildingPlacement`), the client, `web/src/battle/input/pointerIntent.ts` (send only to units
  `web/src/battle/input/commandReach.ts` allows for "garrison", no facing drag), `pointerPaint.ts`
  (cursor and ack read `entries.length`), `orderPreview.ts` (one approach line
  per entry), `rejectedOrder.tsx` and `packages/battle-audio/src/voices/responses.ts`
  (drop the two reasons), `apps/battle-lab/src/routes/ui.tsx` sample ack,
  `routes/garrison.tsx` and `projectileReview.ts` (`occupy_building`),
  `web/scenes/_cursorOrders.mjs`.
- **Rewrite callers:** the `garrison()` test helper (`tests/garrison.rs`),
  `garrison_sight.rs`, `buildings.rs`, `concealment.rs`, `supply.rs`,
  `transport.rs`, `battle/tests.rs`, `skirmish_ai_orders.rs` (drop
  `BuildingOccupied` from its list), `examples/cover_report.rs`. Delete the
  gathering, single-entrant and route-ranking tests in
  `tests/garrison/occupy.rs` and `entry_routes.rs`; keep and rewrite queued
  re-entry, the wall-side entry without an approach, routing round a row of
  buildings to the one behind, and the replay-every-tick test.

## Confirm first

- `garrison::approach` has exactly one caller today (the `Order::Garrison`
  application) and `certify_orders` exactly one (`occupy`). Verified by grep
  at planning time; re-check.
- The grid reachability query the plan relies on exists and answers for a
  building behind a row of others. If it doesn't, stop and reslice (ledger).

## What the human can run or see

In the garrison lab, select three squads and right-click a house: three
approach lines, and each squad walks to the house.

## Verification

- Switch every test helper to `OccupyBuilding` and change nothing else: those
  tests stay green unchanged. That proves the single-squad lowering.
- `UNCHANGED_OTHER_SCENES` and `UNCHANGED_SINGLE_HOSTS` against slice 00's
  baseline: identical, including the defender encounters. The menu-reel
  event-log tests stay green.
- `LIVE_SCRIPT_REPLAY` green.
- The touched web unit tests and the TypeScript type check.
- The preview shot (three squads, one house): `compare-screenshots` against
  the old single-entrant preview, then an unprimed `screenshot-critique` as
  the last check before accepting.

## Delegated

Internal structure of the new `occupy`; how preview and application share the
prepared entries; test names.

## Must stay green

`garrison::`, `garrison_sight::`, `buildings::`, `transport::`, `supply::`,
`encounter::`, `menu_reel::`, `entry_reach::`, `cargo test -p contract`.

## What would change this slice

Reachability answering wrongly for buildings behind rows, or the defender
encounters' digests moving: either sends it back to the ledger.
