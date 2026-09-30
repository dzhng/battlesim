# C01: building aggregates

**Depends on:** C00 and any required prerequisite scale architecture. **Kind:** slice.

## Question
Can one placed building own physical parts and template geometry with one integrity, garrison and terminal state?

## Contract it unlocks
`contract::map` defines `BuildingDefinition` from C00: parts, exposed edges/bays, height/floors/floor heights, entrances, local frame and `template_id`. The map pins `template_catalog_hash` once; appearance identity stays outside simulation data. The sim reads only final placed geometry.

- At world load, parts become props first; an owner prop identifies one building. Keep existing village house PropIds for this aggregate-only cutover.
- `building_of(PropId)` resolves all parts to their owner; structures stores one integrity and garrison admission remains per building.
- Hits wear the owner; destruction replaces every part atomically, including side-known replacement identities.
- Slots use exposed geometry only; band/cap changes wait for C40.
- Prop export gains owner/template reference data, and the browser decoder cuts over in the same commit.
- Existing map fixtures migrate to aggregates in this commit. Their original house geometry/appearance gets a legal descriptor; C13 later bakes reusable placements and C32/C22 resolve them. No per-building GLB or duplicate fact table is introduced.

## API seam
`contract::map`, `sim::world`, `structures`, `garrison`, world export and observation decoder. C04 consumes C00 descriptor types without importing scene-assets implementation.

## What the human can run or see
A part-B hit wears one integrity; a compound collapses atomically; a geometry/appearance overlay shows the descriptor's original house fit.

## Verification
- Hit ownership, atomic replacement and no slots on interior/non-exposed edges.
- Existing village digest/replay parity and lab scenes; C56's new surroundings are a later named geometry change.
- Descriptor geometry is transformed/materialized once with correct units/frame and immutable reference identity.
- Visual evidence: compare ownership/fit overlays against original geometry with compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Owner-ID/export encoding and internal descriptor representation within G0's contract. New geometry owners or rule changes are spec gaps.

## Must stay green
Original aggregate-only outcomes and appearance; final building facts have one owner.

## Feedback that would change this slice
A descriptor/observation seam mismatch must be fixed before mapgen or placement consumers depend on it.
