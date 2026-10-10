# 05 — Contacts carry height

**Status:** planned. **Depends on:** 01. **Owns:** D33, L9 (contract and data).

## Contract

A lost or heard helicopter leaves a contact at its height and layer. The layer travels from the simulation through WASM publication to the browser's decoded observation. The renderer keeps drawing the old ground glyph at XY until slice 13.

## API seam

- `knowledge::Contact.center: V3` plus `layer`.
- `contract::observation::ApproximateContact.center: [f64; 3]` plus `layer`.
- `publication.rs` `CONTACT_FIELDS` gains `z` and `layer`, and the layout gets a `layers` name table.
- Firing-report contacts take the shooter's z and layer (`battle.rs`).
- The knowledge digest includes both.
- Web: `ContactView.center: Point3` plus `layer` (`web/src/battle/sim/observation.ts`); `contactPick.ts` tests a sphere at height for air contacts; `readouts.tsx` anchors at z.
- Area fire and Attack orders refuse air contacts (D22).

**This changes the digest layout,** and that's a named decision. Re-record parity fixtures with the parity tool's own blessing flag.

## What you can run or see

A sim publication test, and a web decode test under `web/tests`.

## Verification

Tests:
- `a_lost_helicopter_leaves_an_airborne_contact`
- `a_heard_helicopter_report_is_airborne`
- `area_fire_refuses_an_air_contact`
- a web decode round-trip
- parity records re-recorded and matching

## Delegated to the implementer

Field order in the packed group.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Ground-contact behaviour is unchanged. Re-recorded parity is the only digest change, and it's named here.

## Feedback that would change this slice

None expected. This is plumbing.
