# 05 — Contacts carry height

**Status:** done (2026-10-10). **Depends on:** 01. **Owns:** D33, L9 (contract and data).

**Evidence.** `air::a_lost_helicopter_leaves_an_airborne_contact`, `air::a_heard_helicopter_report_is_airborne` and `air::area_fire_refuses_an_air_contact` (refused order is `OrderError::AirContact`; a ground report from the same spot still draws HE) went red, then green; `publication::a_contact_publishes_its_height_and_layer` reads both fields back by the layout alone; `web/tests/observation.test.ts` decodes an air contact from the codec-vector record the Rust encoder packs. Before the digest and layout gained the fields, every pinned battle digest (publication streams, menu reel, skirmish and objective records) was unchanged, so ground behaviour is untouched. Then re-recorded with `BLESS_PARITY=1`: `codec-vectors.json` (layout and an air contact in `base`), `combat.json` (digests only, from the first contact) and `contact-lifecycle.json`; Wasm matches (`fogDelivery`, `groupDelivery`, `observation` tests).

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
