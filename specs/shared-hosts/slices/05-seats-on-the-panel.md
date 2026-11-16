# 05 · Seats on the panel

**Unlocks:** the player sees how full a held house is.

**Slice variable:** the IN BUILDING row's text. Rings, outlines and other
panel rows are out of scope.

## Contract

- `HostRef::Building` gains `seats: u32`, set in `hold::state` from the
  building's window plan total (from `seats_for_state` while the squad is
  still stepping in).
- The publication carries it on the own-unit row (`publication/layout.rs`,
  the reader, the browser hold view). Carriers publish none.
- Battle digests don't move. Re-record the publication vectors as a named
  layout change: only the new field may differ.

## Seam

`web/src/battle/present/panelRows.ts`: the `in_building` row reads
`IN BUILDING used/total`. `used` is the living soldiers of own squads whose
hold names the same building, with exactly the phase rule the carrier Seats
row (`seatsRow`) uses. Entering and leaving keep their timer rows. Enemy
panels show nothing new. Add a shared-house specimen to
`apps/battle-lab/src/panelSpecimens.ts`.

## Confirm first

- Which hold phases `seatsRow` counts; mirror it.

## What the human can see

The panels lab: a shared house's occupants at normal and compact zoom,
including one stepping in and one stepping out.

## Verification

- Native encoder → reader → browser view parity for the new field.
- The `panelRows` web unit test covers the row's derivation (mechanics, not
  wording).
- Shot of the specimen: `compare-screenshots` against the current IN BUILDING
  specimen, then an unprimed `screenshot-critique` as the last check. Show it
  with `preview-shots`; non-blocking, about five minutes, then decide and
  record.

## Delegated

The publication field's name; the row's exact layout within the panel's
existing grammar.

## Must stay green

All battle digests; `publication::` after the named re-record; the panels
scene.
