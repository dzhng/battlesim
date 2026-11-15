# 10 Stand-in mechanics

**Question:** can a unit the side sees be drawn, selected and targeted from
its physical body alone, before its art exists?

**Depends on:** 00. Independent of the engine slices.

## What happens today

- A unit whose type the committed catalog lacks resolves to no appearance and
  is skipped when drawing
  ([world.rs `resolve`](../../../crates/presentation/src/world.rs),
  [drawn.rs](../../../crates/presentation/src/units/drawn.rs)).
- The frame's picks are built in Rust
  ([picks.rs](../../../crates/presentation/src/presentation/picks.rs)). A
  hull's size comes from the battle's effects section, which covers fielded
  types only
  ([fielded/effects.rs](../../../crates/presentation/src/fielded/effects.rs)).
  So an unfielded vehicle gets a pick of no size, and an unfielded squad gets
  a soldier box per member.
- The unit catalog has every type's hull size without any art.

## Confirm first

- On screen, with one type's art withheld: whether the enemy shows a HUD
  readout, and whether a vehicle and a squad can be clicked.
- That each seen soldier's position is in the frame sample for a type with no
  art (the pick code reads `members`).

## Contract

Every body the side sees whose type's art is not committed is drawn as one
block of its physical size: one for a hull, one per soldier for a squad. It is
selected and targeted exactly as the unit will be. It is gone in the frame its
model commits. Side knowledge is unchanged: a block appears only where a model
would.

## Seam

- **One body box.** Presentation computes a body's box once, from the hull's
  half extents or the soldier's cylinder, and that one value feeds both the
  stand-in's instance scale and its pick. The drawn block and the click box
  cannot differ.
- **Sizes without art.** The battle's body sizes cover every type in the
  session's unit catalog, not only fielded ones.
- **One stand-in appearance.** The battle input names one canonical appearance
  for stand-ins. Its mesh is the existing box primitive
  ([proxies.ts](../../../packages/battle-renderer/src/proxies.ts)), admitted
  by the browser in the first generation, so first contact triggers no
  admission for it. Where the input names none (the native client, which never
  has an unfielded type), the skip stays.
- **Fixed:** a stand-in gets the fog and x-ray treatment of the model it
  replaces. It is never an impostor card.

## Verification

Write first, on stand-in test units (not the roster):

1. A seen unit of an unfielded type yields one stand-in instance per body,
   scaled to its body box.
2. Its picks are the same before and after its art commits, for the same
   pose, and a vehicle's pick has its hull's size.
3. After commit there are no stand-in instances for that type.
4. A unit the side does not see yields nothing.

Then a new lab fixture that withholds one vehicle type's and one squad type's
art through the loader's fetcher: a click selects each and an attack order
targets each.

## What you can run or see

The fixture: grey blocks where the withheld units stand, which you can click
and order attacks on, swapping to models when the art is released.

The fixture's shots are for mechanics only. The look is slice 11.

## Delegated

Where the body-size table lives in the battle input, and the stand-in
appearance's name.

## Stays green

Picks and selection for every fielded unit, readouts, the cursor scenes, fog
scenes, battle digests.

## Feedback that would change this slice

If the confirmation shows readouts or selection already behave differently
from what the code suggests, record it and adjust the contract before coding.
