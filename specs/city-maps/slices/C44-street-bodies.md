# C44: street bodies

**Depends on:** G0. **Kind:** slice.

## Question
Does each street prop behave by its war-film row (Q9)?

## Contract it unlocks
Catalog rows in `fixtures/props/city/*.json` (a prop kind is a catalog rank, `catalog.rs:213`). Kinds: lamp, bench, bollard, bins, hydrant, utility box, scooter, planter, parked car, car wreck, Jersey barrier, bus shelter, scaffold; plus **construction-site kinds** (ideas from a commercial pack whose files we can't use, see below): Heras mesh fence panel, skip bin, pallet stack, site cabin, traffic cone, road barrier. A street tree is the existing tree body. Starting points for the war-film audit: a Heras panel blocks movement but not sight or rounds; a skip is hard cover a tank can shove; a pallet stack is light, destructible cover; cones and barriers are thin and give no cover. **A site cabin that can be garrisoned would be a one-part building (C01), not a street prop;** the audit decides which it is. Run tweak-mechanics **per kind**, and record a one-line war-film note per row. Rows only, no named cases.

## API seam
`fixtures/props/city/`, the catalog.

## What the human can run or see
One movement or fire scenario GIF per kind, for example: cover behind a car; a tank shoves a car; a bollard stops a jeep, not a tank; a glass shelter blocks movement but not rounds or sight.

## Verification
- Native test per row behaviour.
- Village digests unchanged (no village rows change).

## Delegated to the implementer
Per-kind values after the audit (in `choices.md`). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village rows.

## Feedback that would change this slice
A street body with an implausible blocking/destruction effect reopens fixture-owned properties through tweak-mechanics.


## Outcome — physical catalog pass (2026-10-01)

The physical catalog rows are implemented. The per-kind war-film audit and
appearance boundary are recorded in [choices](../choices.md#c44-street-bodies--physical-catalog-pass-2026-10-01).
Native checks exercise every row through world sight, projectile passage,
blocking and cover queries; the shove battle matrix now includes every blocking
street row. The paired village test removes the unused city document and proves
all 200 tick digests unchanged. Both focused body tests and the appearance gate
pass. The ordinary asset-binding checks remain strict; systems-only rows have
explicit unavailable-art status and cannot borrow an existing scenery binding.

C45 models, C46 placement and the per-kind visual/GIF evidence remain open. No
new street prop is placed in the village. The closed site storage cabin has no
garrison; an occupied cabin would use a placed building aggregate.

Closeout review: one catalog owns every property, no production id branches or
new physical mechanics, and generated catalog output is regenerated. Independent
CLI review could not run because the configured model was rejected as unsupported
for this account; integrated second review and whole-repo gates belong to the
orchestrator before merge.
